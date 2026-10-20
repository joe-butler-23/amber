use std::sync::Arc;

use async_trait::async_trait;
use injector_derive::ScopeInjectable;
use tokio::{fs, sync::Mutex};

use super::write_json;
use crate::{
    infrastructure::value_objects::app_data_directory::AppDataDirectory,
    settings::{
        entities::settings::Settings,
        repositories::settings_repository::{SettingsRepository, SettingsRepositoryError},
    },
};

#[cfg(not(debug_assertions))]
const SETTINGS_FILE_NAME: &str = "settings.json";
#[cfg(debug_assertions)]
const SETTINGS_FILE_NAME: &str = "settings.dev.json";

#[derive(ScopeInjectable)]
pub struct DiskSettingsRepository {
    settings: Arc<Mutex<Settings>>,
}

impl DiskSettingsRepository {
    pub async fn get_or_create_settings(
        directory: &AppDataDirectory,
        defaults: Settings,
    ) -> Result<Settings, SettingsRepositoryError> {
        match fs::read_to_string(directory.get_path().join(SETTINGS_FILE_NAME)).await {
            Ok(json) => Ok(Settings::from_json(&json)?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Self::save_to_disk(&defaults, directory).await?;
                Ok(defaults)
            }
            Err(err) => Err(err.into()),
        }
    }

    pub async fn save_to_disk(
        settings: &Settings,
        directory: &AppDataDirectory,
    ) -> Result<(), SettingsRepositoryError> {
        Ok(write_json(&directory.get_path().join(SETTINGS_FILE_NAME), settings).await?)
    }
}

#[async_trait]
impl SettingsRepository for DiskSettingsRepository {
    async fn get_settings(&self) -> Settings {
        self.settings.lock().await.clone()
    }
}
