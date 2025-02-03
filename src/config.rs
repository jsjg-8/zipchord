// src/config.rs
use anyhow::{Context, Result};
use dirs::{config_dir, home_dir};
use std::{
    env, path::{Path, PathBuf}, time::Duration
};

const DEFAULT_TIMEOUT_MS: u64 = 20;
const APP_NAME: &str = "chords";

#[derive(Debug)]
pub struct AppConfig {
    pub library_path: PathBuf,
    pub chord_timeout: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid configuration value: {0}")]
    Validation(String),
    #[error("Missing required directory path")]
    MissingDirectory,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let mut config = Self::defaults()?;
        
        if let Some(config_path) = Self::config_file_path() {
            if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)
                    .with_context(|| format!("Failed to read {}", config_path.display()))?;
                Self::parse_ini(&content, &mut config)?;
            }
        }

        config.validate()?;
        Ok(config)
    }

    fn parse_ini(content: &str, config: &mut Self) -> Result<()> {
        for line in content.lines() {
            let line = line.trim().split(';').next().unwrap_or("").trim(); // Handle comments
            if line.is_empty() || line.starts_with('[') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_lowercase();
                let value = value.trim();

                match key.as_str() {
                    "library_path" => {
                        config.library_path = Self::expand_path(value)
                            .context("Failed to expand library path")?;
                    }
                    "chord_timeout" => {
                        config.chord_timeout = Duration::from_millis(
                            value.parse()
                                .context("Failed to parse chord timeout")?
                        );
                    }
                    _ => continue
                }
            }
        }
        
        Ok(())
    }

    fn config_file_path() -> Option<PathBuf> {
        config_dir().map(|path| path.join(APP_NAME).join("config.ini"))
    }

    fn defaults() -> Result<Self> {
        Ok(Self {
            library_path: Self::default_library_path()?,
            chord_timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
        })
    }

    fn default_library_path() -> Result<PathBuf> {
        effective_user_dir()
            .map(|path| path.join(".config").join(APP_NAME).join("lib"))
    }

    fn expand_path(path: &str) -> Result<PathBuf> {
        let path = Path::new(path);
        
        if path.starts_with("~") {
            let home = effective_user_dir()?;  // Use this instead of home_dir()
            return Ok(home.join(path.strip_prefix("~").unwrap()));
        }

        Ok(path.to_path_buf())
    }

    fn validate(&self) -> Result<()> {
        if !self.library_path.exists() {
            return Err(ConfigError::Validation(
                format!("Library path {} does not exist or can't be accessed", self.library_path.display())
            ).into());
        }
        
        if self.chord_timeout > Duration::from_secs(1) {
            return Err(ConfigError::Validation(
                format!("Chord timeout cannot exceed 1000ms (got {}ms)", self.chord_timeout.as_millis())
            ).into());
        }
        
        Ok(())
    }
}

fn effective_user_dir() -> Result<PathBuf> {
    match env::var_os("SUDO_USER") {
        Some(user) => Ok(PathBuf::from("/home").join(user)),
        None => home_dir().ok_or(ConfigError::MissingDirectory.into()),
    }
}