use std::path::Path;
use std::time::Duration;
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use rig::message::Message;
use crate::dto::{CallRecord, ChatSession, SessionSummary};

pub struct SqliteStore {
    db: Connection,
}

impl SqliteStore {
    pub fn open(data_dir: &Path, db_name: &str) -> Result<Self> {
        let db_path = data_dir.join(db_name);
    
        let db = Connection::open(&db_path)
            .with_context(|| {
                format!("Failed to open SQLite database at {:?}", db_path)
            })?;
    
        db.busy_timeout(Duration::from_secs(5))?;
    
        db.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;"
        )?;
    
        let store = Self { db };
        store.init_schema()?;
    
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        self.db.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS sessions (
                id         TEXT PRIMARY KEY,
                title      TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS messages (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id   TEXT NOT NULL,
                content_json TEXT NOT NULL,
                created_at   TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS call_stats (
                id                   INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id           TEXT NOT NULL,
                provider             TEXT,
                model                TEXT,
                input_tokens         INTEGER,
                output_tokens        INTEGER,
                load_duration        INTEGER,
                prompt_eval_duration INTEGER,
                eval_duration        INTEGER,
                total_duration       INTEGER,
                done_reason          TEXT,
                created_at           TEXT NOT NULL DEFAULT (datetime('now'))
            );
            ",
        )?;
        Ok(())
    }

    // --- Sessions and messages ---

    pub fn save_session(&mut self, session: &ChatSession) -> Result<()> {
        let tx = self.db.transaction()?;

        tx.execute(
            "INSERT INTO sessions (id, title, updated_at)
             VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET title = excluded.title, updated_at = datetime('now')",
            params![session.id, session.title],
        )?;

        tx.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            params![session.id],
        )?;

        {
            let mut stmt = tx.prepare(
                "INSERT INTO messages (session_id, content_json) VALUES (?1, ?2)",
            )?;
            for msg in &session.history {
                let json = serde_json::to_string(msg)?;
                stmt.execute(params![session.id, json])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn load_latest_session(&self) -> Result<Option<ChatSession>> {
        let mut stmt = self.db.prepare(
            "SELECT id, title FROM sessions ORDER BY updated_at DESC LIMIT 1",
        )?;

        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let session = self.load_session(&id, title)?;
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    pub fn load_session(&self, session_id: &str, title: String) -> Result<ChatSession> {
        let mut stmt = self.db.prepare(
            "SELECT content_json FROM messages WHERE session_id = ?1 ORDER BY id ASC",
        )?;

        let rows = stmt.query_map(params![session_id], |row| row.get::<_, String>(0))?;

        let mut history = Vec::new();
        for json_res in rows {
            let msg: Message = serde_json::from_str(&json_res?)?;
            history.push(msg);
        }

        Ok(ChatSession {
            id: session_id.to_string(),
            title,
            history,
        })
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        let mut stmt = self.db.prepare(
            "SELECT s.id, s.title, s.updated_at, COUNT(m.id)
             FROM sessions s
             LEFT JOIN messages m ON m.session_id = s.id
             GROUP BY s.id
             ORDER BY s.updated_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(SessionSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                updated_at: row.get(2)?,
                message_count: row.get::<_, i64>(3)? as usize,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    }

    pub fn delete_session(&self, session_id: &str) -> Result<bool> {
        let affected = self.db.execute(
            "DELETE FROM sessions WHERE id = ?1",
            params![session_id],
        )?;
        Ok(affected > 0)
    }

    pub fn clear_session_messages(&self, session_id: &str) -> Result<()> {
        self.db.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            params![session_id],
        )?;
        Ok(())
    }

    // --- Statistics ---

    pub fn insert_call_stat(&self, rec: &CallRecord) -> Result<()> {
        self.db.execute(
            "INSERT INTO call_stats (
                session_id, provider, model, input_tokens, output_tokens,
                load_duration, prompt_eval_duration, eval_duration,
                total_duration, done_reason
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                rec.session_id,
                rec.provider,
                rec.model,
                rec.input_tokens.map(|v| v as i64),
                rec.output_tokens.map(|v| v as i64),
                rec.load_duration.map(|v| v as i64),
                rec.prompt_eval_duration.map(|v| v as i64),
                rec.eval_duration.map(|v| v as i64),
                rec.total_duration.map(|v| v as i64),
                rec.done_reason,
            ],
        )?;
        Ok(())
    }

    pub fn load_session_stats(&self, session_id: &str) -> Result<Vec<CallRecord>> {
        let mut stmt = self.db.prepare(
            "SELECT session_id, provider, model, input_tokens, output_tokens,
                    load_duration, prompt_eval_duration, eval_duration,
                    total_duration, done_reason
             FROM call_stats WHERE session_id = ?1 ORDER BY id ASC",
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            Ok(CallRecord {
                session_id: row.get(0)?,
                provider: row.get(1)?,
                model: row.get(2)?,
                input_tokens: row.get::<_, Option<i64>>(3)?.map(|v| v as u64),
                output_tokens: row.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                load_duration: row.get::<_, Option<i64>>(5)?.map(|v| v as u64),
                prompt_eval_duration: row.get::<_, Option<i64>>(6)?.map(|v| v as u64),
                eval_duration: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
                total_duration: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
                done_reason: row.get(9)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    }

    pub fn clear_stats(&self) -> Result<()> {
        self.db.execute("DELETE FROM call_stats", [])?;
        Ok(())
    }
}