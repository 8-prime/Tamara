use super::Database;
use crate::{
    error::{Result, bad, missing},
    models::{Assessment, AssessmentChange, Period},
};
use crate::{pdf, validation::valid_date};
use std::collections::HashSet;

impl Database {
    pub(crate) async fn check_period(&self, class: &str, period: &str) -> Result<()> {
        if sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM periods WHERE id=? AND classroom_id=?",
        )
        .bind(period)
        .bind(class)
        .fetch_one(&self.pool)
        .await?
            != 1
        {
            return Err(missing());
        }
        Ok(())
    }

    pub(crate) async fn periods(&self, class: &str) -> Result<Vec<Period>> {
        Ok(sqlx::query_as(
            "SELECT id, name, source_period_id, created_at
             FROM periods
             WHERE classroom_id=?
             ORDER BY created_at DESC, rowid DESC",
        )
        .bind(class)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn assessments(&self, period: &str) -> Result<Vec<Assessment>> {
        Ok(sqlx::query_as(
            "SELECT student_id, criterion_id, value, updated_by, updated_at, achieved_on
             FROM math_assessments
             WHERE period_id=?",
        )
        .bind(period)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn create_period(
        &self,
        id: &str,
        class: &str,
        name: &str,
        source_period_id: Option<&str>,
        teacher: &str,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO periods(id, classroom_id, name, source_period_id, created_by)
             VALUES(?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(class)
        .bind(name)
        .bind(source_period_id)
        .bind(teacher)
        .execute(&mut *tx)
        .await?;
        if let Some(source) = source_period_id {
            sqlx::query(
                "INSERT INTO math_assessments(classroom_id, period_id, student_id,
                 criterion_id, value, updated_by, updated_at, achieved_on)
                 SELECT classroom_id, ?, student_id, criterion_id, value, ?, unixepoch(),
                 achieved_on
                 FROM math_assessments
                 WHERE period_id=? AND classroom_id=?",
            )
            .bind(id)
            .bind(teacher)
            .bind(source)
            .bind(class)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn save_assessments(
        &self,
        class: &str,
        period: &str,
        teacher: &str,
        changes: Vec<AssessmentChange>,
    ) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let students: HashSet<String> =
            sqlx::query_scalar("SELECT id FROM students WHERE classroom_id=?")
                .bind(class)
                .fetch_all(&mut *tx)
                .await?
                .into_iter()
                .collect();
        let criteria: HashSet<String> = pdf::catalog()
            .criteria
            .iter()
            .map(|c| c.id.clone())
            .collect();
        for ch in &changes {
            if !students.contains(&ch.student_id) || !criteria.contains(&ch.criterion_id) {
                return Err(bad("Unbekanntes Kind oder unbekannte Kompetenz."));
            }
            if ch.value.as_deref().is_some_and(|v| v != "yes" && v != "no") {
                return Err(bad("Die Bewertung muss yes, no oder null sein."));
            }
            if let Some(ref date) = ch.achieved_on
                && (ch.value.as_deref() != Some("yes") || !valid_date(date))
            {
                return Err(bad(
                    "Ein Erreicht-Datum muss zur Bewertung Erreicht gehören und ein gültiges Datum im Format JJJJ-MM-TT sein.",
                ));
            }
        }
        for ch in changes {
            if let Some(value) = ch.value {
                // Explicit dates allow earlier observations. Re-saving an achieved
                // proficiency keeps its original date; changing to no clears it.
                sqlx::query(
                    "INSERT INTO math_assessments(classroom_id, period_id, student_id,
                     criterion_id, value, updated_by, achieved_on)
                     VALUES(?, ?, ?, ?, ?, ?, CASE WHEN ?='yes' THEN COALESCE(?, date('now'))
                     ELSE NULL END)
                     ON CONFLICT(period_id, student_id, criterion_id)
                     DO UPDATE
                     SET value=excluded.value, updated_by=excluded.updated_by,
                     updated_at=unixepoch(), achieved_on=CASE WHEN excluded.value='yes' THEN
                     COALESCE(?, math_assessments.achieved_on, excluded.achieved_on) ELSE NULL
                     END",
                )
                .bind(class)
                .bind(period)
                .bind(ch.student_id)
                .bind(ch.criterion_id)
                .bind(&value)
                .bind(teacher)
                .bind(&value)
                .bind(&ch.achieved_on)
                .bind(&ch.achieved_on)
                .execute(&mut *tx)
                .await?;
            } else {
                sqlx::query(
                    "DELETE
                     FROM math_assessments
                     WHERE period_id=? AND student_id=? AND criterion_id=?",
                )
                .bind(period)
                .bind(ch.student_id)
                .bind(ch.criterion_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }
}
