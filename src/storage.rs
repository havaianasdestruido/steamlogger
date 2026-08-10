use std::path::Path;

use crate::model::GameLog;

/// Read the JSON log. A missing or unreadable-vs-new file yields an empty log
/// (callers may treat malformed JSON as an error by using `read_log` and
/// propagating). Parent directories of `path` are created on write.
pub fn read_log(path: &Path) -> anyhow::Result<GameLog> {
    // TODO(subagent storage): if !path.exists() -> Ok(GameLog::default()); else
    //   read file, serde_json::from_str -> GameLog.
    let _ = path;
    Ok(GameLog::default())
}

/// Atomically write the log: write a temp file next to `path`, then rename it
/// over the target so a crash never leaves a half-written JSON file.
pub fn write_log(path: &Path, log: &GameLog) -> anyhow::Result<()> {
    // TODO(subagent storage): serde_json::to_string_pretty, temp file in same dir,
    //   fs::rename over target.
    let _ = (path, log);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_reads_empty() {
        let p = Path::new("definitely_missing_steamlog.json");
        let log = read_log(p).unwrap();
        assert!(log.games.is_empty());
    }
}
