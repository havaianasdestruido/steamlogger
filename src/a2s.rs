use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

const MAGIC: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];
const TYPE_STD: u8 = 0x49;
const TYPE_GOLDSRC: u8 = 0x6D;
const TYPE_CHALLENGE: u8 = 0x41;
const DEFAULT_PORT: u16 = 27015;

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
        let target = resolve_addr(addr)?;
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(self.timeout))?;
        socket.connect(target)?;

        socket.send(&request_payload(None))?;
        let mut buf = [0u8; 4096];
        let mut packet = receive_packet(&socket, &mut buf)?;

        if is_challenge(&packet) {
            let challenge = read_challenge(&packet)?;
            socket.send(&request_payload(Some(challenge)))?;
            packet = receive_packet(&socket, &mut buf)?;
            if is_challenge(&packet) {
                anyhow::bail!("server replied to A2S_INFO challenge with another challenge");
            }
        }

        parse_response(&packet)
    }
}

impl Default for A2sClient {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_addr(addr: &str) -> anyhow::Result<SocketAddr> {
    if let Ok(sa) = addr.parse::<SocketAddr>() {
        return Ok(sa);
    }
    let with_port = format!("{addr}:{DEFAULT_PORT}");
    let mut it = with_port.to_socket_addrs()?;
    it.next()
        .ok_or_else(|| anyhow::anyhow!("could not resolve address {addr}"))
}

fn request_payload(challenge: Option<i32>) -> Vec<u8> {
    let mut v = Vec::with_capacity(16);
    v.extend_from_slice(&MAGIC);
    match challenge {
        None => v.extend_from_slice(b"TSOURCE\0"),
        Some(c) => {
            v.push(TYPE_CHALLENGE);
            v.extend_from_slice(&c.to_le_bytes());
        }
    }
    v
}

fn receive_packet(socket: &UdpSocket, buf: &mut [u8; 4096]) -> anyhow::Result<Vec<u8>> {
    let n = socket
        .recv(buf)
        .map_err(|e| anyhow::anyhow!("A2S_INFO query failed: {e}"))?;
    Ok(buf[..n].to_vec())
}

fn is_challenge(packet: &[u8]) -> bool {
    packet.len() >= 5 && packet[..4] == MAGIC && packet[4] == TYPE_CHALLENGE
}

fn read_challenge(packet: &[u8]) -> anyhow::Result<i32> {
    if packet.len() < 9 {
        anyhow::bail!("A2S challenge packet too short");
    }
    Ok(i32::from_le_bytes(packet[5..9].try_into().unwrap()))
}

fn parse_response(packet: &[u8]) -> anyhow::Result<ServerInfo> {
    if packet.len() < 5 || packet[..4] != MAGIC {
        anyhow::bail!("bad A2S packet (wrong magic or too short)");
    }
    let mut idx = 4usize;
    let ty = packet[idx];
    idx += 1;
    match ty {
        TYPE_STD | TYPE_GOLDSRC => {
            idx += 1; // protocol
            let name = parse_cstr(packet, &mut idx);
            let map = parse_cstr(packet, &mut idx);
            Ok(ServerInfo { name, map })
        }
        TYPE_CHALLENGE => anyhow::bail!("unexpected A2S challenge response"),
        other => anyhow::bail!("unknown A2S packet type 0x{other:02X}"),
    }
}

fn parse_cstr(buf: &[u8], idx: &mut usize) -> String {
    let start = *idx;
    let end = buf[start..]
        .iter()
        .position(|&b| b == 0)
        .map(|p| start + p)
        .unwrap_or(buf.len());
    *idx = end.saturating_add(1);
    String::from_utf8_lossy(&buf[start..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;
    use std::sync::mpsc;
    use std::thread;

    fn craft_std_packet() -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(&MAGIC);
        p.push(TYPE_STD);
        p.push(17); // protocol
        p.extend_from_slice(b"Test Server\0");
        p.extend_from_slice(b"de_dust2\0");
        p.extend_from_slice(b"cstrike\0");
        p.extend_from_slice(b"Counter-Strike\0");
        p.extend_from_slice(&240u16.to_le_bytes()); // appid
        p.push(10); // players
        p.push(16); // max players
        p.push(2); // bots
        p.push(b'd'); // server type
        p.push(b'l'); // environment
        p.push(0); // visibility
        p.push(1); // VAC
        p.extend_from_slice(b"1.1.2.7\0");
        p.push(0x01); // EDF
        p.extend_from_slice(&2021u16.to_le_bytes()); // port
        p.extend_from_slice(&2021u16.to_le_bytes()); // steamid port
        p.extend_from_slice(b"127.0.0.1\0"); // source tv addr
        p.extend_from_slice(&0u16.to_le_bytes());
        p.extend_from_slice(b"gamedir\0"); // source tv name
        p.extend_from_slice(&"abcdef".as_bytes().to_vec()[..]); // keywords
        p.push(0);
        p.extend_from_slice(&0u64.to_le_bytes()); // game id
        p
    }

    fn craft_goldsrc_packet() -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(&MAGIC);
        p.push(TYPE_GOLDSRC);
        p.push(48); // protocol
        p.extend_from_slice(b"Old GoldSrc\0");
        p.extend_from_slice(b"crossfire\0");
        p.extend_from_slice(b"valve\0");
        p.extend_from_slice(b"Half-Life\0");
        p.push(5); // players
        p.push(12); // max players
        p.push(48); // protocol2
        p.extend_from_slice(b"127.0.0.1:27015\0");
        p.extend_from_slice(b"Half-Life 1.1.1.0\0");
        p
    }

    #[test]
    fn parses_standard_response() {
        let info = parse_response(&craft_std_packet()).unwrap();
        assert_eq!(
            info,
            ServerInfo {
                name: "Test Server".into(),
                map: "de_dust2".into()
            }
        );
    }

    #[test]
    fn parses_goldsrc_response() {
        let info = parse_response(&craft_goldsrc_packet()).unwrap();
        assert_eq!(
            info,
            ServerInfo {
                name: "Old GoldSrc".into(),
                map: "crossfire".into()
            }
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_response(b"hello world").is_err());
        assert!(parse_response(b"\x00\x01\x02\x03Iabc").is_err());
        assert!(parse_response(&[]).is_err());
    }

    #[test]
    fn handles_utf8_names_lossy() {
        let mut p = Vec::new();
        p.extend_from_slice(&MAGIC);
        p.push(TYPE_STD);
        p.push(17);
        p.extend_from_slice(&[0xFF, 0xFE, b'x', 0]);
        p.extend_from_slice(b"map\0");
        let info = parse_response(&p).unwrap();
        assert_eq!(info.map, "map");
        assert!(!info.name.is_empty());
    }

    #[test]
    fn round_trip_with_echo_server() {
        let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let reply = craft_std_packet();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let (n, from) = listener.recv_from(&mut buf).unwrap();
            tx.send((buf[..n].to_vec(), from)).unwrap();
            listener.send_to(&reply, from).unwrap();
        });

