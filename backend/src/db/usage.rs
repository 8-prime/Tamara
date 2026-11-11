//! School-wide usage counters: daily totals per action, without identifiers.
use super::Database;
use crate::error::Result;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

/// Every counted action, in display order. The database layer counts each one
/// in the same transaction as the write it describes; exports are counted by
/// the API because they write nothing else.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Metric {
    TeacherActive,
    SessionStarted,
    TeacherRegistered,
    ClassroomCreated,
    PupilAdded,
    LessonCreated,
    AttendanceTaken,
    PerformanceRecorded,
    PerformanceCorrected,
    NoteCreated,
    NoteEdited,
    HomeworkCreated,
    HomeworkHandedIn,
    HomeworkRated,
    WorksheetUploaded,
    TaskCreated,
    TaskCompleted,
    PeriodCreated,
    AssessmentRecorded,
    PdfExported,
    ZipExported,
}

impl Metric {
    pub(crate) const ALL: [Metric; 21] = [
        Metric::TeacherActive,
        Metric::SessionStarted,
        Metric::TeacherRegistered,
        Metric::ClassroomCreated,
        Metric::PupilAdded,
        Metric::LessonCreated,
        Metric::AttendanceTaken,
        Metric::PerformanceRecorded,
        Metric::PerformanceCorrected,
        Metric::NoteCreated,
        Metric::NoteEdited,
        Metric::HomeworkCreated,
        Metric::HomeworkHandedIn,
        Metric::HomeworkRated,
        Metric::WorksheetUploaded,
        Metric::TaskCreated,
        Metric::TaskCompleted,
        Metric::PeriodCreated,
        Metric::AssessmentRecorded,
        Metric::PdfExported,
        Metric::ZipExported,
    ];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Metric::TeacherActive => "teacher_active",
            Metric::SessionStarted => "session_started",
            Metric::TeacherRegistered => "teacher_registered",
            Metric::ClassroomCreated => "classroom_created",
            Metric::PupilAdded => "pupil_added",
            Metric::LessonCreated => "lesson_created",
            Metric::AttendanceTaken => "attendance_taken",
            Metric::PerformanceRecorded => "performance_recorded",
            Metric::PerformanceCorrected => "performance_corrected",
            Metric::NoteCreated => "note_created",
            Metric::NoteEdited => "note_edited",
            Metric::HomeworkCreated => "homework_created",
            Metric::HomeworkHandedIn => "homework_handed_in",
            Metric::HomeworkRated => "homework_rated",
            Metric::WorksheetUploaded => "worksheet_uploaded",
            Metric::TaskCreated => "task_created",
            Metric::TaskCompleted => "task_completed",
            Metric::PeriodCreated => "period_created",
            Metric::AssessmentRecorded => "assessment_recorded",
            Metric::PdfExported => "pdf_exported",
            Metric::ZipExported => "zip_exported",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Bucket {
    Day,
    Week,
    Month,
}

#[derive(Serialize)]
pub(crate) struct UsagePeriod {
    /// First day of the bucket: the day, the week's Monday or the month's first.
    pub(crate) start: String,
    /// Counted days of this bucket within the requested range.
    pub(crate) days: u32,
    pub(crate) counts: BTreeMap<&'static str, i64>,
}

#[derive(Serialize)]
pub(crate) struct Usage {
    pub(crate) counting_since: String,
    pub(crate) from: String,
    pub(crate) until: String,
    pub(crate) days: u32,
    pub(crate) totals: BTreeMap<&'static str, i64>,
    /// The highest number of active teachers on a single day of the range.
    pub(crate) peak_active_teachers: i64,
    pub(crate) series: Vec<UsagePeriod>,
}

/// Adds `n` to today's (UTC) counter, within the caller's transaction so the
/// count commits or rolls back together with the write it describes.
pub(super) async fn count<'e, E: sqlx::SqliteExecutor<'e>>(
    executor: E,
    metric: Metric,
    n: i64,
) -> Result<()> {
    if n > 0 {
        sqlx::query(
            "INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), ?, ?)
             ON CONFLICT(day, metric) DO UPDATE SET count=count+excluded.count",
        )
        .bind(metric.key())
        .bind(n)
        .execute(executor)
        .await?;
    }
    Ok(())
}

impl Database {
    pub(crate) async fn count_usage(&self, metric: Metric) -> Result<()> {
        count(&self.pool, metric, 1).await
    }

