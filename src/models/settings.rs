use std::fmt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;      
use crate::provider::ollama::response::{
    ShowModel,
    ThinkingMetadata,
};

// ---- Model ----

#[skip_serializing_none]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ModelSettings {
    /// --- General parameters (OpenAI + Ollama) ---
    pub temperature:    Option<f64>,
    pub max_tokens:     Option<u64>,
    pub top_p:          Option<f64>,
    pub seed:           Option<u64>,
    pub stop:           Option<Vec<String>>,

    /// --- Specific parameters Ollama ---
    pub keep_alive:     Option<String>,
    pub num_ctx:        Option<u32>,
    pub num_predict:    Option<i32>,
    pub top_k:          Option<u32>,
    pub min_p:          Option<f64>,
    pub repeat_penalty: Option<f64>,
    pub repeat_last:    Option<i32>,
    pub mirostat:       Option<MirostatMode>,
    pub mirostat_tau:   Option<f64>,
    pub mirostat_eta:   Option<f64>,
    pub thinking:       Option<ThinkingConfig>,
}

impl ModelSettings {
    pub fn sync_with_config(&mut self, config: &ModelSettings) -> bool {
        let mut updated = false;

        macro_rules! sync_field {
            ($field:ident) => {
                // 1. If the config contains None — do nothing; keep the model's value (from the registry)
                // 2. If the config contains Some(val) — check whether it already matches the model's value
                if let Some(ref cfg_val) = config.$field {
                    if self.$field.as_ref() != Some(cfg_val) {
                        self.$field = Some(cfg_val.clone());
                        updated = true;
                    }
                }
            };
        }    

        // General
        sync_field!(temperature);
        sync_field!(max_tokens);
        sync_field!(top_p);
        sync_field!(seed);
        sync_field!(stop);
        sync_field!(thinking);

        // Ollama
        sync_field!(keep_alive);
        sync_field!(num_ctx);
        sync_field!(num_predict);
        sync_field!(top_k);
        sync_field!(min_p);
        sync_field!(repeat_penalty);
        sync_field!(repeat_last);
        sync_field!(mirostat);
        sync_field!(mirostat_tau);
        sync_field!(mirostat_eta);

        updated
    }
}

// ---- Mirostat algorithm modes (specific to Ollama) ----

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum MirostatMode {
    #[default]
    Disabled  = 0,
    Mirostat1 = 1,
    Mirostat2 = 2,
}

// ---- Thinking ----

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThinkingMode {
    Toggle(bool),
    Effort(Effort),
}

impl Default for ThinkingMode {
    fn default() -> Self {
        Self::Toggle(false) 
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThinkingConfig {
    pub mode:       ThinkingMode,
    pub is_visible: bool,
}

impl Default for ThinkingConfig {
    fn default() -> Self {
        Self {
            is_visible: false,
            mode: ThinkingMode::Toggle(false)
        }
    }
}

impl ThinkingConfig {
    pub fn update_mode(&mut self, s: &str) -> Result<(), String> {
        match s.trim_matches('"').to_lowercase().as_str() {
            "true" | "1" | "on" => {
                self.mode = ThinkingMode::Toggle(true);
                self.is_visible = true;
            }
            "false" | "0" | "off" => {
                self.mode = ThinkingMode::Toggle(false);
                self.is_visible = false;
            }
            "low" => self.mode = ThinkingMode::Effort(Effort::Low),
            "medium" | "middle" => self.mode = ThinkingMode::Effort(Effort::Medium),
            "high" => self.mode = ThinkingMode::Effort(Effort::High),
            _ => return Err(format!("Invalid thinking parameter value: '{}'", s)),
        }
        Ok(())
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.is_visible = visible;
        // Если режим был простым Toggle, синхронизируем его с видимостью
        if let ThinkingMode::Toggle(ref mut enabled) = self.mode {
            *enabled = visible;
        }
    }
}

// ---- Effort(Reasoning) ----

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    Low,
    #[default]
    Medium,
    High,
}

impl Effort {
    pub fn to_budget_tokens(self) -> u32 {
        match self {
            Effort::Low => 1024,
            Effort::Medium => 4096,
            Effort::High => 16384,
        }
    }
}

// ---- Display ----

impl fmt::Display for Effort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Effort::Low => write!(f, "low"),
            Effort::Medium => write!(f, "medium"),
            Effort::High => write!(f, "high"),
        }
    }
}

impl fmt::Display for ThinkingConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.mode {
            ThinkingMode::Toggle(true) => write!(f, "on"),
            ThinkingMode::Toggle(false) => write!(f, "off"),
            ThinkingMode::Effort(effort) => {
                let visibility = if self.is_visible { "visible" } else { "hidden" };
                write!(f, "{} ({})", effort, visibility)
            }
        }
    }
}

