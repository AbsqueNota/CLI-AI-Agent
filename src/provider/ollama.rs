pub mod response;

use anyhow::{anyhow, Result, Context};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Map, Value};
use rig::{__core::{
        DynModel,
        operation::Completion,
    }, providers::ollama::{
        Ollama,
    },
};  
use response::{ 
    TagModel,
    TagsResponse, 
    ShowModel, 
};
use crate::{
    dto::CallRecord,
    provider::LlmProvider,
    models::{
        settings::ThinkingMode,
        CapabilityKey, 
        Model, 
    }   
};  

pub struct OllamaProvider {
    id: &'static str,
    client: Ollama,
    http: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self {
            id:       "ollama",
            client:   Ollama::new(),
            http:     Client::new(),
            base_url: String::from("http://localhost:11434"),
        }
    }
    async fn assemble_model(&self, tag_model: &TagModel) -> Result<Model> {
        let url = format!("{}/api/show", self.base_url);
        let response = self
            .http
            .post(url)
            .json(&json!({"model": tag_model.name}))
            .send()
            .await
            .context("Request failed")?
            .error_for_status()
            .context("HTTP error")?
            .json::<ShowModel>()
            .await
            .context("Failed to deserialize Ollama model")?;
    
        Model::try_from((response, tag_model, self.id)).map_err(|e| anyhow!(e))
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn id(&self) -> &'static str {
        self.id
    }

    fn to_additional_params(&self, model: &Model) -> Option<Value> {
        let s = &model.settings;
        let mut root = Map::new();
        let mut options = Map::new();

        // 1. Model parameters (must be placed strictly inside the "options" object in the Ollama /api/chat API)
        if let Some(v) = s.num_ctx        { options.insert("num_ctx".into(), json!(v)); }
        if let Some(v) = s.num_predict    { options.insert("num_predict".into(), json!(v)); }
        if let Some(v) = s.top_k          { options.insert("top_k".into(), json!(v)); }
        if let Some(v) = s.min_p          { options.insert("min_p".into(), json!(v)); }
        if let Some(v) = s.repeat_penalty { options.insert("repeat_penalty".into(), json!(v)); }
        if let Some(v) = s.repeat_last    { options.insert("repeat_last_n".into(), json!(v)); }
        if let Some(ref v) = s.mirostat   { options.insert("mirostat".into(), json!(v)); }
        if let Some(v) = s.mirostat_tau   { options.insert("mirostat_tau".into(), json!(v)); }
        if let Some(v) = s.mirostat_eta   { options.insert("mirostat_eta".into(), json!(v)); }

        if !options.is_empty() {
            root.insert("options".into(), Value::Object(options));
        }

        // 2. Root-level Ollama request parameters (/api/chat)
        if let Some(ref ka) = s.keep_alive {
            if let Ok(secs) = ka.parse::<i64>() {
                root.insert("keep_alive".into(), json!(secs));
            } else {
                root.insert("keep_alive".into(), json!(ka));
            }
        }

        if model.capabilities.supports(CapabilityKey::Thinking) {
            if let Some(ref thinking) = s.thinking {
                let val = match thinking.mode {
                    ThinkingMode::Toggle(enabled) => json!(enabled),
                    ThinkingMode::Effort(effort) => json!(effort),
                };
                root.insert("think".into(), val);
            }
        }

        if root.is_empty() {
            None
        } else {
            Some(Value::Object(root))
        }
    }

    fn parse_call_metrics(&self, model_id: &str, raw: &Value) -> CallRecord {
        CallRecord {
            session_id: String::new(), 
            provider: Some(self.id.to_string()),
            model:                raw.get("model")               .and_then(|v| v.as_str())
                .map(String::from).or_else(|| Some(model_id.to_string())),
            input_tokens:         raw.get("prompt_eval_count")   .and_then(|v| v.as_u64()),
            output_tokens:        raw.get("eval_count")          .and_then(|v| v.as_u64()),
            load_duration:        raw.get("load_duration")       .and_then(|v| v.as_u64()),
            prompt_eval_duration: raw.get("prompt_eval_duration").and_then(|v| v.as_u64()),
            eval_duration:        raw.get("eval_duration")       .and_then(|v| v.as_u64()),
            total_duration:       raw.get("total_duration")      .and_then(|v| v.as_u64()),
            done_reason:          raw.get("done_reason")         .and_then(|v| v.as_str())
                .map(String::from),
        }
    }

    async fn list_models(&self) -> Result<Vec<Model>> {
        let url = format!("{}/api/tags", self.base_url);
        let mut result = Vec::new();
        let response = self.http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<TagsResponse>()
            .await?;

        for item in response.models {
            let model = self
                .assemble_model(&item)
                .await
                .map_err(|e| anyhow!("Could not assemble model {}: {e}", item.name))?;
                result.push(model);
        }
        Ok(result)
    }
     
    async fn stop_model(&self, model_id: &str) -> Result<()> {
        let url = format!("{}/api/generate", self.base_url);
    
        self.http
            .post(url)
            .json(&json!({
                "model": model_id,
                "keep_alive": 0,
                "stream": false
            }))
            .send()
            .await?
            .error_for_status()?;
    
        Ok(())
    } 

    fn native_completion(&self, model_id: &str) -> Result<DynModel<Completion>> {
        Ok(self.client.native_completion(model_id).into())
    }
}
