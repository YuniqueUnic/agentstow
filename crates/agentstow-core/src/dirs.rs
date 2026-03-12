use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::{AgentStowError, Result};

#[derive(Debug, Clone)]
pub struct AgentStowDirs {
    pub home_dir: PathBuf,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl AgentStowDirs {
    pub fn from_env() -> Result<Self> {
        let home_override = std::env::var_os("AGENTSTOW_HOME").map(PathBuf::from);
        let config_override = std::env::var_os("AGENTSTOW_CONFIG_DIR").map(PathBuf::from);
        let data_override = std::env::var_os("AGENTSTOW_DATA_DIR").map(PathBuf::from);
        let cache_override = std::env::var_os("AGENTSTOW_CACHE_DIR").map(PathBuf::from);

        let fallback = ProjectDirs::from("io", "agentstow", "agentstow").ok_or_else(|| {
            AgentStowError::Other(anyhow::anyhow!(
                "无法推断默认目录（directories::ProjectDirs::from 返回 None）；请设置 AGENTSTOW_HOME 或 AGENTSTOW_*_DIR"
            ))
        })?;

        let default_home = fallback.data_dir().to_path_buf();
        let home_dir = home_override.clone().unwrap_or(default_home);

        let config_dir = match config_override {
            Some(p) => p,
            None => home_override
                .as_ref()
                .map(|h| h.join("config"))
                .unwrap_or_else(|| fallback.config_dir().to_path_buf()),
        };
        let data_dir = match data_override {
            Some(p) => p,
            None => home_override
                .as_ref()
                .map(|h| h.join("data"))
                .unwrap_or_else(|| fallback.data_dir().to_path_buf()),
        };
        let cache_dir = match cache_override {
            Some(p) => p,
            None => home_override
                .as_ref()
                .map(|h| h.join("cache"))
                .unwrap_or_else(|| fallback.cache_dir().to_path_buf()),
        };

        Ok(Self {
            home_dir,
            config_dir,
            data_dir,
            cache_dir,
        })
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        create_dir_all(&self.config_dir)?;
        create_dir_all(&self.data_dir)?;
        create_dir_all(&self.cache_dir)?;
        Ok(())
    }
}

fn create_dir_all(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).map_err(AgentStowError::from)
}
