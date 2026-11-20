use anyhow::Result;
use dialoguer::Input;
use dialoguer::theme::Theme;
use colored::Colorize;
use std::io::{self, Write};

use crate::{
    app::{commands::execute_command, state::AppState},
    agent::runtime::run_stream,
};

pub async fn chat_loop(
    state: &mut AppState,
) -> Result<()> {
    println!("\n{}",
        "✦ Local AI Assistant ✦".color(state.config.theme.text_light).bold());
    println!("{} {} {}",
        "Type".color(state.config.theme.text_muted),
        "/help".color(state.config.theme.accent_pink).bold(),
        "for commands.\n".color(state.config.theme.text_muted));
   
    loop {
        println!("{} {}", "🤖:",
            state.current_model.id.color(state.config.theme.text_muted),
        );
        let input: String = Input::with_theme(&*state.config.theme.dialoguer_theme)
            .with_prompt("You")
            .interact_text()?;
        let input = input.trim();

        print!("\x1b[2A\x1b[J");
        io::stdout().flush()?;
        
        let mut formatted_line = String::new();
            state
            .config
            .theme
            .dialoguer_theme
            .format_input_prompt_selection(&mut formatted_line, "You", input)?;
        println!("{}", formatted_line);

        if input.is_empty() {
            continue;
        }

        if input == "/exit" || input == "/q" {
            println!("👋 {}", "Goodbye!".color(state.config.theme.accent_pink));
            break;
        }

        if input.starts_with('/') {
            execute_command(input, state).await?;
            continue;
        }

        if let Err(error) = run_stream(state, input).await {
            eprintln!("\nAgent error: {error}");
        }
    }

    Ok(())
}