        let mut client = A2sClient::new();
        let info = client.query(&format!("127.0.0.1:{port}")).unwrap();
        assert_eq!(info.name, "Test Server");
        assert_eq!(info.map, "de_dust2");

        let (sent, _) = rx.recv().unwrap();
        assert_eq!(sent, request_payload(None));
    }

    #[test]
    fn query_without_port_uses_default() {
        let listener = UdpSocket::bind("127.0.0.1:27015").unwrap();
        let reply = craft_std_packet();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let (n, from) = listener.recv_from(&mut buf).unwrap();
            tx.send((buf[..n].to_vec(), from)).unwrap();
            listener.send_to(&reply, from).unwrap();
        });

        let mut client = A2sClient::new();
        let info = client.query("127.0.0.1").unwrap();
        assert_eq!(info.map, "de_dust2");
        let (sent, _) = rx.recv().unwrap();
        assert_eq!(sent, request_payload(None));
    }

    #[test]
    fn challenge_round_trip() {
        let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let reply = craft_std_packet();
        let challenge: i32 = 0x12345678;
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let (n, from) = listener.recv_from(&mut buf).unwrap();
            assert_eq!(buf[..n], request_payload(None));
            let mut c = Vec::new();
            c.extend_from_slice(&MAGIC);
            c.push(TYPE_CHALLENGE);
            c.extend_from_slice(&challenge.to_le_bytes());
            listener.send_to(&c, from).unwrap();
            let (n, from) = listener.recv_from(&mut buf).unwrap();
            assert_eq!(buf[..n], request_payload(Some(challenge)));
            listener.send_to(&reply, from).unwrap();
        });

        let mut client = A2sClient::new();
        let info = client.query(&format!("127.0.0.1:{port}")).unwrap();
        assert_eq!(info.name, "Test Server");
    }

    #[test]
    fn challenge_repeat_gives_up() {
        let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let challenge: i32 = 42;
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let (n, from) = listener.recv_from(&mut buf).unwrap();
            assert_eq!(buf[..n], request_payload(None));
            let mut c = Vec::new();
            c.extend_from_slice(&MAGIC);
            c.push(TYPE_CHALLENGE);
            c.extend_from_slice(&challenge.to_le_bytes());
            listener.send_to(&c, from).unwrap();
            let (n, from) = listener.recv_from(&mut buf).unwrap();
            assert_eq!(buf[..n], request_payload(Some(challenge)));
            listener.send_to(&c, from).unwrap();
        });

        let mut client = A2sClient::new();
        assert!(client.query(&format!("127.0.0.1:{port}")).is_err());
    }

    #[test]
    fn query_unreachable_port_errors() {
        let unused = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = unused.local_addr().unwrap().port();
        drop(unused);

        let mut client = A2sClient::new();
        client.timeout = Duration::from_millis(300);
        let err = client.query(&format!("127.0.0.1:{port}"));
        assert!(err.is_err(), "expected error, got {err:?}");
    }
}
