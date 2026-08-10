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
    // TODO(subagent enrich): filter summaries where game_id == Some(game.app_id).
    let _ = (summaries, game);
    Vec::new()
}

/// Friends among `summaries` in the exact same lobby as `game` (matching
/// lobby_steam_id). Returns sorted, deduplicated persona names.
pub fn lobby_friends(summaries: &[PlayerSummary], game: &CurrentGame) -> Vec<String> {
    // TODO(subagent enrich): match lobby_steam_id when game.lobby_steam_id is Some.
    let _ = (summaries, game);
    Vec::new()
}

/// Resolve server name + current map from the game server Steam reports.
/// Returns `(server_name, map)` or `(None, None)` when there is no public
/// server or the query fails.
pub fn server_and_map(a2s: &mut A2sClient, game: &CurrentGame) -> (Option<String>, Option<String>) {
    // TODO(subagent enrich): if game.gameserver_ip is Some, query it; swallow errors.
    let _ = (a2s, game);
    (None, None)
}

/// Collect all enrichment for a running game. Never returns an error: every
/// component degrades gracefully when the API is unavailable.
pub fn analyze(
    client: &SteamClient,
    a2s: &mut A2sClient,
    me: u64,
    game: &CurrentGame,
) -> Enrichment {
    // TODO(subagent enrich): friends -> summaries -> same_game/lobby filters,
    //   plus server_and_map. Use best-effort error handling per step.
    let _ = (client, a2s, me, game);
    Enrichment::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_friends_plays_nothing() {
        let game = CurrentGame {
            app_id: 440,
            name: "TF2".into(),
            gamedir: Some("tf".into()),
            lobby_steam_id: None,
            gameserver_ip: None,
        };
        assert!(same_game_friends(&[], &game).is_empty());
        assert!(lobby_friends(&[], &game).is_empty());
    }
}
