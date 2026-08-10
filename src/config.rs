use std::path::{Path, PathBuf};

use anyhow::Context;
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
    if !path.is_file() {
        anyhow::bail!(
            "config file not found: {} (see steamlogger.example.toml for a commented example)",
            path.display()
        );
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    let raw = toml::from_str::<RawConfig>(&text)
        .with_context(|| format!("invalid TOML in config file {}", path.display()))?;

    let api_key = match std::env::var("STEAMLOGGER_API_KEY") {
        Ok(key) if !key.is_empty() => Some(key),
        _ => raw.api_key,
    };
    let steam_id = match std::env::var("STEAMLOGGER_STEAM_ID") {
        Ok(id) => Some(
            id.trim()
                .parse::<u64>()
                .context("STEAMLOGGER_STEAM_ID must be a valid u64")?,
        ),
        Err(_) => raw.steam_id,
    };

    let api_key = api_key.ok_or_else(|| {
        anyhow::anyhow!(
            "missing required config key `api_key` (set it in steamlogger.toml or via STEAMLOGGER_API_KEY)"
        )
    })?;
    let steam_id = steam_id.ok_or_else(|| {
        anyhow::anyhow!(
            "missing required config key `steam_id` (set it in steamlogger.toml or via STEAMLOGGER_STEAM_ID)"
        )
    })?;

    Ok(Config {
        api_key,
        steam_id,
        poll_interval_secs: raw.poll_interval_secs,
        output_file: raw.output_file,
        enrich: raw.enrich,
    })
}

/// Renders a fully commented example TOML config for documentation.
pub fn example_config() -> String {
    r#"# SteamLogger example configuration.
# Copy this file to `steamlogger.toml` next to the binary and edit the values.

# Steam Web API key. Required.
# Get one at https://steamcommunity.com/dev/apikey
# May be overridden by the STEAMLOGGER_API_KEY environment variable.
api_key = "YOUR_STEAM_API_KEY"

# Numeric SteamID64 of the tracked user. Required.
# Find yours at https://steamid.io or in your Steam profile URL's vanity lookup.
# May be overridden by the STEAMLOGGER_STEAM_ID environment variable.
steam_id = 76561198000000000

# Seconds between polls of the Steam API. Optional, defaults to 30.
poll_interval_secs = 30

# Path of the JSON log file written by the app. Optional, defaults to "steamlog.json".
output_file = "steamlog.json"

# Whether to collect friends/lobby/map enrichment data. Optional, defaults to true.
enrich = true
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn temp_config_file(contents: &str) -> PathBuf {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "steamlogger_test_{}_{}.toml",
            std::process::id(),
            id
        ));
        std::fs::write(&path, contents).expect("write temp config");
        path
    }

    fn minimal_toml() -> String {
        "api_key = \"key123\"\nsteam_id = 76561198000000000\n".to_string()
    }

    #[test]
    fn defaults_are_sane() {
        let raw = RawConfig::default();
        assert_eq!(raw.poll_interval_secs, 30);
        assert_eq!(raw.output_file, PathBuf::from("steamlog.json"));
        assert!(raw.enrich);
        assert!(raw.api_key.is_none());
        assert!(raw.steam_id.is_none());
    }

    #[test]
    fn minimal_config_loads_with_defaults() {
        let path = temp_config_file(&minimal_toml());
        let cfg = load(&path).expect("load valid config");
        assert_eq!(cfg.api_key, "key123");
        assert_eq!(cfg.steam_id, 76561198000000000);
        assert_eq!(cfg.poll_interval_secs, 30);
        assert_eq!(cfg.output_file, PathBuf::from("steamlog.json"));
        assert!(cfg.enrich);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn explicit_values_override_defaults() {
        let toml = "api_key = \"k\"\nsteam_id = 123\npoll_interval_secs = 5\noutput_file = \"out.json\"\nenrich = false\n";
        let path = temp_config_file(toml);
        let cfg = load(&path).expect("load config");
        assert_eq!(cfg.poll_interval_secs, 5);
        assert_eq!(cfg.output_file, PathBuf::from("out.json"));
        assert!(!cfg.enrich);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn missing_api_key_is_error() {
        let _guard = ENV_LOCK.lock();
        std::env::remove_var("STEAMLOGGER_API_KEY");
        std::env::remove_var("STEAMLOGGER_STEAM_ID");
        let path = temp_config_file("steam_id = 76561198000000000\n");
        let err = load(&path).unwrap_err().to_string();
        assert!(err.contains("api_key"), "unexpected error: {err}");
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn missing_steam_id_is_error() {
        let _guard = ENV_LOCK.lock();
        std::env::remove_var("STEAMLOGGER_API_KEY");
        std::env::remove_var("STEAMLOGGER_STEAM_ID");
        let path = temp_config_file("api_key = \"key123\"\n");
        let err = load(&path).unwrap_err().to_string();
        assert!(err.contains("steam_id"), "unexpected error: {err}");
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn nonexistent_file_is_error() {
        let path = std::env::temp_dir().join("steamlogger_does_not_exist.toml");
        std::fs::remove_file(&path).ok();
        let err = load(&path).unwrap_err().to_string();
        assert!(
            err.contains("steamlogger.example.toml"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn invalid_toml_is_error() {
        let path = temp_config_file("this is not [valid toml");
        assert!(load(&path).is_err());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn api_key_env_override() {
        let _guard = ENV_LOCK.lock();
        let path = temp_config_file("api_key = \"from_file\"\nsteam_id = 76561198000000000\n");
        std::env::set_var("STEAMLOGGER_API_KEY", "from_env");
        let result = load(&path).map(|c| c.api_key);
        std::env::remove_var("STEAMLOGGER_API_KEY");
        assert_eq!(result.expect("load config"), "from_env");
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn steam_id_env_override() {
        let _guard = ENV_LOCK.lock();
        let path = temp_config_file("api_key = \"key123\"\nsteam_id = 1\n");
        std::env::set_var("STEAMLOGGER_STEAM_ID", "76561198000000000");
        let result = load(&path).map(|c| c.steam_id);
        std::env::remove_var("STEAMLOGGER_STEAM_ID");
        assert_eq!(result.expect("load config"), 76561198000000000);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn env_can_supply_missing_keys() {
        let _guard = ENV_LOCK.lock();
        let path = temp_config_file("poll_interval_secs = 10\n");
        std::env::set_var("STEAMLOGGER_API_KEY", "from_env");
        std::env::set_var("STEAMLOGGER_STEAM_ID", "42");
        let result = load(&path);
        std::env::remove_var("STEAMLOGGER_API_KEY");
        std::env::remove_var("STEAMLOGGER_STEAM_ID");
        let cfg = result.expect("load config from env");
        assert_eq!(cfg.api_key, "from_env");
        assert_eq!(cfg.steam_id, 42);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn example_config_parses_and_has_required_keys() {
        let text = example_config();
        assert!(text.contains("api_key"));
        assert!(text.contains("steam_id"));
        let raw: RawConfig = toml::from_str(&text).expect("example config is valid TOML");
        assert!(raw.api_key.is_some());
        assert!(raw.steam_id.is_some());
    }
}
