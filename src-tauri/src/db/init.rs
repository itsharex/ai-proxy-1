use super::pool::{get_pool, init_pool};
use tracing::info;

pub async fn init_db(db_path: &str) -> Result<(), sqlx::Error> {
    init_pool(db_path).await?;
    let pool = get_pool().await;

    let migration = include_str!("../../migrations/001_init.sql");
    sqlx::query(migration).execute(pool).await?;

    info!("Database schema initialized");
    Ok(())
}
