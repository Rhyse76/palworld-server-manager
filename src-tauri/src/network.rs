//! Connectivity helper: public/local IP, the game port, a local "is the server
//! listening" check, and one-click UPnP port forwarding.
//!
//! Palworld's game port is **UDP** `PublicPort` (default 8211) — that's what
//! friends connect to and what must be port-forwarded.

use std::net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket};
use std::time::Duration;

use serde::Serialize;
use tauri::AppHandle;

use crate::{config, game, settings};

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
        .unwrap_or(game::active().spec().default_game_port)
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
        .unwrap_or(game::active().spec().default_game_port);
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

/// True for addresses that reach this PC over a private/overlay network
/// (Tailscale, LAN, CGNAT) rather than the public internet — in which case
/// router port-forwarding is irrelevant.
fn is_overlay_ip(ip: &str) -> bool {
    if let Ok(v4) = ip.parse::<Ipv4Addr>() {
        let o = v4.octets();
        return o[0] == 10                              // 10.0.0.0/8   (private / Tailscale often)
            || (o[0] == 192 && o[1] == 168)            // 192.168.0.0/16 (LAN)
            || (o[0] == 172 && (16..=31).contains(&o[1])) // 172.16.0.0/12 (private)
            || (o[0] == 100 && (64..=127).contains(&o[1])); // 100.64.0.0/10 (CGNAT / Tailscale)
    }
    // Tailscale IPv6 (fd7a:115c:a1e0::/48) and any other non-v4 literal → treat
    // hostnames/domains as public (they resolve to a real address).
    ip.starts_with("fd7a:") || ip.starts_with("fd")
}

/// Ask the UPnP router whether the UDP game port is currently forwarded to this
/// PC. `Some(true/false)` when a router answered; `None` when we couldn't reach
/// one (so the user must verify manually).
fn router_forwarding(port: u16, local: Option<Ipv4Addr>) -> Option<bool> {
    let gateway = igd::search_gateway(Default::default()).ok()?;
    let local_str = local.map(|i| i.to_string());
    // Walk the router's mapping table; a client may not see every entry, but a
    // matching enabled UDP entry for our port is a definite "yes".
    for i in 0..1024u32 {
        match gateway.get_generic_port_mapping_entry(i) {
            Ok(e) => {
                let ours = e.protocol == igd::PortMappingProtocol::UDP
                    && e.external_port == port
                    && e.enabled
                    && local_str.as_deref().map_or(true, |l| e.internal_client == l);
                if ours {
                    return Some(true);
                }
            }
            // End of the table (index out of bounds) — we saw no match.
            Err(_) => return Some(false),
        }
    }
    Some(false)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reachability {
    /// Something is bound to the game port on this PC (i.e. the server is up).
    pub server_running: bool,
    /// Whether the connect address is a Tailscale/VPN/LAN overlay (no forwarding needed).
    pub using_overlay: bool,
    /// Router forwarding state: `Some(true/false)` if a UPnP router answered, else `None`.
    pub router_forwarding: Option<bool>,
    /// "ready" | "not_ready" | "unknown".
    pub verdict: String,
    pub message: String,
}

/// Assess whether friends can actually reach this server, and say why not.
pub fn reachability(app: &AppHandle) -> Reachability {
    let n = info(app);
    let local = local_ipv4();
    let server_running = n.port_listening;
    let using_overlay = !n.configured_ip.is_empty() && is_overlay_ip(&n.configured_ip);

    // Overlay (Tailscale/VPN/LAN): router forwarding doesn't apply.
    if using_overlay {
        let (verdict, message) = if server_running {
            ("ready", format!(
                "Reachable over your VPN/Tailscale address ({}). Friends on the same network connect to {}:{} — no port forwarding needed.",
                n.configured_ip, n.configured_ip, n.port
            ))
        } else {
            ("not_ready", "Your VPN/Tailscale address is set, but the server isn't running. Start it, then friends on your network can connect.".into())
        };
        return Reachability { server_running, using_overlay, router_forwarding: None, verdict: verdict.into(), message };
    }

    // Public-internet path: forwarding matters.
    if !server_running {
        return Reachability {
            server_running,
            using_overlay,
            router_forwarding: None,
            verdict: "not_ready".into(),
            message: "The server isn't running on this PC. Start it before testing reachability.".into(),
        };
    }

    let router_forwarding = router_forwarding(n.port, local);
    let (verdict, message) = match router_forwarding {
        Some(true) => ("ready", format!(
            "Your router is forwarding UDP {} to this PC. Friends connect to {}:{}.",
            n.port, if n.configured_ip.is_empty() { &n.public_ip } else { &n.configured_ip }, n.port
        )),
        Some(false) => ("not_ready", format!(
            "The server is up, but your router isn't forwarding UDP {}. Use \"Open port automatically\" below, or forward it manually.",
            n.port
        )),
        None => ("unknown", format!(
            "The server is up. We couldn't reach a UPnP router to confirm forwarding — verify UDP {} is forwarded to {} on your router.",
            n.port, local.map(|i| i.to_string()).unwrap_or_else(|| "this PC".into())
        )),
    };
    Reachability { server_running, using_overlay, router_forwarding, verdict: verdict.into(), message }
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
            "RhyseGaming Server Manager",
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
