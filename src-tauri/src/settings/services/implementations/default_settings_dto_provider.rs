use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use injector_derive::ScopeInjectable;

use crate::{
    secrets::repositories::secrets_repository::SecretsRepository,
    settings::{
        dto::{ai_provider_settings_dto::AiProviderSettingsDto, settings_dto::SettingsDto},
        repositories::settings_repository::SettingsRepository,
        services::settings_dto_provider::SettingsDtoProvider,
        value_objects::ai_provider::AiProvider,
    },
};

#[derive(ScopeInjectable)]
pub struct DefaultSettingsDtoProvider {
    settings_repository: Arc<dyn SettingsRepository>,
    secrets_repository: Arc<dyn SecretsRepository>,
}

#[async_trait]
impl SettingsDtoProvider for DefaultSettingsDtoProvider {
    async fn get_settings_dto(&self) -> SettingsDto {
        let settings = self.settings_repository.get_settings().await;

        let mut ai_providers = HashMap::new();
        for provider in AiProvider::ALL {
            let api_key_is_set = match provider.api_key_secret() {
                Some(secret) => self
                    .secrets_repository
                    .get_secret(secret)
                    .await
                    .is_some_and(|k| !k.is_empty()),
                None => false,
            };
            let provider_settings = settings.ai_provider_settings(provider);
            ai_providers.insert(
                provider,
                AiProviderSettingsDto {
                    model_name: provider_settings.model_name,
                    embeddings_model_name: provider_settings.embeddings_model_name,
                    api_key_is_set,
                },
            );
        }

        SettingsDto {
            base_database_directory: settings.base_database_directory_as_string(),
            theme: settings.theme,
            font: settings.font,
            font_headings: settings.font_headings,
            font_monospace: settings.font_monospace,
            zoom_percentage: settings.zoom_percentage,
            line_width: settings.line_width,
            auto_sync: settings.auto_sync,
            trash_retention_days: settings.trash_retention_days,
            enable_ai: settings.enable_ai,
            ai_provider: settings.ai_provider,
            ai_providers,
        }
    }
}
