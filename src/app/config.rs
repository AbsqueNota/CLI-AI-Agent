use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::ui::theme::preset::ThemePreset;
use crate::dto::SavedModelProfile;


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProviderConfig {
    pub initial_model: Option<String>,
    pub models: HashMap<String, SavedModelProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub provider: String,
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(skip)]
    pub theme: ThemePreset,
    // We retain the memory used by the OS and applications.
    pub memory_reserve_bytes: u64,
    // Buffer for KV-cache, context, and runtime.
    pub model_memory_multiplier: f64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".into(),
            providers: HashMap::new(),
            theme:     ThemePreset::default(),
            memory_reserve_bytes: 2 * 1024 * 1024 * 1024,
            model_memory_multiplier: 1.35,
        }
    }
}

impl AppConfig {
    pub fn profile_key(provider: &str, model_id: &str) -> String {
        format!("{}/{}", provider.to_lowercase(), model_id)
    }

    pub fn provider_config(&self, provider: &str) -> Option<&ProviderConfig> {
        self.providers.get(&provider.to_lowercase())
    }

    pub fn provider_config_mut(
        &mut self,
        provider: &str,
    ) -> &mut ProviderConfig {
        self.providers
            .entry(provider.to_lowercase())
            .or_default()
    }

    pub fn set_initial_model(
        &mut self,
        provider: &str,
        model_id: &str,
    ) {
        self.provider_config_mut(provider)
            .initial_model = Some(model_id.to_string());
    }
    
    pub fn upsert_model_profile(
        &mut self,
        provider: &str,
        model_id: &str,
        profile: SavedModelProfile,
    ) {
        self.provider_config_mut(provider)
            .models
            .insert(model_id.to_string(), profile);
    }
   
    pub fn get_model_profile(
        &self,
        provider: &str,
        model_id: &str,
    ) -> Option<&SavedModelProfile> {
        self.provider_config(provider)?
            .models
            .get(model_id)
    }
}