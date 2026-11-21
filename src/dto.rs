use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::collections::HashMap;
use rig::message::Message;
use crate::models::{settings::ModelSettings,Model};

// ---- Calls ----

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CallRecord {
    pub session_id:           String,
    pub provider:             Option<String>,
    pub model:                Option<String>,
    pub input_tokens:         Option<u64>,
    pub output_tokens:        Option<u64>,
    pub load_duration:        Option<u64>,
    pub prompt_eval_duration: Option<u64>,
    pub eval_duration:        Option<u64>,
    pub total_duration:       Option<u64>,
    pub done_reason:          Option<String>,
}

pub struct CallsStats {
    pub calls: Vec<CallRecord>,
}

impl CallsStats {
    pub fn new(calls: Vec<CallRecord>) -> Self {
        Self { calls }
    }
    
    pub fn format_calls(&self) -> Vec<String> {
        macro_rules! write_line {
            ($dst:expr, $opt:expr, sec $fmt:literal $(, $($arg:tt)*)?) => {
                if let Some(val) = $opt {
                    let _ = writeln!($dst, $fmt, val as f64 / 1e9 $(, $($arg)*)?);
                }
            };
            ($dst:expr, $opt:expr, $fmt:literal $(, $($arg:tt)*)?) => {
                if let Some(val) = $opt {
                    let _ = writeln!($dst, $fmt, val $(, $($arg)*)?);
                }
            };
        }

        self.calls
            .iter()
            .enumerate()
            .map(|(i, metrics)| {
                let mut item = String::new();
                let generation_speed = metrics
                    .output_tokens
                    .zip(metrics.eval_duration)
                    .filter(|&(_, duration)| duration > 0)
                    .map(|(tokens, duration)| tokens as f64 * 1e9 / duration as f64);

                let _ = writeln!(item,                              "Model call         #{}", i + 1);
                write_line!(item, &metrics.provider,                "Provider:          {}");
                write_line!(item, &metrics.model,                   "Model:             {}");
                write_line!(item, metrics.input_tokens,             "Input tokens:      {}");
                write_line!(item, metrics.output_tokens,            "Output tokens:     {}");
                write_line!(item, generation_speed,                 "Generation speed:  {:.2} tokens/sec");
                write_line!(item, metrics.load_duration,        sec "Load time:         {:.3}s");
                write_line!(item, metrics.prompt_eval_duration, sec "Prompt evaluation: {:.3}s");
                write_line!(item, metrics.eval_duration,        sec "Generation time:   {:.3}s");
                write_line!(item, metrics.total_duration,       sec "Ollama total:      {:.3}s");
                write_line!(item, &metrics.done_reason,             "Finish reason:     {}");
                item.to_string()
            })
            .collect()
    }
}

// ---- SavedModelProfile ----

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SavedModelProfile {
    pub id:       String,
    pub name:     String,
    pub preamble: String,
    pub settings: ModelSettings,
}

impl From<&Model> for SavedModelProfile {
    fn from(m: &Model) -> Self {
        Self {
            id: m.id.clone(),
            name: m.name.clone(),
            preamble: m.preamble.clone(),
            settings: m.settings.clone(),
        }
    }
}

// ---- Session ----

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id:            String,
    pub title:         String,
    pub message_count: usize,
    pub updated_at:    String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatSession {
    pub id:      String,
    pub title:   String,
    pub history: Vec<Message>,
}

impl ChatSession {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            history: Vec::new(),
        }
    }

    /// Removes the last turn (the assistant's/tools' responses and the user's latest message)
    pub fn undo_last_turn(&mut self) {
        while let Some(msg) = self.history.pop() {
            if matches!(msg, Message::User { .. }) {
                break;
            }
        }
    }
    
    pub fn strip_reasoning(messages: Vec<Message>) -> Vec<Message> {
        messages
            .into_iter()
            .filter_map(|msg| {
                let mut val = serde_json::to_value(&msg).ok()?;
                if val.get("role").and_then(|r| r.as_str()) == Some("assistant") {
                    if let Some(arr) = val.get_mut("content").and_then(|c| c.as_array_mut()) {
                        arr.retain(|item| {
                            item.get("type").and_then(|t| t.as_str()) != Some("reasoning")
                        });
                        if arr.is_empty() {
                            return None;
                        }
                    }
                }
                serde_json::from_value::<Message>(val).ok()
            })
            .collect()
    }
}