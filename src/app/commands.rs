pub mod snippets;

use anyhow::{Result};
use crate::app::AppContext;
use super::commands::snippets::{
    settings_menu,
    select_provider,
    new_session,
    undo_last_turn,
    clear_history,
    manage_sessions,
    prepare_report,
    show_models,
    select_model,
    show_capabilities,
    handle_set_command,
    stop_current_model,
    help,
};

pub async fn execute_command(
    command: &str,
    ctx: &mut AppContext,
) -> Result<bool> {
    let AppContext { state, ui } = ctx;
    let mut parts = command.trim().splitn(3, ' ');
    let cmd = parts.next().unwrap_or("");
    let arg1 = parts.next();
    let arg2 = parts.next();

    let result: Result<String> = async {
        let msg = match cmd {
            // ---- Settings ----
            "/settings"     | "/cfg"    => settings_menu(state, ui).await?,
            "/set"                      => handle_set_command(state, ui, arg1, arg2)?,

            // ---- Provider ----
            "/provider"     | "/pr"     => select_provider(state, ui).await?,
            
            // ---- Sessions and history ----
            "/new"          | "/n"      => new_session(state)?,
            "/sessions"     | "/ss"     => manage_sessions(state, ui)?,
            "/undo"         | "/u"      => undo_last_turn(state)?,
            "/clear"        | "/cl"     => clear_history(state)?,

            // ---- Models ----
            "/models"       | "/mls"    => show_models(state),
            "/model"        | "/ml"     => select_model(state, ui).await?,
            "/capabilities" | "/cp"     => show_capabilities(state),            
            "/stop"         | "/st"     => stop_current_model(state).await?,

            // ---- Statistics and reference information ----
            "/stats"        | "/sts"    => prepare_report(state.stats.format_calls()),
            "/help"         | "/h"      => help(),
            
            _ => "Unknown command. Use /help.".to_string(),
        };
        Ok(msg)
    }.await;

    match result {
        Ok(message) => {
            ui.info(&message);
            Ok(true)
        }
        Err(error) => {
            ui.error(&format!("[execute_command]: {error:#}"));
            Ok(false)
        }
    }
}

