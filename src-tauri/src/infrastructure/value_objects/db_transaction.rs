use std::ops::Deref;
use std::sync::Arc;

use sqlx::{Sqlite, Transaction};
use tokio::sync::Mutex;

use super::db_pool::Library;

pub type SqliteTransaction = Transaction<'static, Sqlite>;

pub struct DbTransaction {
    pub library: Arc<Library>,
    tx: Mutex<SqliteTransaction>,
}

impl DbTransaction {
    pub fn new(tx: SqliteTransaction, library: Arc<Library>) -> Self {
        Self {
            library,
            tx: Mutex::new(tx),
        }
    }
}

impl Deref for DbTransaction {
    type Target = Mutex<SqliteTransaction>;

    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}
