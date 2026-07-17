//! ARK: Survival Ascended player access lists — the exclusive-join allow list and
//! the admin list. Both are plain text files, one EOS/Steam ID per line, verified
//! against real-world reports (not guessed — see the punch-list note this closes).
//! ARK-specific; callers only reach these from ARK's Config page.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config;
use crate::game;

/// Only enforced by the server when the `ExclusiveJoin` config toggle (which drives
/// the `-exclusivejoin` launch flag, see `game/ark/config.rs`) is on, and only after
/// a restart. Community reports say ARK's own enforcement of this has been flaky at
/// times — that's the game's behavior, not something this app controls.
const EXCLUSIVE_JOIN_REL: &str = "ShooterGame/Binaries/Win64/PlayersExclusiveJoinList.txt";

/// Default admin-list path used the first time this app manages the list, if
/// `AdminListURL` isn't already set to something else.
const DEFAULT_ADMIN_LIST_REL: &str = "ShooterGame/Saved/adminlist.txt";

const ADMIN_LIST_URL_KEY: &str = "gus|[ServerSettings]|AdminListURL#0";

fn require_ark() -> Result<(), String> {
    if game::active().spec().id != "ark-sa" {
        return Err("Player access lists are only available for ARK: Survival Ascended.".into());
    }
    Ok(())
}

fn read_list(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn write_list(path: &Path, ids: &[String]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let body: Vec<&str> = ids.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    fs::write(path, body.join("\n")).map_err(|e| e.to_string())
}

pub fn exclusive_join_list(install_dir: &Path) -> Result<Vec<String>, String> {
    require_ark()?;
    Ok(read_list(&install_dir.join(EXCLUSIVE_JOIN_REL)))
}

pub fn set_exclusive_join_list(install_dir: &Path, ids: &[String]) -> Result<(), String> {
    require_ark()?;
    write_list(&install_dir.join(EXCLUSIVE_JOIN_REL), ids)
}

/// Resolve `AdminListURL`'s current value to a local path we can manage. `None`
/// means it's set to a remote `http(s)://` URL, which this app can't edit — the
/// caller should point the user at the raw config field instead.
fn admin_list_path(install_dir: &Path) -> Result<Option<PathBuf>, String> {
    require_ark()?;
    let fields = config::read(install_dir)?;
    let raw = config::find(&fields, ADMIN_LIST_URL_KEY).unwrap_or_default();
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(Some(install_dir.join(DEFAULT_ADMIN_LIST_REL)));
    }
    match raw.strip_prefix("file://") {
        Some(path) => Ok(Some(PathBuf::from(path))),
        None => Ok(None),
    }
}

const REMOTE_ADMIN_LIST_ERR: &str =
    "AdminListURL is set to a remote URL — edit it directly on the Config page to manage a local list.";

pub fn admins_list(install_dir: &Path) -> Result<Vec<String>, String> {
    match admin_list_path(install_dir)? {
        Some(path) => Ok(read_list(&path)),
        None => Err(REMOTE_ADMIN_LIST_ERR.into()),
    }
}

pub fn set_admins_list(install_dir: &Path, ids: &[String]) -> Result<(), String> {
    let path = admin_list_path(install_dir)?.ok_or(REMOTE_ADMIN_LIST_ERR)?;
    write_list(&path, ids)?;

    // First time this app manages the list: point AdminListURL at the file so the
    // server actually reads it.
    let mut fields = config::read(install_dir)?;
    if config::find(&fields, ADMIN_LIST_URL_KEY).unwrap_or_default().trim().is_empty() {
        let url = format!("file://{}", path.to_string_lossy().replace('\\', "/"));
        config::upsert(&mut fields, ADMIN_LIST_URL_KEY, &url, "string");
        config::write(install_dir, &fields)?;
    }
    Ok(())
}
