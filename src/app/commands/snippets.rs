use anyhow::{anyhow, Result};
use std::fmt::{Write,Display};
use dialoguer::{theme::ColorfulTheme, Input, Select};
use crate::{
    app::state::AppState, 
    models::{
        CapabilityKey, 
        Model, 
        settings::{
            MirostatMode, 
            ThinkingConfig,
        }
    },
};

// ---- Provider ----
pub async fn select_provider(
    state: &mut AppState,
) -> Result<String> {
    let providers = crate::provider::ProviderFactory::available();

    let default_index = providers
        .iter()
        .position(|provider| *provider == state.provider.id())
        .unwrap_or(0);

    let selection = Select::with_theme(
        &*state.config.theme.dialoguer_theme
    )
        .with_prompt("Select LLM provider")
        .items(providers)
        .default(default_index)
        .interact_opt()?;

    let Some(index) = selection else {
        return Ok("Provider selection cancelled.".to_string());
    };

    let selected = providers[index];

    if selected == state.provider.id() {
        return Ok(format!(
            "Provider '{}' is already active.",
            selected
        ));
    }

    state.switch_provider(selected).await?;

    Ok(format!(
        "Switched provider to '{}'. Current model: {}",
        state.provider.id(),
        state.current_model.id,
    ))
}

// ---- Sessions and history ----
pub fn new_session(state: &mut AppState) -> Result<String> {
    state.start_new_session()?;
    Ok(format!("Started a new session ({}).", state.session.id))
}

pub fn undo_last_turn(state: &mut AppState) -> Result<String> {
    if state.session.history.is_empty() {
        return Ok("Nothing to undo — conversation history is empty.".to_string());
    }
    state.undo_last_turn()?;
    Ok("Removed last user message and model response.".to_string())
}

pub fn clear_history(state: &mut AppState) -> Result<String> {
    state.clear_current_session()?;
    Ok("Conversation history cleared.".to_string())
}

pub fn manage_sessions(state: &mut AppState) -> Result<String> {
    let sessions = state.storage.list_sessions()?;
    if sessions.is_empty() {
        return Ok("No saved sessions found.".to_string());
    }

    let theme = &*state.config.theme.dialoguer_theme;

    let items: Vec<String> = sessions
        .iter()
        .map(|s| {
            let active = if s.id == state.session.id { " [ACTIVE]" } else { "" };
            format!("{} ({} msgs, {}){}", s.title, s.message_count, s.updated_at, active)
        })
        .collect();

    let default_idx = sessions
        .iter()
        .position(|s| s.id == state.session.id)
        .unwrap_or(0);

    let Some(selection) = Select::with_theme(theme)
        .with_prompt("Select session")
        .items(&items)
        .default(default_idx)
        .interact_opt()?
    else {
        return Ok("Session selection cancelled.".to_string());
    };

    let chosen = &sessions[selection];
    let actions = ["Switch to this session", "Delete this session", "Cancel"];
    
    let Some(action) = Select::with_theme(theme)
        .with_prompt(format!("Action for '{}'", chosen.title))
        .items(&actions)
        .default(0)
        .interact_opt()?
    else {
        return Ok("Action cancelled.".to_string());
    };

    match action {
        0 => {
            if chosen.id == state.session.id {
                return Ok(format!("Session '{}' is already active.", chosen.title));
            }
            state.switch_session(&chosen.id, chosen.title.clone())?;
            Ok(format!("Switched to session '{}'.", chosen.title))
        }
        1 => {
            state.delete_session_by_id(&chosen.id)?;
            Ok(format!("Deleted session '{}'.", chosen.title))
        }
        _ => Ok("Cancelled.".to_string()),
    }
}

// --- Models ----
pub fn show_models(state: &AppState) -> String{
    let mut result = String::from("\nAvailable models:\n");
    for model in state.registry.by_provider(state.provider.id()) {
        let active = if model.id == state.current_model.id 
            {" [active]" } else { "" };
        result.push_str(&format!("\n- {}{}", model.id, active));
    }
    return result;
}

