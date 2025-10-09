use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// `OtterShipper` server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,
}

/// Server transport and binding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Transport mode: "stdio" or "http"
    #[serde(default = "default_transport")]
    pub transport: String,

    /// HTTP bind address (only used when transport = "http")
    #[serde(default = "default_bind_address")]
    pub bind_address: String,

    /// HTTP port (only used when transport = "http")
    #[serde(default = "default_port")]
    pub port: u16,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Path to `SQLite` database file
    #[serde(default = "default_database_path")]
    pub path: PathBuf,
}

fn default_transport() -> String {
    "stdio".to_string()
}

fn default_bind_address() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_database_path() -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from("./ottershipper.db")
    } else {
        PathBuf::from("/var/lib/ottershipper/ottershipper.db")
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            transport: default_transport(),
            bind_address: default_bind_address(),
            port: default_port(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_database_path(),
        }
    }
}

impl Config {
    /// Load configuration from file, falling back to defaults
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            tracing::info!(
                "Config file not found at {}, using defaults",
                path.display()
            );
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        tracing::info!("Loaded configuration from {}", path.display());
        Ok(config)
    }

    /// Load from default locations in order:
    /// 1. ./ottershipper.toml (current directory)
    /// 2. /etc/ottershipper/config.toml (system-wide)
    /// 3. Built-in defaults
    pub fn load_default() -> Result<Self> {
        let paths = vec![
            PathBuf::from("./ottershipper.toml"),
            PathBuf::from("/etc/ottershipper/config.toml"),
        ];

        for path in paths {
            if path.exists() {
                return Self::load(&path);
            }
        }

        tracing::info!("No config file found, using built-in defaults");
        Ok(Self::default())
    }

    /// Generate example configuration file
    #[must_use]
    pub fn example() -> String {
        let example = Config::default();
        toml::to_string_pretty(&example).expect("Failed to serialize example config")
    }
}
