use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use steamlogger::config;
use steamlogger::enrich;
use steamlogger::model;
use steamlogger::steam::api;
use steamlogger::storage;
use steamlogger::tracker;

const TRACKED_STEAM_ID: u64 = 76561198000000000;
const TF2_FRIEND_STEAM_ID: u64 = 76561198000000001;
const CS2_FRIEND_STEAM_ID: u64 = 76561198000000002;
const LOBBY_STEAM_ID: &str = "109775241716070123";
const SERVER_ADDR: &str = "85.190.8.1:27015";

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn unique_path(tag: &str, ext: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "steamlogger_it_{}_{}_{}.{}",
        std::process::id(),
        n,
        tag,
        ext
    ))
}

fn game(name: &str) -> api::CurrentGame {
    api::CurrentGame {
        app_id: 0,
        name: name.to_string(),
        gamedir: None,
        lobby_steam_id: None,
        gameserver_ip: None,
    }
}

#[test]
fn config_load_roundtrip() {
    let path = unique_path("config", "toml");
    std::fs::write(
        &path,
        "api_key = \"key123\"\nsteam_id = 76561198000000000\n",
    )
    .expect("write config");
    let cfg = config::load(&path).expect("load valid config");
    assert_eq!(cfg.api_key, "key123");
    assert_eq!(cfg.steam_id, TRACKED_STEAM_ID);
    assert_eq!(cfg.poll_interval_secs, 30);
    assert_eq!(cfg.output_file, PathBuf::from("steamlog.json"));
    assert!(cfg.enrich);
    std::fs::remove_file(path).ok();
}

#[test]
fn config_missing_required_key_errors() {
    let path = unique_path("config_missing", "toml");
    std::fs::write(&path, "steam_id = 76561198000000000\n").expect("write config");
    let err = config::load(&path).unwrap_err().to_string();
    assert!(err.contains("api_key"), "unexpected error: {err}");
    std::fs::remove_file(path).ok();
}

#[test]
fn parse_player_summaries_fixture() {
    let json = include_str!("fixtures/player_summaries.json");
    let summaries = api::parse_player_summaries(json).expect("parse fixture");
    assert_eq!(summaries.len(), 3);

    let me = &summaries[0];
    assert_eq!(me.steam_id, TRACKED_STEAM_ID);
    assert_eq!(me.persona_name, "rocketjumper42");
    assert_eq!(me.game_id, Some(440));
    assert_eq!(me.game_extra_info.as_deref(), Some("Team Fortress 2"));
    assert_eq!(me.lobby_steam_id.as_deref(), Some(LOBBY_STEAM_ID));

    let tf2_friend = &summaries[1];
    assert_eq!(tf2_friend.steam_id, TF2_FRIEND_STEAM_ID);
    assert_eq!(tf2_friend.game_id, Some(440));
    assert_eq!(tf2_friend.lobby_steam_id.as_deref(), Some(LOBBY_STEAM_ID));

    let cs2_friend = &summaries[2];
    assert_eq!(cs2_friend.steam_id, CS2_FRIEND_STEAM_ID);
    assert_eq!(cs2_friend.game_id, Some(730));
    assert!(cs2_friend.lobby_steam_id.is_none());
}

#[test]
fn summary_to_current_game_fixture() {
    let json = include_str!("fixtures/player_summaries.json");
    let summaries = api::parse_player_summaries(json).expect("parse fixture");
    let game = api::summary_to_current_game(&summaries[0]).expect("tracked user in game");
    assert_eq!(
        game,
        api::CurrentGame {
            app_id: 440,
            name: "Team Fortress 2".to_string(),
            gamedir: Some("tf".to_string()),
            lobby_steam_id: Some(LOBBY_STEAM_ID.to_string()),
            gameserver_ip: Some(SERVER_ADDR.to_string()),
        }
    );
}

#[test]
fn parse_friends_fixture() {
    let json = include_str!("fixtures/friends.json");
    let friends = api::parse_friends(json).expect("parse fixture");
    assert_eq!(
        friends,
        vec![
            api::Friend {
                steam_id: TF2_FRIEND_STEAM_ID,
                persona_name: String::new(),
            },
            api::Friend {
                steam_id: CS2_FRIEND_STEAM_ID,
                persona_name: String::new(),
            },
        ]
    );
}

#[test]
fn enrich_same_lobby_from_fixture() {
    let json = include_str!("fixtures/player_summaries.json");
    let summaries = api::parse_player_summaries(json).expect("parse fixture");
    let game = api::summary_to_current_game(&summaries[0]).expect("tracked user in game");
    let friends: Vec<api::PlayerSummary> = summaries
        .into_iter()
        .filter(|s| s.steam_id != TRACKED_STEAM_ID)
        .collect();

    assert_eq!(
        enrich::same_game_friends(&friends, &game),
        vec!["medicmain99"]
    );
    assert_eq!(enrich::lobby_friends(&friends, &game), vec!["medicmain99"]);
}