pub async fn select_model(state: &mut AppState) -> Result<String> {
    let models = state.registry
        .by_provider(state.provider.id());

    if models.is_empty() {
        return Ok(String::from("No models available."));
    }

    let names: Vec<String> = models
        .iter()
        .map(|model| model.id.clone())
        .collect();

    let default = names
        .iter()
        .position(|name| name == state.current_model_name())
        .unwrap_or(0);

    let selection = Select::with_theme(&*state.config.theme.dialoguer_theme)
        .with_prompt("Select model")
        .items(&names)
        .default(default)
        .interact_opt()?;

    if let Some(index) = selection {
        let selected = &names[index];

        if selected == state.current_model_name() {
            return Ok(format!("Model '{}' is already active.", selected));
        }

        state
            .switch_model(selected)
            .await
            .map_err(|error| anyhow!("Could not switch model: {error}"))?;
        state.persist_config()?;

        return Ok(
            format!("Switched to model: {selected}")
        );
    }

    Ok(String::from("Model selection cancelled."))
}

pub fn show_capabilities(state: &AppState) -> String {
    let caps = &state.current_model.capabilities;
    let mut output = String::new();

    let _ = writeln!(output, "Model Capabilities:\n");
    let _ = writeln!(output, "Model:      {}", state.current_model.full_name());
    let _ = writeln!(output, "Tools:      {}", caps.supports(CapabilityKey::Tools));
    let _ = writeln!(output, "Thinking:   {}", caps.supports(CapabilityKey::Thinking));
    let _ = writeln!(output, "Vision:     {}", caps.supports(CapabilityKey::Vision));
    let _ = writeln!(output, "Embedding:  {}", caps.supports(CapabilityKey::Embedding));
    let _ = write!  (output, "Decision:   {}", caps.supports(CapabilityKey::Decision));

    output
}

// ---- Settings ----
pub async fn settings_menu(
    state: &mut AppState,
) -> Result<String> {
    let items = [
        "Change provider",
        "Show current model settings",
        "Cancel",
    ];

    let selection = Select::with_theme(
        &*state.config.theme.dialoguer_theme
    )
        .with_prompt(format!(
            "Settings [{} / {}]",
            state.provider.id(),
            state.current_model.id,
        ))
        .items(&items)
        .default(0)
        .interact_opt()?;

    match selection {
        Some(0) => select_provider(state).await,

        Some(1) => Ok(show_settings(&state.current_model)),

        _ => Ok("Settings menu closed.".to_string()),
    }
}

fn show_settings(model: &Model) -> String {
    let s = &model.settings;
    
    format!(
        "=== Settings for {} ({}) ===\n\
         [General]\n\
         preamble:       {}\n\
         temperature:    {}\n\
         max_tokens:     {}\n\
         top_p:          {}\n\
         seed:           {}\n\
         stop:           {}\n\n\
         [Ollama Specific]\n\
         keep_alive:     {}\n\
         num_ctx:        {}\n\
         num_predict:    {}\n\
         top_k:          {}\n\
         min_p:          {}\n\
         repeat_penalty: {}\n\
         repeat_last_n:  {}\n\
         mirostat:       {}\n\
         mirostat_tau:   {}\n\
         mirostat_eta:   {}\n\
         thinking:       {}",
        model.name, model.id,
        if model.preamble.is_empty() { "<empty>" } else { &model.preamble },
        fmt_opt(s.temperature        .map(|v| v.to_string())),
        fmt_opt(s.max_tokens         .map(|v| v.to_string())),
        fmt_opt(s.top_p              .map(|v| v.to_string())),
        fmt_opt(s.seed               .map(|v| v.to_string())),
        fmt_opt(s.stop.as_ref()      .map(|v| format!("{:?}", v))),
        fmt_opt(s.keep_alive.as_ref().map(|v| v.to_string())),
        fmt_opt(s.num_ctx            .map(|v| v.to_string())),
        fmt_opt(s.num_predict        .map(|v| v.to_string())),
        fmt_opt(s.top_k              .map(|v| v.to_string())),
        fmt_opt(s.min_p              .map(|v| v.to_string())),
        fmt_opt(s.repeat_penalty     .map(|v| v.to_string())),
        fmt_opt(s.repeat_last        .map(|v| v.to_string())),
        fmt_opt(s.mirostat.as_ref()  .map(|v| v.to_string())),
        fmt_opt(s.mirostat_tau       .map(|v| v.to_string())),
        fmt_opt(s.mirostat_eta       .map(|v| v.to_string())),
        fmt_opt(s.thinking.as_ref()  .map(|v| v.to_string())),
    )
}

