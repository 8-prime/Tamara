//! School-wide usage counters: daily totals per action, without identifiers.
use super::Database;
use crate::error::Result;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

/// Every reported metric, in display order. Triggers in the migration count
/// most of them; the application counts assessments and exports.
pub(crate) const METRICS: &[&str] = &[
    "teacher_active",
    "session_started",
    "teacher_registered",
    "classroom_created",
    "pupil_added",
    "lesson_created",
    "attendance_taken",
    "performance_recorded",
    "performance_corrected",
    "note_created",
    "note_edited",
    "homework_created",
    "homework_handed_in",
    "homework_rated",
    "worksheet_uploaded",
    "task_created",
    "task_completed",
    "period_created",
    "assessment_recorded",
    "pdf_exported",
    "zip_exported",
];

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

/// Adds `n` to today's counter within the caller's transaction or connection.
pub(super) async fn count<'e, E: sqlx::SqliteExecutor<'e>>(
    executor: E,
    metric: &str,
    n: i64,
) -> Result<()> {
    debug_assert!(METRICS.contains(&metric));
    if n > 0 {
        sqlx::query(
            "INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), ?, ?)
             ON CONFLICT(day, metric) DO UPDATE SET count=count+excluded.count",
        )
        .bind(metric)
        .bind(n)
        .execute(executor)
        .await?;
    }
    Ok(())
}

impl Database {
    pub(crate) async fn count_usage(&self, metric: &str) -> Result<()> {
        count(&self.pool, metric, 1).await
    }

    /// Records the teacher's first activity of the UTC day; a trigger counts it.
    /// The condition keeps concurrent first requests from counting twice.
    pub(crate) async fn mark_active(&self, teacher: &str) -> Result<()> {
        sqlx::query(
            "UPDATE teachers SET last_active_day=date('now')
             WHERE id=? AND last_active_day IS NOT date('now')",
        )
        .bind(teacher)
        .execute(&self.pool)
        .await?;
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
        let zero = || METRICS.iter().map(|&m| (m, 0)).collect::<BTreeMap<_, _>>();
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
            for &metric in METRICS {
                let n = daily
                    .get(&(day.clone(), metric.to_owned()))
                    .copied()
                    .unwrap_or(0);
                *period.counts.get_mut(metric).expect("all metrics") += n;
                *totals.get_mut(metric).expect("all metrics") += n;
                if metric == "teacher_active" {
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
