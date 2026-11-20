pub mod ollama;
pub mod openai;

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
    pub fn create(provider_name: &str) -> Result<Box<dyn LlmProvider>> {
        match provider_name.to_lowercase().as_str() {
            "ollama" => {
                let provider = OllamaProvider::new();
                Ok(Box::new(provider))
            }
            "openai" => {
                let provider = OpenAiProvider::new()?;
                Ok(Box::new(provider))
            }
            other => anyhow::bail!("Unsupported provider: {other}"),
        }
    }
}