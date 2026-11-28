use anyhow::{bail, Result};
use std::sync::Arc;
use uuid::Uuid;
use tokio::{
    sync::mpsc,
    task::JoinHandle,
};

use crate::{
    agent::runtime::spawn_agent,
    app::{
        events::{AppCommand, AppEvent},
        state::AppState,
    },
    provider::LlmProvider,
};

struct ActiveAgent {
    id: String,
    task: JoinHandle<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchResult {
    Continue,
    Exit,
}

pub struct Dispatcher {
    tx: mpsc::Sender<AppEvent>,
    active: Option<ActiveAgent>,
}

impl Dispatcher {
    pub fn new(tx: mpsc::Sender<AppEvent>) -> Self {
        Self {
            tx,
            active: None,
        }
    }

    pub fn is_agent_running(&self) -> bool {
        self.active.is_some()
    }

    pub async fn dispatch(
        &mut self,
        command: AppCommand,
        state: &AppState,
    ) -> Result<DispatchResult> {
        match command {
            AppCommand::Submit(prompt) => {
                self.start_agent(prompt, state)?;
            }

            AppCommand::Interrupt => {
                self.interrupt().await?;
            }

            AppCommand::Exit => {
                self.interrupt().await?;
                return Ok(DispatchResult::Exit);
            }
        }

        Ok(DispatchResult::Continue)
    }

    fn start_agent(
        &mut self,
        prompt: String,
        state: &AppState,
    ) -> Result<()> {
        if self.active.is_some() {
            bail!("An agent is already running");
        }

        let agent_id = Uuid::now_v7().to_string();

        let provider: Arc<dyn LlmProvider> =
            Arc::clone(&state.provider);

        let model = state.current_model.clone();
        let history = state.session.history.clone();

        let task = spawn_agent(
            agent_id.clone(),
            provider,
            model,
            history,
            prompt,
            self.tx.clone(),
        );

        self.active = Some(ActiveAgent {
            id: agent_id,
            task,
        });

        Ok(())
    }

    pub async fn interrupt(&mut self) -> Result<()> {
        let Some(active) = self.active.take() else {
            return Ok(());
        };

        active.task.abort();

        let _ = active.task.await;

        self.tx.send(AppEvent::AgentCancelled {
            agent_id: active.id,
        }).await?;

        Ok(())
    }

    pub async fn on_agent_terminal(
        &mut self,
        agent_id: &str,
    ) -> bool {
        let matches = self.active
            .as_ref()
            .is_some_and(|active| active.id == agent_id);
    
        if !matches {
            return false;
        }
    
        if let Some(active) = self.active.take() {
            let _ = active.task.await;
        }
    
        true
    }
}