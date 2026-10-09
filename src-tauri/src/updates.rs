use std::fmt;
use std::io::Read;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::background::CREATE_NO_WINDOW;
use crate::{lock, logging, net, paths};

const LATEST_RELEASE_API: &str = "https://api.github.com/repos/zachlagden/linkgate/releases/latest";
const RELEASES_PAGE: &str = "https://github.com/zachlagden/linkgate/releases/latest";
const SETUP_FILE: &str = "linkgate-setup.exe";
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
const LOCK_STALE_SECS: u64 = 10 * 60;
const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_RESPONSE_BYTES: u64 = 1_000_000;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let text = text.strip_prefix('v').unwrap_or(text);
        let mut parts = text.split('.');
        let major = number(parts.next()?)?;
        let minor = number(parts.next()?)?;
        let patch = number(parts.next()?)?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self { major, minor, patch })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn number(part: &str) -> Option<u64> {
    if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMeta {
    pub latest: Option<String>,
    pub checked_at: u64,
    pub last_error: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub available: Option<String>,
    pub checked_at: u64,
}

fn meta_path() -> std::path::PathBuf {
    paths::data_dir().join("update.json")
}

fn lock_path() -> std::path::PathBuf {
    paths::data_dir().join("update.lock")
}

fn read_meta() -> UpdateMeta {
    std::fs::read(meta_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_meta(meta: &UpdateMeta) -> std::io::Result<()> {
    let tmp = paths::data_dir().join("update.json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(meta)?)?;
    std::fs::rename(tmp, meta_path())
}

pub fn newer_version(latest: Option<&str>, current: &str) -> Option<String> {
    let latest = Version::parse(latest?)?;
    let current = Version::parse(current)?;
    (latest > current).then(|| latest.to_string())
}

pub fn status() -> UpdateStatus {
    let meta = read_meta();
    UpdateStatus {
        available: newer_version(meta.latest.as_deref(), env!("CARGO_PKG_VERSION")),
        checked_at: meta.checked_at,
    }
}

pub fn is_stale() -> bool {
    paths::now_secs().saturating_sub(read_meta().checked_at) >= CHECK_INTERVAL_SECS
}

pub fn is_checking() -> bool {
    lock::is_held(&lock_path(), LOCK_STALE_SECS)
}

fn parse_release(body: &str) -> Result<Version, String> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let tag = value
        .get("tag_name")
        .and_then(|tag| tag.as_str())
        .ok_or("the release has no tag name")?;
    Version::parse(tag).ok_or_else(|| "the release tag is not a version number".to_string())
}

fn fetch_latest(agent: &ureq::Agent) -> Result<Option<Version>, String> {
    match agent
        .get(LATEST_RELEASE_API)
        .set("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(response) => {
            let mut body = String::new();
            response
                .into_reader()
                .take(MAX_RESPONSE_BYTES)
                .read_to_string(&mut body)
                .map_err(|e| e.to_string())?;
            parse_release(&body).map(Some)
        }
        Err(ureq::Error::Status(404, _)) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

pub fn check() -> Result<(), String> {
    std::fs::create_dir_all(paths::data_dir()).map_err(|e| e.to_string())?;
    let Some(_lock) = lock::acquire(&lock_path(), LOCK_STALE_SECS) else {
        return Err("A check is already running.".into());
    };
    let agent = net::agent(CHECK_TIMEOUT)?;
    let mut meta = read_meta();
    match fetch_latest(&agent) {
        Ok(latest) => {
            meta.latest = latest.map(|version| version.to_string());
            meta.last_error = None;
        }
        Err(error) => {
            logging::error("update_check_failed", &error);
            meta.last_error = Some(error);
        }
    }
    meta.checked_at = paths::now_secs();
    write_meta(&meta).map_err(|e| e.to_string())
}

pub fn launch_installer() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let setup = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .map(|dir| dir.join(SETUP_FILE))
        .filter(|path| path.exists());
    let result = match setup {
        Some(path) => std::process::Command::new(path).arg("--update").spawn(),
        None => std::process::Command::new("cmd")
            .args(["/c", "start", "", RELEASES_PAGE])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn(),
    };
    result
        .map(|_| ())
        .map_err(|e| format!("Couldn't start the update: {e}"))
        .inspect_err(|e| logging::error("update_launch_failed", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Option<String> {
        Version::parse(text).map(|v| v.to_string())
    }

    #[test]
    fn parses_plain_and_prefixed_versions() {
        assert_eq!(parse("1.2.3").as_deref(), Some("1.2.3"));
        assert_eq!(parse("v0.10.4").as_deref(), Some("0.10.4"));
        assert_eq!(parse(" v1.0.0 ").as_deref(), Some("1.0.0"));
    }

    #[test]
    fn rejects_prereleases_and_malformed_tags() {
        for text in ["1.2.3-rc1", "v1.2", "1.2.3.4", "v", "", "latest", "1.x.3", "1..3", "+1.2.3", "1.2.3+build"] {
            assert!(Version::parse(text).is_none(), "{text}");
        }
    }

    #[test]
    fn compares_numbers_not_strings() {
        assert!(Version::parse("0.10.0") > Version::parse("0.9.9"));
        assert!(Version::parse("1.0.0") > Version::parse("0.99.99"));
        assert!(Version::parse("1.2.4") > Version::parse("1.2.3"));
    }

    #[test]
    fn reports_only_newer_versions() {
        assert_eq!(newer_version(Some("v0.2.0"), "0.1.0").as_deref(), Some("0.2.0"));
        assert_eq!(newer_version(Some("0.1.0"), "0.1.0"), None);
        assert_eq!(newer_version(Some("0.0.9"), "0.1.0"), None);
        assert_eq!(newer_version(Some("0.2.0-beta"), "0.1.0"), None);
        assert_eq!(newer_version(None, "0.1.0"), None);
    }

    #[test]
    fn reads_the_tag_from_a_release_response() {
        let body = r#"{"tag_name":"v1.4.0","name":"linkgate 1.4.0","draft":false}"#;
        assert_eq!(parse_release(body).unwrap().to_string(), "1.4.0");
    }

    #[test]
    fn a_malformed_release_response_is_an_error() {
        assert!(parse_release("not json").is_err());
        assert!(parse_release(r#"{"name":"x"}"#).is_err());
        assert!(parse_release(r#"{"tag_name":"nightly"}"#).is_err());
        assert!(parse_release(r#"{"tag_name":7}"#).is_err());
    }
}
