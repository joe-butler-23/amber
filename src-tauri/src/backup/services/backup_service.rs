use async_trait::async_trait;
use thiserror::Error;

use crate::database::database_connection_manager::DatabaseConnectionManagerError;

#[derive(Error, Debug)]
pub enum BackupServiceError {
    #[error(transparent)]
    DatabaseConnectionManager(#[from] DatabaseConnectionManagerError),
    #[error("Failed to access backups: {0}")]
    Io(#[from] std::io::Error),
}

pub const TIME_BETWEEN_BACKUPS_IN_MINUTES: u64 = 120;

#[async_trait]
pub trait BackupService: Send + Sync {
    async fn ensure_backup(&self) -> Result<(), BackupServiceError>;
}
