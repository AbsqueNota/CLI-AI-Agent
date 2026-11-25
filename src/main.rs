use anyhow::Result;

use agent_rust::{
    app::{
        AppContext,
        config::AppConfig,
        state::AppState,
        commands::snippets::{
            select_initial_provider,
        },
    },
    ui::{
        TerminalUi,
        theme::AppTheme,
    },
    provider::InitialProviderSelection,
    storage::StorageManager,
    chat,
};

#[tokio::main]
async fn main() -> Result<()> {
    let app_name = env!("CARGO_PKG_NAME");
    let storage = StorageManager::new(app_name)?;

    let config = storage.load_config().unwrap_or_else(|error| {
        TerminalUi::warning(
            &format!("Failed to load config ({error:#}), using defaults."));
        AppConfig::default()
    });

    let ui = TerminalUi::new(
        AppTheme::from_preset(config.theme)
    );

    let provider = 
        match select_initial_provider(&config.provider, &ui)? {
            InitialProviderSelection::Selected(provider) => provider,
        
            InitialProviderSelection::Exit => {
                ui.goodbye();
                return Ok(());
            }
        };

    let state = AppState::new(
        provider,
        config,
        storage,
    ).await?;

    let mut ctx = AppContext::new(state, ui);

    let chat_result = chat::chat_loop(&mut ctx).await;
    
    let shutdown_result = ctx.state.storage.shutdown().await;

    chat_result?;
    shutdown_result?;
    Ok(())
}