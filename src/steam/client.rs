use std::time::Duration;

use anyhow::Context;

use crate::steam::api::{
    friend_list_url, parse_friends, parse_player_summaries, player_summaries_url,
    summary_to_current_game, CurrentGame, Friend, PlayerSummary,
};

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
        let url = player_summaries_url(&self.key, &[steam_id]);
        let body = self.get_text(url, "player summaries")?;
        let summaries = parse_player_summaries(&body)?;
        Ok(summaries.first().and_then(summary_to_current_game))
    }

    /// Friend list (relationship == "friend").
    pub fn friends(&self, steam_id: u64) -> anyhow::Result<Vec<Friend>> {
        let url = friend_list_url(&self.key, steam_id);
        let body = self.get_text(url, "friend list")?;
        parse_friends(&body)
    }

    /// Player summaries for up to 100 ids. Chunks larger inputs internally
    /// and merges results.
    pub fn summaries(&self, ids: &[u64]) -> anyhow::Result<Vec<PlayerSummary>> {
        let mut out = Vec::with_capacity(ids.len());
        for chunk in chunks(ids) {
            let url = player_summaries_url(&self.key, chunk);
            let body = self.get_text(url, "player summaries")?;
            out.extend(parse_player_summaries(&body)?);
        }
        Ok(out)
    }

    fn get_text(&self, url: String, what: &str) -> anyhow::Result<String> {
        self.http
            .get(url)
            .send()
            .with_context(|| format!("fetching {what}"))?
            .error_for_status()
            .with_context(|| format!("{what} request failed"))?
            .text()
            .with_context(|| format!("reading {what} response body"))
    }
}

/// Splits `ids` into chunks of at most 100, the Steam API per-request limit.
fn chunks(ids: &[u64]) -> Vec<&[u64]> {
    ids.chunks(100).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_constructs() {
        let _c = SteamClient::new("dummy-key".into());
    }

    #[test]
    fn chunks_empty() {
        assert!(chunks(&[]).is_empty());
    }

    #[test]
    fn chunks_under_limit() {
        let ids = [1u64, 2, 3];
        let got = chunks(&ids);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0], &ids[..]);
    }

    #[test]
    fn chunks_exact_limit() {
        let ids: Vec<u64> = (0..100).collect();
        let got = chunks(&ids);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 100);
    }

    #[test]
    fn chunks_over_limit() {
        let ids: Vec<u64> = (0..101).collect();
        let got = chunks(&ids);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].len(), 100);
        assert_eq!(got[1], &ids[100..]);
    }

    #[test]
    fn chunks_preserve_order() {
        let ids: Vec<u64> = (0..250).collect();
        let flat: Vec<u64> = chunks(&ids)
            .iter()
            .flat_map(|c| c.iter().copied())
            .collect();
        assert_eq!(flat, ids);
    }
}
