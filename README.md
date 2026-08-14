# SteamLogger

Rust app that logs every Steam game session to a JSON file, with optional
enrichment: **friends playing the same game**, **friends in your same lobby**,
and the **server name + current map** you are playing on.

Output format (matches `steamlog.json`, enrichment fields are optional):

```json
{
  "games": [
    {
      "name": "Team Fortress 2",
      "start": "2026-08-10T14:32:15-03:00",
      "end": "2026-08-10T16:47:03-03:00",
      "map": "cp_dustbowl",
      "server": "Team Fortress 2 #42",
      "friends_playing": ["medicmain99"],
      "friends_in_lobby": ["medicmain99"]
    }
  ]
}
```

## How it works

SteamLogger polls the [Steam Web API](https://steamcommunity.com/dev) every few
seconds to watch the configured player's currently-running game:

- `ISteamUser/GetPlayerSummaries` -> detect game start/stop, plus the lobby
  SteamID and game server IP while playing.
- `ISteamUser/GetFriendList` + `GetPlayerSummaries` (batch) -> which friends are
  playing the same game, and which share your exact lobby.
- `gameserver_ip` -> a direct [A2S_INFO](https://developer.valvesoftware.com/wiki/Server_queries)
  query to the game server returns its display name and current map.

Every detection source degrades gracefully: if the API is down, enrichment is
skipped and the base `name`/`start`/`end` session log is still written. Brief
missing current-game reports are ignored until `missed_poll_tolerance`
consecutive empty polls occur, which avoids fragmented or missing-looking logs
when Steam briefly omits status data. The log
is saved atomically (temp file + rename) on every poll, so a crash never
corrupts it. If the app is restarted mid-game, the open session is resumed from
`end: null` on the last entry.

Enrichment limits/notes:

- "Same lobby" is detected via matching `lobbysteamid`, which the Steam API
  only reports for games running through Steam lobbies (P2P matchmaking). For
  other games the field is simply omitted.
- Server name/map come from a public A2S_INFO query to the IP Steam reports;
  games without a public game server (or behind a firewall) simply omit it.

## Requirements

- Rust 1.75+ (`cargo`).
- A free [Steam Web API key](https://steamcommunity.com/dev/apikey).

## Setup

```bash
cp steamlogger.example.toml steamlogger.toml
```

Edit `steamlogger.toml`:

```toml
api_key = "YOUR_STEAM_API_KEY"        # required (or env STEAMLOGGER_API_KEY)
steam_id = 76561198000000000          # required (or env STEAMLOGGER_STEAM_ID)
poll_interval_secs = 30               # default 30
missed_poll_tolerance = 2             # default 2
output_file = "steamlog.json"         # default steamlog.json
enrich = true                          # default true
```

## Run

```bash
cargo run --release            # or: cargo run --release -- path/to/config.toml
```

The app prints the log path, polls until you press `Ctrl+C`, and writes the log
on every poll and on exit.

## Tests

```bash
cargo test
```

Unit + integration tests cover config loading (incl. env overrides), Steam API
response parsing, the A2S_INFO protocol (against a local UDP echo server), the
session state machine (start/switch/resume/end), enrichment matching, and
atomic JSON persistence. No test touches the network outside the loopback
A2S mock.

## Layout

```
src/
  main.rs      poll loop, wiring, Ctrl+C handling
  config.rs    TOML config + env overrides
  model.rs     LogEntry / GameLog JSON types
  tracker.rs   session state machine (start/switch/resume/end)
  enrich.rs    friends / lobby / map-server enrichment
  storage.rs   atomic JSON read + write
  a2s.rs       Source A2S_INFO server query (map + server name)
  steam/
    api.rs     Steam Web API URL builders + response parsing
    client.rs  blocking HTTP client (chunked summaries, error context)
tests/         integration tests + fixtures
```
