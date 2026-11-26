use std::collections::HashMap;
use super::Model;
use crate::provider::LlmProvider;
#[derive(Default)]
pub struct ModelRegistry {
    models: HashMap<String, Model>,
}

impl ModelRegistry {
    pub async fn load(
        provider: &dyn LlmProvider,
    ) -> Result<Self, String> {
        let models = provider
            .list_models()
            .await
            .map_err(|e| format!("Failed to list models for {}: {}", provider.id(), e))?;
    
        if models.is_empty() {
            return Err(format!("No models found for provider {}", provider.id()));
        }
    
        let mut registry = ModelRegistry::default();
        for model in models {
            registry.insert(model);
        }
    
        Ok(registry)
    }

    pub fn insert(&mut self, model: Model) {
        self.models.insert(model.full_name(), model);
    }

    pub fn get(&self, key: &str) -> Option<&Model> {
        self.models.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Model> {
        self.models.get_mut(key)
    }

    pub fn get_all_models_mut(&mut self) -> impl Iterator<Item = &mut Model> {
        self.models.values_mut()
    }

    pub fn get_by_key(&self, provider_id: &str, model_id: &str) -> Option<&Model> {
        let key = format!("{}/{}", provider_id, model_id);
        self.models.get(&key)
    }

    pub fn first(&self) -> Option<&Model> {
        self.models.values().next()
    }

    pub fn by_provider(&self, provider: &str) -> Vec<&Model> {
        self.models
            .values()
            .filter(|model| model.provider == provider)
            .collect()
    }
}