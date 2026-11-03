use super::Database;
use crate::error::Result;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize, sqlx::FromRow)]
struct PupilPerformance {
    student_id: String,
    lesson_id: String,
    date: String,
    time: String,
    subject_id: String,
    subject_name: String,
    rating: i64,
    observation: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct PupilHomework {
    student_id: String,
    id: String,
    title: String,
    due_date: String,
    subject_id: String,
    subject_name: String,
    handed_in_at: Option<i64>,
    quality: Option<String>,
    note: String,
}

impl Database {
    pub(crate) async fn pupil_history(&self, class: &str) -> Result<Value> {
        let performances = sqlx::query_as::<_, PupilPerformance>(
            "SELECT p.student_id,l.id lesson_id,l.lesson_date date,l.starts_at time,
             s.id subject_id,s.name subject_name,p.rating,p.observation
             FROM lesson_performances p JOIN lessons l ON l.id=p.lesson_id
             JOIN subjects s ON s.id=l.subject_id WHERE p.classroom_id=?
             ORDER BY l.lesson_date,l.starts_at,l.id,p.student_id",
        )
        .bind(class)
        .fetch_all(&self.pool)
        .await?;
        let homework = sqlx::query_as::<_, PupilHomework>(
            "SELECT a.student_id,h.id,h.title,h.due_date,s.id subject_id,s.name subject_name,
             a.handed_in_at,a.quality,a.note
             FROM homework_submissions a JOIN homework h ON h.id=a.homework_id
             JOIN subjects s ON s.id=h.subject_id WHERE a.classroom_id=?
             ORDER BY h.due_date DESC,h.created_at DESC,h.id,a.student_id",
        )
        .bind(class)
        .fetch_all(&self.pool)
        .await?;
        Ok(json!({"performances": performances, "homework": homework}))
    }
}
