use crate::model::{GameLog, LogEntry};
use crate::steam::api::CurrentGame;

/// What happened during one poll tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    /// A new session began; payload is its index in `games`.
    Started(usize),
    /// A session is still running; payload is its index in `games`.
    Running(usize),
    /// The active session ended.
    Ended,
    /// Nothing was in game before or after.
    Idle,
}

/// Tracks the current game session and maintains the in-memory log.
///
/// Resume semantics: if the last entry on disk has `end == None`, the session
/// is treated as still running (SteamLogger was restarted mid-game).
pub struct Tracker {
    log: GameLog,
    active_index: Option<usize>,
    missed_observations: u32,
    max_missed_observations: u32,
}

impl Tracker {
    pub fn new(log: GameLog) -> Self {
        Self::with_missed_tolerance(log, 0)
    }

    pub fn with_missed_tolerance(log: GameLog, max_missed_observations: u32) -> Self {
        let active_index = log.games.last().and_then(|last| match last.end {
            Some(_) => None,
            None => log.games.len().checked_sub(1),
        });
        Self {
            log,
            active_index,
            missed_observations: 0,
            max_missed_observations,
        }
    }

    pub fn into_log(self) -> GameLog {
        self.log
    }

    pub fn log(&self) -> &GameLog {
        &self.log
    }

    pub fn active_index(&self) -> Option<usize> {
        self.active_index
    }

    /// Feed one poll result. A missing observation is tolerated for
    /// `max_missed_observations` consecutive polls before the active session is
    /// closed. Steam can briefly omit current-game data during API hiccups or
    /// status transitions, and closing immediately would make logging look
    /// intermittent.
    pub fn observe(
        &mut self,
        game: Option<&CurrentGame>,
        now: impl FnOnce() -> String,
    ) -> SessionEvent {
        match game {
            Some(game) => {
                self.missed_observations = 0;
                self.update(game, now)
            }
            None => match self.active_index {
                Some(index) if self.missed_observations < self.max_missed_observations => {
                    self.missed_observations += 1;
                    SessionEvent::Running(index)
                }
                Some(_) => {
                    self.missed_observations = 0;
                    self.end_active(now)
                }
                None => SessionEvent::Idle,
            },
        }
    }

    /// Feed the currently-observed game. Handles start/continue/switch:
    /// - no active + game -> push new entry, `Started`
    /// - active + same game -> `Running`
    /// - active + different game -> close old, push new, `Started`
    pub fn update(&mut self, game: &CurrentGame, now: impl FnOnce() -> String) -> SessionEvent {
        self.missed_observations = 0;
        // Keep timestamp generation lazy: continuing a session needs no allocation.
        match self.active_index {
            None => {
                self.log.games.push(LogEntry::new(game.name.clone(), now()));
                let index = self.log.games.len() - 1;
                self.active_index = Some(index);
                SessionEvent::Started(index)
            }
            Some(index) if self.log.games[index].name == game.name => SessionEvent::Running(index),
            Some(index) => {
                let current_time = now();
                self.close_at(index, &current_time);
                self.log
                    .games
                    .push(LogEntry::new(game.name.clone(), current_time));
                let new_index = self.log.games.len() - 1;
                self.active_index = Some(new_index);
                SessionEvent::Started(new_index)
            }
        }
    }

    /// Explicitly close the active session (used when the poll reports no game).
    pub fn end_active(&mut self, now: impl FnOnce() -> String) -> SessionEvent {
        self.missed_observations = 0;
        match self.active_index {
            Some(index) => {
                self.close_at(index, &now());
                self.active_index = None;
                SessionEvent::Ended
            }
            None => SessionEvent::Idle,
        }
    }

    /// Attach enrichment data to entry `index`. Empty friend lists are stored
    /// as `None` so the JSON stays minimal.
    /// Returns `true` if any fields were changed.
    pub fn apply_meta(
        &mut self,
        index: usize,
        map: Option<&str>,
        server: Option<&str>,
        friends_playing: &[String],
        friends_in_lobby: &[String],
    ) -> anyhow::Result<bool> {
        let entry = self
            .log
            .games
            .get_mut(index)
            .ok_or_else(|| anyhow::anyhow!("entry index {} out of bounds", index))?;

        let mut changed = false;

        if entry.map.as_deref() != map {
            entry.map = map.map(str::to_string);
            changed = true;
        }

        if entry.server.as_deref() != server {
            entry.server = server.map(str::to_string);
            changed = true;
        }

        let friends_playing_opt = (!friends_playing.is_empty()).then_some(friends_playing);
        if entry.friends_playing.as_deref() != friends_playing_opt {
            entry.friends_playing = friends_playing_opt.map(|f| f.to_vec());
            changed = true;
        }

        let friends_in_lobby_opt = (!friends_in_lobby.is_empty()).then_some(friends_in_lobby);
        if entry.friends_in_lobby.as_deref() != friends_in_lobby_opt {
            entry.friends_in_lobby = friends_in_lobby_opt.map(|f| f.to_vec());
            changed = true;
        }

        Ok(changed)
    }

