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

impl RawPlayerSummary {
    /// Maps a raw row to a `PlayerSummary`, skipping rows with unparseable ids.
    fn into_summary(self) -> Option<PlayerSummary> {
        let steam_id = self.steamid.parse().ok()?;
        let game_id = self.gameid.as_deref().map(str::parse).transpose().ok()?;
        Some(PlayerSummary {
            steam_id,
            persona_name: self.personaname,
            game_id,
            game_extra_info: self.gameextrainfo,
            lobby_steam_id: self.lobbysteamid,
            gamedir: self.gamedir,
            gameserver_ip: self.gameserverip,
        })
    }
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
    pub gamedir: Option<String>,
    pub gameserver_ip: Option<String>,
}

/// Converts a player summary into a `CurrentGame` when the player is in-game.
pub fn summary_to_current_game(summary: &PlayerSummary) -> Option<CurrentGame> {
    let app_id = summary.game_id?;
    let name = summary
        .game_extra_info
        .clone()
        .unwrap_or_else(|| format!("App {app_id}"));
    Some(CurrentGame {
        app_id,
        name,
        gamedir: summary.gamedir.clone(),
        lobby_steam_id: summary.lobby_steam_id.clone(),
        gameserver_ip: summary.gameserver_ip.clone(),
    })
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
    let ids = ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
    format!(
        "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/?key={key}&steamids={ids}"
    )
}

/// `ISteamUser/GetFriendList/v1` URL.
pub fn friend_list_url(key: &str, steam_id: u64) -> String {
    format!(
        "https://api.steampowered.com/ISteamUser/GetFriendList/v1/?key={key}&steamid={steam_id}&relationship=friend"
    )
}

// ---------------------------------------------------------------------------
// JSON parsing helpers (pure, unit-testable without a network).
// ---------------------------------------------------------------------------

/// Parse a GetPlayerSummaries response body into player summaries.
pub fn parse_player_summaries(json: &str) -> anyhow::Result<Vec<PlayerSummary>> {
    let resp: PlayerSummariesResponse = serde_json::from_str(json)?;
    Ok(resp
        .response
        .players
        .into_iter()
        .filter_map(RawPlayerSummary::into_summary)
        .collect())
}

/// Parse a GetFriendList response body into friends (relationship == "friend").
pub fn parse_friends(json: &str) -> anyhow::Result<Vec<Friend>> {
    let resp: FriendListResponse = serde_json::from_str(json)?;
    Ok(resp
        .friendslist
        .friends
        .into_iter()
        .filter(|f| f.relationship == "friend")
        .filter_map(|f| {
            let steam_id = f.steamid.parse().ok()?;
            Some(Friend {
                steam_id,
                persona_name: String::new(),
            })
        })
        .collect())
}

