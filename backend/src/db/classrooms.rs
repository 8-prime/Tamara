use super::Database;
use super::homework;
use crate::{
    error::{Result, bad, missing},
    models::{Classroom, Collaborator, Student},
};

impl Database {
    pub(crate) async fn classroom_role(&self, class: &str, teacher: &str) -> Result<String> {
        sqlx::query_scalar::<_, String>(
            "SELECT role FROM classroom_teachers WHERE classroom_id=? AND teacher_id=?",
        )
        .bind(class)
        .bind(teacher)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(missing)
    }

    pub(crate) async fn classes(&self, teacher: &str) -> Result<Vec<Classroom>> {
        Ok(sqlx::query_as(
            "SELECT c.id, c.name, c.school_year, m.role, (SELECT COUNT(*)
             FROM students st
             WHERE st.classroom_id=c.id) student_count
             FROM classrooms c
             JOIN classroom_teachers m ON m.classroom_id=c.id
             WHERE m.teacher_id=?
             ORDER BY c.created_at, c.name",
        )
        .bind(teacher)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn create_class(
        &self,
        id: &str,
        name: &str,
        year: &str,
        teacher: &str,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO classrooms(id, name, school_year)
             VALUES(?, ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(year)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO classroom_teachers
             VALUES(?, ?, 'owner')",
        )
        .bind(id)
        .bind(teacher)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn students(&self, class: &str) -> Result<Vec<Student>> {
        Ok(sqlx::query_as(
            "SELECT id,name FROM students WHERE classroom_id=? ORDER BY name COLLATE NOCASE,id",
        )
        .bind(class)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn collaborators(&self, class: &str) -> Result<Vec<Collaborator>> {
        Ok(sqlx::query_as(
            "SELECT t.id, t.name, t.email, m.role
             FROM classroom_teachers m
             JOIN teachers t ON m.teacher_id=t.id
             WHERE m.classroom_id=?
             ORDER BY m.role DESC, t.name",
        )
        .bind(class)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn add_students(&self, class: &str, names: Vec<String>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for n in names {
            sqlx::query(
                "INSERT INTO students(id, classroom_id, name)
                 VALUES(?, ?, ?)",
            )
            .bind(crate::util::id())
            .bind(class)
            .bind(n)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn rename_student(
        &self,
        class: &str,
        student: &str,
        name: &str,
    ) -> Result<()> {
        if sqlx::query(
            "UPDATE students
             SET name=?
             WHERE id=? AND classroom_id=?",
        )
        .bind(name)
        .bind(student)
        .bind(class)
        .execute(&self.pool)
        .await?
        .rows_affected()
            != 1
        {
            return Err(missing());
        }
        Ok(())
    }

    pub(crate) async fn remove_student(&self, class: &str, student: &str) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        if sqlx::query(
            "DELETE
             FROM students
             WHERE id=? AND classroom_id=?",
        )
        .bind(student)
        .bind(class)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            != 1
        {
            return Err(missing());
        }
        homework::reconcile(&mut tx, class).await?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn share_class(&self, class: &str, email: &str) -> Result<()> {
        let teacher=sqlx::query_scalar::<_,String>(
            "SELECT id
             FROM teachers
             WHERE email=?",
        )
            .bind(email)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(||bad("Für diese E-Mail gibt es noch kein Konto. Die Lehrkraft muss sich zuerst registrieren."))?;
        sqlx::query(
            "INSERT INTO classroom_teachers(classroom_id, teacher_id, role)
             VALUES(?, ?, 'editor')
             ON CONFLICT DO NOTHING",
        )
        .bind(class)
        .bind(teacher)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn unshare_class(&self, class: &str, teacher: &str) -> Result<()> {
        sqlx::query(
        "DELETE FROM classroom_teachers WHERE classroom_id=? AND teacher_id=? AND role='editor'",
    )
    .bind(class)
    .bind(teacher)
    .execute(&self.pool)
    .await?;
        Ok(())
    }
}
