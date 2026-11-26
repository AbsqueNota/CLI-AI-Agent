pub mod theme;
pub mod labels;
pub mod mpsc_ui;

use anyhow::Result;
use colored::Colorize;
use dialoguer::{
    theme::Theme,
    Confirm, Input, Select,
};

use std::{
    fmt::Display,
    io::{self, Write},
    sync::Arc,
};
use tokio::sync::mpsc;

use crate::{
    ui::{
        labels::MenuLabels,
        theme::{
            AppTheme,
        },
        mpsc_ui::{
            UiActor,
            UiEvent,
        }
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

    pub fn welcome_hint(&self) {
        println!(
            "\n{} {} {} {} {}\n",
            "Type".color(self.theme.palette.hint),
            "/help".color(self.theme.palette.main).bold(),
            "or".color(self.theme.palette.hint),
            "/h".color(self.theme.palette.main).bold(),
            "for commands.".color(self.theme.palette.hint)
        );
    }

    // ---- Model ----

    pub fn selected_model(&self, model_id: &str) {
        println!(
            "{}{}",
            "🤖:",
            model_id.color(self.theme.palette.main)
        );
    }

    // ---- Dialoguer ----

    pub fn select<T: Display>(
        &self,
        prompt: &str,
        items: &[T],
        default: usize,
        max_length: Option<usize>,
    ) -> Result<Option<usize>> {
        let mut select = Select::with_theme(&*self.theme.select_theme)
            .with_prompt(prompt)
            .items(items)
            .default(default);

        if let Some(max_length) = max_length {
            select = select.max_length(max_length);
        }

        Ok(select.interact_opt()?)
    }

    pub fn input(
        &self,
        prompt: &str,
        allow_empty: bool,
    ) -> Result<String> {
        Ok(
            Input::<String>::with_theme(&*self.theme.input_theme)
                .with_prompt(prompt)
                .allow_empty(allow_empty)
                .interact_text()?,
        )
    }

    pub fn confirm(
        &self,
        prompt: &str,
        default: bool,
    ) -> Result<bool> {
        Ok(
            Confirm::with_theme(&*self.theme.select_theme)
                .with_prompt(prompt)
                .default(default)
                .interact()?,
        )
    }

    pub fn echo_input(&self, prompt: &str, input: &str) -> Result<()> {
        print!("\x1b[2A\x1b[J");
        io::stdout().flush()?;

        let mut formatted = String::new();

        self.theme
            .input_theme
            .format_input_prompt_selection(
                &mut formatted,
                prompt,
                input,
            )?;

        println!("{formatted}");

        Ok(())
    }

    // ---- Status messages ----

    pub fn new_line(&self){
        println!();
    }

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
            format!("Warning: {message}").color(AppTheme::warning_color())
        );
    }

    pub fn info(&self, message: &str) {
        println!(
            "\n{}\n",
            message.color(self.theme.palette.info)
        );
    }

    // ---- MPSC UI ----

    pub fn start_output(&mut self) {
        self.mpsc_ui.start((*self.theme).clone());
    }

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