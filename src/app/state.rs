use anyhow::Result;
use rig::{
    agent::Agent,
};
use crate::{
    agent::AgentFactory, 
    dto::{CallRecord, CallsStats, ChatSession, SavedModelProfile}, 
    models::{Model, ModelRegistry,}, 
    provider::{LlmProvider, ProviderFactory}, 
    storage::StorageManager,
    app::config::AppConfig,
};

pub struct AppState {
    pub storage: StorageManager,
    pub config: AppConfig,
    pub provider: Box<dyn LlmProvider>,
    pub registry: ModelRegistry,
    pub current_model: Model,
    pub agent: Agent,
    pub session: ChatSession,
    pub stats: CallsStats,
}

impl AppState {
    pub async fn new(
        provider: Box<dyn LlmProvider>,
        mut config: AppConfig,
        storage: StorageManager,
    ) -> Result<Self> {
        let provider_id = provider.id().to_lowercase();
    
        config.provider = provider_id.clone();
    
        let mut registry = ModelRegistry::load(&provider)
            .await
            .map_err(anyhow::Error::msg)?;
    
        // Restore only profiles belonging to the active provider.
        if let Some(provider_config) = config.providers.get(&provider_id) {
            for model in registry.get_all_models_mut() {
                if let Some(profile) = provider_config.models.get(&model.id) {
                    model.sync_with_profile(profile);
                }
            }
        }
    
        // Restore the last selected model for this provider.
        let selected = config
            .providers
            .get(&provider_id)
            .and_then(|p| p.initial_model.as_deref())
            .and_then(|id| registry.get_by_key(&provider_id, id))
            .or_else(|| registry.first())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No selectable models"))?;
    
        let session = storage.load_latest_or_create_session()?;
        let stats_records = storage.load_session_stats(&session.id)?;
    
        let agent = AgentFactory::build(&provider, &selected)?;
    
        Ok(Self {
            config,
            provider,
            registry,
            current_model: selected,
            agent,
            session,
            storage,
            stats: CallsStats { calls: stats_records },
        })
    }

    pub fn rebuild_agent(&mut self) -> Result<()> {
        let key = self.current_model.full_name();
        // Synchronize the current model's setting changes back to the registry
        if let Some(reg_model) = self.registry.get_mut(&key) {
            reg_model.preamble = self.current_model.preamble.clone();
            reg_model.settings = self.current_model.settings.clone();
        }

        let agent = AgentFactory::build(
            &self.provider,
            &self.current_model,
        )?;
        self.agent = agent;
        Ok(())
    }

    fn update_provider_config(
        config: &mut AppConfig,
        model: &Model,
    ) {
        let provider_id = model.provider.to_lowercase();
    
        config.provider = provider_id.clone();
    
        let provider_config = config
            .providers
            .entry(provider_id)
            .or_default();
    
        provider_config.initial_model = Some(model.id.clone());
    
        provider_config.models.insert(
            model.id.clone(),
            SavedModelProfile::from(model),
        );
    }

    pub async fn switch_provider(
        &mut self,
        provider_name: &str,
    ) -> Result<()> {
        let provider_name = provider_name.to_lowercase();
    
        if provider_name == self.provider.id() {
            return Ok(());
        }
    
        // Save the current provider's model and profile.
        self.persist_config()?;
    
        let new_provider = ProviderFactory::create(&provider_name)?;
    
        let mut new_registry = ModelRegistry::load(&new_provider)
            .await
            .map_err(anyhow::Error::msg)?;
    
        // Restore profiles belonging to the new provider.
        if let Some(provider_config) = self.config.providers.get(&provider_name) {
            for model in new_registry.get_all_models_mut() {
                if let Some(profile) = provider_config.models.get(&model.id) {
                    model.sync_with_profile(profile);
                }
            }
        }
    
        // Restore this provider's previously selected model.
        let selected_model = self.config
            .providers
            .get(&provider_name)
            .and_then(|p| p.initial_model.as_deref())
            .and_then(|id| new_registry.get_by_key(
                new_provider.id(),
                id,
            ))
            .or_else(|| new_registry.first())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!(
                "No models available for provider '{}'",
                provider_name
            ))?;
    
