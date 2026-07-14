//! CurseForge API client — mod search for games with `ModsKind::CurseForgeIds`
//! (currently ARK: SA). Read-only: results just feed `mods::add_id`, which is what
//! actually activates a mod.

use std::time::Duration;

use serde::{Deserialize, Serialize};

const BASE: &str = "https://api.curseforge.com/v1";
const TIMEOUT: Duration = Duration::from_secs(10);

/// Baked into release builds via `CURSEFORGE_API_KEY` at compile time (see
/// CLAUDE.md's release process — same pattern as the updater signing key: the
/// value lives outside the repo, injected only for the real build). Unset in
/// normal dev builds, so a user-supplied key in Settings is the only way to test
/// search locally unless you export it yourself.
const BUILTIN_KEY: Option<&str> = option_env!("CURSEFORGE_API_KEY");

/// A user-supplied key (Settings) always wins, so power users can use their own
/// quota; otherwise fall back to the key baked into this build.
fn resolve_key(user_key: &str) -> Option<&str> {
    let user_key = user_key.trim();
    if !user_key.is_empty() {
        Some(user_key)
    } else {
        BUILTIN_KEY.map(str::trim).filter(|k| !k.is_empty())
    }
}

#[derive(Deserialize)]
struct SearchResponse {
    data: Vec<RawMod>,
}

#[derive(Deserialize)]
struct RawMod {
    id: u32,
    name: String,
    summary: String,
    #[serde(rename = "downloadCount")]
    download_count: u64,
    logo: Option<RawLogo>,
    links: Option<RawLinks>,
}

#[derive(Deserialize)]
struct RawLogo {
    #[serde(rename = "thumbnailUrl")]
    thumbnail_url: Option<String>,
}

#[derive(Deserialize)]
struct RawLinks {
    #[serde(rename = "websiteUrl")]
    website_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModResult {
    pub id: u32,
    pub name: String,
    pub summary: String,
    pub download_count: u64,
    pub thumbnail_url: Option<String>,
    pub website_url: Option<String>,
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| e.to_string())
}

fn friendly_err(status: reqwest::StatusCode) -> String {
    match status.as_u16() {
        401 | 403 => "CurseForge rejected the API key. Check it in Settings.".into(),
        _ => format!("CurseForge API error: {status}"),
    }
}

/// Search mods for `game_id` matching `query`, sorted by popularity. `user_key` is
/// whatever's saved in Settings (may be empty, in which case the build's baked-in
/// key is used if there is one).
pub async fn search(user_key: &str, game_id: u32, query: &str) -> Result<Vec<ModResult>, String> {
    let key = resolve_key(user_key)
        .ok_or("No CurseForge API key configured. Add one in Settings.")?;
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let resp = client()?
        .get(format!("{BASE}/mods/search"))
        .header("x-api-key", key)
        .query(&[
            ("gameId", game_id.to_string()),
            ("searchFilter", query.trim().to_string()),
            ("pageSize", "20".to_string()),
            ("sortField", "2".to_string()), // popularity
            ("sortOrder", "desc".to_string()),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(friendly_err(resp.status()));
    }

    let parsed: SearchResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(parsed
        .data
        .into_iter()
        .map(|m| ModResult {
            id: m.id,
            name: m.name,
            summary: m.summary,
            download_count: m.download_count,
            thumbnail_url: m.logo.and_then(|l| l.thumbnail_url),
            website_url: m.links.and_then(|l| l.website_url),
        })
        .collect())
}
