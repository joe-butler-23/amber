use std::sync::Arc;

use injector_derive::ScopeInjectable;
use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

use crate::{
    common::repository_error::RepositoryError,
    infrastructure::value_objects::db_transaction::DbTransaction,
};

#[derive(Debug, Error)]
pub enum LocalConfigurationError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error("Failed to deserialize local configuration value")]
    Deserialize(#[source] serde_json::Error),
    #[error("Failed to serialize local configuration value")]
    Serialize(#[source] serde_json::Error),
}

#[derive(ScopeInjectable)]
pub struct SqliteLocalConfigurationRepository {
    tx: Arc<DbTransaction>,
}

impl SqliteLocalConfigurationRepository {
    pub async fn get_by_name<T: DeserializeOwned + Send>(
        &self,
        name: &str,
    ) -> Result<Option<T>, LocalConfigurationError> {
        let row = {
            let mut tx = self.tx.lock().await;
            sqlx::query!(
                r#"SELECT * FROM local_configurations WHERE name = $1"#,
                name
            )
            .fetch_optional(tx.as_mut())
            .await
            .map_err(RepositoryError::from)?
        };
        row.map(|row| {
            serde_json::from_str(&row.value).map_err(LocalConfigurationError::Deserialize)
        })
        .transpose()
    }

    pub async fn set_by_name<T: Serialize + Sync>(
        &self,
        name: &str,
        value: &T,
    ) -> Result<(), LocalConfigurationError> {
        let value = serde_json::to_string(value).map_err(LocalConfigurationError::Serialize)?;
        let mut tx = self.tx.lock().await;
        sqlx::query!(
            r#"INSERT INTO local_configurations(name, value) VALUES ($1, $2)
            ON CONFLICT(name) DO UPDATE
            SET name = $1, value = $2
            "#,
            name,
            value
        )
        .execute(tx.as_mut())
        .await
        .map_err(RepositoryError::from)?;
        Ok(())
    }
}
