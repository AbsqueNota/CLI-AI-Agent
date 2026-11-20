pub mod snippets;

use anyhow::{Result};
use crate::app::state::AppState;
use super::commands::snippets::{
    new_session,
    undo_last_turn,
    clear_history,
    manage_sessions,
    prepare_report,
    show_models,
    select_model,
    show_capabilities,
    show_settings,
    handle_set_command,
    help,
};

pub async fn execute_command(
    command: &str,
    state: &mut AppState,
) -> Result<bool> {
    let mut parts = command.trim().splitn(3, ' ');
    let cmd = parts.next().unwrap_or("");
    let arg1 = parts.next();
    let arg2 = parts.next();

    let result: Result<String> = async {
        let msg = match cmd {
            // ---- Sessions and history ----
            "/new"                      => new_session(state)?,
            "/sessions"     | "/ss"     => manage_sessions(state)?,
            "/undo"         | "/u"      => undo_last_turn(state)?,
            "/clear"        | "/cl"     => clear_history(state)?,

            // ---- Models and settings ----
            "/models"       | "/ms"     => show_models(state),
            "/model"        | "/m"      => select_model(state).await?,
            "/capabilities" | "/cp"     => show_capabilities(state),
            "/settings"     | "/cfg"    => show_settings(&state.current_model),
            "/set"          | "/s"      => handle_set_command(state, arg1, arg2)?,

            // ---- Statistics and reference information ----
            "/stats"        | "/st"     => prepare_report(state.stats.format_calls()),
            "/help"         | "/h"      => help(),
            
            _ => "Unknown command. Use /help.".to_string(),
        };
        Ok(msg)
    }.await;

    match result {
        Ok(status_message) => {
            println!("\n{status_message}\n");
            Ok(true)
        }
        Err(e) => {
            eprintln!("\n [err][execute_command]: {e:#}\n");
            Ok(false)
        }
    }
}

