pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod open_compatible;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use rig::{
    __core::{
        DynModel,
        operation::Completion, 
    },
};
use crate::{
    models::Model,
    dto::CallRecord,
    provider::{
        ollama::OllamaProvider,
        openai::OpenAiProvider,
        openrouter::OpenRouterProvider,
    }
};

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn id(&self) -> &'static str;
    async fn list_models(&self) -> Result<Vec<Model>>;
    fn to_additional_params(&self, model: &Model ) -> Option<Value>;
    fn native_completion( &self, model_id: &str, ) -> Result<DynModel<Completion>>;
    fn parse_call_metrics(&self, model_id: &str, raw: &Value) -> CallRecord;
}

pub struct ProviderFactory;

impl ProviderFactory {
    pub fn available() -> &'static [&'static str] {
        &["ollama", "openai", "openrouter"]
    }
    pub fn create(provider_name: &str) -> Result<Box<dyn LlmProvider>> {
        match provider_name.to_lowercase().as_str() {
            "ollama" => {
                Ok(Box::new(OllamaProvider::new()))
            }
            "openai" => {
                Ok(Box::new(OpenAiProvider::new()?))
            }
            "openrouter" => {
                Ok(Box::new(OpenRouterProvider::new()?))
            }
            other => anyhow::bail!("Unsupported provider: {other}"),
        }
    }
}