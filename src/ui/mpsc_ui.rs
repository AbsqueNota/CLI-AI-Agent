use std::collections::HashMap;
use tokio::sync::mpsc;

pub type AgentId = String;

pub const UI_CHANNEL_CAPACITY: usize = 256;

#[derive(Debug)]
pub enum UiEvent {
    Started {
        agent_id: AgentId,
        model_id: String,
    },
    Reasoning {
        agent_id: AgentId,
        text: String,
    },
    Chunk {
        agent_id: AgentId,
        text: String,
    },
    Finished {
        agent_id: AgentId,
        elapsed: f64,
    },
    Failed {
        agent_id: AgentId,
        error: String,
    },
}

#[derive(Debug, Default)]
pub struct AgentBuffer {
    pub model_id: String,
    pub reasoning: String,
    pub output: String,
    pub finished: bool,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct UiState {
    pub agents: HashMap<AgentId, AgentBuffer>,
}

pub struct UiActor {
    tx: mpsc::Sender<UiEvent>,
}

impl UiActor {
    pub fn new() -> (Self, mpsc::Receiver<UiEvent>) {
        let (tx, rx) = mpsc::channel(UI_CHANNEL_CAPACITY);
        (Self { tx }, rx)
    }

    pub fn sender(&self) -> mpsc::Sender<UiEvent> {
        self.tx.clone()
    }
}

impl UiState {
    pub fn handle(&mut self, event: UiEvent) {
        match event {
            UiEvent::Started { agent_id, model_id } => {
                self.agents.insert(
                    agent_id,
                    AgentBuffer {
                        model_id,
                        ..Default::default()
                    },
                );
            }

            UiEvent::Reasoning { agent_id, text } => {
                if let Some(agent) = self.agents.get_mut(&agent_id) {
                    agent.reasoning.push_str(&text);
                }
            }

            UiEvent::Chunk { agent_id, text } => {
                if let Some(agent) = self.agents.get_mut(&agent_id) {
                    agent.output.push_str(&text);
                }
            }

            UiEvent::Finished { agent_id, .. } => {
                if let Some(agent) = self.agents.get_mut(&agent_id) {
                    agent.finished = true;
                }
            }

            UiEvent::Failed { agent_id, error } => {
                if let Some(agent) = self.agents.get_mut(&agent_id) {
                    agent.error = Some(error);
                    agent.finished = true;
                }
            }
        }
    }
}