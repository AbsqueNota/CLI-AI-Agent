use anyhow::Result;
use agent_rust::{
    app::{
        AppContext,
        config::AppConfig,
        state::AppState,
    },
    ui::{
        TerminalUi,
        theme::AppTheme
    },
    provider::{LlmProvider, ProviderFactory},
    storage::StorageManager,
    chat,
};

#[tokio::main]
async fn main() -> Result<()> {
    let app_name = env!("CARGO_PKG_NAME");

    let storage = StorageManager::new(app_name)?;

    let (mut config, config_warning) = 
    match storage.load_config() {
        Ok(config) => (config, None),
        Err(error) => (
            AppConfig::default(),
            Some(format!(
                "Failed to load config ({error:#}), using defaults."
            )),
        ),
    };
    
    let ui = TerminalUi::new(
        AppTheme::from_preset(config.theme)
    );
    
    if let Some(warning) = config_warning {
        ui.warning(&warning);
    }

    let providers = ProviderFactory::available();
    
    let default_index = providers
        .iter()
        .position(|provider| *provider == config.provider)
        .unwrap_or(0);
    
    if let Some(index) = ui.select(
        "Select LLM provider",
        &providers,
        default_index,
        None,
    )? {
        config.provider = providers[index].to_string();
    }

    let provider: Box<dyn LlmProvider> =
        ProviderFactory::create(&config.provider)?;

    let state = AppState::new(
        provider,
        config,
        storage,
    ).await?;
    
    let mut ctx = AppContext::new(state, ui);
    
    chat::chat_loop(&mut ctx).await?;

    Ok(())
}