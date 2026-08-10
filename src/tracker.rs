use crate::model::GameLog;
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
}

impl Tracker {
    pub fn new(log: GameLog) -> Self {
        // TODO(subagent tracker): if last entry has end == None, set active_index.
        Self {
            log,
            active_index: None,
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

    /// Feed the currently-observed game. Handles start/continue/switch:
    /// - no active + game -> push new entry, `Started`
    /// - active + same game -> `Running`
    /// - active + different game -> close old, push new, `Started`
    /// - active + none -> close old, `Ended`
    pub fn update(&mut self, game: &CurrentGame, now: &str) -> SessionEvent {
        // TODO(subagent tracker): implement the state machine.
        let _ = (game, now);
        SessionEvent::Idle
    }

    /// Explicitly close the active session (used when the poll reports no game).
    pub fn end_active(&mut self, now: &str) -> SessionEvent {
        // TODO(subagent tracker): set end on active entry, clear active_index.
        let _ = now;
        SessionEvent::Idle
    }

    /// Attach enrichment data to entry `index`. Empty friend lists are stored
    /// as `None` so the JSON stays minimal.
    pub fn apply_meta(
        &mut self,
        index: usize,
        map: Option<&str>,
        server: Option<&str>,
        friends_playing: &[String],
        friends_in_lobby: &[String],
    ) -> anyhow::Result<()> {
        // TODO(subagent tracker): set optional fields on log.games[index].
        let _ = (index, map, server, friends_playing, friends_in_lobby);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tracker_starts_idle() {
        let t = Tracker::new(GameLog::default());
        assert!(t.active_index().is_none());
    }
}
