## 2024-03-24 - Streaming JSON directly to disk to avoid repeated allocations
**Learning:** In a long-running polling service like SteamLogger, continuously allocating the entire state into memory as a `String` (e.g. using `serde_json::to_string_pretty`) inside the main loop creates an unnecessary and growing memory bottleneck over time as the log expands.
**Action:** Use `std::io::BufWriter` and streaming serialization (like `serde_json::to_writer_pretty`) for periodic disk writes to keep memory footprint bounded and avoid large repeating string allocations on every tick.
