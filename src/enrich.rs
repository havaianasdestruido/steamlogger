use crate::a2s::A2sClient;
use crate::steam::api::{CurrentGame, PlayerSummary};
use crate::steam::client::SteamClient;

/// Best-effort enrichment data collected while a game is running.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Enrichment {
    pub map: Option<String>,
    pub server: Option<String>,
    pub friends_playing: Vec<String>,
    pub friends_in_lobby: Vec<String>,
}

/// Friends among `summaries` currently playing the same game as `game`.
/// Returns sorted, deduplicated persona names.
pub fn same_game_friends(summaries: &[PlayerSummary], game: &CurrentGame) -> Vec<String> {
    let mut names: Vec<String> = summaries
        .iter()
        .filter(|s| s.game_id == Some(game.app_id))
        .map(|s| s.persona_name.clone())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Friends among `summaries` in the exact same lobby as `game` (matching
/// lobby_steam_id). Returns sorted, deduplicated persona names.
pub fn lobby_friends(summaries: &[PlayerSummary], game: &CurrentGame) -> Vec<String> {
    let Some(lobby) = game.lobby_steam_id.as_deref() else {
        return Vec::new();
    };
    let mut names: Vec<String> = summaries
        .iter()
        .filter(|s| s.lobby_steam_id.as_deref() == Some(lobby))
        .map(|s| s.persona_name.clone())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Resolve server name + current map from the game server Steam reports.
/// Returns `(server_name, map)` or `(None, None)` when there is no public
/// server or the query fails.
pub fn server_and_map(a2s: &mut A2sClient, game: &CurrentGame) -> (Option<String>, Option<String>) {
    let Some(addr) = game.gameserver_ip.as_deref() else {
        return (None, None);
    };
    match a2s.query(addr) {
        Ok(info) => (Some(info.name), Some(info.map)),
        Err(_) => (None, None),
    }
}

/// Collect all enrichment for a running game. Never returns an error: every
/// component degrades gracefully when the API is unavailable.
pub fn analyze(
    client: &SteamClient,
    a2s: &mut A2sClient,
    me: u64,
    game: &CurrentGame,
) -> Enrichment {
    let Ok(friends) = client.friends(me) else {
        return Enrichment::default();
    };
    let ids: Vec<u64> = friends.iter().map(|f| f.steam_id).collect();
    let Ok(summaries) = client.summaries(&ids) else {
        return Enrichment::default();
    };
    let (server, map) = server_and_map(a2s, game);
    build(&summaries, game, map, server)
}

fn build(
    summaries: &[PlayerSummary],
    game: &CurrentGame,
    map: Option<String>,
    server: Option<String>,
) -> Enrichment {
    Enrichment {
        map,
        server,
        friends_playing: same_game_friends(summaries, game),
        friends_in_lobby: lobby_friends(summaries, game),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(
        steam_id: u64,
        persona_name: &str,
        game_id: Option<u32>,
        lobby: Option<&str>,
    ) -> PlayerSummary {
        PlayerSummary {
            steam_id,
            persona_name: persona_name.into(),
            game_id,
            game_extra_info: None,
            lobby_steam_id: lobby.map(String::from),
            gamedir: None,
            gameserver_ip: None,
        }
    }

    fn game(app_id: u32, lobby: Option<&str>, gameserver_ip: Option<&str>) -> CurrentGame {
        CurrentGame {
            app_id,
            name: "TF2".into(),
            gamedir: Some("tf".into()),
            lobby_steam_id: lobby.map(String::from),
            gameserver_ip: gameserver_ip.map(String::from),
        }
    }

    #[test]
    fn no_friends_plays_nothing() {
        let game = game(440, None, None);
        assert!(same_game_friends(&[], &game).is_empty());
        assert!(lobby_friends(&[], &game).is_empty());
    }

    #[test]
    fn same_game_friends_filters_sorts_dedupes() {
        let game = game(440, None, None);
        let summaries = [
            summary(1, "zed", Some(440), None),
            summary(2, "alpha", Some(440), None),
            summary(3, "bravo", Some(730), None),
            summary(4, "alpha", Some(440), None),
            summary(5, "delta", None, None),
        ];
        assert_eq!(same_game_friends(&summaries, &game), vec!["alpha", "zed"]);
    }

    #[test]
    fn lobby_friends_matches_same_lobby() {
        let game = game(440, Some("lobby-1"), None);
        let summaries = [
            summary(1, "zed", Some(440), Some("lobby-1")),
            summary(2, "alpha", Some(440), Some("lobby-1")),
            summary(3, "bravo", Some(440), Some("lobby-2")),
            summary(4, "alpha", Some(440), Some("lobby-1")),
            summary(5, "delta", Some(440), None),
        ];
        assert_eq!(lobby_friends(&summaries, &game), vec!["alpha", "zed"]);
    }

    #[test]
    fn lobby_friends_ignored_when_no_lobby() {
        let game = game(440, None, None);
        let summaries = [summary(1, "zed", Some(440), Some("lobby-1"))];
        assert!(lobby_friends(&summaries, &game).is_empty());
    }

    #[test]
    fn no_gameserver_ip_yields_no_server_or_map() {
        let mut a2s = A2sClient::new();
        let game = game(440, None, None);
        assert_eq!(server_and_map(&mut a2s, &game), (None, None));
    }

    #[test]
    fn build_assembles_enrichment() {
        let game = game(440, Some("lobby-1"), None);
        let summaries = [
            summary(1, "zed", Some(440), Some("lobby-1")),
            summary(2, "bravo", Some(730), None),
        ];
        let got = build(
            &summaries,
            &game,
            Some("cp_dustbowl".into()),
            Some("server-1".into()),
        );
        assert_eq!(
            got,
            Enrichment {
                map: Some("cp_dustbowl".into()),
                server: Some("server-1".into()),
                friends_playing: vec!["zed".into()],
                friends_in_lobby: vec!["zed".into()],
            }
        );
    }
}
