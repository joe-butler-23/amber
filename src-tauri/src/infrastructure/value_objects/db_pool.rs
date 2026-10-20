use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::sync::sql_functions::SyncClock;

pub struct DbPool(Mutex<Arc<Library>>);

pub struct Library {
    pub pool: SqlitePool,
    pub clock: Arc<SyncClock>,
}

impl DbPool {
    pub fn new(pool: SqlitePool, clock: Arc<SyncClock>) -> Self {
        Self(Mutex::new(Arc::new(Library { pool, clock })))
    }

    pub async fn current(&self) -> Arc<Library> {
        self.0.lock().await.clone()
    }

    pub async fn set_pool(&self, pool: SqlitePool, clock: Arc<SyncClock>) {
        *self.0.lock().await = Arc::new(Library { pool, clock });
    }
}
