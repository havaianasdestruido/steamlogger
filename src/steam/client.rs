use std::time::Duration;

use crate::steam::api::{CurrentGame, Friend, PlayerSummary};

/// Blocking HTTP client wrapping the Steam Web API. One instance is enough
/// for the whole process; reqwest's blocking client keeps an internal
/// connection pool.
#[derive(Debug)]
pub struct SteamClient {
    key: String,
    http: reqwest::blocking::Client,
}

impl SteamClient {
    pub fn new(key: String) -> Self {
        Self::with_timeout(key, Duration::from_secs(10))
    }

    pub fn with_timeout(key: String, timeout: Duration) -> Self {
        let http = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .user_agent("SteamLogger/0.1 (github.com/steamlogger)")
            .build()
            .expect("failed to build HTTP client");
        Self { key, http }
    }

    /// Current game of `steam_id`, `None` when not in-game.
    pub fn current_game(&self, steam_id: u64) -> anyhow::Result<Option<CurrentGame>> {
        // TODO(subagent client): GET player_summaries_url(self.key, &[steam_id]),
        //   parse, map first summary with summary_to_current_game.
        let _ = steam_id;
        Ok(None)
    }

    /// Friend list (relationship == "friend").
    pub fn friends(&self, steam_id: u64) -> anyhow::Result<Vec<Friend>> {
        // TODO(subagent client): GET friend_list_url.
        let _ = steam_id;
        Ok(Vec::new())
    }

    /// Player summaries for up to 100 ids. Chunks larger inputs internally
    /// and merges results.
    pub fn summaries(&self, ids: &[u64]) -> anyhow::Result<Vec<PlayerSummary>> {
        // TODO(subagent client): chunk by 100, GET summaries per chunk, merge.
        let _ = ids;
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_constructs() {
        let _c = SteamClient::new("dummy-key".into());
    }
}
