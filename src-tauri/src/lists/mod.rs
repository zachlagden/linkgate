mod index;

use std::collections::BTreeMap;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{logging, paths};

const SOURCE_BASE: &str =
    "https://media.githubusercontent.com/media/zachlagden/Pi-hole-Optimized-Blocklists/main/lists";
pub const LIST_NAMES: [&str; 3] = ["malicious", "suspicious", "tracking"];
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
const LOCK_STALE_SECS: u64 = 60 * 60;

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ListMeta {
    pub etag: Option<String>,
    pub file: Option<String>,
    pub count: usize,
    pub updated_at: u64,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub checked_at: u64,
    pub last_error: Option<String>,
    pub lists: BTreeMap<String, ListMeta>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ListHit {
    pub list: String,
    pub matched: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListStatus {
    pub name: String,
    pub count: usize,
    pub updated_at: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListsStatus {
    pub checked_at: u64,
    pub last_error: Option<String>,
    pub updating: bool,
    pub lists: Vec<ListStatus>,
}

fn meta_path() -> std::path::PathBuf {
    paths::lists_dir().join("meta.json")
}

fn lock_path() -> std::path::PathBuf {
    paths::lists_dir().join("update.lock")
}

pub fn read_meta() -> Meta {
    std::fs::read(meta_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_meta(meta: &Meta) -> std::io::Result<()> {
    let tmp = paths::lists_dir().join("meta.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(meta)?)?;
    std::fs::rename(tmp, meta_path())
}

fn lock_age() -> Option<u64> {
    let modified = std::fs::metadata(lock_path()).ok()?.modified().ok()?;
    Some(modified.elapsed().map(|d| d.as_secs()).unwrap_or(0))
}

pub fn is_updating() -> bool {
    lock_age().is_some_and(|age| age < LOCK_STALE_SECS)
}

pub fn is_stale() -> bool {
    paths::now_secs().saturating_sub(read_meta().checked_at) >= CHECK_INTERVAL_SECS
}

pub fn status() -> ListsStatus {
    let meta = read_meta();
    ListsStatus {
        checked_at: meta.checked_at,
        last_error: meta.last_error.clone(),
        updating: is_updating(),
        lists: LIST_NAMES
            .iter()
            .map(|name| {
                let entry = meta.lists.get(*name).cloned().unwrap_or_default();
                ListStatus {
                    name: name.to_string(),
                    count: entry.count,
                    updated_at: entry.updated_at,
                }
            })
            .collect(),
    }
}

fn candidates(host: &str) -> Vec<String> {
    let registrable = psl::domain_str(host).unwrap_or(host);
    let mut out = Vec::new();
    let mut current = host;
    loop {
        out.push(current.to_string());
        if current.len() <= registrable.len() {
            break;
        }
        match current.split_once('.') {
            Some((_, rest)) => current = rest,
            None => break,
        }
    }
    out
}

pub fn lookup(host: &str) -> Vec<ListHit> {
    let meta = read_meta();
    let names = candidates(host);
    let mut hits = Vec::new();
    for list in LIST_NAMES {
        let Some(file) = meta.lists.get(list).and_then(|m| m.file.clone()) else {
            continue;
        };
        let Ok(handle) = std::fs::File::open(paths::lists_dir().join(&file)) else {
            continue;
        };
        let Ok(map) = (unsafe { memmap2::Mmap::map(&handle) }) else {
            continue;
        };
        let Some(index) = index::Index::open(&map) else {
            logging::error("list_index_corrupt", list);
            continue;
        };
        if let Some(matched) = names.iter().find(|name| index.contains(name)) {
            hits.push(ListHit {
                list: list.to_string(),
                matched: matched.clone(),
            });
        }
    }
    hits
}

struct LockGuard;

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(lock_path());
    }
}

fn acquire_lock() -> Option<LockGuard> {
    if lock_age().is_some_and(|age| age >= LOCK_STALE_SECS) {
        let _ = std::fs::remove_file(lock_path());
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path())
        .ok()
        .map(|_| LockGuard)
}

pub fn update_all() -> Result<(), String> {
    std::fs::create_dir_all(paths::lists_dir()).map_err(|e| e.to_string())?;
    let Some(_lock) = acquire_lock() else {
        return Err("An update is already running.".into());
    };
    let agent = build_agent()?;
    let mut meta = read_meta();
    let mut failure = None;
    for list in LIST_NAMES {
        let previous = meta.lists.get(list).cloned().unwrap_or_default();
        match update_list(&agent, list, &previous) {
            Ok(Some(updated)) => {
                if let Some(old) = previous.file.as_ref().filter(|f| Some(*f) != updated.file.as_ref()) {
                    let _ = std::fs::remove_file(paths::lists_dir().join(old));
                }
                meta.lists.insert(list.to_string(), updated);
            }
            Ok(None) => {}
            Err(error) => {
                logging::error("list_update_failed", format!("{list}: {error}"));
                failure = Some(format!("Couldn't update the {list} list: {error}"));
            }
        }
    }
    meta.checked_at = paths::now_secs();
    meta.last_error = failure.clone();
    write_meta(&meta).map_err(|e| e.to_string())?;
    cleanup_orphans(&meta);
    failure.map_or(Ok(()), Err)
}

fn build_agent() -> Result<ureq::Agent, String> {
    let tls = native_tls::TlsConnector::new().map_err(|e| e.to_string())?;
    Ok(ureq::AgentBuilder::new()
        .tls_connector(Arc::new(tls))
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(600))
        .user_agent(concat!("linkgate/", env!("CARGO_PKG_VERSION")))
        .build())
}

fn update_list(agent: &ureq::Agent, list: &str, previous: &ListMeta) -> Result<Option<ListMeta>, String> {
    let has_file = previous
        .file
        .as_ref()
        .is_some_and(|f| paths::lists_dir().join(f).exists());
    let mut request = agent.get(&format!("{SOURCE_BASE}/{list}.txt"));
    if let (true, Some(etag)) = (has_file, previous.etag.as_ref()) {
        request = request.set("If-None-Match", etag);
    }
    let response = request.call().map_err(|e| e.to_string())?;
    if response.status() == 304 {
        return Ok(None);
    }
    let etag = response.header("etag").map(str::to_string);
    let mut body = String::new();
    response
        .into_reader()
        .read_to_string(&mut body)
        .map_err(|e| e.to_string())?;
    let domains = index::parse_hosts(&body);
    drop(body);
    if domains.is_empty() {
        return Err("the download contained no domains".into());
    }
    let file = format!("{list}-{}.idx", paths::now_secs());
    index::write(&paths::lists_dir().join(&file), &domains).map_err(|e| e.to_string())?;
    logging::info("list_updated", format!("{list}: {} domains", domains.len()));
    Ok(Some(ListMeta {
        etag,
        file: Some(file),
        count: domains.len(),
        updated_at: paths::now_secs(),
    }))
}

fn cleanup_orphans(meta: &Meta) {
    let Ok(entries) = std::fs::read_dir(paths::lists_dir()) else {
        return;
    };
    let current: Vec<&str> = meta.lists.values().filter_map(|m| m.file.as_deref()).collect();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".idx") && !current.contains(&name.as_str()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

pub fn spawn_background_update() {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    if let Err(error) = std::process::Command::new(exe)
        .arg("--update-lists")
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
        .spawn()
    {
        logging::error("list_update_spawn_failed", error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_stop_at_registrable_domain() {
        assert_eq!(
            candidates("a.b.example.co.uk"),
            vec!["a.b.example.co.uk", "b.example.co.uk", "example.co.uk"]
        );
        assert_eq!(candidates("example.com"), vec!["example.com"]);
    }
}