#[test]
fn tracker_end_to_end() {
    let mut t = tracker::Tracker::new(model::GameLog::default());
    let game1 = game("Team Fortress 2");
    let game2 = game("Counter-Strike 2");

    assert_eq!(t.update(&game1, "T0"), tracker::SessionEvent::Started(0));
    assert_eq!(t.update(&game1, "T1"), tracker::SessionEvent::Running(0));
    t.apply_meta(
        0,
        Some("cp_dustbowl"),
        Some("Team Fortress 2 #42"),
        &["alice".to_string()],
        &["alice".to_string()],
    )
    .expect("apply meta");

    assert_eq!(t.update(&game2, "T2"), tracker::SessionEvent::Started(1));
    assert_eq!(t.log().games[0].end.as_deref(), Some("T2"));

    assert_eq!(t.end_active("T3"), tracker::SessionEvent::Ended);

    let log = t.into_log();
    assert_eq!(log.games.len(), 2);
    let e0 = &log.games[0];
    assert_eq!(e0.name, "Team Fortress 2");
    assert_eq!(e0.map.as_deref(), Some("cp_dustbowl"));
    assert_eq!(e0.server.as_deref(), Some("Team Fortress 2 #42"));
    assert_eq!(
        e0.friends_playing.as_deref(),
        Some(&["alice".to_string()][..])
    );
    assert_eq!(
        e0.friends_in_lobby.as_deref(),
        Some(&["alice".to_string()][..])
    );
    let e1 = &log.games[1];
    assert_eq!(e1.name, "Counter-Strike 2");
    assert_eq!(e1.end.as_deref(), Some("T3"));
}

#[test]
fn tracker_resume_closes_open_session() {
    let mut log = model::GameLog::default();
    log.games
        .push(model::LogEntry::new("Team Fortress 2".into(), "T0".into()));
    let mut t = tracker::Tracker::new(log);
    assert_eq!(t.active_index(), Some(0));

    let game1 = game("Team Fortress 2");
    assert_eq!(t.update(&game1, "T1"), tracker::SessionEvent::Running(0));
    assert_eq!(t.end_active("T2"), tracker::SessionEvent::Ended);

    let log = t.into_log();
    assert_eq!(log.games.len(), 1);
    assert_eq!(log.games[0].name, "Team Fortress 2");
    assert_eq!(log.games[0].start, "T0");
    assert_eq!(log.games[0].end.as_deref(), Some("T2"));
}

#[test]
fn log_json_shape() {
    let full = model::LogEntry {
        name: "Team Fortress 2".to_string(),
        start: "2026-08-10T14:32:15-03:00".to_string(),
        end: Some("2026-08-10T16:47:03-03:00".to_string()),
        map: Some("cp_dustbowl".to_string()),
        server: Some("vanilla.steamlogger.dev:27015".to_string()),
        friends_playing: Some(vec!["medicmain99".to_string()]),
        friends_in_lobby: Some(vec!["medicmain99".to_string()]),
    };
    let v: serde_json::Value =
        serde_json::from_str(&api::entry_to_json(&full)).expect("valid json");
    let obj = v.as_object().expect("object");
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "end",
            "friends_in_lobby",
            "friends_playing",
            "map",
            "name",
            "server",
            "start"
        ]
    );
    assert_eq!(obj["name"], "Team Fortress 2");
    assert_eq!(obj["map"], "cp_dustbowl");

    let minimal = model::LogEntry::new(
        "Half-Life".to_string(),
        "2026-08-01T10:00:00-03:00".to_string(),
    );
    let v: serde_json::Value =
        serde_json::from_str(&api::entry_to_json(&minimal)).expect("valid json");
    let obj = v.as_object().expect("object");
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["name", "start"]);
}

#[test]
fn storage_roundtrip_temp() {
    let path = unique_path("log", "json");
    let log = model::GameLog {
        games: vec![model::LogEntry {
            name: "Team Fortress 2".to_string(),
            start: "2026-08-10T14:32:15-03:00".to_string(),
            end: Some("2026-08-10T16:47:03-03:00".to_string()),
            map: Some("cp_dustbowl".to_string()),
            server: Some("vanilla.steamlogger.dev:27015".to_string()),
            friends_playing: Some(vec!["medicmain99".to_string()]),
            friends_in_lobby: Some(vec!["medicmain99".to_string()]),
        }],
    };
    storage::write_log(&path, &log).expect("write log");
    let back = storage::read_log(&path).expect("read log");
    assert_eq!(back, log);
    std::fs::remove_file(path).ok();
}

#[test]
fn steamlog_sample_fixture_parses() {
    let json = include_str!("fixtures/steamlog_sample.json");
    let log: model::GameLog = serde_json::from_str(json).expect("parse fixture");
    assert_eq!(log.games.len(), 2);

    let e0 = &log.games[0];
    assert_eq!(e0.name, "Team Fortress 2");
    assert!(e0.end.is_some());
    assert_eq!(e0.map.as_deref(), Some("cp_dustbowl"));
    assert_eq!(e0.server.as_deref(), Some("vanilla.steamlogger.dev:27015"));
    assert_eq!(
        e0.friends_playing.as_deref(),
        Some(&["medicmain99".to_string()][..])
    );
    assert_eq!(
        e0.friends_in_lobby.as_deref(),
        Some(&["medicmain99".to_string()][..])
    );

    let e1 = &log.games[1];
    assert_eq!(e1.name, "Counter-Strike 2");
    assert!(e1.end.is_none());
}
