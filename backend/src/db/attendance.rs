use super::Database;
use crate::{
    error::{Result, bad},
    models::{Attendance, AttendanceChange},
};
use std::collections::HashSet;

impl Database {
    pub(crate) async fn attendance(&self, class: &str, date: &str) -> Result<Vec<Attendance>> {
        let records = sqlx::query_as::<_, Attendance>(
            "SELECT student_id, status, updated_by, updated_at
             FROM attendance
             WHERE classroom_id=? AND attendance_date=?
             ORDER BY student_id",
        )
        .bind(class)
        .bind(date)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    pub(crate) async fn save_attendance(
        &self,
        class: &str,
        date: &str,
        teacher: &str,
        changes: Vec<AttendanceChange>,
    ) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let students: HashSet<String> =
            sqlx::query_scalar("SELECT id FROM students WHERE classroom_id=?")
                .bind(class)
                .fetch_all(&mut *tx)
                .await?
                .into_iter()
                .collect();
        let mut seen = HashSet::new();
        for ch in &changes {
            if !students.contains(&ch.student_id) || !seen.insert(&ch.student_id) {
                return Err(bad("Unbekanntes oder mehrfach angegebenes Kind."));
            }
            if ch
                .status
                .as_deref()
                .is_some_and(|v| v != "present" && v != "absent")
            {
                return Err(bad("Die Anwesenheit muss present, absent oder null sein."));
            }
        }
        for ch in changes {
            if let Some(status) = ch.status {
                sqlx::query(
                    "INSERT INTO attendance(classroom_id, student_id, attendance_date, status,
                     updated_by)
                     VALUES(?, ?, ?, ?, ?)
                     ON CONFLICT(classroom_id, attendance_date, student_id)
                     DO UPDATE
                     SET status=excluded.status, updated_by=excluded.updated_by,
                     updated_at=unixepoch()",
                )
                .bind(class)
                .bind(ch.student_id)
                .bind(date)
                .bind(status)
                .bind(teacher)
                .execute(&mut *tx)
                .await?;
            } else {
                sqlx::query(
                    "DELETE
                     FROM attendance
                     WHERE classroom_id=? AND attendance_date=? AND student_id=?",
                )
                .bind(class)
                .bind(date)
                .bind(ch.student_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }
}
