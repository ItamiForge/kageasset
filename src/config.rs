use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = ".kat.toml";
const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    "**/node_modules/**",
    "**/.git/**",
    "**/.hg/**",
    "**/.svn/**",
    "**/target/**",
    "**/build/**",
    "**/dist/**",
    "**/__pycache__/**",
    "**/.pytest_cache/**",
    "**/venv/**",
    "**/.venv/**",
    "**/.idea/**",
    "**/.vscode/**",
    "**/android/**",
    "**/ios/**",
    "**/.expo/**",
    "**/.expo-shared/**",
    "**/Pods/**",
    "**/DerivedData/**",
    "**/.gradle/**",
    "**/.buckd/**",
];

#[derive(Debug, Clone)]
pub struct Config {
    pub scan: ScanSection,
    pub source_path: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan: ScanSection::default(),
            source_path: None,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ScanSection {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ignore: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_default_ignores: Option<bool>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ConfigFile {
    #[serde(skip_serializing_if = "ScanSection::is_empty", default)]
    pub scan: ScanSection,
}

impl ScanSection {
    pub fn is_empty(&self) -> bool {
        self.ignore.is_empty() && self.extensions.is_empty() && self.use_default_ignores.is_none()
    }
}

pub fn load(path_override: Option<PathBuf>) -> Result<Config> {
    let override_specified = path_override.is_some();

    let config_path = if let Some(path) = path_override {
        if path.is_dir() {
            bail!("config path '{}' is a directory", path.display());
        }
        path
    } else {
        std::env::current_dir()
            .context("failed to resolve current directory when loading config")?
            .join(CONFIG_FILE_NAME)
    };

    if !config_path.exists() {
        if override_specified {
            bail!("configuration file '{}' not found", config_path.display());
        }
        return Ok(Config::default());
    }

    let contents = fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read config file {}", config_path.display()))?;
    let parsed: ConfigFile = toml::from_str(&contents)
        .with_context(|| format!("failed to parse config file {}", config_path.display()))?;

    Ok(Config {
        scan: parsed.scan,
        source_path: Some(config_path),
    })
}

pub fn default_ignore_patterns() -> &'static [&'static str] {
    DEFAULT_IGNORE_PATTERNS
}

pub fn default_config_file_name() -> &'static str {
    CONFIG_FILE_NAME
}

pub fn write_config_file(path: &Path, config: &ConfigFile) -> Result<()> {
    let serialized = toml::to_string_pretty(config)?;
    fs::write(path, serialized)
        .with_context(|| format!("failed to write config file {}", path.display()))?;
    Ok(())
}
