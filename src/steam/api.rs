use serde::Deserialize;

use crate::model::LogEntry;

// ---------------------------------------------------------------------------
// Domain types produced by the Steam Web API.
// ---------------------------------------------------------------------------

/// The game a player is currently running, as reported by GetPlayerSummaries.
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentGame {
    pub app_id: u32,
    /// Display name, e.g. "Team Fortress 2".
    pub name: String,
    /// Game dir, e.g. "tf", "csgo", "garrysmod".
    pub gamedir: Option<String>,
    /// SteamID64 of the lobby, when the player is in one.
    pub lobby_steam_id: Option<String>,
    /// `ip:port` of the game server, when connected to a public one.
    pub gameserver_ip: Option<String>,
}

/// A friend, as reported by GetFriendList.
#[derive(Debug, Clone, PartialEq)]
pub struct Friend {
    pub steam_id: u64,
    pub persona_name: String,
}

/// A player summary row from GetPlayerSummaries.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerSummary {
    pub steam_id: u64,
    pub persona_name: String,
    pub game_id: Option<u32>,
    pub game_extra_info: Option<String>,
    pub lobby_steam_id: Option<String>,
}

/// Converts a player summary into a `CurrentGame` when the player is in-game.
pub fn summary_to_current_game(summary: &PlayerSummary) -> Option<CurrentGame> {
    // TODO(subagent api): build CurrentGame when summary.game_id is Some.
    let _ = summary;
    None
}

// ---------------------------------------------------------------------------
// Raw JSON shapes (mirror the Steam Web API responses).
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct PlayerSummariesResponse {
    pub response: PlayerSummariesBody,
}

#[derive(Debug, Deserialize)]
pub struct PlayerSummariesBody {
    pub players: Vec<RawPlayerSummary>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawPlayerSummary {
    pub steamid: String,
    pub personaname: String,
    #[serde(default)]
    pub gameid: Option<String>,
    #[serde(default)]
    pub gameextrainfo: Option<String>,
    #[serde(default)]
    pub lobbysteamid: Option<String>,
    #[serde(default)]
    pub gameserverip: Option<String>,
    #[serde(default)]
    pub gamedir: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FriendListResponse {
    pub friendslist: FriendListBody,
}

#[derive(Debug, Deserialize)]
pub struct FriendListBody {
    pub friends: Vec<RawFriend>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawFriend {
    pub steamid: String,
    pub relationship: String,
}

// ---------------------------------------------------------------------------
// URL builders.
// ---------------------------------------------------------------------------

/// `ISteamUser/GetPlayerSummaries/v2` URL. Accepts up to 100 comma-separated ids.
pub fn player_summaries_url(key: &str, ids: &[u64]) -> String {
    // TODO(subagent api): build the URL.
    let _ = (key, ids);
    String::new()
}

/// `ISteamUser/GetFriendList/v1` URL.
pub fn friend_list_url(key: &str, steam_id: u64) -> String {
    // TODO(subagent api): build the URL.
    let _ = (key, steam_id);
    String::new()
}

// ---------------------------------------------------------------------------
// JSON parsing helpers (pure, unit-testable without a network).
// ---------------------------------------------------------------------------

/// Parse a GetPlayerSummaries response body into player summaries.
pub fn parse_player_summaries(json: &str) -> anyhow::Result<Vec<PlayerSummary>> {
    // TODO(subagent api): deserialize and map RawPlayerSummary -> PlayerSummary.
    //   steamid is a string of digits -> parse to u64. gameid likewise -> u32.
    //   Skip rows that fail to parse rather than erroring the whole batch.
    let _ = json;
    anyhow::bail!("not implemented yet")
}

/// Parse a GetFriendList response body into friends (relationship == "friend").
pub fn parse_friends(json: &str) -> anyhow::Result<Vec<Friend>> {
    // TODO(subagent api): deserialize and filter by relationship.
    let _ = json;
    anyhow::bail!("not implemented yet")
}

/// Build the JSON object string for a log entry (used by docs/tests).
pub fn entry_to_json(entry: &LogEntry) -> String {
    // TODO(subagent api): serialize with serde_json.
    let _ = entry;
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_without_game_is_none() {
        let s = PlayerSummary {
            steam_id: 1,
            persona_name: "x".into(),
            game_id: None,
            game_extra_info: None,
            lobby_steam_id: None,
        };
        assert!(summary_to_current_game(&s).is_none());
    }
}
