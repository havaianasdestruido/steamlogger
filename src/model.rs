use serde::{Deserialize, Serialize};

/// One recorded game session. Times are RFC3339 strings, e.g.
/// `2026-08-10T14:32:15-03:00`. Enrichment fields are optional so the base
/// format stays compatible with the minimal expected JSON shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    pub name: String,
    pub start: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub friends_playing: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub friends_in_lobby: Option<Vec<String>>,
}

impl LogEntry {
    pub fn new(name: String, start: String) -> Self {
        Self {
            name,
            start,
            end: None,
            map: None,
            server: None,
            friends_playing: None,
            friends_in_lobby: None,
        }
    }
}

/// Root object of the log file: `{ "games": [ ... ] }`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GameLog {
    pub games: Vec<LogEntry>,
}
