use std::collections::{HashMap, VecDeque};
use rig::message::Message;
use serde_json::Value;
use crate::ui::input::Input;

pub type AgentId = String;

const MAX_HISTORY_MESSAGES: usize = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
    Info,
    Warning,
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
    pub notices:  VecDeque<ChatMessage>,
    pub input:    Input,
    pub status:   String,
    pub agents:   HashMap<AgentId, AgentViewState>,
    pub scroll_offset: u16,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            messages: VecDeque::new(),
            notices:  VecDeque::new(),
            input:    Input::new(),
            status:   String::from("Ready"),
            agents:   HashMap::new(),
            scroll_offset: 0,
        }
    }
}

impl UiState {
    pub fn messages_from_history(
        history: &[Message],
    ) -> VecDeque<ChatMessage> {
        let mut messages = VecDeque::new();

        for message in history {
            let Ok(value) = serde_json::to_value(message) else {
                continue;
            };

            let Some(role) = value
                .get("role")
                .and_then(Value::as_str)
            else {
                continue;
            };

            let role = match role {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                "system" => MessageRole::System,
                "tool" => MessageRole::Tool,
                _ => continue,
            };

            let Some(content) = value.get("content") else {
                continue;
            };

            let text = match content {
                Value::String(text) => text.clone(),

                Value::Array(parts) => parts
                    .iter()
                    .filter_map(|part| {
                        part.get("text")
                            .and_then(Value::as_str)
                    })
                    .collect::<String>(),

                Value::Object(object) => object
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),

                _ => String::new(),
            };

            if text.is_empty() {
                continue;
            }

            if messages.len() >= MAX_HISTORY_MESSAGES {
                messages.pop_front();
            }

            messages.push_back(ChatMessage {
                role,
                content: text,
            });
        }

        messages
    }

    /// Заменяет только историю диалога.
    /// Уведомления и активные агенты сохраняются.
    pub fn load_history(&mut self, history: &[Message]) {
        self.messages = Self::messages_from_history(history);
        self.scroll_to_bottom();
    }

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

        self.scroll_to_bottom();
    }

    pub fn push_notice(
        &mut self,
        role: MessageRole,
        content: impl Into<String>,
    ) {
        if self.notices.len() >= MAX_HISTORY_MESSAGES {
            self.notices.pop_front();
        }

        self.notices.push_back(ChatMessage {
            role,
            content: content.into(),
        });

        self.scroll_to_bottom();
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
