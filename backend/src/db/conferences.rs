use super::Database;
use crate::{
    error::{Result, missing},
    models::Conference,
};
impl Database {
    pub(crate) async fn conferences(&self, teacher: &str) -> Result<Vec<Conference>> {
        Ok(sqlx::query_as("SELECT id, classroom_id, date, title, content, updated_at FROM conferences WHERE teacher_id=? ORDER BY date DESC, rowid DESC").bind(teacher).fetch_all(&self.pool).await?)
    }
    pub(crate) async fn save_conference(
        &self,
        teacher: &str,
        id: &str,
        input: &Conference,
    ) -> Result<Conference> {
        sqlx::query_as("INSERT INTO conferences(id,teacher_id,classroom_id,date,title,content) VALUES(?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET classroom_id=excluded.classroom_id,date=excluded.date,title=excluded.title,content=excluded.content,updated_at=unixepoch() WHERE conferences.teacher_id=excluded.teacher_id RETURNING id,classroom_id,date,title,content,updated_at")
        .bind(id).bind(teacher).bind(&input.classroom_id).bind(&input.date).bind(&input.title).bind(&input.content).fetch_optional(&self.pool).await?.ok_or_else(missing)
    }
    pub(crate) async fn remove_conference(&self, teacher: &str, id: &str) -> Result<()> {
        if sqlx::query("DELETE FROM conferences WHERE id=? AND teacher_id=?")
            .bind(id)
            .bind(teacher)
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
