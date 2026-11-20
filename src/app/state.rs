use anyhow::Result;
use rig::{
    agent::Agent,
};
use crate::{
    agent::AgentFactory, 
    dto::{CallRecord, CallsStats, ChatSession, SavedModelProfile}, 
    models::{Model, ModelRegistry,}, 
    provider::LlmProvider, 
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
        config: AppConfig,
        storage: StorageManager
    ) -> Result<Self> {
        let mut registry = ModelRegistry::load(&provider)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        
        for model in registry.get_all_models_mut() {
            println!("model_id: {},thinking: {}", 
                model.id, 
                model.settings.thinking
                    .as_ref()
                    .map_or("None".to_string(), ToString::to_string)
            );
            if let Some(profile) = config.get_model_profile(&model.provider, &model.id) {
                model.sync_with_profile(profile);
                println!("profile_id: {},thinking: {}\n", 
                    profile.id,  
                    model.settings.thinking
                    .as_ref()
                    .map_or("None".to_string(), ToString::to_string)
                );
            }
        }
        
        let selected = config.initial_model.as_ref()
            .and_then(|id| {
                registry
                    .get_by_key(provider.id(), id) 
            })
            .or_else(|| registry.first())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No selectable models"))?;

        let session = storage.load_latest_or_create_session()?;
        let stats_records = storage.load_session_stats(&session.id)?;
        

        let agent = AgentFactory::build(
            &provider,
            &selected
        )?;
        
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
    pub async fn switch_model(&mut self, model_id: &str) -> Result<()> {
        let key = format!("{}/{}", self.provider.id(), model_id);

        let Some(mut next_model) = self.registry.get(&key).cloned() else {
            anyhow::bail!("Model '{}' not found in registry", key);
        };

        if let Some(profile) = self.config.get_model_profile(self.provider.id(), model_id) {
            next_model.apply_profile(profile);
        }

        let new_agent = AgentFactory::build(
            &self.provider,
            &next_model,
        )?;

        self.current_model = next_model;
        self.agent = new_agent;
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
        let profile = SavedModelProfile::from(&self.current_model);
        self.config.upsert_model_profile(
            &self.current_model.provider,
            &self.current_model.id,
            profile,
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

    /// Records call statistics to memory and SQLite
    pub fn record_call(&mut self, mut record: CallRecord) -> Result<()> {
        record.session_id = self.session.id.clone();
        self.storage.insert_call_stat(&record)?;
        self.stats.calls.push(record);
        Ok(())
    }

    /// Removes the last move and updates the database
    pub fn undo_last_turn(&mut self) -> Result<()> {
        self.session.undo_last_turn();
        self.storage.save_session(&self.session)
    }

    /// Clears the current session history (/clear)
    pub fn clear_current_session(&mut self) -> Result<()> {
        self.session.history.clear();
        self.storage.clear_session_messages(&self.session.id)
    }

    /// Creates a new session (/new)
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

    /// Deletes a session by ID; if the current session is deleted, it switches to the next one or creates a new one.
    pub fn delete_session_by_id(&mut self, id: &str) -> Result<bool> {
        let deleted = self.storage.delete_session(id)?;
        if deleted && self.session.id == id {
            self.session = self.storage.load_latest_or_create_session()?;
            self.stats.calls = self.storage.load_session_stats(&self.session.id)?;
        }
        Ok(deleted)
    }
}