    /// Counts the teacher's first activity of the UTC day. The condition keeps
    /// concurrent first requests from counting twice.
    pub(crate) async fn mark_active(&self, teacher: &str) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let first = sqlx::query(
            "UPDATE teachers SET last_active_day=date('now')
             WHERE id=? AND last_active_day IS NOT date('now')",
        )
        .bind(teacher)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        count(&mut *tx, Metric::TeacherActive, first as i64).await?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn counting_since(&self) -> Result<String> {
        Ok(sqlx::query_scalar(
            "SELECT COALESCE(
               (SELECT MIN(day) FROM usage_counters WHERE metric='counting_started'),
               date('now'))",
        )
        .fetch_one(&self.pool)
        .await?)
    }

    /// Sums the daily counters of `from..=until` into day, week or month buckets.
    /// Both dates must be valid and `from <= until`.
    pub(crate) async fn usage(&self, from: &str, until: &str, bucket: Bucket) -> Result<Usage> {
        let counting_since = self.counting_since().await?;
        let from = from.max(counting_since.as_str()).to_owned();
        let mut tx = self.pool.begin().await?;
        let days: Vec<(String, String)> = sqlx::query_as(
            "WITH RECURSIVE days(day) AS (
               SELECT ? WHERE ? <= ?
               UNION ALL SELECT date(day, '+1 day') FROM days WHERE day < ?)
             SELECT day, CASE ?
               WHEN 'week' THEN date(day, '-6 days', 'weekday 1')
               WHEN 'month' THEN strftime('%Y-%m-01', day)
               ELSE day END
             FROM days",
        )
        .bind(&from)
        .bind(&from)
        .bind(until)
        .bind(until)
        .bind(match bucket {
            Bucket::Day => "day",
            Bucket::Week => "week",
            Bucket::Month => "month",
        })
        .fetch_all(&mut *tx)
        .await?;
        let rows: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT day, metric, count FROM usage_counters WHERE day BETWEEN ? AND ?",
        )
        .bind(&from)
        .bind(until)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;

        let mut daily: HashMap<(String, String), i64> = HashMap::new();
        for (day, metric, count) in rows {
            daily.insert((day, metric), count);
        }
        let zero = || {
            Metric::ALL
                .iter()
                .map(|m| (m.key(), 0))
                .collect::<BTreeMap<_, _>>()
        };
        let mut totals = zero();
        let mut peak_active_teachers = 0;
        let mut series: Vec<UsagePeriod> = Vec::new();
        for (day, start) in &days {
            if series.last().is_none_or(|p| &p.start != start) {
                series.push(UsagePeriod {
                    start: start.clone(),
                    days: 0,
                    counts: zero(),
                });
            }
            let period = series.last_mut().expect("pushed above");
            period.days += 1;
            for metric in Metric::ALL {
                let key = metric.key();
                let n = daily
                    .get(&(day.clone(), key.to_owned()))
                    .copied()
                    .unwrap_or(0);
                *period.counts.get_mut(key).expect("all metrics") += n;
                *totals.get_mut(key).expect("all metrics") += n;
                if metric == Metric::TeacherActive {
                    peak_active_teachers = peak_active_teachers.max(n);
                }
            }
        }
        Ok(Usage {
            counting_since,
            days: days.len() as u32,
            until: until.to_owned(),
            from,
            totals,
            peak_active_teachers,
            series,
        })
    }
}
