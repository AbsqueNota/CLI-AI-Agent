
pub mod theme;
pub mod labels;
pub mod mpsc_ui;
pub mod terminal;
pub mod state;
pub mod renderer;
pub mod menu;
pub mod input;
pub mod dialog;

use anyhow::{anyhow, Result};
use std::{
    fmt::Display,
    sync::Arc,
};

use rig::message::Message;
use tokio::sync::mpsc;

use crate::ui::{
    labels::MenuLabels,
    theme::AppTheme,
    state::{MessageRole, UiState},
    mpsc_ui::{
        UiActor,
        UiAction,
        UiEvent,
    },
};

pub struct TerminalUi {
    theme: Arc<AppTheme>,
    menu_labels: MenuLabels,
    mpsc_ui: UiActor,
    is_suspended: bool,
}

impl TerminalUi {
    pub fn new(theme: AppTheme) -> Self {
        Self {
            theme: Arc::new(theme),
            menu_labels: MenuLabels::default(),
            mpsc_ui: UiActor::new(),
            is_suspended: false,
        }
    }

    // ---- Chat ----

    pub fn start_chat(
        &mut self,
        history: &[Message],
    ) -> Result<mpsc::Receiver<UiAction>> {
        let history = UiState::messages_from_history(history);

        self.mpsc_ui.start(
            (*self.theme).clone(),
            history,
        );

        self.mpsc_ui
            .take_actions()
            .ok_or_else(|| anyhow!("Chat UI actions already taken"))
    }

    pub async fn suspend_chat(&mut self) -> Result<()> {
        self.mpsc_ui.suspend().await?;
        self.is_suspended = true;
        Ok(())
    }
    
    pub async fn resume_chat(
        &mut self,
        history: &[Message],
    ) -> Result<()> {
        let history = UiState::messages_from_history(history);
    
        self.mpsc_ui
            .resume((*self.theme).clone(), history)
            .await?;
    
        self.is_suspended = false;
    
        Ok(())
    }

    pub fn set_theme(&mut self, theme: AppTheme) {
        self.theme = Arc::new(theme);
    }

    // ---- Ratatui menus ----

    pub fn select<T: Display>(
        &self,
        prompt: &str,
        items: &[T],
        default: usize,
        max_length: Option<usize>,
    ) -> Result<Option<usize>> {
        menu::select(
            &self.theme,
            prompt,
            items,
            default,
            max_length,
        )
    }

    pub fn input(
        &self,
        prompt: &str,
        allow_empty: bool,
    ) -> Result<String> {
        menu::input(
            &self.theme,
            prompt,
            allow_empty,
        )
    }

    pub fn confirm(
        &self,
        prompt: &str,
        default: bool,
    ) -> Result<bool> {
        menu::confirm(
            &self.theme,
            prompt,
            default,
        )
    }

    // ---- Notifications ----

    fn notify(
        &self,
        role: MessageRole,
        title: &str,
        message: &str,
    ) -> Result<()> {
        if self.mpsc_ui.is_started() && !self.is_suspended {
            self.mpsc_ui
                .sender()
                .try_send(UiEvent::Notice {
                    role,
                    message: message.to_string(),
                    ack: None,
                })
                .map_err(|error| {
                    anyhow!("Failed to enqueue UI notification: {error}")
                })?;
        } else {
            menu::notice(&self.theme, title, message)?;
        }
        Ok(())
    }

    pub fn info(&self, message: &str) -> Result<()> {
        self.notify(
            MessageRole::Info,
            "Info",
            message,
        )
    }

    pub fn warning(&self, message: &str) -> Result<()> {
        self.notify(
            MessageRole::Warning,
            "Warning",
            message,
        )
    }

    pub fn error(&self, message: &str) -> Result<()> {
        self.notify(
            MessageRole::Error,
            "Error",
            message,
        )
    }

    pub fn goodbye(&self) -> Result<()> {
        menu::notice(
            &self.theme,
            "Goodbye",
            "Local AI Assistant has been stopped.",
        )
    }

    // ---- MPSC UI ----

    pub fn event_sender(&self) -> mpsc::Sender<UiEvent> {
        self.mpsc_ui.sender()
    }

    pub async fn shutdown_output(&mut self) -> Result<()> {
        self.mpsc_ui.shutdown().await
    }

    // ---- Accessors ----

    pub fn theme(&self) -> &AppTheme {
        &self.theme
    }

    pub fn labels(&self) -> &MenuLabels {
        &self.menu_labels
    }
}
