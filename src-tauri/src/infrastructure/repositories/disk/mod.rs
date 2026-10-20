use std::{io, path::Path};

use serde::Serialize;
use tokio::{fs, io::AsyncWriteExt};
use uuid::Uuid;

pub mod disk_secrets_repository;
pub mod disk_settings_repository;

async fn write_json(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let bytes = serde_json::to_vec(value)?;
    fs::create_dir_all(path.parent().unwrap()).await?;
    let temporary = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let result = async {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&temporary).await?;
        file.write_all(&bytes).await?;
        file.sync_all().await?;
        drop(file);
        fs::rename(&temporary, path).await?;
        #[cfg(unix)]
        fs::File::open(path.parent().unwrap())
            .await?
            .sync_all()
            .await?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = fs::remove_file(temporary).await;
    }
    result
}
