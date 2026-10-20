use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};

use crate::sync::sql_functions::SyncClock;

pub struct DbPool(Mutex<Database>);

struct Database {
    pool: SqlitePool,
    clock: Arc<SyncClock>,
}

impl DbPool {
    pub fn new(pool: SqlitePool, clock: Arc<SyncClock>) -> Self {
        Self(Mutex::new(Database { pool, clock }))
    }

    pub async fn sync_clock(&self) -> Arc<SyncClock> {
        self.0.lock().await.clock.clone()
    }

    pub async fn pool(&self) -> MappedMutexGuard<'_, SqlitePool> {
        MutexGuard::map(self.0.lock().await, |database| &mut database.pool)
    }

    pub async fn set_pool(&self, pool: SqlitePool, clock: Arc<SyncClock>) {
        *self.0.lock().await = Database { pool, clock };
    }
}
