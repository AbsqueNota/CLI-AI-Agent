pub mod capabilities;
pub mod settings;
pub mod registry;
pub mod details;

pub use details::ModelDetails;
pub use settings::ModelSettings;
pub use registry::ModelRegistry;
pub use capabilities::{CapabilityKey, ModelCapabilities,};

use crate::dto::SavedModelProfile;
use crate::provider::ollama::response::{
    ShowModel,
    TagModel,
};

#[derive(Debug, Clone)]
pub struct Model {
    pub id:           String,
    pub provider:     String,
    pub name:         String,
    pub preamble:     String,
    pub size_bytes:   Option<u64>,
    pub capabilities: ModelCapabilities,
    pub settings:     ModelSettings,
    pub details:      ModelDetails,
}

impl Model {
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.provider, self.id)
    }
    
    pub fn apply_profile(&mut self, profile: &SavedModelProfile) {
        self.preamble = profile.preamble.clone();
        self.settings = profile.settings.clone();
    }

    pub fn to_profile(&self) -> SavedModelProfile {
        SavedModelProfile {
            id: self.id.clone(),
            name: self.name.clone(),
            preamble: self.preamble.clone(),
            settings: self.settings.clone(),
        }
    }
    pub fn sync_with_profile(&mut self, profile: &SavedModelProfile) -> bool {
        let mut updated = false;

        if self.preamble != profile.preamble {
            self.preamble = profile.preamble.clone();
            updated = true;
        }

        if self.settings.sync_with_config(&profile.settings) {
            updated = true;
        }

        updated
    }
}

impl<'a> TryFrom<(ShowModel, &'a TagModel, &'a str)> for Model {
    type Error = String;
    fn try_from((show_model,tag_model, provider_id): (ShowModel, &'a TagModel, &'a str)) -> Result<Self, Self::Error> {
        let mut capabilities = ModelCapabilities::default();
        let settings = ModelSettings::try_from(&show_model)?;
    
        for capability in show_model.capabilities {
            match capability.as_str() {
                "decision"  => { capabilities.features.insert(CapabilityKey::Decision);  }
                "tools"     => { capabilities.features.insert(CapabilityKey::Tools);     }
                "thinking"  => { capabilities.features.insert(CapabilityKey::Thinking);  }
                "vision"    => { capabilities.features.insert(CapabilityKey::Vision);    }
                "embedding" => { capabilities.features.insert(CapabilityKey::Embedding); }
                _ => {}
            }
        }
        
        Ok(Model{
            id:           tag_model.name.clone(),
            name:         String::from("local-agent"),
            preamble:     String::from("You are a helpful local assistant."),
            provider:     provider_id.to_string(),
            size_bytes:   Some(tag_model.size),
            details:      show_model.details.clone(),
            settings,
            capabilities,
        })
    }  
}
