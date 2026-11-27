pub mod theme;
pub mod labels;
pub mod mpsc_ui;
pub mod terminal;
pub mod state;
pub mod renderer;
pub mod menu;

use anyhow::Result;
use colored::Colorize;

use std::{
    io::{self, Write},
    fmt::Display,
    sync::Arc,
};

use tokio::sync::mpsc;

use crate::ui::{
    labels::MenuLabels,
    theme::AppTheme,
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
}

impl TerminalUi {
    pub fn new(theme: AppTheme) -> Self {
        Self {
            theme: Arc::new(theme),
            menu_labels: MenuLabels::default(),
            mpsc_ui: UiActor::new(),
        }
    }

    // ---- Chat ----

    pub fn start_chat(&mut self) -> Result<mpsc::Receiver<UiAction>> {
        self.mpsc_ui.start((*self.theme).clone());

        self.mpsc_ui
            .take_actions()
            .ok_or_else(|| anyhow::anyhow!("Chat UI actions already taken"))
    }

    pub async fn suspend_chat(&self) -> Result<()> {
        self.mpsc_ui.suspend().await?;
        Ok(())
    }

    pub async fn resume_chat(&self) -> Result<()> {
        self.mpsc_ui.resume((*self.theme).clone()).await?;
        Ok(())
    }

    pub fn set_theme(&mut self, theme: AppTheme) {
        self.theme = Arc::new(theme);
    }

    // ---- Welcome ----

    pub fn welcome_title(&self) {
        println!(
            "\n{}",
            "✦ Local AI Assistant ✦"
                .color(self.theme.palette.main)
                .bold()
        );
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

    // ---- Status messages ----

    pub fn goodbye(&self) {
        println!(
            "\n👋 {}\n",
            "Goodbye!".color(self.theme.palette.info)
        );
    }

    pub fn error(message: &str) {
        eprintln!(
            "\n{}",
            message.color(AppTheme::error_color()).bold()
        );
    }

    pub fn warning(message: &str) {
        eprintln!(
            "\n{}",
            format!("Warning: {message}")
                .color(AppTheme::warning_color())
        );
    }

    pub fn info(&self, message: &str) {
        println!(
            "\n{}\n",
            message.color(self.theme.palette.info)
        );
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