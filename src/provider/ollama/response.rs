use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::models::ModelDetails;
use chrono::{DateTime, Utc};

#[derive(Deserialize)]
pub struct ShowModel {
    #[serde(default)]
    pub modelfile: String,
    #[serde(default)]
    pub template: String,
    #[serde(default)]
    pub parameters: Option<String>,
    pub details: ModelDetails,
    #[serde(default)]
    pub model_info: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub projector_info: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub thinking: Option<ThinkingMetadata>,

    // Ollama Cloud metadata
    #[serde(default)]
    pub remote_model: Option<String>,
    #[serde(default)]
    pub remote_host: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagModel {
    pub name: String,
    pub model: String,
    pub modified_at: DateTime<Utc>,
    pub size: u64,
    pub digest: String,
    pub details: ModelDetails,

    // Ollama Cloud metadata
    #[serde(default)]
    pub remote_model: Option<String>,
    #[serde(default)]
    pub remote_host: Option<String>,
}

#[derive(Deserialize)]
pub struct TagsResponse {
    pub models: Vec<TagModel>,
}

#[derive(Deserialize)]
pub struct ThinkingMetadata {
    #[serde(default)]
    pub values: Vec<serde_json::Value>,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}


