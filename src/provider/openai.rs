use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client as HttpClient;
use rig::client::CompletionClient;
use rig::completion::CompletionModel;
use rig::providers::openai::Client as RigOpenAiClient;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::dto::CallRecord;
use crate::models::{
    capabilities::{CapabilityKey, ModelCapabilities},
    details::ModelDetails,
    settings::{ModelSettings, ThinkingMode},
    Model,
};
use crate::provider::{DynModel, LlmProvider};

pub struct OpenAiProvider {
    id: &'static str,
    rig_client: RigOpenAiClient,
    http: HttpClient,
    base_url: String,
    api_key: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModelEntry>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModelEntry {
    id: String,
    #[serde(default)]
    owned_by: Option<String>,
}

impl OpenAiProvider {
    pub fn new(api_key: impl Into<String>, base_url: Option<String>) -> Self {
        let api_key = api_key.into();
        let base_url = base_url
            .filter(|u| !u.trim().is_empty())
            .unwrap_or_else(|| "https://openai.com".to_string());

        let rig_client = RigOpenAiClient::from_url(&api_key, &base_url);

        Self {
            id: "openai",
            rig_client,
            http: HttpClient::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }

    /// Определяет базовые возможности модели по её идентификатору
    fn infer_capabilities(model_id: &str) -> ModelCapabilities {
        let mut caps = ModelCapabilities::default();
        let lower = model_id.to_lowercase();

        // Эмбеддинги
        if lower.contains("embedding") {
            caps.features.insert(CapabilityKey::Embedding);
            return caps;
        }

        // Инструменты (поддерживают почти все современные chat-модели)
        caps.features.insert(CapabilityKey::Tools);

        // Reasoning-модели (o1, o3, o4-mini, gpt-5, deepseek-r1, qwq и т.д.)
        let is_reasoning = lower.starts_with("o1")
            || lower.starts_with("o3")
            || lower.starts_with("o4")
            || lower.contains("gpt-5")
            || lower.contains("reasoner")
            || lower.contains("-r1")
            || lower.contains("qwq");

        if is_reasoning {
            caps.features.insert(CapabilityKey::Thinking);
        }

        // Vision (мультимодальные модели)
        let is_vision = lower.contains("gpt-4o")
            || lower.contains("gpt-4.1")
            || lower.contains("gpt-5")
            || lower.contains("vision")
            || lower.contains("claude-3")
            || lower.contains("gemini");

        if is_vision {
            caps.features.insert(CapabilityKey::Vision);
        }

        caps
    }

    fn assemble_model(&self, entry: OpenAiModelEntry) -> Model {
        let capabilities = Self::infer_capabilities(&entry.id);
        let settings = ModelSettings::default();

        Model {
            id: entry.id.clone(),
            provider: self.id.to_string(),
            name: entry.id,
            preamble: String::from("You are a helpful assistant."),
            size_bytes: None,
            capabilities,
            settings,
            details: ModelDetails::default(),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn id(&self) -> &'static str {
        self.id
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        let url = format!("{}/models", self.base_url);

        let response = self
            .http
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .context("Failed to request OpenAI /models")?
            .error_for_status()
            .context("OpenAI /models returned error status")?
            .json::<OpenAiModelsResponse>()
            .await
            .context("Failed to deserialize OpenAI /models response")?;

        let mut models: Vec<Model> = response
            .data
            .into_iter()
            // Отфильтровываем модели, не предназначенные для chat completions
            .filter(|m| {
                let id = m.id.to_lowercase();
                !id.contains("whisper")
                    && !id.contains("tts")
                    && !id.contains("dall-e")
                    && !id.contains("moderation")
            })
            .map(|entry| self.assemble_model(entry))
            .collect();

        models.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(models)
    }

    fn to_additional_params(&self, model: &Model) -> Option<Value> {
        let s = &model.settings;
        let mut params = Map::new();

        // Уровень размышлений для reasoning-моделей OpenAI (`reasoning_effort`)
        if model.capabilities.supports(CapabilityKey::Thinking) {
            if let Some(ref thinking) = s.thinking {
                match thinking.mode {
                    ThinkingMode::Effort(effort) => {
                        params.insert("reasoning_effort".into(), json!(effort));
                    }
                    ThinkingMode::Toggle(true) => {
                        params.insert("reasoning_effort".into(), json!("medium"));
                    }
                    ThinkingMode::Toggle(false) => {}
                }
            }
        }

        if params.is_empty() {
            None
        } else {
            Some(Value::Object(params))
        }
    }

    fn native_completion(&self, model_id: &str) -> Result<DynModel<CompletionModel>> {
        Ok(self.rig_client.completion_model(model_id).into())
    }

    fn parse_call_metrics(&self, model_id: &str, raw: &Value) -> CallRecord {
        let usage = raw.get("usage");

        let done_reason = raw
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c0| c0.get("finish_reason"))
            .and_then(|r| r.as_str())
            .map(String::from);

        CallRecord {
            session_id: String::new(), // Заполнится в AppState::record_call
            provider: Some(self.id.to_string()),
            model: raw
                .get("model")
                .and_then(|v| v.as_str())
                .map(String::from)
                .or_else(|| Some(model_id.to_string())),
            input_tokens: usage
                .and_then(|u| u.get("prompt_tokens").or_else(|| u.get("input_tokens")))
                .and_then(|v| v.as_u64()),
            output_tokens: usage
                .and_then(|u| u.get("completion_tokens").or_else(|| u.get("output_tokens")))
                .and_then(|v| v.as_u64()),
            // OpenAI API не возвращает наносекунды фаз инференса
            load_duration: None,
            prompt_eval_duration: None,
            eval_duration: None,
            total_duration: None,
            done_reason,
        }
    }
}