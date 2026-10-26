//! The session factory.

use std::path::PathBuf;
use std::sync::Arc;

use orexis_ports::{PortResult, SessionAppend, SessionFactory, SessionRead, SessionRefs};

use crate::store::SqliteSessionStore;

pub struct SqliteSessionFactory {
    db: PathBuf,
}

impl SqliteSessionFactory {
    pub fn new(db: impl Into<PathBuf>) -> Self {
        Self { db: db.into() }
    }
}

impl SessionFactory for SqliteSessionFactory {
    fn open(&self, session: Option<&str>) -> PortResult<SessionRefs> {
        let store = match session {
            Some(id) => Arc::new(SqliteSessionStore::open_session(&self.db, id)?),
            None => Arc::new(SqliteSessionStore::open(&self.db)?),
        };
        let append: Arc<dyn SessionAppend> = store.clone();
        let read: Arc<dyn SessionRead> = store.clone();
        Ok(SessionRefs {
            id: store.session_id().to_string(),
            append,
            read,
        })
    }

    fn list(&self) -> PortResult<Vec<String>> {
        SqliteSessionStore::list_sessions(&self.db)
    }
}