/// Build the JSON object string for a log entry (used by docs/tests).
pub fn entry_to_json(entry: &LogEntry) -> String {
    serde_json::to_string(entry).expect("LogEntry serialization cannot fail")
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
            gamedir: None,
            gameserver_ip: None,
        };
        assert!(summary_to_current_game(&s).is_none());
    }

    #[test]
    fn summary_to_current_game_maps_fields() {
        let s = PlayerSummary {
            steam_id: 76561198000000000,
            persona_name: "Foo".into(),
            game_id: Some(440),
            game_extra_info: Some("Team Fortress 2".into()),
            lobby_steam_id: Some("109775241716070123".into()),
            gamedir: Some("tf".into()),
            gameserver_ip: Some("192.168.1.1:27015".into()),
        };
        let g = summary_to_current_game(&s).expect("in game");
        assert_eq!(
            g,
            CurrentGame {
                app_id: 440,
                name: "Team Fortress 2".into(),
                gamedir: Some("tf".into()),
                lobby_steam_id: Some("109775241716070123".into()),
                gameserver_ip: Some("192.168.1.1:27015".into()),
            }
        );
    }

    #[test]
    fn summary_missing_game_extra_info_uses_app_name() {
        let s = PlayerSummary {
            steam_id: 1,
            persona_name: "x".into(),
            game_id: Some(440),
            game_extra_info: None,
            lobby_steam_id: None,
            gamedir: None,
            gameserver_ip: None,
        };
        let g = summary_to_current_game(&s).expect("in game");
        assert_eq!(g.app_id, 440);
        assert_eq!(g.name, "App 440");
    }

    #[test]
    fn parse_player_summaries_maps_in_game_and_idle_players() {
        let json = r#"{"response":{"players":[
            {"steamid":"76561198000000000","personaname":"Foo","gameid":"440","gameextrainfo":"Team Fortress 2","lobbysteamid":"109775241716070123","gameserverip":"192.168.1.1:27015","gamedir":"tf"},
            {"steamid":"76561198000000001","personaname":"Bar"}
        ]}}"#;
        let rows = parse_player_summaries(json).expect("valid payload");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            PlayerSummary {
                steam_id: 76561198000000000,
                persona_name: "Foo".into(),
                game_id: Some(440),
                game_extra_info: Some("Team Fortress 2".into()),
                lobby_steam_id: Some("109775241716070123".into()),
                gamedir: Some("tf".into()),
                gameserver_ip: Some("192.168.1.1:27015".into()),
            }
        );
        assert_eq!(
            rows[1],
            PlayerSummary {
                steam_id: 76561198000000001,
                persona_name: "Bar".into(),
                game_id: None,
                game_extra_info: None,
                lobby_steam_id: None,
                gamedir: None,
                gameserver_ip: None,
            }
        );
    }

    #[test]
    fn parse_player_summaries_empty_players() {
        let rows = parse_player_summaries(r#"{"response":{"players":[]}}"#).expect("valid");
        assert!(rows.is_empty());
    }

    #[test]
    fn parse_player_summaries_garbage_is_error() {
        assert!(parse_player_summaries("not json").is_err());
    }

    #[test]
    fn parse_player_summaries_skips_row_with_bad_steamid() {
        let json = r#"{"response":{"players":[
            {"steamid":"not-a-number","personaname":"Broken"},
            {"steamid":"76561198000000001","personaname":"Good"}
        ]}}"#;
        let rows = parse_player_summaries(json).expect("valid payload");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].steam_id, 76561198000000001);
    }

    #[test]
    fn parse_friends_filters_non_friends() {
        let json = r#"{"friendslist":{"friends":[
            {"steamid":"76561198000000001","relationship":"friend","friend_since":1700000000},
            {"steamid":"76561198000000002","relationship":"blocked"},
            {"steamid":"not-a-number","relationship":"friend"},
            {"steamid":"76561198000000003","relationship":"friend","friend_since":1700000001}
        ]}}"#;
        let friends = parse_friends(json).expect("valid payload");
        assert_eq!(
            friends,
            vec![
                Friend {
                    steam_id: 76561198000000001,
                    persona_name: String::new()
                },
                Friend {
                    steam_id: 76561198000000003,
                    persona_name: String::new()
                },
            ]
        );
    }

    #[test]
    fn url_builders_produce_expected_urls() {
        assert_eq!(
            player_summaries_url("K", &[76561198000000000, 76561198000000001]),
            "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/?key=K&steamids=76561198000000000,76561198000000001"
        );
        assert_eq!(
            friend_list_url("K", 76561198000000000),
            "https://api.steampowered.com/ISteamUser/GetFriendList/v1/?key=K&steamid=76561198000000000&relationship=friend"
        );
    }

    #[test]
    fn entry_to_json_round_trips() {
        let entry = LogEntry::new("Team Fortress 2".into(), "2026-08-10T14:32:15-03:00".into());
        let back: LogEntry = serde_json::from_str(&entry_to_json(&entry)).expect("round trip");
        assert_eq!(entry, back);
    }
}
