use super::Database;
use crate::error::{Result, internal, missing};
use serde_json::{Value, json};
use sqlx::Row;

impl Database {
    pub(crate) async fn parent_pupils(&self, teacher: &str) -> Result<Vec<Value>> {
        let rows = sqlx::query("SELECT s.id,s.name,s.classroom_id,c.name class_name FROM students s JOIN classrooms c ON c.id=s.classroom_id JOIN classroom_teachers m ON m.classroom_id=c.id WHERE m.teacher_id=? ORDER BY s.name COLLATE NOCASE,s.id").bind(teacher).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(|r| json!({"id":r.get::<String,_>("id"),"name":r.get::<String,_>("name"),"classroom_id":r.get::<String,_>("classroom_id"),"class_name":r.get::<String,_>("class_name")})).collect())
    }
    pub(crate) async fn parent_conferences(&self, teacher: &str) -> Result<Vec<Value>> {
        let rows = sqlx::query("SELECT p.*,s.name student_name,s.classroom_id,c.name class_name FROM parent_conferences p JOIN students s ON s.id=p.student_id JOIN classrooms c ON c.id=s.classroom_id JOIN classroom_teachers m ON m.classroom_id=c.id AND m.teacher_id=p.teacher_id WHERE p.teacher_id=? ORDER BY p.date DESC,p.rowid DESC").bind(teacher).fetch_all(&self.pool).await?;
        rows.iter().map(|r| Ok(json!({
            "id":r.get::<String,_>("id"),"student_id":r.get::<String,_>("student_id"),"student_name":r.get::<String,_>("student_name"),"classroom_id":r.get::<String,_>("classroom_id"),"class_name":r.get::<String,_>("class_name"),"date":r.get::<String,_>("date"),"content":r.get::<String,_>("content"),"attendees":serde_json::from_str::<Value>(r.get::<&str,_>("attendees")).map_err(internal)?,"review_snapshot":r.get::<Option<String>,_>("review_snapshot").map(|s|serde_json::from_str::<Value>(&s)).transpose().map_err(internal)?
        }))).collect()
    }
    // Every detail/document operation rechecks both ownership and current roster access.
    pub(crate) async fn parent_conference_pupil(
        &self,
        teacher: &str,
        id: &str,
    ) -> Result<(String, String)> {
        sqlx::query_as("SELECT p.student_id,s.classroom_id FROM parent_conferences p JOIN students s ON s.id=p.student_id JOIN classroom_teachers m ON m.classroom_id=s.classroom_id AND m.teacher_id=p.teacher_id WHERE p.id=? AND p.teacher_id=?").bind(id).bind(teacher).fetch_optional(&self.pool).await?.ok_or_else(missing)
    }
    pub(crate) async fn save_parent_conference(
        &self,
        teacher: &str,
        id: &str,
        student: &str,
        date: &str,
        content: &str,
        attendees: &[String],
    ) -> Result<()> {
        let result = sqlx::query("INSERT INTO parent_conferences(id,teacher_id,student_id,date,content,attendees) SELECT ?,?,?,?,?,? WHERE EXISTS(SELECT 1 FROM students s JOIN classroom_teachers m ON m.classroom_id=s.classroom_id WHERE s.id=? AND m.teacher_id=?) ON CONFLICT(id) DO UPDATE SET date=excluded.date,content=excluded.content,attendees=excluded.attendees,updated_at=unixepoch() WHERE parent_conferences.teacher_id=excluded.teacher_id AND parent_conferences.student_id=excluded.student_id")
            .bind(id).bind(teacher).bind(student).bind(date).bind(content).bind(serde_json::to_string(attendees).map_err(internal)?).bind(student).bind(teacher).execute(&self.pool).await?;
        if result.rows_affected() == 0 {
            return Err(missing());
        }
        Ok(())
    }
    pub(crate) async fn remove_parent_conference(&self, teacher: &str, id: &str) -> Result<()> {
        self.parent_conference_pupil(teacher, id).await?;
        sqlx::query("DELETE FROM parent_conferences WHERE id=? AND teacher_id=?")
            .bind(id)
            .bind(teacher)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub(crate) async fn capture_parent_review(
        &self,
        teacher: &str,
        id: &str,
        period: &str,
    ) -> Result<()> {
        let (student, class) = self.parent_conference_pupil(teacher, id).await?;
        let mut tx = self.pool.begin().await?;
        let (name,year): (String,String)=sqlx::query_as("SELECT p.name,c.school_year FROM periods p JOIN classrooms c ON c.id=p.classroom_id WHERE p.id=? AND p.classroom_id=?").bind(period).bind(&class).fetch_optional(&mut *tx).await?.ok_or_else(missing)?;
        let rows=sqlx::query("SELECT c.id,c.label,c.domain,c.section,c.level,a.value,a.achieved_on FROM math_criteria c LEFT JOIN math_assessments a ON a.criterion_id=c.id AND a.student_id=? AND a.period_id=? ORDER BY c.page,c.rowid").bind(student).bind(period).fetch_all(&mut *tx).await?;
        let assessments: Vec<Value>=rows.iter().map(|r|json!({"criterion_id":r.get::<String,_>("id"),"label":r.get::<String,_>("label"),"domain":r.get::<String,_>("domain"),"section":r.get::<String,_>("section"),"level":r.get::<String,_>("level"),"value":r.get::<Option<String>,_>("value"),"achieved_on":r.get::<Option<String>,_>("achieved_on")})).collect();
        let snapshot = json!({"period_name":name,"school_year":year,"captured_at":crate::util::now(),"assessments":assessments});
        // A snapshot is immutable: remove it explicitly before capturing another.
        let n=sqlx::query("UPDATE parent_conferences SET review_snapshot=?,updated_at=unixepoch() WHERE id=? AND teacher_id=? AND review_snapshot IS NULL").bind(snapshot.to_string()).bind(id).bind(teacher).execute(&mut *tx).await?.rows_affected();
        if n == 0 {
            return Err(crate::error::bad(
                "Bitte den vorhandenen Stand zuerst entfernen.",
            ));
        }
        tx.commit().await?;
        Ok(())
    }
    pub(crate) async fn remove_parent_review(&self, teacher: &str, id: &str) -> Result<()> {
        self.parent_conference_pupil(teacher, id).await?;
        sqlx::query(
            "UPDATE parent_conferences SET review_snapshot=NULL WHERE id=? AND teacher_id=?",
        )
        .bind(id)
        .bind(teacher)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub(crate) async fn parent_documents(&self, teacher: &str, id: &str) -> Result<Vec<Value>> {
        self.parent_conference_pupil(teacher, id).await?;
        let rows=sqlx::query("SELECT id,name,length(content) size FROM parent_conference_documents WHERE conference_id=? ORDER BY created_at,rowid").bind(id).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(|r|json!({"id":r.get::<String,_>("id"),"name":r.get::<String,_>("name"),"size":r.get::<i64,_>("size")})).collect())
    }
    pub(crate) async fn add_parent_document(
        &self,
        teacher: &str,
        id: &str,
        name: &str,
        bytes: &[u8],
    ) -> Result<()> {
        self.parent_conference_pupil(teacher, id).await?;
        sqlx::query("INSERT INTO parent_conference_documents(id,conference_id,name,content) VALUES(?,?,?,?)").bind(crate::util::id()).bind(id).bind(name).bind(bytes).execute(&self.pool).await?;
        Ok(())
    }
    pub(crate) async fn parent_document(
        &self,
        teacher: &str,
        id: &str,
        document: &str,
    ) -> Result<(String, Vec<u8>)> {
        self.parent_conference_pupil(teacher, id).await?;
        sqlx::query_as(
            "SELECT name,content FROM parent_conference_documents WHERE id=? AND conference_id=?",
        )
        .bind(document)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(missing)
    }
    pub(crate) async fn remove_parent_document(
        &self,
        teacher: &str,
        id: &str,
        document: &str,
    ) -> Result<()> {
        self.parent_conference_pupil(teacher, id).await?;
        if sqlx::query("DELETE FROM parent_conference_documents WHERE id=? AND conference_id=?")
            .bind(document)
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            == 0
        {
            return Err(missing());
        }
        Ok(())
    }
}