        let new_agent = AgentFactory::build(
            &new_provider,
            &selected_model,
        )?;
    
        // Prepare the new configuration.
        let mut new_config = self.config.clone();
    
        Self::update_provider_config(
            &mut new_config,
            &selected_model,
        );
        self.storage.save_config(&new_config)?;
        self.provider = new_provider;
        self.registry = new_registry;
        self.current_model = selected_model;
        self.agent = new_agent;
        self.config = new_config;
    
        Ok(())
    }

    pub async fn switch_model(&mut self, model_id: &str) -> Result<()> {
        let key = format!("{}/{}", self.provider.id(), model_id);
    
        let mut next_model = self.registry
            .get(&key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!(
                "Model '{}' not found in registry",
                key
            ))?;
    
        // Restore the model's saved settings.
        if let Some(profile) = self.config
            .providers
            .get(self.provider.id())
            .and_then(|p| p.models.get(model_id))
        {
            next_model.apply_profile(profile);
        }
    
        let new_agent = AgentFactory::build(
            &self.provider,
            &next_model,
        )?;
    
        // Save selection and profile before applying state.
        let mut new_config = self.config.clone();
    
        Self::update_provider_config(
            &mut new_config,
            &next_model,
        );
    
        self.storage.save_config(&new_config)?;
        self.registry.insert(next_model.clone());
        self.current_model = next_model;
        self.agent = new_agent;
        self.config = new_config;
    
        Ok(())
    }

    pub fn switch_session(&mut self, session_id: &str, title: String) -> Result<()> {
        self.session = self.storage.load_session(session_id, title)?;
        let records = self.storage.load_session_stats(session_id)?;
        self.stats = CallsStats::new(records);
        Ok(())
    }
    
    pub fn clear_history(&mut self) {
        self.session.history.clear();
    }

    pub fn current_model_name(&self) -> &str {
        &self.current_model.id
    }

    /// Saves the settings of the current model and provider to config.json
    pub fn persist_config(&mut self) -> Result<()> {
        // Synchronize current model with registry.
        let key = self.current_model.full_name();
    
        if let Some(model) = self.registry.get_mut(&key) {
            model.preamble = self.current_model.preamble.clone();
            model.settings = self.current_model.settings.clone();
        }
    
        Self::update_provider_config(
            &mut self.config,
            &self.current_model,
        );
    
        self.storage.save_config(&self.config)
    }

    /// Saves the current session to SQLite
    pub fn persist_session(&mut self) -> Result<()> {
        // If this is the first message in a new session, we generate a title from it.
        if self.session.title == "Default Session" {
            if let Some(first_msg) = self.session.history.first() {
                let raw = format!("{first_msg:?}");
                self.session.title = raw.chars().take(40).collect();
            }
        }
        self.storage.save_session(&self.session)
    }

    pub fn record_call(&mut self, mut record: CallRecord) -> Result<()> {
        record.session_id = self.session.id.clone();
        self.storage.insert_call_stat(&record)?;
        self.stats.calls.push(record);
        Ok(())
    }

    pub fn undo_last_turn(&mut self) -> Result<()> {
        self.session.undo_last_turn();
        self.storage.save_session(&self.session)
    }

    pub fn clear_current_session(&mut self) -> Result<()> {
        self.session.history.clear();
        self.storage.clear_session_messages(&self.session.id)
    }

    pub fn start_new_session(&mut self) -> Result<()> {
        let new_id = format!(
            "session_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
        self.session = ChatSession::new(new_id, "Default Session");
        self.stats.calls.clear();
        self.storage.save_session(&self.session)
    }

    pub fn delete_session_by_id(&mut self, id: &str) -> Result<bool> {
        let deleted = self.storage.delete_session(id)?;
        if deleted && self.session.id == id {
            self.session = self.storage.load_latest_or_create_session()?;
            self.stats.calls = self.storage.load_session_stats(&self.session.id)?;
        }
        Ok(deleted)
    }
}