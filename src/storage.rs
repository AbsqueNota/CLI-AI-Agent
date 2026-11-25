pub mod mpsc_db;
mod json;
mod sqlite;

use std::fs;
use tokio::sync::mpsc;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use uuid::Uuid;

use self::mpsc_db::{DbActor, DbCommand};
use self::json::JsonStore;
use self::sqlite::SqliteStore;

use crate::app::config::AppConfig;
use crate::dto::{CallRecord, ChatSession, SessionSummary};

pub struct StorageManager {
    config_store: JsonStore,
    db_store: SqliteStore,
    db_actor: DbActor,
}

impl StorageManager {
    pub fn new(app_name: &str) -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "local", app_name)
            .context("Failed to determine system directories")?;

        let config_dir = proj_dirs.config_dir();
        let data_dir = proj_dirs.data_local_dir();

        fs::create_dir_all(config_dir)?;
        fs::create_dir_all(data_dir)?;

        let db_name = format!("{app_name}.db");

        let db_store = SqliteStore::open(data_dir, &db_name)?;

        let db_actor = DbActor::spawn(
            data_dir.to_path_buf(),
            db_name,
        )?;

        Ok(Self {
            config_store: JsonStore::new(config_dir, "config.json"),
            db_store,
            db_actor,
        })
    }

    pub(crate) fn db_sender(&self) -> mpsc::UnboundedSender<DbCommand> {
        self.db_actor.sender()
    }

    // --- Config ---

    pub fn load_config(&self) -> Result<AppConfig> {
        self.config_store.load()
    }

    pub fn save_config(&self, cfg: &AppConfig) -> Result<()> {
        self.config_store.save(cfg)
    }

    // --- Sessions ---

    pub async fn save_session(&self, session: &ChatSession) -> Result<()> {
        self.db_actor.save_session(session.clone()).await
    }

    pub fn load_latest_or_create_session(&self) -> Result<ChatSession> {
        match self.db_store.load_latest_session()? {
            Some(session) => Ok(session),
            None => Ok(ChatSession::new(
                Self::generate_session_id(),
                "Default Session",
            )),
        }
    }

    pub fn load_session(
        &self,
        session_id: &str,
        title: String,
    ) -> Result<ChatSession> {
        self.db_store.load_session(session_id, title)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        self.db_store.list_sessions()
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<bool> {
        self.db_actor
            .delete_session(session_id.to_string())
            .await
    }

    pub async fn clear_session_messages(&self, session_id: &str) -> Result<()> {
        self.db_actor
            .clear_session_messages(session_id.to_string())
            .await
    }

    // --- Stats ---

    pub async fn insert_call_stat(&self, rec: &CallRecord) -> Result<()> {
        self.db_actor.insert_call_stat(rec.clone()).await
    }

    pub fn load_session_stats(&self, session_id: &str) -> Result<Vec<CallRecord>> {
        self.db_store.load_session_stats(session_id)
    }

    pub async fn clear_stats(&self) -> Result<()> {
        self.db_actor.clear_stats().await
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.db_actor.shutdown().await
    }

    pub fn generate_session_id() -> String {
        Uuid::now_v7().to_string()
    }
}