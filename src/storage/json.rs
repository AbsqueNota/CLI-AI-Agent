use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use crate::app::config::AppConfig;

pub struct JsonStore {
    path: PathBuf,
}

impl JsonStore {
    pub fn new(config_dir: &Path, filename: &str) -> Self {
        Self {
            path: config_dir.join(filename),
        }
    }

    pub fn load(&self) -> Result<AppConfig> {
        if !self.path.exists() {
            return Ok(AppConfig::default());
        }
        let raw = fs::read_to_string(&self.path)
            .with_context(|| format!("Failed to read config at {:?}", self.path))?;
        serde_json::from_str(&raw).context("Failed to parse config JSON")
    }

    pub fn save(&self, cfg: &AppConfig) -> Result<()> {
        let json = serde_json::to_string_pretty(cfg)?;
        Self::atomic_write(&self.path, &json)
    }

    fn atomic_write(path: &Path, content: &str) -> Result<()> {
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, content)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }
}