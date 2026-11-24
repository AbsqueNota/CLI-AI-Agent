use anyhow::Result;
use colored::{Color as ColoredColor, Colorize};
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
use crate::app::theme::AppTheme;

pub struct TerminalUi {
    theme: Arc<AppTheme>,
}

impl TerminalUi {
    pub fn new(theme: AppTheme) -> Self {
        Self {
            theme: Arc::new(theme),
        }
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
        eprintln!("\n{message}");
    }

    pub fn info(&self, message: &str) {
        println!("\n{message}\n");
    }

    pub fn build_spinner(&self, model_id: &str) -> ProgressBar {
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
                .expect("Invalid spinner template"),
        );

        pb.set_message(message);
        pb.enable_steady_tick(Duration::from_millis(150));

        pb
    }

    pub fn theme(&self) -> &AppTheme {
        &self.theme
    }
}