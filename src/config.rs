use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Fully resolved runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Steam Web API key.
    pub api_key: String,
    /// Numeric SteamID64 of the tracked user.
    pub steam_id: u64,
    /// Seconds between polls of the Steam API.
    pub poll_interval_secs: u64,
    /// Path of the JSON log file written by the app.
    pub output_file: PathBuf,
    /// Whether to collect friends/lobby/map enrichment data.
    pub enrich: bool,
}

/// Raw TOML shape with defaults, loaded from `steamlogger.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RawConfig {
    pub api_key: Option<String>,
    pub steam_id: Option<u64>,
    pub poll_interval_secs: u64,
    pub output_file: PathBuf,
    pub enrich: bool,
}

impl Default for RawConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            steam_id: None,
            poll_interval_secs: 30,
            output_file: PathBuf::from("steamlog.json"),
            enrich: true,
        }
    }
}

/// Load and validate a config file. Missing file or missing required keys
/// (`api_key`, `steam_id`) is an error; optional keys fall back to defaults.
/// Environment variables `STEAMLOGGER_API_KEY` / `STEAMLOGGER_STEAM_ID`
/// override the file values when set.
pub fn load(path: &Path) -> anyhow::Result<Config> {
    // TODO(subagent config): read file, parse RawConfig, merge env vars, validate.
    let _ = path;
    anyhow::bail!("config loading not implemented yet")
}

/// Renders a fully commented example TOML config for documentation.
pub fn example_config() -> String {
    // TODO(subagent config): return commented example.
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let raw = RawConfig::default();
        assert_eq!(raw.poll_interval_secs, 30);
        assert_eq!(raw.output_file, PathBuf::from("steamlog.json"));
        assert!(raw.enrich);
    }
}
