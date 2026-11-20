use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDetails {
    pub parent_model:       String,
    pub format:             String,
    pub family:             String,
    pub families:           Vec<String>,
    pub parameter_size:     String,
    pub quantization_level: String,
}

impl Default for ModelDetails {
    fn default() -> Self {
        Self {
            parent_model:       String::new(),
            format:             String::new(),
            family:             String::new(),
            families:           Vec::new(),
            parameter_size:     String::new(),
            quantization_level: String::new(),
        }
    }
}