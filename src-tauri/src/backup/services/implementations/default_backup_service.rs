use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime, Utc};
use injector_derive::ScopeInjectable;
use tokio::fs;

use crate::{
    backup::services::backup_service::{
        BackupService, BackupServiceError, TIME_BETWEEN_BACKUPS_IN_MINUTES,
    },
    database::database_connection_manager::DatabaseConnectionManager,
    settings::repositories::settings_repository::SettingsRepository,
};

const MAX_BACKUPS: usize = 8;
const DATE_FORMAT: &str = "%Y_%m_%d_%H_%M_%S";

#[derive(ScopeInjectable)]
pub struct DefaultBackupService {
    database_connection_manager: Arc<dyn DatabaseConnectionManager>,
    settings_repository: Arc<dyn SettingsRepository>,
}

#[async_trait]
impl BackupService for DefaultBackupService {
    async fn ensure_backup(&self) -> Result<(), BackupServiceError> {
        let directory = self
            .settings_repository
            .get_settings()
            .await
            .database_directory()
            .join("backups");
        fs::create_dir_all(&directory).await?;
        let mut backups = Vec::new();
        let mut entries = fs::read_dir(&directory).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if entry.file_type().await?.is_file()
                && path.extension().is_some_and(|ext| ext == "backup")
                && let Some(date) = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .and_then(|name| NaiveDateTime::parse_from_str(name, DATE_FORMAT).ok())
            {
                backups.push((date, path));
            }
        }
        backups.sort();
        let now = Utc::now();
        let recent = backups.last().is_some_and(|(date, _)| {
            now - date.and_utc() < Duration::minutes(TIME_BETWEEN_BACKUPS_IN_MINUTES as i64)
        });
        if !recent {
            let path = directory.join(format!("{}.backup", now.format(DATE_FORMAT)));
            let pending = path.with_extension("pending");
            let result = async {
                self.database_connection_manager
                    .copy_database_to(&pending)
                    .await?;
                fs::rename(&pending, &path).await?;
                Ok::<_, BackupServiceError>(())
            }
            .await;
            if result.is_err() {
                let _ = fs::remove_file(pending).await;
            }
            result?;
            backups.push((now.naive_utc(), path));
        }
        let excess = backups.len().saturating_sub(MAX_BACKUPS);
        for (_, path) in backups.into_iter().take(excess) {
            fs::remove_file(path).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use injector::register_scope;
    use tokio::sync::Mutex;

    use super::*;
    use crate::{
        common::utils::create_sqlite_pool::create_sqlite_pool,
        infrastructure::{
            extensions::unit_of_work::UnitOfWorkExt,
            repositories::{
                disk::disk_settings_repository::DiskSettingsRepository,
                sqlite::sqlite_local_configuration_repository::SqliteLocalConfigurationRepository,
            },
        },
        settings::{
            entities::settings::Settings, value_objects::settings_profile::SettingsProfile,
        },
        test_utils::{create_file_backed_test_injector, create_temp_directory},
    };

    #[tokio::test]
    async fn ensure_backup_no_backups_created_backup() {
        // Arrange
        let mut injector = create_file_backed_test_injector().await;
        let directory = create_temp_directory().await;
        injector.register_singleton(Arc::new(Mutex::new(Settings::new(
            directory.clone(),
            SettingsProfile::Default,
        ))));
        register_scope!(injector, dyn SettingsRepository, DiskSettingsRepository);
        register_scope!(injector, SqliteLocalConfigurationRepository);
        register_scope!(injector, dyn BackupService, DefaultBackupService);
        let scope = injector.start_scope();
        scope
            .resolve::<SqliteLocalConfigurationRepository>()
            .await
            .set_by_name("round-trip", &"saved")
            .await
            .unwrap();
        scope.save_changes().await.unwrap();

        // Act
        scope
            .resolve::<dyn BackupService>()
            .await
            .ensure_backup()
            .await
            .unwrap();

        // Assert
        let backup = fs::read_dir(directory.join("backups"))
            .await
            .unwrap()
            .next_entry()
            .await
            .unwrap()
            .unwrap()
            .path();
        let (pool, _) = create_sqlite_pool(&format!("sqlite://{}", backup.display()))
            .await
            .unwrap();
        let value: String =
            sqlx::query_scalar("SELECT value FROM local_configurations WHERE name = 'round-trip'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(value, serde_json::to_string("saved").unwrap());
        assert_eq!(
            "ok",
            sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
                .fetch_one(&pool)
                .await
                .unwrap()
        );
    }
}
