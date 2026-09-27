use anyhow::Result;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteConnection, SqliteJournalMode, SqlitePool, SqlitePoolOptions,
    SqliteSynchronous,
};
use sqlx::{Sqlite, Transaction};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
use tracing::info;

/// How long a writer waits for the database lock before giving up.
///
/// Several local servers can share one `opencode.db`; a generous timeout lets
/// a short-lived writer (for example another server syncing sessions) release
/// the lock instead of surfacing `SQLITE_BUSY` (`database is locked`).
const CONCURRENT_WRITER_BUSY_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    ConnectionError(String),

    #[error("Migration error: {0}")]
    MigrationError(String),

    #[error("Query error: {0}")]
    QueryError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),
}

pub struct Database {
    pool: SqlitePool,
}

pub type SqliteTransaction<'a> = Transaction<'a, Sqlite>;

impl Database {
    pub async fn new() -> Result<Self, DatabaseError> {
        let db_path = Self::get_database_path()?;

        Self::open(db_path).await
    }

    /// Open a database at an explicit path.
    ///
    /// Shared by [`Database::new`] and tests that need to exercise multiple
    /// connections against the same file.
    pub async fn open(db_path: impl Into<PathBuf>) -> Result<Self, DatabaseError> {
        let db_path = db_path.into();

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        }

        info!("Connecting to database at {}", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(Self::connect_options(&db_path))
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        let db = Self { pool };
        db.run_migrations().await?;

        Ok(db)
    }

    /// Baseline SQLite connection options for the shared `opencode.db`.
    ///
    /// WAL mode lets readers proceed while one writer holds the write lock,
    /// and the busy timeout makes a second writer wait for the lock to clear
    /// instead of failing immediately with `SQLITE_BUSY` (`database is
    /// locked`). This is what makes several local servers sharing one file
    /// safe from `failed to sync sessions to storage` write failures.
    fn connect_options(db_path: &Path) -> SqliteConnectOptions {
        SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(CONCURRENT_WRITER_BUSY_TIMEOUT)
    }

    pub async fn in_memory() -> Result<Self, DatabaseError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        let db = Self { pool };
        db.run_migrations().await?;

        Ok(db)
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn begin(&self) -> Result<SqliteTransaction<'_>, DatabaseError> {
        self.pool
            .begin()
            .await
            .map_err(|e| DatabaseError::TransactionError(e.to_string()))
    }

    pub async fn transaction<F, T, Fut>(&self, f: F) -> Result<T, DatabaseError>
    where
        F: FnOnce(&mut SqliteTransaction<'_>) -> Fut,
        Fut: Future<Output = Result<T, DatabaseError>>,
    {
        let mut tx = self.begin().await?;
        let result = f(&mut tx).await?;
        tx.commit()
            .await
            .map_err(|e| DatabaseError::TransactionError(e.to_string()))?;
        Ok(result)
    }

    pub async fn get_connection(&self) -> Result<SqliteConnection, DatabaseError> {
        self.pool
            .acquire()
            .await
            .map(|conn| conn.detach())
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))
    }

    async fn run_migrations(&self) -> Result<(), DatabaseError> {
        info!("Running database migrations");

        for migration in crate::schema::ALL_MIGRATIONS {
            sqlx::query(migration)
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::MigrationError(e.to_string()))?;
        }

        self.ensure_sessions_workspace_identity_column().await?;

        Ok(())
    }

    async fn ensure_sessions_workspace_identity_column(&self) -> Result<(), DatabaseError> {
        let rows = sqlx::query_as::<_, (String,)>("SELECT name FROM pragma_table_info('sessions')")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DatabaseError::MigrationError(e.to_string()))?;

        if !rows.iter().any(|(name,)| name == "workspace_identity") {
            sqlx::query("ALTER TABLE sessions ADD COLUMN workspace_identity TEXT")
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::MigrationError(e.to_string()))?;
        }

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_sessions_workspace_identity ON sessions(workspace_identity)",
        )
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::MigrationError(e.to_string()))?;

        Ok(())
    }

    fn get_database_path() -> Result<PathBuf, DatabaseError> {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("opencode");

        Ok(data_dir.join("opencode.db"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(label: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "opencode-bug052-{}-{}-{:?}.db",
            label,
            std::process::id(),
            std::thread::current().id()
        ));
        remove_db_files(&path);
        path
    }

    fn remove_db_files(path: &Path) {
        for suffix in ["", "-wal", "-shm"] {
            let candidate = PathBuf::from(format!("{}{}", path.display(), suffix));
            let _ = std::fs::remove_file(candidate);
        }
    }

    #[tokio::test]
    async fn shared_database_uses_wal_and_busy_timeout() {
        let path = temp_db_path("pragmas");
        let db = Database::open(&path).await.expect("open database");

        let (journal_mode,): (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(db.pool())
            .await
            .expect("read journal_mode");
        assert_eq!(journal_mode.to_lowercase(), "wal");

        let (busy_timeout,): (i64,) = sqlx::query_as("PRAGMA busy_timeout")
            .fetch_one(db.pool())
            .await
            .expect("read busy_timeout");
        assert_eq!(
            busy_timeout,
            CONCURRENT_WRITER_BUSY_TIMEOUT.as_millis() as i64
        );

        drop(db);
        remove_db_files(&path);
    }

    /// Two servers sharing one file must not lose a write to `SQLITE_BUSY`.
    ///
    /// One connection holds a write transaction while a second connection
    /// attempts an insert. With WAL mode plus a busy timeout the second writer
    /// waits for the lock and succeeds instead of returning `database is
    /// locked`.
    #[tokio::test]
    async fn concurrent_writer_waits_for_lock_instead_of_failing() {
        let path = temp_db_path("concurrent");
        let db_a = Database::open(&path).await.expect("open first database");
        let db_b = Database::open(&path).await.expect("open second database");

        sqlx::query("CREATE TABLE IF NOT EXISTS busy_probe (id INTEGER PRIMARY KEY)")
            .execute(db_a.pool())
            .await
            .expect("create probe table");

        let mut tx = db_a.begin().await.expect("begin write transaction");
        sqlx::query("INSERT INTO busy_probe (id) VALUES (1)")
            .execute(&mut *tx)
            .await
            .expect("hold write lock");

        let second_writer = tokio::spawn(async move {
            let result = sqlx::query("INSERT INTO busy_probe (id) VALUES (2)")
                .execute(db_b.pool())
                .await;
            (result.is_ok(), db_b)
        });

        // Keep the lock held long enough that an immediate busy failure would
        // be observable before the transaction is released.
        tokio::time::sleep(Duration::from_millis(300)).await;
        tx.commit().await.expect("release write lock");

        let (succeeded, db_b) = second_writer.await.expect("join second writer");
        assert!(
            succeeded,
            "second writer should wait for the lock instead of failing with SQLITE_BUSY"
        );

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM busy_probe")
            .fetch_one(db_b.pool())
            .await
            .expect("count rows");
        assert_eq!(count, 2);

        drop(db_a);
        drop(db_b);
        remove_db_files(&path);
    }
}
