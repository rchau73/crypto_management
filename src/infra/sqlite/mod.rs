pub mod repo;

pub use repo::SqliteRepo;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::str::FromStr;
use std::time::Duration;

/// Opens the database and applies every pending migration.
///
/// - WAL lets readers keep working while a write is in progress.
/// - `busy_timeout` makes a writer wait for a lock instead of failing
///   immediately with "database is locked".
/// - Foreign keys are enforced (sqlx's default, stated explicitly).
pub async fn connect(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// A fresh, fully migrated in-memory database for tests.
#[cfg(test)]
pub async fn test_repo() -> std::sync::Arc<SqliteRepo> {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    std::sync::Arc::new(SqliteRepo::new(pool))
}