    fn close_at(&mut self, index: usize, now: &str) {
        let entry = &mut self.log.games[index];
        if entry.end.is_none() {
            entry.end = Some(now.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(name: &str) -> CurrentGame {
        CurrentGame {
            app_id: 0,
            name: name.to_string(),
            gamedir: None,
            lobby_steam_id: None,
            gameserver_ip: None,
        }
    }

    #[test]
    fn empty_tracker_starts_idle() {
        let t = Tracker::new(GameLog::default());
        assert!(t.active_index().is_none());
    }

    #[test]
    fn start() {
        let mut t = Tracker::new(GameLog::default());
        assert_eq!(
            t.update(&game("Dota 2"), || "T0".to_string()),
            SessionEvent::Started(0)
        );
        assert_eq!(t.active_index(), Some(0));
        let entries = &t.log().games;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Dota 2");
        assert_eq!(entries[0].start, "T0");
        assert!(entries[0].end.is_none());
    }

    #[test]
    fn running() {
        let mut t = Tracker::new(GameLog::default());
        t.update(&game("Dota 2"), || "T0".to_string());
        assert_eq!(
            t.update(&game("Dota 2"), || "T1".to_string()),
            SessionEvent::Running(0)
        );
        assert_eq!(t.active_index(), Some(0));
        assert_eq!(t.log().games.len(), 1);
        assert!(t.log().games[0].end.is_none());
    }

    #[test]
    fn switch() {
        let mut t = Tracker::new(GameLog::default());
        t.update(&game("Dota 2"), || "T0".to_string());
        assert_eq!(
            t.update(&game("CS2"), || "T1".to_string()),
            SessionEvent::Started(1)
        );
        assert_eq!(t.active_index(), Some(1));
        let entries = &t.log().games;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].end.as_deref(), Some("T1"));
        assert_eq!(entries[1].name, "CS2");
        assert_eq!(entries[1].start, "T1");
        assert!(entries[1].end.is_none());
    }

    #[test]
    fn end() {
        let mut t = Tracker::new(GameLog::default());
        t.update(&game("Dota 2"), || "T0".to_string());
        assert_eq!(t.end_active(|| "T2".to_string()), SessionEvent::Ended);
        assert_eq!(t.active_index(), None);
        assert_eq!(t.log().games[0].end.as_deref(), Some("T2"));
        assert_eq!(t.end_active(|| "T3".to_string()), SessionEvent::Idle);
        assert_eq!(t.log().games[0].end.as_deref(), Some("T2"));
    }

    #[test]
    fn resume() {
        let mut log = GameLog::default();
        log.games
            .push(LogEntry::new("Dota 2".to_string(), "T0".to_string()));
        let mut t = Tracker::new(log);
        assert_eq!(t.active_index(), Some(0));
        assert_eq!(
            t.update(&game("Dota 2"), || "T1".to_string()),
            SessionEvent::Running(0)
        );
        assert_eq!(t.log().games.len(), 1);
        assert_eq!(t.end_active(|| "T2".to_string()), SessionEvent::Ended);
        assert_eq!(t.log().games[0].end.as_deref(), Some("T2"));
        assert_eq!(t.active_index(), None);
    }

    #[test]
    fn observe_tolerates_brief_missing_game_reports() {
        let mut t = Tracker::with_missed_tolerance(GameLog::default(), 2);
        assert_eq!(
            t.observe(Some(&game("Dota 2")), || "T0".to_string()),
            SessionEvent::Started(0)
        );
        assert_eq!(
            t.observe(None, || "T1".to_string()),
            SessionEvent::Running(0)
        );
        assert_eq!(
            t.observe(None, || "T2".to_string()),
            SessionEvent::Running(0)
        );
        assert_eq!(t.active_index(), Some(0));
        assert!(t.log().games[0].end.is_none());

        assert_eq!(t.observe(None, || "T3".to_string()), SessionEvent::Ended);
        assert_eq!(t.active_index(), None);
        assert_eq!(t.log().games[0].end.as_deref(), Some("T3"));
    }

    #[test]
    fn observe_resets_miss_count_when_game_reappears() {
        let mut t = Tracker::with_missed_tolerance(GameLog::default(), 1);
        assert_eq!(
            t.observe(Some(&game("Dota 2")), || "T0".to_string()),
            SessionEvent::Started(0)
        );
        assert_eq!(
            t.observe(None, || "T1".to_string()),
            SessionEvent::Running(0)
        );
        assert_eq!(
            t.observe(Some(&game("Dota 2")), || "T2".to_string()),
            SessionEvent::Running(0)
        );
        assert_eq!(
            t.observe(None, || "T3".to_string()),
            SessionEvent::Running(0)
        );
        assert_eq!(t.active_index(), Some(0));
    }

    #[test]
    fn observe_evaluates_time_only_on_transitions_and_resets_misses() {
        let mut t = Tracker::with_missed_tolerance(GameLog::default(), 1);
        assert_eq!(
            t.observe(None, || panic!("idle timestamp")),
            SessionEvent::Idle
        );
        assert_eq!(
            t.observe(Some(&game("Dota 2")), || "T0".into()),
            SessionEvent::Started(0)
        );
        assert_eq!(
            t.observe(None, || panic!("tolerated timestamp")),
            SessionEvent::Running(0)
        );
        assert_eq!(
            t.update(&game("Dota 2"), || panic!("running timestamp")),
            SessionEvent::Running(0)
        );
        assert_eq!(
            t.observe(None, || panic!("reset timestamp")),
            SessionEvent::Running(0)
        );
        assert_eq!(
            t.observe(Some(&game("CS2")), || "T1".into()),
            SessionEvent::Started(1)
        );
        assert_eq!(t.log().games[0].end.as_deref(), Some("T1"));
        assert_eq!(t.log().games[1].start, "T1");
        assert_eq!(
            t.observe(None, || panic!("switch reset timestamp")),
            SessionEvent::Running(1)
        );
        assert_eq!(t.observe(None, || "T2".into()), SessionEvent::Ended);
        assert_eq!(t.log().games[1].end.as_deref(), Some("T2"));
        assert_eq!(
            t.end_active(|| panic!("idle timestamp")),
            SessionEvent::Idle
        );
    }

    #[test]
    fn new_tracker_does_not_tolerate_missing_observations() {
        let mut t = Tracker::new(GameLog::default());
        t.observe(Some(&game("Dota 2")), || "T0".into());
        assert_eq!(t.observe(None, || "T1".into()), SessionEvent::Ended);
    }

    #[test]
    fn closed_last_entry_not_resumed() {
        let mut log = GameLog::default();
        log.games
            .push(LogEntry::new("Dota 2".to_string(), "T0".to_string()));
        log.games[0].end = Some("T1".to_string());
        let t = Tracker::new(log);
        assert_eq!(t.active_index(), None);
    }

    #[test]
    fn apply_meta() {
        let mut t = Tracker::new(GameLog::default());
        t.update(&game("Dota 2"), || "T0".to_string());
        t.apply_meta(
            0,
            Some("dota"),
            Some("1.2.3.4:27015"),
            &["alice".to_string(), "bob".to_string()],
            &[],
        )
        .unwrap();
        let e = &t.log().games[0];
        assert_eq!(e.map.as_deref(), Some("dota"));
        assert_eq!(e.server.as_deref(), Some("1.2.3.4:27015"));
        assert_eq!(
            e.friends_playing.as_deref(),
            Some(&["alice".to_string(), "bob".to_string()][..])
        );
        assert!(e.friends_in_lobby.is_none());

        t.apply_meta(0, None, None, &[], &["carol".to_string()])
            .unwrap();
        let e = &t.log().games[0];
        assert!(e.map.is_none());
        assert!(e.server.is_none());
        assert!(e.friends_playing.is_none());
        assert_eq!(
            e.friends_in_lobby.as_deref(),
            Some(&["carol".to_string()][..])
        );
    }

    #[test]
    fn apply_meta_out_of_bounds() {
        let mut t = Tracker::new(GameLog::default());
        let err = t.apply_meta(3, None, None, &[], &[]).unwrap_err();
        assert!(err.to_string().contains("out of bounds"));
    }

    #[test]
    fn into_log_returns_underlying() {
        let mut t = Tracker::new(GameLog::default());
        t.update(&game("Dota 2"), || "T0".to_string());
        t.end_active(|| "T1".to_string());
        let log = t.into_log();
        assert_eq!(log.games.len(), 1);
        assert_eq!(log.games[0].name, "Dota 2");
        assert_eq!(log.games[0].start, "T0");
        assert_eq!(log.games[0].end.as_deref(), Some("T1"));
    }
}
