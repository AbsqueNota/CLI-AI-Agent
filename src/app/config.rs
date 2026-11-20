use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::theme::AppTheme;
use crate::dto::SavedModelProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub provider: String,
    pub initial_model: Option<String>,
    #[serde(skip)]
    pub theme: AppTheme,
    pub model_profiles: HashMap<String, SavedModelProfile>,
    
    // We retain the memory used by the OS and applications.
    pub memory_reserve_bytes: u64,
    // Buffer for KV-cache, context, and runtime.
    pub model_memory_multiplier: f64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".into(),
            initial_model: None,
            theme: AppTheme::default(),
            memory_reserve_bytes: 2 * 1024 * 1024 * 1024,
            model_memory_multiplier: 1.35,
            model_profiles: HashMap::new(),
        }
    }
}

impl AppConfig {
    pub fn profile_key(provider: &str, model_id: &str) -> String {
        format!("{}/{}", provider.to_lowercase(), model_id)
    }

    /// Saves model settings to AppConfig memory
    pub fn upsert_model_profile(&mut self, provider: &str, model_id: &str, profile: SavedModelProfile) {
        let key = Self::profile_key(provider, model_id);
        self.model_profiles.insert(key, profile);
        self.provider = provider.to_string();
        self.initial_model = Some(model_id.to_string());
    }

    /// Retrieves a saved profile for the model, if one exists
    pub fn get_model_profile(&self, provider: &str, model_id: &str) -> Option<&SavedModelProfile> {
        let key = Self::profile_key(provider, model_id);
        self.model_profiles.get(&key)
    }
}