#![allow(dead_code)]

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const CONFIG_FILENAME: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub default_dir: Option<String>,
    pub cloud_detection: Option<bool>,
    pub interactive_mode: Option<bool>,
    pub dry_run_default: Option<bool>,
    pub ignored_patterns: Option<Vec<String>>,
    pub hash_cloud_files: Option<bool>,
    pub fetch_arxiv: Option<bool>,
    pub extract_metadata: Option<bool>,
    pub cleanup_downloads: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_dir: None,
            cloud_detection: Some(true),
            interactive_mode: Some(false),
            dry_run_default: Some(false),
            ignored_patterns: Some(vec![]),
            hash_cloud_files: Some(true),
            fetch_arxiv: Some(false),
            extract_metadata: Some(false),
            cleanup_downloads: Some(false),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = get_config_path()?;

        if !config_path.exists() {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| anyhow!("Failed to read config file: {}", e))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse config file: {}", e))?;

        Ok(config)
    }

    pub fn save(&self) -> Result<PathBuf> {
        let config_path = get_config_path()?;

        if let Some(parent) = config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| anyhow!("Failed to serialize config: {}", e))?;

        fs::write(&config_path, content)?;

        Ok(config_path)
    }

    pub fn get_default_dir(&self) -> Option<PathBuf> {
        self.default_dir.as_ref().map(|d| {
            if d.starts_with("~") {
                if let Some(home) = dirs::home_dir() {
                    return home.join(&d[2..]);
                }
            }
            PathBuf::from(d)
        })
    }

    pub fn cloud_detection(&self) -> bool {
        self.cloud_detection.unwrap_or(true)
    }

    pub fn interactive_mode(&self) -> bool {
        self.interactive_mode.unwrap_or(false)
    }

    pub fn dry_run_default(&self) -> bool {
        self.dry_run_default.unwrap_or(false)
    }

    pub fn hash_cloud_files(&self) -> bool {
        self.hash_cloud_files.unwrap_or(true)
    }

    pub fn fetch_arxiv(&self) -> bool {
        self.fetch_arxiv.unwrap_or(false)
    }

    pub fn extract_metadata(&self) -> bool {
        self.extract_metadata.unwrap_or(false)
    }

    pub fn cleanup_downloads(&self) -> bool {
        self.cleanup_downloads.unwrap_or(false)
    }

    pub fn ignored_patterns(&self) -> Vec<String> {
        self.ignored_patterns.clone().unwrap_or_default()
    }
}

fn get_config_path() -> Result<PathBuf> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        if !config_home.is_empty() {
            return Ok(PathBuf::from(config_home).join("ebook-renamer").join(CONFIG_FILENAME));
        }
    }

    if let Some(home) = dirs::home_dir() {
        return Ok(home.join(".config").join("ebook-renamer").join(CONFIG_FILENAME));
    }

    Err(anyhow!("Could not determine config directory"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.cloud_detection());
        assert!(!config.interactive_mode());
        assert!(!config.dry_run_default());
        assert!(config.hash_cloud_files());
    }

    #[test]
    fn test_config_custom_values() {
        let config = Config {
            default_dir: Some("/books".to_string()),
            cloud_detection: Some(false),
            interactive_mode: Some(true),
            dry_run_default: Some(true),
            ignored_patterns: Some(vec!["*.tmp".to_string()]),
            hash_cloud_files: Some(false),
            fetch_arxiv: Some(true),
            extract_metadata: Some(true),
            cleanup_downloads: Some(true),
        };

        assert_eq!(config.get_default_dir(), Some(PathBuf::from("/books")));
        assert!(!config.cloud_detection());
        assert!(config.interactive_mode());
        assert!(config.dry_run_default());
        assert!(!config.hash_cloud_files());
        assert!(config.fetch_arxiv());
        assert!(config.extract_metadata());
        assert!(config.cleanup_downloads());
        assert_eq!(config.ignored_patterns(), vec!["*.tmp"]);
    }

    #[test]
    fn test_config_load_and_save() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let config_path = tmp_dir.path().join(CONFIG_FILENAME);

        let config = Config {
            default_dir: Some("/test/books".to_string()),
            cloud_detection: Some(false),
            interactive_mode: Some(true),
            dry_run_default: Some(false),
            ignored_patterns: Some(vec!["*.log".to_string()]),
            hash_cloud_files: Some(true),
            fetch_arxiv: Some(false),
            extract_metadata: Some(true),
            cleanup_downloads: Some(false),
        };

        let content = toml::to_string_pretty(&config)?;
        fs::write(&config_path, &content)?;

        let loaded: Config = toml::from_str(&content)?;
        assert_eq!(loaded.default_dir, config.default_dir);
        assert_eq!(loaded.cloud_detection, config.cloud_detection);
        assert_eq!(loaded.interactive_mode, config.interactive_mode);

        Ok(())
    }

    #[test]
    fn test_default_dir_tilde_expansion() {
        let config = Config {
            default_dir: Some("~/Books".to_string()),
            ..Config::default()
        };

        let path = config.get_default_dir();
        assert!(path.is_some());
        let binding = path.unwrap();
        let path_str = binding.to_string_lossy();
        assert!(path_str.contains("Books"));
    }
}
