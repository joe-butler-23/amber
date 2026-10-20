use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;

use thiserror::Error;

use crate::common::repository_error::RepositoryError;

/// Domain work after a successful pull and before push. Invariant repairs must
/// choose deterministically from synced state; retention applies local policy
/// only after that fresh state has been reconciled.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait PostSyncTask: Send + Sync {
    /// Human-readable name, used only for logging.
    fn name(&self) -> &'static str;

    async fn run(&self) -> Result<(), PostSyncTaskError>;
}

#[derive(Debug, Error)]
pub enum PostSyncTaskError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),

    #[error("Database error")]
    Database(#[from] sqlx::Error),
}
