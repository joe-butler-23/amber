use std::path::Path;

use async_trait::async_trait;
use thiserror::Error;

#[cfg(test)]
use mockall::automock;

use crate::SourceError;
use crate::settings::value_objects::database_location::DatabaseLocation;

#[derive(Error, Debug)]
#[error("Database operation failed: {0}")]
pub struct DatabaseConnectionManagerError(#[from] pub SourceError);

#[async_trait]
#[cfg_attr(test, automock)]
pub trait DatabaseConnectionManager: Send + Sync {
    async fn connect_to_database(
        &self,
        database_location: DatabaseLocation,
    ) -> Result<(), DatabaseConnectionManagerError>;

    async fn copy_database_to(&self, path: &Path) -> Result<(), DatabaseConnectionManagerError>;

    /// Defers foreign key constraint checks on the current transaction until
    /// it commits, instead of enforcing them immediately on each statement.
    async fn disable_foreign_key_constraint_for_current_transaction(
        &self,
    ) -> Result<(), sqlx::Error>;

    /// Reverts `disable_foreign_key_constraint_for_current_transaction`,
    /// restoring immediate foreign key constraint enforcement.
    async fn enable_foreign_key_constraint_for_current_transaction(
        &self,
    ) -> Result<(), sqlx::Error>;
}
