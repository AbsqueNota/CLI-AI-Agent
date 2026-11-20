use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityKey {
    Tools,
    Thinking,
    Vision,
    Embedding,
    Decision,
    Completion
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ModelCapabilities {
    pub features: HashSet<CapabilityKey>
}

impl ModelCapabilities {
    pub fn supports(&self, capability_key: CapabilityKey) -> bool {
        self.features.contains(&capability_key)
    }
}