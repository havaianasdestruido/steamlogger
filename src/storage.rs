use std::path::Path;

use anyhow::Context;

use crate::model::{GameLog, LogEntry};

/// Read the JSON log. A missing or unreadable-vs-new file yields an empty log
/// (callers may treat malformed JSON as an error by using `read_log` and
/// propagating). Parent directories of `path` are created on write.
pub fn read_log(path: &Path) -> anyhow::Result<GameLog> {
    if !path.exists() {
        return Ok(GameLog::default());
    }
    let text = std::fs::read_to_string(path)?;
    let trimmed = text.trim_start();
    if trimmed.starts_with('[') {
        let games: Vec<LogEntry> = serde_json::from_str(&text)
            .with_context(|| format!("malformed JSON log array in {}", path.display()))?;
        return Ok(GameLog { games });
    }
    serde_json::from_str(&text).with_context(|| format!("malformed JSON log in {}", path.display()))
}

/// Atomically write the log: write a temp file next to `path`, then rename it
/// over the target so a crash never leaves a half-written JSON file.
pub fn write_log(path: &Path, log: &GameLog) -> anyhow::Result<()> {
    let mut json = serde_json::to_string_pretty(log)?;
    json.push('\n');
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let tmp = temp_path(path);
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn temp_path(path: &Path) -> std::path::PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".tmp");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_path(tag: &str) -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "steamlogger_test_{}_{}_{}",
            std::process::id(),
            n,
            tag
        ))
    }

    fn full_entry() -> LogEntry {
        LogEntry {
            name: "Team Fortress 2".to_string(),
            start: "2026-08-10T14:32:15-03:00".to_string(),
            end: Some("2026-08-10T16:47:03-03:00".to_string()),
            map: Some("cp_dustbowl".to_string()),
            server: Some("vanilla.steamlogger.dev:27015".to_string()),
            friends_playing: Some(vec!["Alice".to_string(), "Bob".to_string()]),
            friends_in_lobby: Some(vec!["Alice".to_string()]),
        }
    }

    #[test]
    fn missing_file_reads_empty() {
        let p = test_path("missing");
        let log = read_log(&p).unwrap();
        assert!(log.games.is_empty());
    }

    #[test]
    fn round_trip_preserves_entry() {
        let p = test_path("roundtrip");
        let log = GameLog {
            games: vec![full_entry()],
        };
        write_log(&p, &log).unwrap();
        let read = read_log(&p).unwrap();
        assert_eq!(log, read);
        let entry = &read.games[0];
        assert_eq!(entry.name, "Team Fortress 2");
        assert_eq!(entry.start, "2026-08-10T14:32:15-03:00");
        assert_eq!(entry.end.as_deref(), Some("2026-08-10T16:47:03-03:00"));
        assert_eq!(entry.map.as_deref(), Some("cp_dustbowl"));
        assert_eq!(
            entry.server.as_deref(),
            Some("vanilla.steamlogger.dev:27015")
        );
        assert_eq!(
            entry.friends_playing.as_deref(),
            Some(&["Alice".to_string(), "Bob".to_string()][..])
        );
        assert_eq!(
            entry.friends_in_lobby.as_deref(),
            Some(&["Alice".to_string()][..])
        );
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn pretty_output_with_skipped_optionals() {
        let p = test_path("pretty");
        let log = GameLog {
            games: vec![LogEntry::new(
                "Half-Life".to_string(),
                "2026-08-01T10:00:00-03:00".to_string(),
            )],
        };
        write_log(&p, &log).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("\n  \"games\""));
        assert!(!text.contains("\"end\""));
        assert!(!text.contains("\"map\""));
        assert!(!text.contains("\"server\""));
        assert!(!text.contains("\"friends_playing\""));
        assert!(!text.contains("\"friends_in_lobby\""));
        assert!(text.ends_with('\n'));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn overwrite_replaces_old_log() {
        let p = test_path("overwrite");
        let first = GameLog {
            games: vec![LogEntry::new(
                "Portal".to_string(),
                "2026-08-02T09:00:00-03:00".to_string(),
            )],
        };
        let second = GameLog {
            games: vec![LogEntry::new(
                "Portal 2".to_string(),
                "2026-08-03T09:00:00-03:00".to_string(),
            )],
        };
        write_log(&p, &first).unwrap();
        write_log(&p, &second).unwrap();
        let read = read_log(&p).unwrap();
        assert_eq!(read, second);
        assert_eq!(read.games.len(), 1);
        assert_eq!(read.games[0].name, "Portal 2");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn no_stray_temp_file_after_write() {
        let p = test_path("atomic");
        let log = GameLog {
            games: vec![LogEntry::new(
                "Doom".to_string(),
                "2026-08-04T12:00:00-03:00".to_string(),
            )],
        };
        write_log(&p, &log).unwrap();
        let mut tmp_name = p.as_os_str().to_owned();
        tmp_name.push(".tmp");
        let tmp = p.with_file_name(tmp_name);
        assert!(!tmp.exists());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn malformed_json_is_error() {
        let p = test_path("corrupt");
        std::fs::write(&p, "not json").unwrap();
        assert!(read_log(&p).is_err());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn bare_array_is_normalized() {
        let p = test_path("array");
        std::fs::write(
            &p,
            r#"[{"name":"Quake","start":"2026-08-05T08:00:00-03:00"}]"#,
        )
        .unwrap();
        let read = read_log(&p).unwrap();
        assert_eq!(read.games.len(), 1);
        assert_eq!(read.games[0].name, "Quake");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn nested_parent_dir_created() {
        let p = test_path("nested").join("deep").join("log.json");
        let log = GameLog {
            games: vec![LogEntry::new(
                "Minecraft".to_string(),
                "2026-08-06T07:00:00-03:00".to_string(),
            )],
        };
        write_log(&p, &log).unwrap();
        let read = read_log(&p).unwrap();
        assert_eq!(read, log);
        let _ = std::fs::remove_dir_all(test_path("nested"));
    }
}
