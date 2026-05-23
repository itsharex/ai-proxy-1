use super::pool::{get_pool, init_pool};
use tracing::info;

pub async fn init_db(db_path: &str) -> Result<(), sqlx::Error> {
    init_pool(db_path).await?;
    let pool = get_pool().await;

    let migration = include_str!("../../migrations/001_init.sql");
    sqlx::query(migration).execute(pool).await?;

    let migration2 = include_str!("../../migrations/002_proxy_auth_key.sql");
    sqlx::query(migration2).execute(pool).await?;

    // Migration 003: check if already applied by looking for 'format' column on providers
    let has_format: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('providers') WHERE name = 'format'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !has_format {
        let migration3 = include_str!("../../migrations/003_simplify_routing.sql");
        sqlx::query(migration3).execute(pool).await?;
        info!("Applied migration 003: simplify routing");
    }

    // Migration 004: drop auth_type/auth_header (determined by format now)
    let has_auth_type: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('providers') WHERE name = 'auth_type'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if has_auth_type {
        let migration4 = include_str!("../../migrations/004_drop_auth_columns.sql");
        sqlx::query(migration4).execute(pool).await?;
        info!("Applied migration 004: drop auth columns");
    }

    info!("Database schema initialized");
    Ok(())
}