pub fn handle_set_command(
    state: &mut AppState,
    key: Option<&str>,
    val: Option<&str>,
) -> Result<String> {
    match (key, val) {
       // Option 1: direct entry of `/set temperature 0.7`
        (Some(k), Some(v)) => {
            apply_setting(&mut state.current_model, k, v)?;
            state.rebuild_agent()?;
            state.persist_config()?;
            Ok(format!("Updated '{k}' -> '{v}' and rebuilt agent."))
        }
        // Incomplete input for `/set temperature
        (Some(k), None) => {
            let input: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Enter value for '{k}' (or 'default')"))
                .interact_text()
                .map_err(|e| anyhow!(e))?;

            let trimmed = input.trim();
            apply_setting(&mut state.current_model, k, trimmed)?;
            state.rebuild_agent()?;
            state.persist_config()?;
            Ok(format!("Updated '{k}' -> '{trimmed}' and rebuilt agent."))
        }
       // Option 2: `/set` without arguments — interactive menu
        (None, None) => interactive_settings_menu(state),
        (None, Some(_)) => unreachable!(),
    }
}

fn interactive_settings_menu(state: &mut AppState) -> Result<String> {
    let mut changed = false;

    // Snapshot of initial settings for rollback if <Cancel> is selected
    let initial_preamble = state.current_model.preamble.clone();
    let initial_settings = state.current_model.settings.clone();
    
    loop {
        let model = &state.current_model;
        let settings = &model.settings;

        let thinking_str = fmt_opt(settings.thinking.as_ref());
        let visible_str = settings.thinking.as_ref().map_or("default", |v| {
            if v.is_visible {
                "true (visible)"
            } else {
                "false (hidden)"
            }
        });
        let stop_str = settings
            .stop
            .as_ref()
            .map_or_else(|| "default".to_string(), |v| v.join(", "));

        let mut items = vec![
            format!("preamble         : {}", if model.preamble.is_empty() { "<empty>" } else { &model.preamble }),
            format!("temperature      : {}", fmt_opt(settings.temperature)),
            format!("max_tokens       : {}", fmt_opt(settings.max_tokens)),
            format!("top_p            : {}", fmt_opt(settings.top_p)),
            format!("seed             : {}", fmt_opt(settings.seed)),
            format!("stop             : {}", stop_str),
            format!("keep_alive       : {}", fmt_opt(settings.keep_alive.as_ref())),
            format!("num_ctx          : {}", fmt_opt(settings.num_ctx)),
            format!("num_predict      : {}", fmt_opt(settings.num_predict)),
            format!("top_k            : {}", fmt_opt(settings.top_k)),
            format!("min_p            : {}", fmt_opt(settings.min_p)),
            format!("repeat_penalty   : {}", fmt_opt(settings.repeat_penalty)),
            format!("repeat_last_n    : {}", fmt_opt(settings.repeat_last)),
            format!("mirostat         : {}", fmt_opt(settings.mirostat)),
            format!("mirostat_tau     : {}", fmt_opt(settings.mirostat_tau)),
            format!("mirostat_eta     : {}", fmt_opt(settings.mirostat_eta)),
        ];

        if model.capabilities.supports(CapabilityKey::Thinking) {
            items.push(format!("thinking         : {}", thinking_str));
            items.push(format!("thinking_visible : {}", visible_str));
        }

        let save_idx = items.len();
        items.push("[Apply]".to_string());
        let cancel_idx = items.len();
        items.push("[Cancel]".to_string());

        let selection = Select::with_theme(&*state.config.theme.dialoguer_theme)
            .with_prompt("Select parameter to edit")
            .items(&items)
            .default(0)
            .max_length(12)
            .interact_opt()
            .map_err(|e| anyhow!(e))?;

        match selection {
            // --- [Cancel] ---
            None => {
                state.current_model.preamble = initial_preamble;
                state.current_model.settings = initial_settings;
                return Ok("Settings editing cancelled.".to_string());
            }
            Some(idx) if idx == cancel_idx => {
                state.current_model.preamble = initial_preamble;
                state.current_model.settings = initial_settings;
                return Ok("Settings editing cancelled.".to_string());
            }
            // --- [Save & Apply] ---
            Some(idx) if idx == save_idx => break,
            Some(idx) => {
                let key = items[idx].split(':').next().unwrap_or("").trim();
                match key {
                    "thinking" => {
                        let options = ["on", "off", "low", "medium", "high", "default", "<Cancel>"];
                        let opt_idx = Select::with_theme(&*state.config.theme.dialoguer_theme)
                            .with_prompt("Select thinking mode")
                            .items(&options)
                            .default(0)
                            .interact()
                            .map_err(|e| anyhow!(e))?;

                        let chosen = options[opt_idx];
                        if chosen != "<Cancel>" {
                            apply_setting(&mut state.current_model, "thinking", chosen)?;
                            changed = true;
                        }
                    }
                    "thinking_visible" => {
                        let options = ["true", "false", "<Cancel>"];
                        let opt_idx = Select::with_theme(&*state.config.theme.dialoguer_theme)
                            .with_prompt("Show thinking output in chat?")
                            .items(&options)
                            .default(0)
                            .interact()
                            .map_err(|e| anyhow!(e))?;

                        let chosen = options[opt_idx];
                        if chosen != "<Cancel>" {
                            apply_setting(&mut state.current_model, "thinking_visible", chosen)?;
                            changed = true;
                        }
                    }
                    _ => {
                        let val: String = Input::with_theme(&*state.config.theme.dialoguer_theme)
                            .with_prompt(format!("New value for '{key}' ('default' to reset, Enter to cancel)"))
                            .allow_empty(true)
                            .interact_text()
                            .map_err(|e| anyhow!(e))?;

                        let trimmed = val.trim();
                        if !trimmed.is_empty() {
                            apply_setting(&mut state.current_model, key, trimmed)?;
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    if changed {
        state.rebuild_agent()?;
        state.persist_config()?;
        Ok("Settings saved and agent rebuilt.".to_string())
    } else {
        Ok("No settings changed.".to_string())
    }
}

fn apply_setting(model: &mut Model, key: &str, raw_val: &str) -> Result<()> {
    let is_reset = raw_val.eq_ignore_ascii_case("none")
        || raw_val.eq_ignore_ascii_case("default");

    let settings = &mut model.settings;

    match key {
        "preamble" => {
            model.preamble = if is_reset { String::new() } else { raw_val.to_string() };
        }
        "keep_alive"     => settings.keep_alive     = if is_reset { None } else { Some(raw_val.trim_matches('"').to_string()) },
        "temperature"    => settings.temperature    = if is_reset { None } else { Some(raw_val.parse()?) },
        "max_tokens"     => settings.max_tokens     = if is_reset { None } else { Some(raw_val.parse()?) },
        "top_p"          => settings.top_p          = if is_reset { None } else { Some(raw_val.parse()?) },
        "seed"           => settings.seed           = if is_reset { None } else { Some(raw_val.parse()?) },
        "num_ctx"        => settings.num_ctx        = if is_reset { None } else { Some(raw_val.parse()?) },
        "num_predict"    => settings.num_predict    = if is_reset { None } else { Some(raw_val.parse()?) },
        "top_k"          => settings.top_k          = if is_reset { None } else { Some(raw_val.parse()?) },
        "min_p"          => settings.min_p          = if is_reset { None } else { Some(raw_val.parse()?) },
        "repeat_penalty" => settings.repeat_penalty = if is_reset { None } else { Some(raw_val.parse()?) },
        "repeat_last"    => settings.repeat_last    = if is_reset { None } else { Some(raw_val.parse()?) },
        "mirostat_tau"   => settings.mirostat_tau   = if is_reset { None } else { Some(raw_val.parse()?) },
        "mirostat_eta"   => settings.mirostat_eta   = if is_reset { None } else { Some(raw_val.parse()?) },
        "stop" => {
            settings.stop = if is_reset { None } else {
                Some(raw_val.split(',')
                    .map(|item| item.trim().to_string()).collect())
            };
        }
        "mirostat" => {
            settings.mirostat = if is_reset { None } else { 
                Some(MirostatMode::try_from(raw_val)
                    .map_err(|e| anyhow::anyhow!(e))?) };
        }
        "thinking" => {
            if !model.capabilities.supports(CapabilityKey::Thinking){
                return Err(anyhow!("the model does not support thinking mode"))
            }
            else if is_reset {
                settings.thinking = None;
            } else {
                let cfg = settings.thinking.get_or_insert_with(ThinkingConfig::default);
                cfg.update_mode(raw_val).map_err(|e| anyhow!(e))?;
            }
        }
        "thinking_visible" => {
            if !model.capabilities.supports(CapabilityKey::Thinking){
                return Err(anyhow!("The model does not support thinking mode"))
            }
            else if is_reset {
                if let Some(ref mut cfg) = settings.thinking {
                    cfg.set_visible(true);
                }
            } else {
                let visible = match raw_val.trim_matches('"').to_lowercase().as_str() {
                    "true"  | "1" | "on"  => true,
                    "false" | "0" | "off" => false,
                    _ => return Err(anyhow!("Invalid boolean for thinking_visible: '{raw_val}'")),
                };
                let cfg = settings.thinking.get_or_insert_with(ThinkingConfig::default);
                cfg.set_visible(visible);
            }
        }
        _ => return Err(anyhow!("Unknown parameter '{key}'. Use /settings to list fields.")),
    }

    Ok(())
}

fn fmt_opt<T: Display>(v: Option<T>) -> String {
    match v {
        Some(val) => val.to_string(),
        None => "default".to_string(),
    }
}

// ---- Statistics and reference information ----
pub fn prepare_report(calls: Vec<String>) -> String {
    if calls.is_empty()  { 
        return format!("No statistics available."); 
    }
    let body = calls.join("\n\n");
    let top_line    = "=========== CALLS STATS ===========";
    let bottom_line = "===================================";
    format!("{top_line}\n\n{body}\n{bottom_line}\n") 
}

pub fn help() -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "\
Available commands:
  /provider,     /p                   Select active LLM provider
  /settings,     /cfg                 Open application settings
  /new,          /n                   Start a new chat session
  /sessions,     /ss                  Switch or delete saved sessions
  /undo,         /u                   Remove last user message and model response
  /clear,        /cl                  Clear current session history
  /models,       /ms                  List available models
  /model,        /m                   Select active model interactively
  /capabilities, /cp                  Show current model capabilities
  /set,          /s                   Open interactive settings menu
  /set <key> <value|default>          Set a specific parameter (e.g., /set temperature 0.7)
  /stats,        /st                  Show API call statistics
  /help,         /h                   Show this help message

Settable parameters (/set):
  General : preamble, temperature, max_tokens, top_p, seed, stop,
            thinking (on|off|low|medium|high), thinking_visible (true|false)
  Ollama  : keep_alive, num_ctx, num_predict, top_k, min_p,
            repeat_penalty, repeat_last_n, mirostat, mirostat_tau, mirostat_eta"
    );
    out
}