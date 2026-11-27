use anyhow::{anyhow, Result};
use std::fmt::{Display, Write};
use crate::{
    provider::ProviderFactory,
    provider::InitialProviderSelection,
    ui::{
        TerminalUi,
        theme::{
            AppTheme,
            preset::ThemePreset
        }
    },
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
pub fn select_initial_provider(
    last_provider: &str,
    ui: &TerminalUi,
) -> Result<InitialProviderSelection> {
    let providers = ProviderFactory::available();

    let mut items: Vec<String> = providers
        .iter()
        .map(|provider| provider.to_string())
        .collect();

    let exit_index = items.len();
    
    items.push(ui.labels().quit.clone());

    let mut default_index = providers
        .iter()
        .position(|provider| *provider == last_provider)
        .unwrap_or(0);

    loop {
        let index = match ui.select(
            "Select LLM provider",
            &items,
            default_index,
            None,
        )? {
            Some(index) => index,
            None => continue,
        };

        if index == exit_index {
            return Ok(InitialProviderSelection::Exit);
        }

        let provider_name = providers[index];
        default_index = index;

        match ProviderFactory::create(provider_name) {
            Ok(provider) => {
                return Ok(InitialProviderSelection::Selected(provider));
            }
            Err(error) => {
                ui.error(&format!(
                    "Failed to initialize provider '{}': {error:#}",
                    provider_name
                ))?;
            }
        }
    }
}

pub async fn select_provider(
    state: &mut AppState,
    ui: &TerminalUi,
) -> Result<String> {
    let providers = crate::provider::ProviderFactory::available();
    let labels = &ui.labels();

    loop {
        let active_provider = state.provider.id();

        let mut items: Vec<String> = providers
            .iter()
            .map(|provider| {
                if *provider == active_provider {
                    format!("{provider} {}", labels.active)
                } else {
                    provider.to_string()
                }
            })
            .collect();

        let cancel_index = items.len();
        items.push(labels.cancel.clone());

        let default_index = providers
            .iter()
            .position(|provider| *provider == active_provider)
            .unwrap_or(0);

        let Some(index) = ui.select(
            "Select LLM provider",
            &items,
            default_index,
            None,
        )? else {
            return Ok("Provider selection cancelled.".to_string());
        };

        if index == cancel_index {
            return Ok("Provider selection cancelled.".to_string());
        }

        let Some(selected) = providers.get(index) else {
            let _ = ui.warning("Invalid provider selection.");
            continue;
        };

        if *selected == active_provider {
            return Ok(format!(
                "Provider '{}' is already active.",
                active_provider
            ));
        }

        match state.switch_provider(selected).await {
            Ok(()) => {
                return Ok(format!(
                    "Switched provider to '{}'. Current model: {}",
                    state.provider.id(),
                    state.current_model.id,
                ));
            }
            Err(error) => {
                let _ = ui.error(&format!(
                    "Could not switch to provider '{selected}': {error:#}"
                ));
            }
        }
    }
}

// ---- Sessions and history ----
pub async fn new_session(state: &mut AppState) -> Result<String> {
    state.start_new_session().await?;
    Ok(format!("Started a new session ({}).", state.session.id))
}

pub async fn undo_last_turn(state: &mut AppState) -> Result<String> {
    if state.session.history.is_empty() {
        return Ok(
            "Nothing to undo — conversation history is empty.".to_string()
        );
    }
    state.undo_last_turn().await;

    Ok("Removed last user message and model response.".to_string())
}

pub async fn clear_history(state: &mut AppState) -> Result<String> {
    state.clear_current_session().await;
    Ok("Conversation history cleared.".to_string())
}

pub async fn manage_sessions(
    state: &mut AppState,
    ui: &TerminalUi,
) -> Result<String> {
    let sessions = state.storage.list_sessions()?;

    if sessions.is_empty() {
        return Ok("No saved sessions found.".to_string());
    }

    let labels = &ui.labels();

    let items: Vec<String> = sessions
        .iter()
        .map(|session| {
            let active = if session.id == state.session.id {
                format!(" {}", labels.active)
            } else {
                String::new()
            };

            format!(
                "{} ({} msgs, {}){}",
                session.title,
                session.message_count,
                session.updated_at,
                active
            )
        })
        .collect();

    let default_idx = sessions
        .iter()
        .position(|session| session.id == state.session.id)
        .unwrap_or(0);

    let Some(selection) = ui.select(
        "Select session",
        &items,
        default_idx,
        None,
    )? else {
        return Ok("Session selection cancelled.".to_string());
    };

    let chosen = &sessions[selection];

    let actions = [
        "Switch to this session",
        "Delete this session",
        "Cancel",
    ];

    let Some(action) = ui.select(
        &format!("Action for '{}'", chosen.title),
        &actions,
        0,
        None,
    )? else {
        return Ok("Action cancelled.".to_string());
    };

    match action {
        0 => {
            if chosen.id == state.session.id {
                return Ok(format!(
                    "Session '{}' is already active.",
                    chosen.title
                ));
            }

            state.switch_session(
                &chosen.id,
                chosen.title.clone(),
            ).await?;

            Ok(format!(
                "Switched to session '{}'.",
                chosen.title
            ))
        }
        1 => {
            state.delete_session_by_id(&chosen.id).await?;

            Ok(format!(
                "Deleted session '{}'.",
                chosen.title
            ))
        }
        _ => Ok("Cancelled.".to_string()),
    }
}

// ---- Models ----
pub fn show_models(state: &AppState, ui: &TerminalUi) -> String {
    let labels = &ui.labels();
    let active_label = &labels.active;
    let mut models: Vec<_> = 
        state.registry.by_provider(state.provider.id());
    models.sort_by_cached_key(|m| m.id.to_string().to_lowercase());
    let max_width = models
        .iter()
        .map(|m| m.id.to_string().chars().count())
        .max()
        .unwrap_or(0);

    let mut result = String::from("Available models:\n");
    for model in models {
        let id_str = model.id.to_string();
        let active = if model.id == state.current_model.id {
            active_label.as_str()
        } else {
            ""
        };

        result.push_str(&format!(
            "\n- {:<width$} {}{}",
            id_str,
            labels.get_cloud_label(model.is_cloud),
            active,
            width = max_width,
        ));
    }
    result
}


pub async fn select_model(
    state: &mut AppState,
    ui: &TerminalUi,
) -> Result<String> {
    let labels = &ui.labels();

    let models = state.registry
        .by_provider(state.provider.id());

    if models.is_empty() {
        return Ok("No models available.".to_string());
    }

    let names: Vec<String> = models
        .iter()
        .map(|model| {
            format!(
                "{}{}",
                model.id,
                labels.get_cloud_label(model.is_cloud),
            )
        })
        .collect();

    let default = models
        .iter()
        .position(|model| {
            model.id == state.current_model_name()
        })
        .unwrap_or(0);

    let Some(index) = ui.select(
        "Select model",
        &names,
        default,
        None,
    )? else {
        return Ok("Model selection cancelled.".to_string());
    };

    let selected = models[index].id.clone();

    if selected == state.current_model_name() {
        return Ok(format!(
            "Model '{}' is already active.",
            selected
        ));
    }

    state
        .switch_model(&selected)
        .await
        .map_err(|error| anyhow!("Could not switch model: {error}"))?;

    state.persist_config()?;

    Ok(format!("Switched to model: {selected}"))
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

pub async fn stop_current_model(state: &AppState) -> Result<String> {
    let model_id = state.current_model.id.clone();

    state.stop_current_model().await?;

    Ok(format!(
        "Model '{}' unloaded from memory.",
        model_id
    ))
}

// ---- Settings ----
pub async fn settings_menu(
    state: &mut AppState,
    ui: &mut TerminalUi,
) -> Result<String> {
    let items = [
        "Change provider",
        "Show current model settings",
        "Change theme",
        "Cancel",
    ];

    let Some(selection) = ui.select(
        &format!(
            "Settings [{} / {}]",
            state.provider.id(),
            state.current_model.id,
        ),
        &items,
        0,
        None,
    )? else {
        return Ok("Settings menu closed.".to_string());
    };

    match selection {
        0 => select_provider(state, ui).await,
        1 => Ok(show_settings(&state.current_model)),
        2 => change_theme(state, ui),
        _ => Ok("Settings menu closed.".to_string()),
    }
}

fn change_theme(
    state: &mut AppState,
    ui: &mut TerminalUi,
) -> Result<String> {
    let presets = [
        ThemePreset::Neon,
        ThemePreset::Pink,
        ThemePreset::Dark,
        ThemePreset::Light,
    ];

    let labels = ui.labels();
    let active_label = labels.active.clone();
    let cancel_label = labels.cancel.clone();

    let default_index = presets
        .iter()
        .position(|preset| *preset == state.config.theme)
        .unwrap_or(0);

    // Формируем отображаемые пункты меню.
    let mut items: Vec<String> = presets
        .iter()
        .map(|preset| {
            if *preset == state.config.theme {
                format!("{preset} {active_label}")
            } else {
                preset.to_string()
            }
        })
        .collect();

    let cancel_index = items.len();
    items.push(cancel_label);

    let Some(index) = ui.select(
        "Select theme",
        &items,
        default_index,
        None,
    )? else {
        return Ok("Theme selection cancelled.".to_string());
    };

    // Отдельный пункт Cancel.
    if index == cancel_index {
        return Ok("Theme selection cancelled.".to_string());
    }

    let selected = presets[index];

    if selected == state.config.theme {
        return Ok(format!("Theme '{selected}' is already active."));
    }

    // Сохраняем выбранную тему.
    state.config.theme = selected;
    state.persist_config()?;

    // Применяем тему в текущем UI.
    ui.set_theme(AppTheme::from_preset(selected));

    Ok(format!("Theme changed to '{selected}'."))
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
    ui: &TerminalUi,
    key: Option<&str>,
    val: Option<&str>,
) -> Result<String> {
    match (key, val) {
        (Some(k), Some(v)) => {
            apply_setting(&mut state.current_model, k, v)?;
            state.rebuild_agent()?;
            state.persist_config()?;

            Ok(format!(
                "Updated '{k}' -> '{v}' and rebuilt agent."
            ))
        }

        (Some(k), None) => {
            let input = ui.input(
                &format!("Enter value for '{k}' (or 'default')"),
                false,
            )?;

            let trimmed = input.trim();

            apply_setting(&mut state.current_model, k, trimmed)?;
            state.rebuild_agent()?;
            state.persist_config()?;

            Ok(format!(
                "Updated '{k}' -> '{trimmed}' and rebuilt agent."
            ))
        }

        (None, None) => interactive_settings_menu(state, ui),

        (None, Some(_)) => unreachable!(),
    }
}

fn interactive_settings_menu(
    state: &mut AppState,
    ui: &TerminalUi,
) -> Result<String> {
    let mut changed = false;

    // Snapshot of initial settings for rollback if <Cancel> is selected
    let initial_preamble = state.current_model.preamble.clone();
    let initial_settings = state.current_model.settings.clone();
    
    loop {
        let model = &state.current_model;
        let settings = &model.settings;
        
        let labels = &ui.labels();
        let visible_label = labels.visible.clone();
        let hidden_label  = labels.hidden.clone();
        let default_label = labels.default.clone();
        let apply_label   = labels.apply.clone();
        let cancel_label  = labels.cancel.clone();

        let thinking_str = fmt_opt(settings.thinking.as_ref());
        let visible_str = settings.thinking.as_ref()
            .map_or(default_label.clone(), |v| {
                if v.is_visible {
                    format!("true {visible_label}")
                } else {
                    format!("false {hidden_label}")
                }
        });
        let stop_str = settings
            .stop
            .as_ref()
            .map_or_else(|| default_label.clone(), |v| v.join(", "));

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
        items.push(apply_label);
        let cancel_idx = items.len();
        items.push(cancel_label.clone());

        let selection = ui.select(
            "Select parameter to edit",
            &items,
            0,
            Some(12),
        )?;
        
        match selection {
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
        
            Some(idx) if idx == save_idx => break,
        
            Some(idx) => {
                let key = items[idx]
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .trim();
        
                match key {
                    "thinking" => {
                        let options = [
                            "on",
                            "off",
                            "low",
                            "medium",
                            "high",
                            "default",
                            cancel_label.as_str(),
                        ];
        
                        let Some(opt_idx) = ui.select(
                            "Select thinking mode",
                            &options,
                            0,
                            None,
                        )? else {
                            continue;
                        };
        
                        let chosen = options[opt_idx];
        
                        if chosen != cancel_label {
                            apply_setting(
                                &mut state.current_model,
                                "thinking",
                                chosen,
                            )?;
        
                            changed = true;
                        }
                    }
        
                    "thinking_visible" => {
                        let options = [
                            "true",
                            "false",
                            cancel_label.as_str(),
                        ];
        
                        let Some(opt_idx) = ui.select(
                            "Show thinking output in chat?",
                            &options,
                            0,
                            None,
                        )? else {
                            continue;
                        };
        
                        let chosen = options[opt_idx];
        
                        if chosen != cancel_label.as_str() {
                            apply_setting(
                                &mut state.current_model,
                                "thinking_visible",
                                chosen,
                            )?;
        
                            changed = true;
                        }
                    }
        
                    _ => {
                        let val = ui.input(
                            &format!(
                                "New value for '{key}' ('default' to reset, Enter to cancel)"
                            ),
                            true,
                        )?;
        
                        let trimmed = val.trim();
        
                        if !trimmed.is_empty() {
                            apply_setting(
                                &mut state.current_model,
                                key,
                                trimmed,
                            )?;
        
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
    let body = calls.join("\n");
    let top_line    = "=========== CALLS STATS ===========";
    let bottom_line = "===================================";
    format!("{top_line}\n\n{body}\n{bottom_line}") 
}

pub fn help() -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "\
Available commands:
  /capabilities, /cp                    Show current model capabilities
  /clear,        /cl                    Clear current session history
  /exit,         /q                     Exit the application
  /help,         /h                     Show this help message
  /model,        /ml                    Select active model interactively
  /models,       /mls                   List available models
  /new,          /n                     Start a new chat session
  /provider,     /pr                    Select active LLM provider
  /sessions,     /ss                    Switch or delete saved sessions
  /set                                  Open interactive settings menu
  /set <key> <value|default>            Set a specific parameter \"/set top_p 0.9\"
  /settings,     /cfg                   Open application settings
  /stats,        /sts                   Show API call statistics
  /stop,         /st                    Unload current local model from memory
  /undo,         /u                     Remove last user message and model response

Settable parameters (/set):
  General : preamble, temperature, max_tokens, top_p, seed, stop,
            thinking (on|off|low|medium|high), thinking_visible (true|false)
  Ollama  : keep_alive, num_ctx, num_predict, top_k, min_p,
            repeat_penalty, repeat_last_n, mirostat, mirostat_tau, mirostat_eta");
    out
}