//! Minimal Source RCON client (Valve RCON protocol over TCP).
//!
//! Used for live control of games that expose RCON (e.g. ARK: Survival Ascended).
//! Palworld uses its REST API instead, so nothing consumes this yet — it gets wired
//! into the live-control layer when the ARK adapter lands. The wire protocol is
//! fixed, so this is safe to build ahead of that.
//!
//! Packet layout (all integers little-endian):
//!   [i32 length][i32 id][i32 type][body bytes + \0][\0]
//! where `length` counts everything after itself (id + type + body + two nulls).

#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const SERVERDATA_AUTH: i32 = 3;
const SERVERDATA_AUTH_RESPONSE: i32 = 2;
const SERVERDATA_EXECCOMMAND: i32 = 2;
const SERVERDATA_RESPONSE_VALUE: i32 = 0;
/// Reasonable upper bound on a single packet's declared length.
const MAX_LEN: i32 = 8192;

pub struct RconClient {
    stream: TcpStream,
    next_id: i32,
    timeout: Duration,
}

/// Encode one packet: `[len][id][type][body\0][\0]`.
fn encode(id: i32, ptype: i32, body: &str) -> Vec<u8> {
    let body = body.as_bytes();
    let len = (4 + 4 + body.len() + 2) as i32; // id + type + body + two nulls
    let mut buf = Vec::with_capacity(len as usize + 4);
    buf.extend_from_slice(&len.to_le_bytes());
    buf.extend_from_slice(&id.to_le_bytes());
    buf.extend_from_slice(&ptype.to_le_bytes());
    buf.extend_from_slice(body);
    buf.push(0);
    buf.push(0);
    buf
}

/// Decode a packet payload (the bytes *after* the length prefix):
/// `[id][type][body\0][\0]` → `(id, type, body)`.
fn decode(payload: &[u8]) -> Result<(i32, i32, String), String> {
    if payload.len() < 10 {
        return Err("RCON packet too short".into());
    }
    let id = i32::from_le_bytes(payload[0..4].try_into().unwrap());
    let ptype = i32::from_le_bytes(payload[4..8].try_into().unwrap());
    let body = String::from_utf8_lossy(&payload[8..payload.len() - 2]).to_string();
    Ok((id, ptype, body))
}

impl RconClient {
    /// Connect and authenticate. Returns an error on a bad password or timeout.
    pub fn connect(host: &str, port: u16, password: &str, timeout: Duration) -> Result<Self, String> {
        let stream = TcpStream::connect((host, port))
            .map_err(|e| format!("RCON connect to {host}:{port} failed: {e}"))?;
        stream.set_read_timeout(Some(timeout)).ok();
        stream.set_write_timeout(Some(timeout)).ok();
        let mut client = RconClient { stream, next_id: 1, timeout };
        client.auth(password)?;
        Ok(client)
    }

    fn auth(&mut self, password: &str) -> Result<(), String> {
        let id = self.write_packet(SERVERDATA_AUTH, password)?;
        // Some servers send an empty RESPONSE_VALUE before the auth result; loop
        // until we get the AUTH_RESPONSE. id == -1 means auth failed.
        loop {
            let (rid, rtype, _) = self.read_packet()?;
            if rtype == SERVERDATA_AUTH_RESPONSE {
                if rid == -1 {
                    return Err("RCON authentication failed (wrong admin password).".into());
                }
                if rid == id {
                    return Ok(());
                }
            }
        }
    }

    /// Run a command and return the server's response text.
    ///
    /// We read the first response packet with the normal timeout, then drain any
    /// additional packets (large responses can span several) with a short timeout,
    /// stopping when no more arrive. This avoids the empty-command "sentinel" trick,
    /// which ARK: SA doesn't answer (it ignores empty commands, causing a hang).
    pub fn exec(&mut self, command: &str) -> Result<String, String> {
        self.write_packet(SERVERDATA_EXECCOMMAND, command)?;
        let (_id, _ty, first) = self.read_packet()?;
        let mut out = first;

        // Drain follow-up packets quickly; a timeout just means the response ended.
        self.stream
            .set_read_timeout(Some(Duration::from_millis(300)))
            .ok();
        while let Ok((_i, _t, body)) = self.read_packet() {
            out.push_str(&body);
        }
        self.stream.set_read_timeout(Some(self.timeout)).ok();
        Ok(out)
    }

    fn write_packet(&mut self, ptype: i32, body: &str) -> Result<i32, String> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.stream
            .write_all(&encode(id, ptype, body))
            .map_err(|e| format!("RCON write failed: {e}"))?;
        Ok(id)
    }

    fn read_packet(&mut self) -> Result<(i32, i32, String), String> {
        let mut len_buf = [0u8; 4];
        self.read_exact(&mut len_buf)?;
        let len = i32::from_le_bytes(len_buf);
        if !(10..=MAX_LEN).contains(&len) {
            return Err(format!("RCON packet length out of range: {len}"));
        }
        let mut payload = vec![0u8; len as usize];
        self.read_exact(&mut payload)?;
        decode(&payload)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), String> {
        self.stream
            .read_exact(buf)
            .map_err(|e| format!("RCON read failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_round_trip() {
        let pkt = encode(7, SERVERDATA_EXECCOMMAND, "ListPlayers");
        // The length prefix counts everything after itself.
        let len = i32::from_le_bytes(pkt[0..4].try_into().unwrap());
        assert_eq!(len as usize, pkt.len() - 4);
        let (id, ptype, body) = decode(&pkt[4..]).unwrap();
        assert_eq!(id, 7);
        assert_eq!(ptype, SERVERDATA_EXECCOMMAND);
        assert_eq!(body, "ListPlayers");
    }

    #[test]
    fn empty_body_has_minimum_length() {
        let pkt = encode(1, SERVERDATA_RESPONSE_VALUE, "");
        let len = i32::from_le_bytes(pkt[0..4].try_into().unwrap());
        assert_eq!(len, 10); // 4 (id) + 4 (type) + 0 (body) + 2 (nulls)
        let (id, ptype, body) = decode(&pkt[4..]).unwrap();
        assert_eq!(id, 1);
        assert_eq!(ptype, SERVERDATA_RESPONSE_VALUE);
        assert_eq!(body, "");
    }

    #[test]
    fn rejects_truncated_payload() {
        assert!(decode(&[0u8; 6]).is_err());
    }
}
