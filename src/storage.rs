mod json;
mod sqlite;

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Context, Result};
use directories::ProjectDirs;

use self::json::JsonStore;
use self::sqlite::SqliteStore;
use crate::app::config::AppConfig;
use crate::dto::{CallRecord, ChatSession, SessionSummary};

pub struct StorageManager {
    config_store: JsonStore,
    db_store: SqliteStore,
}

impl StorageManager {
    pub fn new(app_name: &str) -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "local", app_name)
            .context("Failed to determine system directories")?;

        let config_dir = proj_dirs.config_dir();
        let data_dir = proj_dirs.data_local_dir();

        fs::create_dir_all(config_dir)?;
        fs::create_dir_all(data_dir)?;

        Ok(Self {
            config_store: JsonStore::new(config_dir, "config.json"),
            db_store: SqliteStore::open(data_dir, format!("{app_name}.db").as_str())?,
        })
    }

    // --- Config ---

    pub fn load_config(&self) -> Result<AppConfig> {
        self.config_store.load()
    }

    pub fn save_config(&self, cfg: &AppConfig) -> Result<()> {
        self.config_store.save(cfg)
    }

    // --- Sessions ---

    pub fn save_session(&mut self, session: &ChatSession) -> Result<()> {
        self.db_store.save_session(session)
    }

    pub fn load_latest_or_create_session(&self) -> Result<ChatSession> {
        match self.db_store.load_latest_session()? {
            Some(session) => Ok(session),
            None => Ok(ChatSession::new(Self::generate_session_id(), "Default Session")),
        }
    }

    pub fn load_session(&self, session_id: &str, title: String) -> Result<ChatSession> {
        self.db_store.load_session(session_id, title)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        self.db_store.list_sessions()
    }

    pub fn delete_session(&self, session_id: &str) -> Result<bool> {
        self.db_store.delete_session(session_id)
    }

    pub fn clear_session_messages(&self, session_id: &str) -> Result<()> {
        self.db_store.clear_session_messages(session_id)
    }

    // --- Stats ---

    pub fn insert_call_stat(&self, rec: &CallRecord) -> Result<()> {
        self.db_store.insert_call_stat(rec)
    }

    pub fn load_session_stats(&self, session_id: &str) -> Result<Vec<CallRecord>> {
        self.db_store.load_session_stats(session_id)
    }

    pub fn clear_stats(&self) -> Result<()> {
        self.db_store.clear_stats()
    }

    pub fn generate_session_id() -> String {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("session_{ts}")
    }
}