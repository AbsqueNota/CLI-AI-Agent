use std::collections::{HashMap, VecDeque};

pub type AgentId = String;

const MAX_HISTORY_MESSAGES: usize = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Error,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentPhase {
    Starting,
    Reasoning,
    Answering,
    Finished,
    Failed,
}

#[derive(Debug, Clone)]
pub struct AgentViewState {
    pub model_id: String,
    pub phase: AgentPhase,
    pub reasoning: String,
    pub answer: String,
}

impl AgentViewState {
    pub fn new(model_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            phase: AgentPhase::Starting,
            reasoning: String::new(),
            answer: String::new(),
        }
    }
}

#[derive(Debug)]
pub struct UiState {
    pub messages: VecDeque<ChatMessage>,
    pub input: String,
    pub status: String,
    pub agents: HashMap<AgentId, AgentViewState>,
    pub scroll_offset: u16,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            messages: VecDeque::new(),
            input: String::new(),
            status: String::from("Ready"),
            agents: HashMap::new(),
            scroll_offset: 0,
        }
    }
}

impl UiState {
    pub fn push_message(
        &mut self,
        role: MessageRole,
        content: impl Into<String>,
    ) {
        if self.messages.len() >= MAX_HISTORY_MESSAGES {
            self.messages.pop_front();
        }

        self.messages.push_back(ChatMessage {
            role,
            content: content.into(),
        });

        self.scroll_offset = 0;
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    pub fn active_agents(&self) -> usize {
        self.agents.len()
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }
}