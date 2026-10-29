use super::Database;
use crate::{
    error::{Error, ErrorKind, Result, bad, missing},
    models::{Homework, HomeworkInput, Subject, Submission, SubmissionChange},
};

use std::collections::HashSet;

const SELECT_HOMEWORK: &str = "
    SELECT h.id, h.subject_id, s.name subject_name, h.title, h.description, h.due_date,
        h.status, h.closed_at, h.created_at,
        (SELECT COUNT(*) FROM homework_submissions a
         WHERE a.homework_id=h.id) assigned_count,
        (SELECT COUNT(*) FROM homework_submissions a
         WHERE a.homework_id=h.id AND a.handed_in_at IS NOT NULL) handed_in_count,
        (SELECT name FROM homework_worksheets w
         WHERE w.homework_id=h.id) worksheet_name
    FROM homework h JOIN subjects s ON s.id=h.subject_id";

impl Database {
    pub(crate) async fn subjects(&self, class: &str) -> Result<Vec<Subject>> {
        Ok(sqlx::query_as(
            "SELECT id,name FROM subjects WHERE classroom_id=? ORDER BY name COLLATE NOCASE,id",
        )
        .bind(class)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn create_subject(&self, id: &str, class: &str, name: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO subjects(id, classroom_id, name)
             VALUES(?, ?, ?)",
        )
        .bind(id)
        .bind(class)
        .bind(name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn rename_subject(
        &self,
        class: &str,
        subject: &str,
        name: &str,
    ) -> Result<()> {
        if sqlx::query(
            "UPDATE subjects
             SET name=?
             WHERE id=? AND classroom_id=?",
        )
        .bind(name)
        .bind(subject)
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

    pub(crate) async fn remove_subject(&self, class: &str, subject: &str) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let used: i64 = sqlx::query_scalar(
            "SELECT (SELECT COUNT(*) FROM homework WHERE subject_id=? AND classroom_id=?)
             + (SELECT COUNT(*) FROM lessons WHERE subject_id=? AND classroom_id=?)
             + (SELECT COUNT(*) FROM timetable_entries WHERE subject_id=? AND classroom_id=?)",
        )
        .bind(subject)
        .bind(class)
        .bind(subject)
        .bind(class)
        .bind(subject)
        .bind(class)
        .fetch_one(&mut *tx)
        .await?;
        if used > 0 {
            return Err(Error(
                ErrorKind::Conflict,
                "Dieses Fach wird für Hausaufgaben oder Unterricht verwendet und kann nicht entfernt werden."
                    .into(),
            ));
        }
        if sqlx::query(
            "DELETE
             FROM subjects
             WHERE id=? AND classroom_id=?",
        )
        .bind(subject)
        .bind(class)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            != 1
        {
            return Err(missing());
        }
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn homework(&self, class: &str) -> Result<Vec<Homework>> {
        Ok(sqlx::query_as(&format!(
            "{SELECT_HOMEWORK} WHERE h.classroom_id=? ORDER BY h.due_date,h.created_at,h.id"
        ))
        .bind(class)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn homework_detail(
        &self,
        class: &str,
        homework: &str,
    ) -> Result<(Homework, Vec<Submission>)> {
        let mut tx = self.pool.begin().await?;
        let entry: Homework = sqlx::query_as(&format!(
            "{SELECT_HOMEWORK} WHERE h.classroom_id=? AND h.id=?"
        ))
        .bind(class)
        .bind(homework)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(missing)?;
        let pupils = sqlx::query_as::<_, Submission>(
            "SELECT a.student_id, s.name, a.handed_in_at
             FROM homework_submissions a
             JOIN students s ON s.id=a.student_id
             WHERE a.homework_id=? AND a.classroom_id=?
             ORDER BY s.name COLLATE NOCASE, s.id",
        )
        .bind(homework)
        .bind(class)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok((entry, pupils))
    }

    pub(crate) async fn create_homework(
        &self,
        class: &str,
        teacher: &str,
        input: &HomeworkInput,
    ) -> Result<String> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        check_subject(&mut tx, class, &input.subject_id).await?;
        let id = crate::util::id();
        sqlx::query(
            "INSERT INTO homework(id, classroom_id, subject_id, title, description, due_date,
             created_by)
             VALUES(?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(class)
        .bind(&input.subject_id)
        .bind(&input.title)
        .bind(&input.description)
        .bind(&input.due_date)
        .bind(teacher)
        .execute(&mut *tx)
        .await?;
        let assigned = sqlx::query(
            "INSERT INTO homework_submissions(homework_id, classroom_id, student_id) SELECT ?,
             classroom_id, id
             FROM students
             WHERE classroom_id=?",
        )
        .bind(&id)
        .bind(class)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if assigned == 0 {
            return Err(bad("Bitte zuerst Kinder zur Klasse hinzufügen."));
        }
        tx.commit().await?;
        Ok(id)
    }

    pub(crate) async fn edit_homework(
        &self,
        class: &str,
        homework: &str,
        input: &HomeworkInput,
    ) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        check_subject(&mut tx, class, &input.subject_id).await?;
        if sqlx::query(
            "UPDATE homework
             SET subject_id=?, title=?, description=?, due_date=?
             WHERE id=? AND classroom_id=?",
        )
        .bind(&input.subject_id)
        .bind(&input.title)
        .bind(&input.description)
        .bind(&input.due_date)
        .bind(homework)
        .bind(class)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            != 1
        {
            return Err(missing());
        }
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn save_submissions(
        &self,
        class: &str,
        homework: &str,
        teacher: &str,
        changes: Vec<SubmissionChange>,
    ) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        check_homework(&mut tx, class, homework).await?;
        let mut seen = HashSet::new();
        for ch in changes {
            if !seen.insert(ch.student_id.clone()) {
                return Err(bad("Ein Kind wurde mehrfach angegeben."));
            }
            if sqlx::query(
                "UPDATE homework_submissions
             SET handed_in_at=CASE WHEN ? THEN COALESCE(handed_in_at, unixepoch()) ELSE NULL END,
             updated_by=?
             WHERE homework_id=? AND classroom_id=? AND student_id=?",
            )
            .bind(ch.handed_in)
            .bind(teacher)
            .bind(homework)
            .bind(class)
            .bind(ch.student_id)
            .execute(&mut *tx)
            .await?
            .rows_affected()
                != 1
            {
                return Err(bad("Dieses Kind gehört nicht zu dieser Hausaufgabe."));
            }
        }
        reconcile(&mut tx, class).await?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn set_homework_closed(
        &self,
        class: &str,
        homework: &str,
        closed: bool,
    ) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        check_homework(&mut tx, class, homework).await?;
        if !closed {
            let remaining: i64 = sqlx::query_scalar(
                "SELECT COUNT(*)
                 FROM homework_submissions
                 WHERE homework_id=? AND handed_in_at IS NULL",
            )
            .bind(homework)
            .fetch_one(&mut *tx)
            .await?;
            if remaining == 0 {
                return Err(bad(
                    "Alle Kinder haben abgegeben. Korrigiere zuerst eine Abgabe, um die Hausaufgabe wieder zu öffnen.",
                ));
            }
        }
        sqlx::query(
            "UPDATE homework
             SET status=?, closed_at=CASE WHEN ? THEN unixepoch() ELSE NULL END
             WHERE id=? AND classroom_id=?",
        )
        .bind(if closed { "manually_closed" } else { "open" })
        .bind(closed)
        .bind(homework)
        .bind(class)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn upload_worksheet(
        &self,
        class: &str,
        homework: &str,
        name: &str,
        bytes: &[u8],
    ) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        check_homework(&mut tx, class, homework).await?;
        sqlx::query(
            "INSERT INTO homework_worksheets(homework_id, name, content)
             VALUES(?, ?, ?)
             ON CONFLICT(homework_id)
             DO UPDATE
             SET name=excluded.name, content=excluded.content",
        )
        .bind(homework)
        .bind(name)
        .bind(bytes)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn worksheet(&self, class: &str, homework: &str) -> Result<(String, Vec<u8>)> {
        let (name, content): (String, Vec<u8>) = sqlx::query_as(
            "SELECT w.name, w.content
             FROM homework_worksheets w
             JOIN homework h ON h.id=w.homework_id
             WHERE h.id=? AND h.classroom_id=?",
        )
        .bind(homework)
        .bind(class)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(missing)?;
        Ok((name, content))
    }

    pub(crate) async fn remove_worksheet(&self, class: &str, homework: &str) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        check_homework(&mut tx, class, homework).await?;
        sqlx::query(
            "DELETE
             FROM homework_worksheets
             WHERE homework_id=?",
        )
        .bind(homework)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}

async fn check_subject(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    class: &str,
    subject: &str,
) -> Result<()> {
    if sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM subjects
         WHERE id=? AND classroom_id=?",
    )
    .bind(subject)
    .bind(class)
    .fetch_one(&mut **tx)
    .await?
        != 1
    {
        return Err(bad("Bitte ein Fach dieser Klasse auswählen."));
    }
    Ok(())
}

async fn check_homework(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    class: &str,
    homework: &str,
) -> Result<()> {
    if sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM homework
         WHERE id=? AND classroom_id=?",
    )
    .bind(homework)
    .bind(class)
    .fetch_one(&mut **tx)
    .await?
        != 1
    {
        return Err(missing());
    }
    Ok(())
}

pub(super) async fn reconcile(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    class: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE homework
         SET status=CASE WHEN EXISTS(SELECT 1
         FROM homework_submissions a
         WHERE a.homework_id=homework.id AND a.handed_in_at IS NULL) OR NOT EXISTS(SELECT 1
         FROM homework_submissions a
         WHERE a.homework_id=homework.id) THEN 'open' ELSE 'auto_closed' END, closed_at=CASE
         WHEN EXISTS(SELECT 1
         FROM homework_submissions a
         WHERE a.homework_id=homework.id AND a.handed_in_at IS NULL) OR NOT EXISTS(SELECT 1
         FROM homework_submissions a
         WHERE a.homework_id=homework.id) THEN NULL ELSE COALESCE(closed_at, unixepoch()) END
         WHERE classroom_id=? AND status!='manually_closed'",
    )
    .bind(class)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
