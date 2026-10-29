use super::Database;
use crate::{
    error::{Result, bad, missing},
    models::Student,
};

use std::collections::HashMap;
pub(crate) struct PdfExport {
    pub(crate) name: String,
    pub(crate) year: String,
    pub(crate) period_name: String,
    pub(crate) yes: Vec<String>,
}
pub(crate) struct ClassExport {
    pub(crate) students: Vec<Student>,
    pub(crate) year: String,
    pub(crate) period_name: String,
    pub(crate) marks: HashMap<String, Vec<String>>,
}

impl Database {
    pub(crate) async fn pdf_export(
        &self,
        class: &str,
        period: &str,
        student: &str,
    ) -> Result<PdfExport> {
        let name = sqlx::query_scalar::<_, String>(
            "SELECT name FROM students WHERE id=? AND classroom_id=?",
        )
        .bind(student)
        .bind(class)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(missing)?;
        let year = sqlx::query_scalar::<_, String>(
            "SELECT school_year
             FROM classrooms
             WHERE id=?",
        )
        .bind(class)
        .fetch_one(&self.pool)
        .await?;
        let period_name = sqlx::query_scalar::<_, String>(
            "SELECT name
             FROM periods
             WHERE id=?",
        )
        .bind(period)
        .fetch_one(&self.pool)
        .await?;
        let yes = sqlx::query_scalar::<_, String>(
            "SELECT criterion_id
             FROM math_assessments
             WHERE period_id=? AND student_id=? AND value='yes'",
        )
        .bind(period)
        .bind(student)
        .fetch_all(&self.pool)
        .await?;
        Ok(PdfExport {
            name,
            year,
            period_name,
            yes,
        })
    }

    pub(crate) async fn class_export(&self, class: &str, period: &str) -> Result<ClassExport> {
        let mut tx = self.pool.begin().await?;
        let (year, period_name) = sqlx::query_as::<_, (String, String)>(
            "SELECT c.school_year, p.name
             FROM classrooms c
             JOIN periods p ON p.classroom_id=c.id
             WHERE c.id=? AND p.id=?",
        )
        .bind(class)
        .bind(period)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(missing)?;
        let students = sqlx::query_as::<_, Student>(
            "SELECT id,name FROM students WHERE classroom_id=? ORDER BY name COLLATE NOCASE,id",
        )
        .bind(class)
        .fetch_all(&mut *tx)
        .await?;
        if students.is_empty() {
            return Err(bad("Diese Klasse hat noch keine Kinder für den Export."));
        }
        let assessments = sqlx::query_as::<_, (String, String)>(
            "SELECT a.student_id, a.criterion_id
             FROM math_assessments a
             JOIN students st ON st.id=a.student_id
             WHERE a.period_id=? AND st.classroom_id=? AND a.value='yes'",
        )
        .bind(period)
        .bind(class)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        let mut marks: HashMap<String, Vec<String>> = HashMap::new();
        for (student, criterion) in assessments {
            marks.entry(student).or_default().push(criterion);
        }
        Ok(ClassExport {
            students,
            year,
            period_name,
            marks,
        })
    }
}
