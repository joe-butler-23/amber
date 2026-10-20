use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use injector_derive::ScopeInjectable;
use tokio::fs;

use crate::{
    common::utils::create_sqlite_pool::create_sqlite_pool_from_location,
    database::database_connection_manager::{
        DatabaseConnectionManager, DatabaseConnectionManagerError,
    },
    infrastructure::value_objects::{db_pool::DbPool, db_transaction::DbTransaction},
    settings::value_objects::database_location::DatabaseLocation,
    sync::bootstrap::register_sync_tables_on_pool,
};

#[derive(ScopeInjectable)]
pub struct SqliteDatabaseConnectionManager {
    pool: Arc<DbPool>,
    tx: Arc<DbTransaction>,
}

#[async_trait]
impl DatabaseConnectionManager for SqliteDatabaseConnectionManager {
    async fn connect_to_database(
        &self,
        location: DatabaseLocation,
    ) -> Result<(), DatabaseConnectionManagerError> {
        let (pool, clock) = create_sqlite_pool_from_location(&location).await?;
        register_sync_tables_on_pool(&pool)
            .await
            .map_err(|err| DatabaseConnectionManagerError(Box::new(err)))?;
        let new_tx = pool
            .begin()
            .await
            .map_err(|err| DatabaseConnectionManagerError(Box::new(err)))?;
        let old_tx = std::mem::replace(&mut *self.tx.lock().await, new_tx);
        old_tx
            .commit()
            .await
            .map_err(|err| DatabaseConnectionManagerError(Box::new(err)))?;

        // Old scopes finish against the retained source; the last handle closes its pool.
        self.pool.set_pool(pool, clock).await;
        Ok(())
    }

    async fn copy_database_to(&self, path: &Path) -> Result<(), DatabaseConnectionManagerError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|err| DatabaseConnectionManagerError(Box::new(err)))?;
        }
        sqlx::query("VACUUM main INTO ?1")
            .bind(path.to_string_lossy().as_ref())
            .execute(&*self.pool.pool().await)
            .await
            .map_err(|err| DatabaseConnectionManagerError(Box::new(err)))?;
        Ok(())
    }

    async fn disable_foreign_key_constraint_for_current_transaction(
        &self,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("PRAGMA defer_foreign_keys = ON")
            .execute(self.tx.lock().await.as_mut())
            .await?;
        Ok(())
    }

    async fn enable_foreign_key_constraint_for_current_transaction(
        &self,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("PRAGMA defer_foreign_keys = OFF")
            .execute(self.tx.lock().await.as_mut())
            .await?;
        Ok(())
    }
}
