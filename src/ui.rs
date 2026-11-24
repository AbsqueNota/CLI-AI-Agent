pub mod theme;
pub mod labels;

use anyhow::{Result, Context};
use colored::{Colorize};
use dialoguer::{
    theme::Theme,
    Input, Confirm, Select
};
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    fmt::Display,
    io::{self, Write},
    sync::Arc,
    time::Duration,
};
use crate::ui::theme::AppTheme;
use crate::ui::labels::MenuLabels;


pub struct TerminalUi {
    theme: Arc<AppTheme>,
    menu_labels: MenuLabels,
    is_printed_reasoning:bool,
    is_printed_text:bool,
}

impl TerminalUi {
    pub fn new(theme: AppTheme) -> Self {
        Self {
            theme: Arc::new(theme),
            menu_labels: MenuLabels::default(),
            is_printed_reasoning: false,
            is_printed_text: false
        }
    }

    pub fn set_theme(&mut self, theme: AppTheme) {
        self.theme = Arc::new(theme);
    }

    fn init_printed_flag_stream(&mut self) {
        self.is_printed_reasoning = false;
        self.is_printed_text = false;
    }

    pub fn welcome(&self) {
        println!(
            "\n{}",
            "✦ Local AI Assistant ✦"
                .color(self.theme.text_light)
                .bold()
        );

        println!(
            "{} {} {}\n",
            "Type".color(self.theme.text_muted),
            "/help".color(self.theme.accent_pink).bold(),
            "for commands.".color(self.theme.text_muted)
        );
    }

    pub fn model_prompt(&self, model_id: &str) {
        println!(
            "{} {}",
            "🤖:",
            model_id.color(self.theme.text_muted)
        );
    }

    pub fn select<T: Display>(
        &self,
        prompt: &str,
        items: &[T],
        default: usize,
        max_length: Option<usize>,
    ) -> Result<Option<usize>> {
        let mut select = Select::with_theme(
            &*self.theme.dialoguer_theme
        )
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
            Input::<String>::with_theme(
                &*self.theme.dialoguer_theme
            )
            .with_prompt(prompt)
            .allow_empty(allow_empty)
            .interact_text()?
        )
    }

    pub fn confirm(
        &self,
        prompt: &str,
        default: bool,
    ) -> Result<bool> {
        Ok(
            Confirm::with_theme(
                &*self.theme.dialoguer_theme
            )
            .with_prompt(prompt)
            .default(default)
            .interact()?
        )
    }

    pub fn echo_input(&self,prompt: &str, input: &str) -> Result<()> {
        print!("\x1b[2A\x1b[J");
        io::stdout().flush()?;

        let mut formatted = String::new();

        self.theme
            .dialoguer_theme
            .format_input_prompt_selection(
                &mut formatted,
                prompt,
                input,
            )?;

        println!("{formatted}");

        Ok(())
    }

    pub fn goodbye(&self) {
        println!(
            "👋 {}",
            "Goodbye!".color(self.theme.accent_pink)
        );
    }

    pub fn error(&self, message: &str) {
        eprintln!(
            "\n{}",
            message.color(self.theme.text_error).bold()
        );
    }

    pub fn warning(&self, message: &str) {
        eprintln!(
            "\n{}",
            format!("Warning: {message}")
                .color(self.theme.text_warning)
        );
    }

    pub fn info(&self, message: &str) {
        println!("\n{message}\n");
    }

    pub fn build_spinner(&self, model_id: &str) -> Result<ProgressBar> {
        let color = self.theme.text_muted;

        let model = model_id.color(self.theme.text_light);
        let message = format!(
            "\n🤖: {model} {}",
            "is thinking".color(color)
        );

        let spinner = [
            "".to_string(),
            ".".color(color).to_string(),
            "..".color(color).to_string(),
            "...".color(color).to_string(),
            "".to_string(),
        ];

        let pb = ProgressBar::new_spinner();

        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(
                    &spinner.each_ref().map(|s| s.as_str())
                )
                .template("{msg}{spinner}")
                .context("Invalid spinner template")?,
        );

        pb.set_message(message);
        pb.enable_steady_tick(Duration::from_millis(150));
        io::stdout().flush()?;  
        Ok(pb)
    }

    pub fn write_text(&mut self, text: &str) -> Result<()> {
        if self.is_printed_reasoning && !self.is_printed_text {
            println!("\n");
        }
        print!("{text}");
        io::stdout().flush()?;
        self.is_printed_text = true;
        Ok(())
    }
    
    pub fn write_reasoning(&mut self, text: &str) -> Result<()> {
        print!("{}", text.color(self.theme.dark_gray));
        io::stdout().flush()?;
        self.is_printed_reasoning = true;
        Ok(())
    }
    
    pub fn stream_finished(&mut self, elapsed_seconds: f64) {
        self.init_printed_flag_stream();
        println!("\nFinished in {elapsed_seconds:.2}s");
    }

    pub fn stream_error(
        &self,
        model_id: &str,
        error: &impl std::fmt::Display,
    ) {
        eprintln!(
            "\n{}",
            format!("{model_id} error: {error}")
                .color(self.theme.text_error)
        );
    }

    pub fn theme(&self) -> &AppTheme {
        &self.theme
    }

    pub fn labels(&self) -> &MenuLabels {
        &self.menu_labels
    }
}