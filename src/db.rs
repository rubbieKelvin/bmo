use sqlx::sqlite;
use std::path::Path;

pub struct Database {
    _pool: Option<sqlx::SqlitePool>,
}

impl Database {
    pub async fn create_db_pool(path: impl AsRef<Path>) -> Result<sqlx::SqlitePool, sqlx::Error> {
        let options = sqlite::SqliteConnectOptions::new()
            .filename(path)
            .optimize_on_close(true, None)
            .synchronous(sqlite::SqliteSynchronous::Normal)
            .journal_mode(sqlite::SqliteJournalMode::Wal)
            .create_if_missing(true)
            .statement_cache_capacity(0);

        let pool = sqlx::SqlitePool::connect_with(options).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        return Ok(pool);
    }

    pub fn pool(&self) -> sqlx::SqlitePool {
        let pref = self._pool.clone();
        return pref.unwrap();
    }

    pub fn new(&self) -> Self {
        return Self { _pool: None };
    }
}
