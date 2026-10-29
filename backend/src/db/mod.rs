//! SQLite queries, relational invariants and aggregate transactions.
mod accounts;
mod assessments;
mod attendance;
pub(crate) mod backup;
mod classrooms;
pub(crate) mod exports;
mod homework;
mod notes;

use crate::pdf;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{path::PathBuf, time::Duration};

#[derive(Clone)]
pub(crate) struct Database {
    pool: SqlitePool,
}

impl Database {
    #[cfg(test)]
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

pub(crate) async fn connect(
    url: &str,
) -> std::result::Result<(Database, PathBuf), Box<dyn std::error::Error>> {
    let options: SqliteConnectOptions = url
        .parse::<SqliteConnectOptions>()?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(10));
    if let Some(parent) = options
        .get_filename()
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let database_path = options.get_filename().to_owned();
    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    let db = Database { pool: db };
    db.initialize().await?;
    Ok((db, database_path))
}

impl Database {
    pub(crate) async fn initialize(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        let mut tx = self.pool.begin().await?;
        for c in &pdf::catalog().criteria {
            sqlx::query(
                "INSERT INTO math_criteria(id, label, domain, section, level, page, x, y)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id)
             DO UPDATE
             SET label=excluded.label, domain=excluded.domain, section=excluded.section,
             level=excluded.level, page=excluded.page, x=excluded.x, y=excluded.y",
            )
            .bind(&c.id)
            .bind(&c.label)
            .bind(&c.domain)
            .bind(&c.section)
            .bind(&c.level)
            .bind(c.page)
            .bind(c.x)
            .bind(c.y)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