impl fmt::Display for MirostatMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            MirostatMode::Disabled => "disabled", // или "0"
            MirostatMode::Mirostat1 => "v1",      // или "1"
            MirostatMode::Mirostat2 => "v2",      // или "2"
        };
        write!(f, "{label}")
    }
}
// ---- TryFrom ----

impl TryFrom<&ShowModel> for ModelSettings {
    type Error = String;

    fn try_from(showmodel: &ShowModel) -> Result<ModelSettings, String> {
        let mut settings = ModelSettings::default();
    
        if let Some(metadata) = &showmodel.thinking {
            settings.thinking = Some(ThinkingConfig::try_from(metadata)?);
        }
    
        if let Some(params_str) = &showmodel.parameters {
            let mut stop_tokens = Vec::new();
            for line in params_str.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
    
                if let Some((key, raw_val)) = line.split_once(char::is_whitespace) {
                    let val = raw_val.trim();
    
                    match key {
                        "temperature"    => settings.temperature    = val.parse().ok(),
                        "top_p"          => settings.top_p          = val.parse().ok(),
                        "seed"           => settings.seed           = val.parse().ok(),
                        "num_ctx"        => settings.num_ctx        = val.parse().ok(),
                        "num_predict"    => settings.num_predict    = val.parse().ok(),
                        "top_k"          => settings.top_k          = val.parse().ok(),
                        "min_p"          => settings.min_p          = val.parse().ok(),
                        "repeat_penalty" => settings.repeat_penalty = val.parse().ok(),
                        "repeat_last_n"  => settings.repeat_last    = val.parse().ok(),
                        "mirostat"       => settings.mirostat       = Some(MirostatMode::try_from(val)?),
                        "mirostat_tau"   => settings.mirostat_tau   = val.parse().ok(),
                        "mirostat_eta"   => settings.mirostat_eta   = val.parse().ok(),
                        "thinking"       => if let Some(thinking) = settings.thinking.as_mut(){
                            ThinkingConfig::update_mode(thinking,val)?
                        }
                        "stop" => {
                            let clean_stop = val.trim_matches('"').to_string();
                            stop_tokens.push(clean_stop);
                        }
                        _ => {}
                    }
                }
            }
    
            if !stop_tokens.is_empty() {
                settings.stop = Some(stop_tokens);
            }
        }
    
        Ok(settings)
    }
}

impl TryFrom<&ThinkingMetadata> for ThinkingConfig {
    type Error = String;

    fn try_from(metadata: &ThinkingMetadata) -> Result<ThinkingConfig, String> {
        if metadata.values.is_empty() {
            return Err("The values array is empty; it is impossible to determine the configuration type.".into());
        }
        match &metadata.values[0] {
            Value::Bool(_) => {
                if !metadata.values.iter().all(|v| v.is_boolean()) {
                    return Err("Inconsistent types in thinking values: expected booleans.".into());
                }
                let default_val = match &metadata.default {
                    Some(Value::Bool(b)) => *b,
                    Some(_) => return Err("Expected boolean for default thinking value.".into()),
                    None => metadata.values[0].as_bool().unwrap_or(false),
                };
    
                Ok(ThinkingConfig{
                    mode: ThinkingMode::Toggle(default_val),
                    is_visible: default_val
                })
            }
            Value::String(_) => {
                let efforts: Vec<Effort> = metadata
                    .values
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .ok_or_else(|| "Inconsistent types in thinking values: expected strings.".to_string())
                            .and_then(Effort::try_from)
                    })
                    .collect::<Result<_, _>>()?;
    
                let default_effort = match &metadata.default {
                    Some(Value::String(s)) => Effort::try_from(s.as_str())?,
                    Some(_) => return Err("Expected string for default thinking value.".into()),
                    None => efforts[0].clone(), 
                };
    
                Ok(ThinkingConfig{
                    is_visible:true,
                    mode:ThinkingMode::Effort(default_effort)
                })
            }
            other => Err(format!("Unsupported thinking value type: {:?}", other)),
        }
    }
}

impl TryFrom<&str> for MirostatMode {
    type Error = String;

    fn try_from(val: &str) -> Result<Self, Self::Error> {
        let mode = val
            .trim_matches('"')
            .parse::<u8>()
            .map_err(|_| format!("Invalid mirostat number: '{}'", val))?;
    
        match mode {
            0 => Ok(MirostatMode::Disabled),
            1 => Ok(MirostatMode::Mirostat1),
            2 => Ok(MirostatMode::Mirostat2),
            _ => Err(format!("Unsupported mirostat mode: {} (expected 0, 1, or 2)", mode)),
        }
    }
}

impl TryFrom<&str> for Effort {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "low"    => Ok(Effort::Low),
            "medium" => Ok(Effort::Medium),
            "high"   => Ok(Effort::High),
            unknown => Err(format!("Unknown effort level: '{}'", unknown)),
        }
    }
}
