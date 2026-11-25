
use std::{
    path::PathBuf,
    thread::{self, JoinHandle},
};

use anyhow::{Context, Result, anyhow};
use tokio::sync::{mpsc, oneshot};

use crate::dto::{CallRecord, ChatSession};

use super::sqlite::SqliteStore;

const CHANNEL_CAPACITY: usize = 128;

type Ack = std::result::Result<(), String>;

enum DbCommand {
    SaveSession {
        session: ChatSession,
        reply: oneshot::Sender<Ack>,
    },
    InsertCallStat {
        record: CallRecord,
        reply: oneshot::Sender<Ack>,
    },
    DeleteSession {
        session_id: String,
        reply: oneshot::Sender<std::result::Result<bool, String>>,
    },
    ClearSessionMessages {
        session_id: String,
        reply: oneshot::Sender<Ack>,
    },
    ClearStats {
        reply: oneshot::Sender<Ack>,
    },
    Flush {
        reply: oneshot::Sender<Ack>,
    },
    Shutdown {
        reply: oneshot::Sender<Ack>,
    },
}

pub struct DbActor {
    tx: mpsc::Sender<DbCommand>,
    thread: Option<JoinHandle<()>>,
}

impl DbActor {
    pub fn spawn(data_dir: PathBuf, db_name: String) -> Result<Self> {
        let (tx, mut rx) = mpsc::channel::<DbCommand>(CHANNEL_CAPACITY);
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);

        let handle = thread::Builder::new()
            .name("sqlite-writer".into())
            .spawn(move || {
                let mut store = match SqliteStore::open(&data_dir, &db_name) {
                    Ok(store) => {
                        let _ = ready_tx.send(Ok::<(), String>(()));
                        store
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(format!("{error:#}")));
                        return;
                    }
                };

                while let Some(command) = rx.blocking_recv() {
                    match command {
                        DbCommand::SaveSession { session, reply } => {
                            let result = store
                                .save_session(&session)
                                .map_err(|error| format!("{error:#}"));

                            let _ = reply.send(result);
                        }

                        DbCommand::InsertCallStat { record, reply } => {
                            let result = store
                                .insert_call_stat(&record)
                                .map_err(|error| format!("{error:#}"));

                            let _ = reply.send(result);
                        }

                        DbCommand::DeleteSession { session_id, reply } => {
                            let result = store
                                .delete_session(&session_id)
                                .map_err(|error| format!("{error:#}"));

                            let _ = reply.send(result);
                        }

                        DbCommand::ClearSessionMessages { session_id, reply } => {
                            let result = store
                                .clear_session_messages(&session_id)
                                .map_err(|error| format!("{error:#}"));

                            let _ = reply.send(result);
                        }

                        DbCommand::ClearStats { reply } => {
                            let result = store
                                .clear_stats()
                                .map_err(|error| format!("{error:#}"));

                            let _ = reply.send(result);
                        }

                        DbCommand::Flush { reply } => {
                            // Все предыдущие команды уже обработаны.
                            let _ = reply.send(Ok(()));
                        }

                        DbCommand::Shutdown { reply } => {
                            let _ = reply.send(Ok(()));
                            break;
                        }
                    }
                }
            })
            .context("Failed to spawn SQLite writer thread")?;

        ready_rx
            .recv()
            .context("SQLite writer thread failed during initialization")?
            .map_err(|error| anyhow!("Failed to initialize SQLite writer: {error}"))?;

        Ok(Self {
            tx,
            thread: Some(handle),
        })
    }

    async fn send_ack(
        &self,
        build: impl FnOnce(oneshot::Sender<Ack>) -> DbCommand,
    ) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.tx
            .send(build(reply_tx))
            .await
            .context("SQLite writer channel is closed")?;

        reply_rx
            .await
            .context("SQLite writer dropped the response")?
            .map_err(|error| anyhow!(error))
    }

    pub async fn save_session(&self, session: ChatSession) -> Result<()> {
        self.send_ack(|reply| DbCommand::SaveSession { session, reply })
            .await
    }

    pub async fn insert_call_stat(&self, record: CallRecord) -> Result<()> {
        self.send_ack(|reply| DbCommand::InsertCallStat { record, reply })
            .await
    }

    pub async fn clear_session_messages(&self, session_id: String) -> Result<()> {
        self.send_ack(|reply| DbCommand::ClearSessionMessages {
            session_id,
            reply,
        })
        .await
    }

    pub async fn clear_stats(&self) -> Result<()> {
        self.send_ack(|reply| DbCommand::ClearStats { reply })
            .await
    }

    pub async fn delete_session(&self, session_id: String) -> Result<bool> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.tx
            .send(DbCommand::DeleteSession {
                session_id,
                reply: reply_tx,
            })
            .await
            .context("SQLite writer channel is closed")?;

        reply_rx
            .await
            .context("SQLite writer dropped the response")?
            .map_err(|error| anyhow!(error))
    }

    pub async fn flush(&self) -> Result<()> {
        self.send_ack(|reply| DbCommand::Flush { reply })
            .await
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.tx
            .send(DbCommand::Shutdown { reply: reply_tx })
            .await
            .context("SQLite writer channel is closed")?;

        reply_rx
            .await
            .context("SQLite writer dropped the shutdown response")?
            .map_err(|error| anyhow!(error))?;

        if let Some(handle) = self.thread.take() {
            let joined = tokio::task::spawn_blocking(move || handle.join())
                .await
                .context("Failed to join SQLite writer thread")?;

            if joined.is_err() {
                anyhow::bail!("SQLite writer thread panicked");
            }
        }

        Ok(())
    }
}
