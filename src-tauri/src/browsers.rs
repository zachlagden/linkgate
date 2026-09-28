use std::path::{Path, PathBuf};

use serde::Serialize;
use winreg::enums::{HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::RegKey;

use crate::{icons, logging, settings};

const CLIENT_KEYS: [(bool, &str); 3] = [
    (true, r"SOFTWARE\Clients\StartMenuInternet"),
    (false, r"SOFTWARE\Clients\StartMenuInternet"),
    (false, r"SOFTWARE\WOW6432Node\Clients\StartMenuInternet"),
];
const HIDDEN_BY_DEFAULT: [&str; 1] = ["iexplore.exe"];

#[derive(Clone)]
pub struct Browser {
    pub id: String,
    pub name: String,
    pub exe: PathBuf,
    pub icon_file: PathBuf,
    pub icon_index: i32,
    pub is_default: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserView {
    pub id: String,
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
    pub is_default: bool,
    pub hidden: bool,
}

pub fn detect() -> Vec<Browser> {
    let default_exe = default_browser_exe();
    let mut found: Vec<Browser> = Vec::new();
    for (per_user, path) in CLIENT_KEYS {
        let hive = RegKey::predef(if per_user { HKEY_CURRENT_USER } else { HKEY_LOCAL_MACHINE });
        let Ok(root) = hive.open_subkey(path) else {
            continue;
        };
        for id in root.enum_keys().flatten() {
            if found.iter().any(|b| b.id == id) {
                continue;
            }
            if let Some(browser) = read_browser(&root, &id, default_exe.as_deref()) {
                if !found.iter().any(|b| same_path(&b.exe, &browser.exe)) {
                    found.push(browser);
                }
            }
        }
    }
    found.sort_by(|a, b| {
        b.is_default
            .cmp(&a.is_default)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    found
}

fn read_browser(root: &RegKey, id: &str, default_exe: Option<&Path>) -> Option<Browser> {
    let key = root.open_subkey(id).ok()?;
    let command: String = key.open_subkey(r"shell\open\command").ok()?.get_value("").ok()?;
    let exe = exe_from_command(&command)?;
    if !exe.exists() {
        return None;
    }
    let name: String = key
        .get_value("")
        .ok()
        .filter(|n: &String| !n.trim().is_empty())
        .unwrap_or_else(|| id.to_string());
    let (icon_file, icon_index) = key
        .open_subkey("DefaultIcon")
        .ok()
        .and_then(|k| k.get_value::<String, _>("").ok())
        .and_then(|value| parse_icon_location(&value))
        .unwrap_or_else(|| (exe.clone(), 0));
    Some(Browser {
        id: id.to_string(),
        name,
        is_default: default_exe.is_some_and(|d| same_path(d, &exe)),
        exe,
        icon_file,
        icon_index,
    })
}

fn default_browser_exe() -> Option<PathBuf> {
    let prog_id: String = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\https\UserChoice")
        .ok()?
        .get_value("ProgId")
        .ok()?;
    let command: String = RegKey::predef(HKEY_CLASSES_ROOT)
        .open_subkey(format!(r"{prog_id}\shell\open\command"))
        .ok()?
        .get_value("")
        .ok()?;
    exe_from_command(&command)
}

fn exe_from_command(command: &str) -> Option<PathBuf> {
    let command = command.trim();
    let path = match command.strip_prefix('"') {
        Some(rest) => rest.split('"').next()?.to_string(),
        None => {
            let lower = command.to_lowercase();
            let end = lower.find(".exe")? + 4;
            command[..end].to_string()
        }
    };
    Some(PathBuf::from(expand_env(&path)))
}

fn parse_icon_location(value: &str) -> Option<(PathBuf, i32)> {
    let value = value.trim().trim_matches('"');
    let (file, index) = match value.rsplit_once(',') {
        Some((file, index)) => (file.trim().trim_matches('"'), index.trim().parse().unwrap_or(0)),
        None => (value, 0),
    };
    let path = PathBuf::from(expand_env(file));
    path.exists().then_some((path, index))
}

fn expand_env(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                out.push_str(&std::env::var(name).unwrap_or_else(|_| format!("%{name}%")));
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

pub fn apply_first_seen_defaults(browsers: &[Browser]) -> settings::Settings {
    let mut prefs = settings::load();
    let mut changed = false;
    for browser in browsers {
        if prefs.seen.contains(&browser.id) {
            continue;
        }
        prefs.seen.push(browser.id.clone());
        let file = browser
            .exe
            .file_name()
            .map(|f| f.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if HIDDEN_BY_DEFAULT.contains(&file.as_str()) && !prefs.hidden.contains(&browser.id) {
            prefs.hidden.push(browser.id.clone());
        }
        changed = true;
    }
    if changed {
        if let Err(error) = settings::save(&prefs) {
            logging::error("settings_save_failed", error);
        }
    }
    prefs
}

pub fn views(browsers: &[Browser], prefs: &settings::Settings) -> Vec<BrowserView> {
    browsers
        .iter()
        .map(|b| BrowserView {
            id: b.id.clone(),
            name: b.name.clone(),
            path: b.exe.to_string_lossy().to_string(),
            icon: icons::data_url(&b.icon_file, b.icon_index),
            is_default: b.is_default,
            hidden: prefs.hidden.contains(&b.id),
        })
        .collect()
}

pub fn launch(browser: &Browser, url: &str) -> Result<(), String> {
    std::process::Command::new(&browser.exe)
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Couldn't start {}: {e}", browser.name))
}
