use sqlx::SqlitePool;
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{Mutex, watch};
use zip::{ZipWriter, write::SimpleFileOptions};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
const DAY: u64 = 86_400;
const NIGHT: u64 = 2 * 3_600; // 02:00 UTC, independent of the host's timezone/DST.

pub struct Backups {
    db: SqlitePool,
    directory: PathBuf,
    lock: Mutex<()>,
}

impl Backups {
    pub fn new(db: super::Database, database: &Path) -> Self {
        let mut directory = database.as_os_str().to_owned();
        directory.push(".backups");
        Self {
            db: db.pool,
            directory: directory.into(),
            lock: Mutex::new(()),
        }
    }

    pub async fn manual(&self) -> Result<()> {
        let _guard = self.lock.lock().await;
        let destination = self.directory.join("manual").join(format!(
            "manual-{}-{}.zip",
            crate::util::now(),
            crate::util::id()
        ));
        self.create(&destination).await
    }

    async fn automatic(&self, now: u64) -> Result<()> {
        let _guard = self.lock.lock().await;
        // Before 02:00, the previous night's backup is the latest one due.
        // Its stable name also prevents duplicates after a server restart.
        let scheduled = (now.saturating_sub(NIGHT) / DAY) * DAY + NIGHT;
        let directory = self.directory.join("automatic");
        let destination = directory.join(format!("automatic-{scheduled:010}.zip"));
        if !destination.try_exists()? {
            self.create(&destination).await?;
            tracing::info!("Automatic database backup created");
        }
        // Only prune after a complete backup exists. Manual archives and
        // unrelated files are never candidates for automatic retention.
        let directory = directory.clone();
        tokio::task::spawn_blocking(move || prune(&directory, now)).await??;
        Ok(())
    }

    async fn create(&self, destination: &Path) -> Result<()> {
        private_directory(&self.directory)?;
        private_directory(destination.parent().ok_or("Missing backup directory")?)?;
        let staging = Staging(
            self.directory
                .join(format!(".pending-{}", crate::util::id())),
        );
        private_directory(&staging.0)?;
        let snapshot = staging.0.join("database.sqlite");
        // VACUUM INTO takes a consistent live SQLite snapshot, including
        // committed WAL contents, without copying the live database/WAL files.
        sqlx::query("VACUUM INTO ?")
            .bind(snapshot.to_str().ok_or("Invalid backup path")?)
            .execute(&self.db)
            .await?;
        let destination = destination.to_owned();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let archive_path = staging.0.join("backup.zip");
            let mut archive = ZipWriter::new(File::create(&archive_path)?);
            archive.start_file(
                "database.sqlite",
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )?;
            io::copy(&mut File::open(snapshot)?, &mut archive)?;
            archive.finish()?.sync_all()?;
            // Publish atomically: a partial archive is never a finished backup.
            fs::rename(archive_path, destination)?;
            drop(staging);
            Ok(())
        })
        .await??;
        Ok(())
    }

    pub fn start(
        self: Arc<Self>,
        mut shutdown: watch::Receiver<bool>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                if *shutdown.borrow() {
                    break;
                }
                if let Err(error) = self.automatic(crate::util::now()).await {
                    tracing::error!(%error, "Automatic database backup failed; retrying in one minute");
                }
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(60)) => {},
                    _ = shutdown.changed() => break,
                }
            }
        })
    }
}

fn private_directory(path: &Path) -> io::Result<()> {
    let builder = &mut fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists && path.is_dir() => Ok(()),
        Err(error) => Err(error),
    }
}

struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0)
            && error.kind() != io::ErrorKind::NotFound
        {
            tracing::warn!(%error, "Could not remove temporary backup files");
        }
    }
}

