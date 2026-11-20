

use anyhow::{Context, Result};
use async_trait::async_trait;
use rig::{
    __core::{DynModel, operation::Completion},
    providers::openrouter,
};

use crate::dto::CallRecord;
use crate::models::Model;
use crate::provider::{
    LlmProvider,
    open_compatible::{
        OpenAiCompatibleConfig,
        ReasoningParameter,
    },
};

pub struct OpenRouterProvider {
    common: OpenAiCompatibleConfig,
}

impl OpenRouterProvider {
    pub fn new() -> Result<Self> {
        let mut extra_headers = Vec::new();
        println!("api_key: {:?}", std::env::var("OPENROUTER_API_KEY"));
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .context("OPENROUTER_API_KEY environment variable is not set")?;
        if let Ok(referer) = std::env::var("OPENROUTER_HTTP_REFERER") {
            extra_headers.push(("HTTP-Referer".to_string(), referer));
        }

        if let Ok(title) = std::env::var("OPENROUTER_APP_TITLE") {
            extra_headers.push(("X-Title".to_string(), title));
        }

        let common = OpenAiCompatibleConfig::from_env(
            "openrouter",
            &api_key,
            "https://openrouter.ai/api/v1",
            ReasoningParameter::OpenAiCompatible,
            extra_headers,
        )?;

        Ok(Self { common })
    }
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    fn id(&self) -> &'static str {
        self.common.id()
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        self.common.list_models().await
    }

    fn to_additional_params(&self, model: &Model) -> Option<serde_json::Value> {
        self.common.to_additional_params(model)
    }

    fn native_completion(&self, model_id: &str) -> Result<DynModel<Completion>> {
        let client = openrouter::from_env()
            .context("Failed to initialize OpenRouter Rig client")?;

        Ok(client.completion(model_id).into())
    }

    fn parse_call_metrics(
        &self,
        model_id: &str,
        raw: &serde_json::Value,
    ) -> CallRecord {
        self.common.parse_call_metrics(model_id, raw)
    }
}
