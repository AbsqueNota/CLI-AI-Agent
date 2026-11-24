use anyhow::Result;
use dialoguer::Select;
use agent_rust::{
    app::{
        AppContext,
        config::AppConfig,
        state::AppState,
    },
    ui::TerminalUi,
    provider::{LlmProvider, ProviderFactory},
    storage::StorageManager,
    chat,
};

#[tokio::main]
async fn main() -> Result<()> {
    let app_name = env!("CARGO_PKG_NAME");

    let storage = StorageManager::new(app_name)?;
    let providers = ProviderFactory::available();

    let mut config = storage.load_config().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config ({e}), using defaults.");
        AppConfig::default()
    });

    let default_index = providers
        .iter()
        .position(|provider| *provider == config.provider)
        .unwrap_or(0);

        let selection = Select::with_theme(&*config.theme.dialoguer_theme)
        .with_prompt("Select LLM provider")
        .items(providers)
        .default(default_index)
        .interact_opt()?;

    if let Some(index) = selection {
        config.provider = providers[index].to_string();
    }

    let provider: Box<dyn LlmProvider> =
        ProviderFactory::create(&config.provider)?;

        let state = AppState::new(
            provider,
            config,
            storage,
        ).await?;
        
        let ui = TerminalUi::new(state.config.theme.clone());
        
        let mut ctx = AppContext::new(state, ui);
        
        chat::chat_loop(&mut ctx).await?;

    Ok(())
}