//! Connectivity helper: public/local IP, the game port, a local "is the server
//! listening" check, and one-click UPnP port forwarding.
//!
//! Palworld's game port is **UDP** `PublicPort` (default 8211) — that's what
//! friends connect to and what must be port-forwarded.

use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket};
use std::time::Duration;

use serde::Serialize;
use tauri::AppHandle;

use crate::{config, settings};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    pub public_ip: String,
    pub local_ip: String,
    pub port: u16,
    /// Whether something (the server) is bound to the game port on this machine.
    pub port_listening: bool,
    /// The server's configured `PublicIP` (e.g. a Tailscale IP or domain), if set.
    pub configured_ip: String,
}

/// The server's game port from config (`PublicPort`), default 8211.
pub fn game_port(app: &AppHandle) -> u16 {
    settings::install_dir(app)
        .ok()
        .and_then(|dir| config::read(&dir).ok())
        .and_then(|fields| config::find(&fields, "PublicPort"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(8211)
}

fn public_ip() -> String {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(6))
        .build()
        .ok()
        .and_then(|c| c.get("https://api.ipify.org").send().ok())
        .and_then(|r| r.text().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unavailable".into())
}

/// Primary LAN IPv4 (found by opening a UDP socket toward a public address — no
/// packets are actually sent).
fn local_ipv4() -> Option<Ipv4Addr> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    match sock.local_addr().ok()?.ip() {
        IpAddr::V4(v4) => Some(v4),
        _ => None,
    }
}

/// If we can't bind the UDP game port, something (the server) is already using it.
fn port_listening(port: u16) -> bool {
    UdpSocket::bind(("0.0.0.0", port)).is_err()
}

pub fn info(app: &AppHandle) -> NetworkInfo {
    let fields = settings::install_dir(app).ok().and_then(|dir| config::read(&dir).ok());
    let port = fields
        .as_ref()
        .and_then(|f| config::find(f, "PublicPort"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(8211);
    let configured_ip = fields
        .as_ref()
        .and_then(|f| config::find(f, "PublicIP"))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_default();
    NetworkInfo {
        public_ip: public_ip(),
        local_ip: local_ipv4().map(|i| i.to_string()).unwrap_or_else(|| "unknown".into()),
        port,
        port_listening: port_listening(port),
        configured_ip,
    }
}

/// Try to auto-forward the UDP game port via UPnP. Returns a human message.
pub fn forward(app: &AppHandle) -> Result<String, String> {
    let port = game_port(app);
    let local = local_ipv4().ok_or("Could not determine this PC's local IP.")?;
    let gateway = igd::search_gateway(Default::default())
        .map_err(|e| format!("No UPnP-capable router found ({e}). Forward the port manually."))?;
    gateway
        .add_port(
            igd::PortMappingProtocol::UDP,
            port,
            SocketAddrV4::new(local, port),
            0, // 0 = no lease expiry
            "Palworld Server Manager",
        )
        .map_err(|e| format!("UPnP failed to open the port ({e}). Forward it manually."))?;
    let ext = gateway
        .get_external_ip()
        .map(|i| i.to_string())
        .unwrap_or_else(|_| "your public IP".into());
    Ok(format!("Opened UDP {port} → {local}:{port}. Players connect to {ext}:{port}."))
}

/// Remove the UPnP mapping for the game port.
pub fn unforward(app: &AppHandle) -> Result<String, String> {
    let port = game_port(app);
    let gateway = igd::search_gateway(Default::default())
        .map_err(|e| format!("No UPnP-capable router found ({e})."))?;
    gateway
        .remove_port(igd::PortMappingProtocol::UDP, port)
        .map_err(|e| format!("Could not remove the mapping ({e})."))?;
    Ok(format!("Closed UDP port {port}."))
}
