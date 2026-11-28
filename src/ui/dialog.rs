pub mod state;
use tokio::sync::{mpsc, oneshot};

use crate::app::events::AppEvent;

#[derive(Debug, Clone)]
pub enum DialogSpec {
    Select {
        prompt: String,
        items: Vec<String>,
        selected: usize,
        max_length: Option<usize>,
    },

    Input {
        prompt: String,
        value: String,
        allow_empty: bool,
    },

    Confirm {
        prompt: String,
        default: bool,
    },
}

#[derive(Debug)]
pub enum DialogResult {
    Selected(Option<usize>),
    Input(Option<String>),
    Confirm(bool),
    Cancelled,
}

pub struct DialogRequest {
    pub spec: DialogSpec,
    pub reply: oneshot::Sender<
        Result<DialogResult, String>
    >,
}

#[derive(Clone)]
pub struct DialogClient {
    tx: mpsc::Sender<AppEvent>,
}

impl DialogClient {
    pub fn new(tx: mpsc::Sender<AppEvent>) -> Self {
        Self { tx }
    }

    pub async fn request(
        &self,
        spec: DialogSpec,
    ) -> anyhow::Result<DialogResult> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.tx.send(AppEvent::Dialog(
            DialogRequest {
                spec,
                reply: reply_tx,
            }
        )).await?;

        reply_rx
            .await?
            .map_err(anyhow::Error::msg)
    }

    pub async fn select(
        &self,
        prompt: impl Into<String>,
        items: Vec<String>,
        selected: usize,
        max_length: Option<usize>,
    ) -> anyhow::Result<Option<usize>> {
        match self.request(DialogSpec::Select {
            prompt: prompt.into(),
            items,
            selected,
            max_length,
        }).await? {
            DialogResult::Selected(value) => Ok(value),
            DialogResult::Cancelled => Ok(None),
            _ => anyhow::bail!("Unexpected Dialog result"),
        }
    }

    pub async fn input(
        &self,
        prompt: impl Into<String>,
        allow_empty: bool,
    ) -> anyhow::Result<Option<String>> {
        match self.request(DialogSpec::Input {
            prompt: prompt.into(),
            value: String::new(),
            allow_empty,
        }).await? {
            DialogResult::Input(value) => Ok(value),
            DialogResult::Cancelled => Ok(None),
            _ => anyhow::bail!("Unexpected Dialog result"),
        }
    }

    pub async fn confirm(
        &self,
        prompt: impl Into<String>,
        default: bool,
    ) -> anyhow::Result<Option<bool>> {
        match self.request(DialogSpec::Confirm {
            prompt: prompt.into(),
            default,
        }).await? {
            DialogResult::Confirm(value) => Ok(Some(value)),
            DialogResult::Cancelled => Ok(None),
            _ => anyhow::bail!("Unexpected Dialog result"),
        }
    }
}