use std::net::UdpSocket;
use std::time::Duration;

/// Basic server info returned by a Source engine A2S_INFO query.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerInfo {
    pub name: String,
    pub map: String,
}

/// Minimal Source (goldsrc too) game-server query client using the A2S_INFO
/// protocol over UDP. Used to resolve the `gameserverip` Steam reports for a
/// player into a human-readable server name + current map.
pub struct A2sClient {
    timeout: Duration,
}

impl A2sClient {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(3),
        }
    }

    /// Query the server at `addr` ("ip:port" or bare ip). Returns server name
    /// and current map, or an error on timeout/unreachable/parse failure.
    pub fn query(&mut self, addr: &str) -> anyhow::Result<ServerInfo> {
        // TODO(subagent a2s):
        //   - parse SocketAddr (default port 27015)
        //   - bind UdpSocket to 0.0.0.0:0, set read timeout
        //   - send A2S_INFO request: 0xFFFFFFFF 0x54 0x53 0x4F 0x55 0x52 0x43 0x45 0x00
        //   - receive; if challenge byte 0x41, echo challenge int32 and retry once
        //   - parse 0x49 ('I') response: protocol(u8) name(map) map(map) folder(map)
        //     game(map) appid(u16 LE) players(u8) maxplayers(u8) bots(u8) ...
        //   - tolerate old 0x6D ('m') format too (no appid)
        let _ = (self.timeout, addr);
        anyhow::bail!("a2s not implemented yet")
    }
}

impl Default for A2sClient {
    fn default() -> Self {
        Self::new()
    }
}
