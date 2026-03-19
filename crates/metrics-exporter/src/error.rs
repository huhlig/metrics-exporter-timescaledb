use anyhow::Error as AnyhowError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Anyhow error: {0}")]
    Anyhow(#[from] AnyhowError),
}
