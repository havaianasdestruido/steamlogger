## 2024-10-11 - Prevent unnecessary disk writes and allocations
**Learning:** The polling loop in `main.rs` was writing the entire `GameLog` to a file on disk every 5 seconds (default interval) even when nothing changed. Furthermore, `apply_meta` was allocating strings/vecs every 5 seconds even when the metadata hadn't changed.
**Action:** Always verify if a polling or recurring loop performs expensive I/O operations unconditionally. Implement a dirty flag to conditionally execute I/O operations like disk writes. Track state changes (like `apply_meta` doing an actual update rather than just setting to the same value) to accurately maintain the dirty flag.
## 2024-10-11 - Avoid unconditional allocations in comparisons
**Learning:** `apply_meta` was unconditionally calling `str::to_string` on `Option<&str>` and cloning vectors before checking if the new values actually differed from the stored values.
**Action:** When updating a struct with `Option<String>` fields based on `Option<&str>` inputs, use `.as_deref()` on the stored value to compare against the borrowed input before paying the allocation cost of `.to_string()`.

## 2024-05-18 - Memory-efficient JSON Serialization
**Learning:** For a JSON log file that grows indefinitely over time, allocating the entire serialized output in a single string (e.g., `serde_json::to_string_pretty`) causes an O(N) memory spike on every write.
**Action:** Always stream serialization directly to disk using `serde_json::to_writer` (or `to_writer_pretty`) with a `BufWriter` to maintain O(1) memory usage regardless of file size.
## 2024-05-19 - URL Generation Performance
**Learning:** `player_summaries_url` originally used `collect::<Vec<_>>().join(",")` to format a comma-separated list of IDs. This requires O(N) heap allocations for individual strings plus the Vec buffer, which is very inefficient.
**Action:** Use `std::fmt::Write` to manually push characters and format directly into a `String::with_capacity` buffer for constructing complex API URLs without intermediate allocations.
## 2024-05-20 - Memory-efficient JSON Deserialization
**Learning:** For a JSON log file that grows indefinitely over time, reading the entire file contents into a single string (e.g., `std::fs::read_to_string`) causes an O(N) memory spike upon startup.
**Action:** Always stream deserialization directly from disk using `serde_json::from_reader` with a `BufReader` to maintain O(1) memory usage regardless of file size, complementing the streaming write optimization.
## 2024-10-16 - Zero-copy JSON Deserialization
**Learning:** `FriendListResponse` in `src/steam/api.rs` used `String` fields for `steamid` and `relationship` during intermediate JSON parsing, leading to `2 * N` heap allocations per poll tick where `N` is the number of friends, only to immediately discard the `relationship` string and parse the `steamid` to an integer.
**Action:** When extracting non-string values or discarding strings from a JSON response, use `&'a str` with `#[serde(borrow)]` on the intermediate structs. This allows `serde_json` to borrow slices directly from the input JSON string without allocating intermediate heap buffers, turning parsing into a completely zero-copy operation.
## 2024-10-18 - Zero-copy JSON Deserialization for Player Summaries
**Learning:** The `PlayerSummariesResponse` in `src/steam/api.rs` used `String` fields for all parsed values, leading to multiple heap allocations per player per poll tick (for fields like `steamid`, `gameid`, `lobbysteamid`, `gameserverip`). Many of these strings are either parsed directly into integers or only used conditionally, making the allocations wasteful.
**Action:** When extracting values from a JSON response where strings can be borrowed from the source buffer without modification, use `std::borrow::Cow<'a, str>` with `#[serde(borrow)]`. This allows `serde_json` to borrow slices directly from the input JSON string without allocating intermediate heap buffers unless escape sequences require it, turning much of the parsing into a zero-copy operation.
## 2024-10-23 - Avoid intermediate structs and Vec collections for unused data
**Learning:** The friend list parsing created a `Vec<Friend>` where `persona_name` was just a dummy `String::new()`, which was later mapped into a `Vec<u64>`. Furthermore, chunking an array allocated a `Vec<&[u64]>` before iterating over it.
**Action:** Extract precisely the data needed (`Vec<u64>`) directly from JSON parsing to avoid intermediary allocations, and use iterator methods like `.chunks()` directly in loops instead of collecting slices into temporary vectors.
## 2024-10-24 - Lazy evaluation for recurring timestamp generation
**Learning:** The polling loop in `main.rs` was formatting the current time as an RFC3339 string unconditionally every 5 seconds via `now()`, even when the game state was `Running` or `Idle` (which represents 99% of the app's lifetime). The `Tracker` only actually uses this timestamp when a session starts or ends.
**Action:** Use closures (`impl FnOnce() -> String`) to lazily evaluate expensive or allocating operations (like date formatting) in tight loops or polling ticks, ensuring they are only computed when state transitions actually require them.
