use anyhow::Result;
use agent_rust::{
    app::{config::AppConfig, state::AppState},
    provider::{LlmProvider, ProviderFactory},
    storage::StorageManager,
    chat
};

#[tokio::main]
async fn main() -> Result<()> {
    let app_name = env!("CARGO_PKG_NAME");

    let storage = StorageManager::new(app_name)?;

    let config = storage.load_config().unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config ({e}), using defaults.");
        AppConfig::default()
    });

    let provider: Box<dyn LlmProvider> = ProviderFactory::create(config.provider.as_str())?;

    let mut state = AppState::new(
        provider,
        config,
        storage,
    ).await?;

    chat::chat_loop(&mut state).await?;

    Ok(())
}