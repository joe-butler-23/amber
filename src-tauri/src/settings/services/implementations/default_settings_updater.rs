use std::sync::Arc;

use async_trait::async_trait;
use injector_derive::ScopeInjectable;
use tokio::sync::Mutex;

use crate::{
    database::database_connection_manager::{
        DatabaseConnectionManager, DatabaseConnectionManagerError,
    },
    infrastructure::{
        repositories::disk::disk_settings_repository::DiskSettingsRepository,
        value_objects::app_data_directory::AppDataDirectory,
    },
    secrets::repositories::secrets_repository::SecretsRepository,
    settings::{
        dto::update_settings_request_dto::UpdateSettingsRequestDto,
        entities::settings::Settings,
        services::settings_updater::{SettingsUpdater, SettingsUpdaterError},
    },
};

#[derive(ScopeInjectable)]
pub struct DefaultSettingsUpdater {
    settings: Arc<Mutex<Settings>>,
    app_data_directory: Arc<AppDataDirectory>,
    database_connection_manager: Arc<dyn DatabaseConnectionManager>,
    secrets_repository: Arc<dyn SecretsRepository>,
}

#[async_trait]
impl SettingsUpdater for DefaultSettingsUpdater {
    async fn update_settings(
        &self,
        new_settings: UpdateSettingsRequestDto,
    ) -> Result<(), SettingsUpdaterError> {
        let mut current = self.settings.lock().await;
        let mut settings = current.clone();
        let old_location = settings.database_location();

        macro_rules! apply {
            ($($field:ident),* $(,)?) => {
                $(if let Some(value) = new_settings.$field { settings.$field = value; })*
            };
        }
        apply!(
            base_database_directory,
            profile,
            theme,
            font,
            font_headings,
            font_monospace,
            zoom_percentage,
            line_width,
            auto_sync,
            trash_retention_days,
            enable_ai,
            ai_provider,
        );

        let mut api_keys = Vec::new();
        for (provider, mut options) in new_settings.ai_providers.unwrap_or_default() {
            if let Some(key) = options.api_key.take()
                && let Some(secret) = provider.api_key_secret()
            {
                api_keys.push((secret, key));
            }
            settings.ai_providers.insert(provider, options);
        }

        let new_location = settings.database_location();
        if new_location != old_location {
            if !tokio::fs::try_exists(new_location.get_path())
                .await
                .map_err(|err| DatabaseConnectionManagerError(Box::new(err)))?
            {
                // Copy a snapshot; the source remains available for recovery.
                self.database_connection_manager
                    .copy_database_to(new_location.get_path())
                    .await?;
            }
            self.database_connection_manager
                .connect_to_database(new_location)
                .await?;
        }

        *current = settings;
        DiskSettingsRepository::save_to_disk(&current, &self.app_data_directory).await?;
        for (secret, key) in api_keys {
            self.secrets_repository.set_secret(secret, &key).await?;
        }
        Ok(())
    }
}
