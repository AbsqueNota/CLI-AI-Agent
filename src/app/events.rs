use crossterm::event::KeyEvent;
use rig::message::Message;

use crate::dto::CallRecord;
use crate::ui::dialog::DialogRequest;

pub const EVENT_CHANNEL_CAPACITY: usize = 256;

#[derive(Debug)]
pub enum AppCommand {
    Submit(String),
    Interrupt,
    Exit,
}

#[derive(Debug, Clone, Copy)]
pub enum NoticeKind {
    Info,
    Warning,
    Error,
}

pub enum AppEvent {
    
    Key(KeyEvent),
    Resize {
        width: u16,
        height: u16,
    },

    Command(AppCommand),

    Dialog(DialogRequest),

    Notice {
        kind: NoticeKind,
        message: String,
    },

    AgentStarted {
        agent_id: String,
        model_id: String,
    },

    AgentReasoning {
        agent_id: String,
        text: String,
    },

    AgentChunk {
        agent_id: String,
        text: String,
    },

    AgentCall {
        agent_id: String,
        record: CallRecord,
    },

    AgentFinished {
        agent_id: String,
        prompt: String,
        messages: Vec<Message>,
        elapsed: f64,
    },

    AgentFailed {
        agent_id: String,
        error: String,
    },

    AgentCancelled {
        agent_id: String,
    },
}