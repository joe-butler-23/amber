use async_trait::async_trait;
use thiserror::Error;

use crate::settings::entities::settings::Settings;

#[derive(Error, Debug)]
pub enum SettingsRepositoryError {
    #[error("Failed to access settings: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse settings: {0}")]
    Json(#[from] serde_json::Error),
}

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get_settings(&self) -> Settings;
}
