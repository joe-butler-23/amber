use std::{collections::HashMap, path::PathBuf};

use async_trait::async_trait;
use tokio::{fs, sync::Mutex};

use super::write_json;
use crate::{
    infrastructure::value_objects::app_data_directory::AppDataDirectory,
    secrets::repositories::secrets_repository::{SecretsRepository, SecretsRepositoryError},
};

#[cfg(not(debug_assertions))]
const SECRETS_FILE_NAME: &str = ".secrets";
#[cfg(debug_assertions)]
const SECRETS_FILE_NAME: &str = ".dev.secrets";

pub struct DiskSecretsRepository {
    path: PathBuf,
    writer: Mutex<()>,
}

impl DiskSecretsRepository {
    pub fn new(directory: &AppDataDirectory) -> Self {
        Self {
            path: directory.get_path().join(SECRETS_FILE_NAME),
            writer: Mutex::new(()),
        }
    }

    async fn read_all(&self) -> std::io::Result<HashMap<String, String>> {
        match fs::read(&self.path).await {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(HashMap::new()),
            Err(err) => Err(err),
        }
    }
}

#[async_trait]
impl SecretsRepository for DiskSecretsRepository {
    async fn get_secret(&self, key: &str) -> Option<String> {
        self.read_all().await.ok()?.remove(key)
    }

    async fn set_secret(&self, key: &str, value: &str) -> Result<(), SecretsRepositoryError> {
        let _guard = self.writer.lock().await;
        let result = async {
            let mut values = self.read_all().await?;
            values.insert(key.to_string(), value.to_string());
            write_json(&self.path, &values).await
        }
        .await;
        result.map_err(|err| SecretsRepositoryError::CannotSave(Box::new(err)))
    }
}
