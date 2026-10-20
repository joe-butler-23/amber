use std::sync::Arc;

use async_trait::async_trait;
use injector_derive::ScopeInjectable;

use crate::{
    settings::repositories::settings_repository::SettingsRepository,
    sync::post_sync_task::{PostSyncTask, PostSyncTaskError},
    trash::services::trash_service::TrashService,
};

#[derive(ScopeInjectable)]
pub struct TrashRetentionPostSyncTask {
    settings_repository: Arc<dyn SettingsRepository>,
    trash_service: Arc<dyn TrashService>,
}

#[async_trait]
impl PostSyncTask for TrashRetentionPostSyncTask {
    fn name(&self) -> &'static str {
        "trash retention"
    }

    async fn run(&self) -> Result<(), PostSyncTaskError> {
        let days = self
            .settings_repository
            .get_settings()
            .await
            .trash_retention_days;
        self.trash_service.purge_expired(days).await?;
        Ok(())
    }
}
