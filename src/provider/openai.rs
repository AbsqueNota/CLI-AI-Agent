
use anyhow::{Context, Result};
use async_trait::async_trait;
use rig::{
    __core::{DynModel, operation::Completion},
    providers::openai::OpenAI,
};

use crate::dto::CallRecord;
use crate::models::Model;
use crate::provider::{
    LlmProvider,
    compatible::{
        CloudProviderCompatible,
        ReasoningParameter,
    },
};

pub struct OpenAiProvider {
    common: CloudProviderCompatible,
    client: OpenAI,
}

impl OpenAiProvider {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .context("OPENAI_API_KEY environment variable is not set")?;

        let common = CloudProviderCompatible::new(
            "openai",
            &api_key,
            "https://api.openai.com/v1",
            ReasoningParameter::OpenAiCompatible,
            Vec::new(),
        )?;

        let client = OpenAI::from_env()
            .context("Failed to initialize OpenAI Rig client")?;

        Ok(Self { common, client })
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn id(&self) -> &'static str {
        self.common.id()
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        self.common.list_models().await
    }

    async fn stop_model(&self, model_id: &str) -> Result<()> {
        self.common.stop_model(model_id).await
    }

    fn to_additional_params(&self, model: &Model) -> Option<serde_json::Value> {
        self.common.to_additional_params(model)
    }

    fn native_completion(&self, model_id: &str) -> Result<DynModel<Completion>> {
        Ok(self.client.completion(model_id).into())
    }

    fn parse_call_metrics(
        &self,
        model_id: &str,
        raw: &serde_json::Value,
    ) -> CallRecord {
        self.common.parse_call_metrics(model_id, raw)
    }
}
