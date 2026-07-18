//! Discord webhook notifications: server up/down/crash, player join/leave, and
//! backup events, posted as embeds to a user-configured webhook URL.

use std::collections::HashSet;
use std::time::Duration;

use serde_json::json;
use tauri::AppHandle;

use crate::{server, settings};

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

const BLUE: u32 = 0x3b82f6; // RhyseGaming brand accent
const RED: u32 = 0xe5534b;
const AMBER: u32 = 0xe3b341;
const GRAY: u32 = 0x8b98a5;

/// A given game's webhook URL, so Palworld/ARK/Enshrouded can post to different
/// Discord channels. Empty if the user hasn't set one for that game.
pub fn webhook_url_for(cfg: &settings::Discord, game_id: &str) -> String {
    cfg.webhooks.get(game_id).cloned().unwrap_or_default()
}

/// The active game's webhook URL — see `webhook_url_for`.
pub fn active_webhook_url(cfg: &settings::Discord) -> String {
    webhook_url_for(cfg, crate::game::active().spec().id)
}

/// Post a notification for a specific game/profile, regardless of which one is
/// active in the UI — used by the automation scheduler, which supervises every
/// profile's server. Fire-and-forget: the HTTP POST runs on its own thread.
pub fn notify_for(app: &AppHandle, game_id: &str, event: Event) {
    let cfg = settings::load(app).discord;
    let webhook_url = webhook_url_for(&cfg, game_id);
    notify_with(cfg, webhook_url, event);
}

/// Post a notification for the active game. Fire-and-forget: the HTTP POST runs on
/// its own thread so callers never block.
pub fn notify(app: &AppHandle, event: Event) {
    let cfg = settings::load(app).discord;
    let webhook_url = active_webhook_url(&cfg);
    notify_with(cfg, webhook_url, event);
}

fn notify_with(cfg: settings::Discord, webhook_url: String, event: Event) {
    if !cfg.enabled || webhook_url.trim().is_empty() {
        return;
    }

    let (allowed, title, desc, color) = match &event {
        Event::ServerStarted => (cfg.notify_server, "🟢 Server started".into(), String::new(), BLUE),
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
            BLUE,
        ),
        Event::PlayerJoined(name) => (
            cfg.notify_players,
            format!("➡️ {name} joined"),
            String::new(),
            BLUE,
        ),
        Event::PlayerLeft(name) => (
            cfg.notify_players,
            format!("⬅️ {name} left"),
            String::new(),
            GRAY,
        ),
        Event::Test => (true, "✅ Test message".into(), "Discord notifications are working.".into(), BLUE),
    };
    if !allowed {
        return;
    }

    let url = webhook_url.trim().to_string();
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
            let players = match tauri::async_runtime::block_on(crate::game::live::players(&dir)) {
                Ok(p) => p,
                // Not ready yet (REST/RCON), or this game has no live-control protocol
                // at all (e.g. Enshrouded) — either way, nothing to report this tick.
                Err(_) => continue,
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
