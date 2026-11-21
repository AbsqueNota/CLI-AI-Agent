
use anyhow::{Context, Result};
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::dto::CallRecord;
use crate::models::{
    capabilities::{CapabilityKey, ModelCapabilities},
    details::ModelDetails,
    settings::{ModelSettings, ThinkingMode},
    Model,
};

#[derive(Debug, Clone, Copy)]
pub enum ReasoningParameter {
    OpenAiCompatible,
}

pub struct CloudProviderCompatible {
    provider_id: &'static str,
    api_key: String,
    base_url: String,
    http: HttpClient,
    reasoning_parameter: ReasoningParameter,
    extra_headers: Vec<(String, String)>,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,

    #[serde(default)]
    supported_parameters: Vec<String>,

    #[serde(default)]
    architecture: Option<ModelArchitecture>,
}

#[derive(Debug, Deserialize)]
struct ModelArchitecture {
    #[serde(default)]
    input_modalities: Vec<String>,
}

impl CloudProviderCompatible {
    pub fn new(
        provider_id:         &'static str,
        api_key:             &str,
        base_url:            &str,
        reasoning_parameter: ReasoningParameter,
        extra_headers: Vec<(String, String)>,
    ) -> Result<Self> {

        if api_key.trim().is_empty() {
            anyhow::bail!("{provider_id} api key cannot be empty");
        }

        Ok(Self {
            provider_id,
            api_key: String::from(api_key),
            base_url: base_url.trim_end_matches('/').to_string(),
            http: HttpClient::new(),
            reasoning_parameter,
            extra_headers,
        })
    }

    pub fn id(&self) -> &'static str {
        self.provider_id
    }

    pub async fn list_models(&self) -> Result<Vec<Model>> {
        let url = format!("{}/models", self.base_url);

        let mut request = self
            .http
            .get(&url)
            .bearer_auth(&self.api_key);

        for (name, value) in &self.extra_headers {
            request = request.header(name, value);
        }

        let response = request
            .send()
            .await
            .context("Failed to request provider model list")?
            .error_for_status()
            .context("Provider models endpoint returned an error")?
            .json::<ModelsResponse>()
            .await
            .context("Failed to deserialize provider models response")?;

        let mut models: Vec<Model> = response
            .data
            .into_iter()
            .filter(Self::is_completion_model)
            .map(|entry| self.assemble_model(entry))
            .collect();

        models.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(models)
    }

    pub async fn stop_model(&self, model_id: &str) -> Result<()> {
        anyhow::bail!(
            "Stop: [{model_id}] Provider '{}' does not support stopping models.",
            self.id()
        )
    }

    fn is_completion_model(entry: &ModelEntry) -> bool {
        let id = entry.id.to_lowercase();

        !id.contains("embedding")
            && !id.contains("whisper")
            && !id.contains("transcription")
            && !id.contains("tts")
            && !id.contains("text-to-speech")
            && !id.contains("dall-e")
            && !id.contains("moderation")
            && !id.contains("rerank")
    }

    fn assemble_model(&self, entry: ModelEntry) -> Model {
        let capabilities = Self::infer_capabilities(&entry);

        Model {
            id: entry.id.clone(),
            provider: self.provider_id.to_string(),
            name: entry.id,
            preamble: "You are a helpful assistant.".to_string(),
            size_bytes: None,
            is_cloud: true,
            capabilities,
            settings: ModelSettings::default(),
            details: ModelDetails::default(),
        }
    }

    fn infer_capabilities(entry: &ModelEntry) -> ModelCapabilities {
        let mut capabilities = ModelCapabilities::default();
        let id = entry.id.to_lowercase();

        if id.contains("embedding") {
            capabilities.features.insert(CapabilityKey::Embedding);
            return capabilities;
        }

        // If the API provides supported_parameters, prefer this metadata.
        if !entry.supported_parameters.is_empty() {
            let parameters = &entry.supported_parameters;

            if parameters.iter().any(|p| p == "tools") {
                capabilities.features.insert(CapabilityKey::Tools);
            }

            if parameters.iter().any(|p| {
                p == "reasoning" || p == "include_reasoning" || p == "reasoning_effort"
            }) {
                capabilities.features.insert(CapabilityKey::Thinking);
            }
        } else {
            // Fallback for APIs that return only model identifiers.
            capabilities.features.insert(CapabilityKey::Tools);

            let is_reasoning = id.starts_with("o1")
                || id.starts_with("o3")
                || id.starts_with("o4")
                || id.contains("gpt-5")
                || id.contains("gpt-6")
                || id.contains("deepseek-r1")
                || id.contains("qwq")
                || id.contains("reasoner")
                || id.contains("thinking");

            if is_reasoning {
                capabilities.features.insert(CapabilityKey::Thinking);
            }
        }

        let has_vision_metadata = entry
            .architecture
            .as_ref()
            .is_some_and(|architecture| {
                architecture.input_modalities.iter().any(|m| m == "image")
            });

        let has_vision_name = id.contains("vision")
            || id.contains("gpt-4o")
            || id.contains("gpt-4.1")
            || id.contains("gpt-5")
            || id.contains("gpt-6")
            || id.contains("gemini")
            || id.contains("claude-3")
            || id.contains("claude-sonnet-4")
            || id.contains("pixtral")
            || id.contains("qwen-vl");

        if has_vision_metadata || has_vision_name {
            capabilities.features.insert(CapabilityKey::Vision);
        }

        capabilities
    }

    pub fn to_additional_params(&self, model: &Model) -> Option<Value> {
        if !model.capabilities.supports(CapabilityKey::Thinking) {
            return None;
        }

        let thinking = model.settings.thinking.as_ref()?;
        let mut params = Map::new();

        match thinking.mode {
            ThinkingMode::Effort(effort) => {
                match self.reasoning_parameter {
                    ReasoningParameter::OpenAiCompatible => {
                        params.insert("reasoning_effort".into(), json!(effort));
                    }
                }
            }
            ThinkingMode::Toggle(true) => {
                params.insert("reasoning_effort".into(), json!("medium"));
            }
            ThinkingMode::Toggle(false) => {}
        }

        if params.is_empty() {
            None
        } else {
            Some(Value::Object(params))
        }
    }

    pub fn parse_call_metrics(
        &self,
        model_id: &str,
        raw: &Value,
    ) -> CallRecord {
        let usage = raw.get("usage");

        let done_reason = raw
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("finish_reason"))
            .and_then(Value::as_str)
            .map(String::from)
            .or_else(|| {
                raw.get("status")
                    .and_then(Value::as_str)
                    .map(String::from)
            });

        CallRecord {
            session_id: String::new(),
            provider: Some(self.provider_id.to_string()),
            model: raw
                .get("model")
                .and_then(Value::as_str)
                .map(String::from)
                .or_else(|| Some(model_id.to_string())),
            input_tokens: usage
                .and_then(|u| {
                    u.get("prompt_tokens")
                        .or_else(|| u.get("input_tokens"))
                })
                .and_then(Value::as_u64),
            output_tokens: usage
                .and_then(|u| {
                    u.get("completion_tokens")
                        .or_else(|| u.get("output_tokens"))
                })
                .and_then(Value::as_u64),
            load_duration: None,
            prompt_eval_duration: None,
            eval_duration: None,
            total_duration: None,
            done_reason,
        }
    }
}