fn prune(directory: &Path, now: u64) -> Result<()> {
    let mut archives = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(timestamp) = name
            .to_str()
            .and_then(|n| n.strip_prefix("automatic-"))
            .and_then(|n| n.strip_suffix(".zip"))
            .filter(|n| n.len() == 10 && n.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|n| n.parse::<u64>().ok())
        else {
            continue;
        };
        archives.push((timestamp, entry.path()));
    }
    archives.sort_by_key(|a| std::cmp::Reverse(a.0));
    for (index, (timestamp, path)) in archives.into_iter().enumerate() {
        if index >= 7 || timestamp <= now.saturating_sub(7 * DAY) {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

    async fn fixture() -> (Staging, Backups) {
        let root =
            Staging(std::env::temp_dir().join(format!("tamara-backup-{}", crate::util::id())));
        private_directory(&root.0).unwrap();
        let database = root.0.join("teacher's database.sqlite");
        let db = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&database)
                    .create_if_missing(true)
                    .journal_mode(SqliteJournalMode::Wal)
                    .pragma("wal_autocheckpoint", "0"),
            )
            .await
            .unwrap();
        sqlx::query("CREATE TABLE records (value TEXT NOT NULL)")
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("INSERT INTO records VALUES ('committed WAL data')")
            .execute(&db)
            .await
            .unwrap();
        (root, Backups::new(crate::db::Database::new(db), &database))
    }

    fn files(path: &Path) -> Vec<PathBuf> {
        fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect()
    }

    #[tokio::test]
    async fn archive_is_a_standalone_consistent_snapshot_of_committed_wal_data() {
        let (root, backups) = fixture().await;
        assert!(
            fs::metadata(root.0.join("teacher's database.sqlite-wal"))
                .unwrap()
                .len()
                > 0
        );
        backups.manual().await.unwrap();
        let archives = files(&backups.directory.join("manual"));
        assert_eq!(archives.len(), 1);
        let mut archive = zip::ZipArchive::new(File::open(&archives[0]).unwrap()).unwrap();
        assert_eq!(archive.len(), 1);
        let snapshot = root.0.join("extracted.sqlite");
        io::copy(
            &mut archive.by_name("database.sqlite").unwrap(),
            &mut File::create(&snapshot).unwrap(),
        )
        .unwrap();
        sqlx::query("UPDATE records SET value='changed after backup'")
            .execute(&backups.db)
            .await
            .unwrap();
        let restored = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(snapshot)
                    .read_only(true),
            )
            .await
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
                .fetch_one(&restored)
                .await
                .unwrap(),
            "ok"
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT value FROM records")
                .fetch_one(&restored)
                .await
                .unwrap(),
            "committed WAL data"
        );
        assert_eq!(
            files(&backups.directory),
            vec![backups.directory.join("manual")]
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for directory in [&backups.directory, &backups.directory.join("manual")] {
                assert_eq!(
                    fs::metadata(directory).unwrap().permissions().mode() & 0o777,
                    0o700
                );
            }
        }
        restored.close().await;
        backups.db.close().await;
    }

    #[tokio::test]
    async fn nightly_backups_deduplicate_catch_up_and_retain_only_one_week() {
        let (root, backups) = fixture().await;
        let night = 1_800_000_000 / DAY * DAY + NIGHT;
        let automatic = backups.directory.join("automatic");
        backups.manual().await.unwrap();
        let manual = files(&backups.directory.join("manual"));
        let manual_bytes = fs::read(&manual[0]).unwrap();

        backups.automatic(night - 1).await.unwrap();
        assert!(
            automatic
                .join(format!("automatic-{:010}.zip", night - DAY))
                .exists()
        );
        backups.automatic(night).await.unwrap();
        let first = automatic.join(format!("automatic-{night:010}.zip"));
        let first_bytes = fs::read(&first).unwrap();
        sqlx::query("DELETE FROM records")
            .execute(&backups.db)
            .await
            .unwrap();
        // A reconstructed service represents a process restart: no in-memory
        // last-run marker is needed to avoid replacing the existing snapshot.
        let restarted = Backups::new(
            crate::db::Database::new(backups.db.clone()),
            &root.0.join("teacher's database.sqlite"),
        );
        restarted.automatic(night + DAY - 1).await.unwrap();
        assert_eq!(fs::read(first).unwrap(), first_bytes);
        let unrelated = automatic.join("keep-me.zip");
        fs::write(&unrelated, b"unrelated file").unwrap();
        for day in 1..=10 {
            backups.automatic(night + day * DAY).await.unwrap();
        }
        let retained: Vec<_> = files(&automatic)
            .into_iter()
            .filter(|p| p != &unrelated)
            .collect();
        assert_eq!(retained.len(), 7);
        for day in 4..=10 {
            assert!(
                automatic
                    .join(format!("automatic-{:010}.zip", night + day * DAY))
                    .exists()
            );
        }
        // A long outage creates just the latest due backup and removes stale
        // nightly archives, preserving both manual archives and other files.
        backups.automatic(night + 30 * DAY + 300).await.unwrap();
        assert_eq!(files(&automatic).len(), 2);
        assert_eq!(files(&backups.directory.join("manual")), manual);
        assert_eq!(fs::read(&manual[0]).unwrap(), manual_bytes);
        assert_eq!(fs::read(unrelated).unwrap(), b"unrelated file");
        backups.db.close().await;
    }

    #[tokio::test]
    async fn failed_snapshot_publishes_nothing_and_preserves_old_archives() {
        let (_root, backups) = fixture().await;
        let night = 1_800_000_000 / DAY * DAY + NIGHT;
        backups.automatic(night).await.unwrap();
        let automatic = backups.directory.join("automatic");
        let previous = files(&automatic);
        backups.db.close().await;
        assert!(backups.automatic(night + 30 * DAY).await.is_err());
        assert_eq!(files(&automatic), previous);
        assert_eq!(files(&backups.directory), vec![automatic]);
    }
}
