use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::SqlitePoolOptions;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db_path = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("SQLITE_PATH"))
        .unwrap_or_else(|_| "./data/gateway.db".to_string());

    // Create database file if it doesn't exist
    if !Path::new(&db_path).exists() {
        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;

        // Run migrations
        sqlx::query("CREATE TABLE IF NOT EXISTS _sqlx_migrations (
            version bigint primary key,
            description text,
            installed_on text,
            success boolean,
            checksum bytea,
            execution_time bigint
        )").execute(&pool).await?;

        pool.close().await?;
    }

    println!("cargo:rerun-if-changed=migrations/*");
    println!("cargo:rerun-if-env-changed=DATABASE_URL");
    Ok(())
}