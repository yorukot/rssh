use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub hosts: HashMap<String, HostConfig>,
}

#[derive(Debug, Deserialize)]
pub struct Defaults {
    #[serde(default)]
    pub auto_drop: bool,
    #[serde(default)]
    pub follow_symlinks: bool,
    #[serde(default = "default_max_file_bytes")]
    pub max_file_bytes: u64,
    #[serde(default = "default_max_batch_bytes")]
    pub max_batch_bytes: u64,
    #[serde(default = "default_max_files_per_batch")]
    pub max_files_per_batch: u32,
    #[serde(default = "default_retain_completed_days")]
    pub retain_completed_days: u32,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            auto_drop: false,
            follow_symlinks: false,
            max_file_bytes: default_max_file_bytes(),
            max_batch_bytes: default_max_batch_bytes(),
            max_files_per_batch: default_max_files_per_batch(),
            retain_completed_days: default_retain_completed_days(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct HostConfig {
    #[serde(default)]
    pub allowed_local_roots: Vec<PathBuf>,
    pub remote_root: Option<PathBuf>,
    pub injection_profile: Option<String>,
    pub auto_drop: Option<bool>,
}

fn default_max_file_bytes() -> u64 {
    64 * 1024 * 1024
}
fn default_max_batch_bytes() -> u64 {
    256 * 1024 * 1024
}
fn default_max_files_per_batch() -> u32 {
    8
}
fn default_retain_completed_days() -> u32 {
    14
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn load_default() -> Self {
        let path = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".config/rssh/config.toml"))
            .unwrap_or_else(|_| PathBuf::from(".config/rssh/config.toml"));
        Self::load(&path).unwrap_or_default()
    }

    pub fn host(&self, name: &str) -> Option<&HostConfig> {
        self.hosts.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sane_values() {
        let config = Config::default();
        assert!(!config.defaults.auto_drop);
        assert!(!config.defaults.follow_symlinks);
        assert_eq!(config.defaults.max_file_bytes, 64 * 1024 * 1024);
        assert_eq!(config.defaults.max_files_per_batch, 8);
    }

    #[test]
    fn parse_minimal_toml() {
        let toml_str = r#"
            [defaults]
            auto_drop = false

            [hosts.myserver]
            allowed_local_roots = ["/Users/demo/Desktop"]
            remote_root = "/home/demo/.cache/rssh"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.hosts.contains_key("myserver"));
        let host = config.host("myserver").unwrap();
        assert_eq!(
            host.remote_root.as_deref(),
            Some(Path::new("/home/demo/.cache/rssh"))
        );
    }

    #[test]
    fn missing_host_returns_none() {
        let config = Config::default();
        assert!(config.host("nonexistent").is_none());
    }
}
