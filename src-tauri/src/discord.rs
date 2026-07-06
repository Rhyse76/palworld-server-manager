//! Discord webhook notifications: server up/down/crash, player join/leave, and
//! backup events, posted as embeds to a user-configured webhook URL.

use std::collections::HashSet;
use std::time::Duration;

use serde_json::json;
use tauri::AppHandle;

use crate::{rest, server, settings};

/// Notification categories, mapped to a title/color and the settings toggle that
/// gates them.
pub enum Event {
    ServerStarted,
    ServerStopped,
    Crashed,
    Restarting(String),
    BackupCreated(String),
    PlayerJoined(String),
    PlayerLeft(String),
    Test,
}

const TEAL: u32 = 0x33c9a3;
const RED: u32 = 0xe5534b;
const AMBER: u32 = 0xe3b341;
const GRAY: u32 = 0x8b98a5;

/// Post a notification if Discord is enabled and the relevant toggle is on.
/// Fire-and-forget: the HTTP POST runs on its own thread so callers never block.
pub fn notify(app: &AppHandle, event: Event) {
    let cfg = settings::load(app).discord;
    if !cfg.enabled || cfg.webhook_url.trim().is_empty() {
        return;
    }

    let (allowed, title, desc, color) = match &event {
        Event::ServerStarted => (cfg.notify_server, "🟢 Server started".into(), String::new(), TEAL),
        Event::ServerStopped => (cfg.notify_server, "🔴 Server stopped".into(), String::new(), GRAY),
        Event::Crashed => (
            cfg.notify_server,
            "⚠️ Server crashed — auto-restarting".into(),
            String::new(),
            RED,
        ),
        Event::Restarting(msg) => (cfg.notify_server, "🔄 Server restarting".into(), msg.clone(), AMBER),
        Event::BackupCreated(name) => (
            cfg.notify_backups,
            "💾 Backup created".into(),
            name.clone(),
            TEAL,
        ),
        Event::PlayerJoined(name) => (
            cfg.notify_players,
            format!("➡️ {name} joined"),
            String::new(),
            TEAL,
        ),
        Event::PlayerLeft(name) => (
            cfg.notify_players,
            format!("⬅️ {name} left"),
            String::new(),
            GRAY,
        ),
        Event::Test => (true, "✅ Test message".into(), "Discord notifications are working.".into(), TEAL),
    };
    if !allowed {
        return;
    }

    let url = cfg.webhook_url.trim().to_string();
    std::thread::spawn(move || {
        let mut embed = json!({ "title": title, "color": color });
        if !desc.is_empty() {
            embed["description"] = json!(desc);
        }
        let body = json!({ "embeds": [embed] });
        let _ = reqwest::blocking::Client::new()
            .post(&url)
            .json(&body)
            .send();
    });
}

/// Background poller that watches the live player list and posts join/leave
/// events. Runs only while the server is up and player notifications are on.
pub fn start_player_watch(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last: HashSet<String> = HashSet::new();
        let mut primed = false;

        loop {
            std::thread::sleep(Duration::from_secs(20));

            let cfg = settings::load(&app).discord;
            if !cfg.enabled || !cfg.notify_players || !server::is_running() {
                last.clear();
                primed = false;
                continue;
            }

            let dir = match settings::install_dir(&app) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let players = match tauri::async_runtime::block_on(rest::players(&dir)) {
                Ok(p) => p,
                Err(_) => continue, // REST not ready yet
            };
            let current: HashSet<String> = players
                .iter()
                .map(|p| if p.name.is_empty() { p.player_id.clone() } else { p.name.clone() })
                .filter(|s| !s.is_empty())
                .collect();

            if primed {
                for joined in current.difference(&last) {
                    notify(&app, Event::PlayerJoined(joined.clone()));
                }
                for left in last.difference(&current) {
                    notify(&app, Event::PlayerLeft(left.clone()));
                }
            }
            last = current;
            primed = true;
        }
    });
}
