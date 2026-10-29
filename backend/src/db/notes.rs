use super::Database;
use crate::{
    error::{Result, missing},
    models::Note,
};

impl Database {
    pub(crate) async fn notes(&self, class: &str) -> Result<Vec<Note>> {
        let notes = sqlx::query_as::<_, Note>(
            "SELECT id, student_id, content, created_by, created_at, updated_by, updated_at, lesson_id
             FROM student_notes
             WHERE classroom_id=?
             ORDER BY created_at DESC, rowid DESC",
        )
        .bind(class)
        .fetch_all(&self.pool)
        .await?;
        Ok(notes)
    }

    pub(crate) async fn create_note(
        &self,
        class: &str,
        student: &str,
        content: &str,
        teacher: &str,
        lesson: Option<&str>,
    ) -> Result<Note> {
        // The insert checks class membership of the child in the same statement.
        let note = sqlx::query_as::<_, Note>(
            "INSERT INTO student_notes(
                 id, classroom_id, student_id, content, created_by, updated_by, lesson_id)
             SELECT ?, ?, ?, ?, ?, ?, ?
             WHERE EXISTS(SELECT 1 FROM students WHERE id=? AND classroom_id=?)
             AND (? IS NULL OR EXISTS(SELECT 1 FROM lessons WHERE id=? AND classroom_id=?))
             RETURNING id, student_id, content, created_by, created_at, updated_by, updated_at, lesson_id",
        )
        .bind(crate::util::id())
        .bind(class)
        .bind(student)
        .bind(content)
        .bind(teacher)
        .bind(teacher)
        .bind(lesson)
        .bind(student)
        .bind(class)
        .bind(lesson)
        .bind(lesson)
        .bind(class)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(missing)?;
        Ok(note)
    }

    pub(crate) async fn edit_note(
        &self,
        class: &str,
        note: &str,
        content: &str,
        teacher: &str,
    ) -> Result<Note> {
        let note = sqlx::query_as::<_, Note>(
            "UPDATE student_notes
             SET content=?, updated_by=?, updated_at=unixepoch()
             WHERE id=? AND classroom_id=?
             RETURNING id, student_id, content, created_by, created_at, updated_by, updated_at, lesson_id",
        )
        .bind(content)
        .bind(teacher)
        .bind(note)
        .bind(class)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(missing)?;
        Ok(note)
    }

    pub(crate) async fn remove_note(&self, class: &str, note: &str) -> Result<()> {
        if sqlx::query(
            "DELETE
             FROM student_notes
             WHERE id=? AND classroom_id=?",
        )
        .bind(note)
        .bind(class)
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
