use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::Duration;

use steamlogger::a2s::A2sClient;
use steamlogger::config;
use steamlogger::enrich;
use steamlogger::steam::client::SteamClient;
use steamlogger::storage;
use steamlogger::tracker::Tracker;

fn main() -> ExitCode {
    let config_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("steamlogger.toml"));

    let config = match config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "failed to load config from {}: {e:#}",
                config_path.display()
            );
            eprintln!("copy steamlogger.example.toml to steamlogger.toml and fill it in");
            return ExitCode::FAILURE;
        }
    };

    run(&config)
}

fn run(config: &config::Config) -> ExitCode {
    let client = SteamClient::new(config.api_key.clone());
    let mut a2s = A2sClient::new();
    let mut tracker = Tracker::new(storage::read_log(&config.output_file).unwrap_or_default());
    let interval = Duration::from_secs(config.poll_interval_secs.max(5));

    let (tx, rx) = mpsc::channel::<()>();
    if let Err(e) = ctrlc::set_handler(move || {
        let _ = tx.send(());
    }) {
        eprintln!("failed to install Ctrl+C handler: {e}");
        return ExitCode::FAILURE;
    }

    println!(
        "SteamLogger running. Log file: {}  (Ctrl+C to stop)",
        config.output_file.display()
    );

    loop {
        if rx.try_recv().is_ok() {
            break;
        }

        match client.current_game(config.steam_id) {
            Ok(Some(game)) => {
                let _event = tracker.update(&game, &now());
                if config.enrich {
                    let info = enrich::analyze(&client, &mut a2s, config.steam_id, &game);
                    if let Some(idx) = tracker.active_index() {
                        if let Err(e) = tracker.apply_meta(
                            idx,
                            info.map.as_deref(),
                            info.server.as_deref(),
                            &info.friends_playing,
                            &info.friends_in_lobby,
                        ) {
                            eprintln!("enrichment failed: {e}");
                        }
                    }
                }
            }
            Ok(None) => {
                let _event = tracker.end_active(&now());
            }
            Err(e) => eprintln!("poll error: {e}"),
        }

        if let Err(e) = storage::write_log(&config.output_file, tracker.log()) {
            eprintln!("failed to save log: {e}");
        }

        let _ = rx.recv_timeout(interval);
    }

    if let Err(e) = storage::write_log(&config.output_file, &tracker.into_log()) {
        eprintln!("failed to save log: {e}");
    }
    println!("stopped");
    ExitCode::SUCCESS
}

fn now() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}
