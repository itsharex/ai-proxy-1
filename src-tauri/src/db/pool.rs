use sqlx::sqlite::SqlitePool;
use std::sync::OnceLock;
use tracing::info;

static POOL: OnceLock<SqlitePool> = OnceLock::new();

pub async fn get_pool() -> &'static SqlitePool {
    POOL.get().expect("Database pool not initialized")
}

pub async fn init_pool(path: &str) -> Result<(), sqlx::Error> {
    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", path)).await?;
    sqlx::query("PRAGMA journal_mode = WAL").execute(&pool).await?;
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;
    POOL.set(pool).expect("Database pool already initialized");
    info!("Database pool initialized");
    Ok(())
}
