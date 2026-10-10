use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;

use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::paths::SOURCE_ENV;

const LATEST_URL: &str = "https://api.github.com/repos/zachlagden/linkgate/releases/latest";
const MAX_DOWNLOAD: u64 = 100 * 1024 * 1024;
const CHUNK: usize = 64 * 1024;
const DOWNLOAD_TIMEOUT_SECS: u64 = 600;

#[derive(Clone, Debug)]
pub enum Source {
    Github,
    Local(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub assets: Vec<Asset>,
}

impl Release {
    pub fn has(&self, name: &str) -> bool {
        self.assets.iter().any(|asset| asset.name == name)
    }
}

#[derive(Deserialize)]
struct ReleaseJson {
    tag_name: String,
    #[serde(default)]
    assets: Vec<AssetJson>,
}

#[derive(Deserialize)]
struct AssetJson {
    name: String,
    browser_download_url: String,
}

pub fn version_from_tag(tag: &str) -> String {
    tag.trim().trim_start_matches(['v', 'V']).to_string()
}

pub fn parse_release(json: &str) -> Result<Release, String> {
    let parsed: ReleaseJson =
        serde_json::from_str(json).map_err(|e| format!("GitHub's release answer wasn't understood: {e}"))?;
    Ok(Release {
        version: version_from_tag(&parsed.tag_name),
        assets: parsed
            .assets
            .into_iter()
            .map(|asset| Asset {
                name: asset.name,
                url: Some(asset.browser_download_url),
            })
            .collect(),
    })
}

fn agent() -> ureq::Agent {
    let tls = TlsConfig::builder()
        .provider(TlsProvider::NativeTls)
        .root_certs(RootCerts::PlatformVerifier)
        .build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(60)))
        .timeout_recv_body(Some(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS)))
        .user_agent(concat!("linkgate-setup/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

fn explain_status(status: u16) -> String {
    match status {
        404 => "GitHub has no published linkgate release yet.".to_string(),
        403 | 429 => "GitHub is limiting requests from this connection. Wait a while and try again.".to_string(),
        code => format!("GitHub answered with status {code}."),
    }
}

fn explain_transport(error: ureq::Error) -> String {
    format!("Couldn't reach GitHub: {error}")
}

fn get(url: &str, accept: Option<&str>) -> Result<ureq::http::Response<ureq::Body>, String> {
    let mut request = agent().get(url);
    if let Some(accept) = accept {
        request = request.header("Accept", accept);
    }
    let response = request.call().map_err(explain_transport)?;
    let status = response.status().as_u16();
    if (200..300).contains(&status) {
        Ok(response)
    } else {
        Err(explain_status(status))
    }
}

impl Source {
    pub fn from_env() -> Source {
        match std::env::var_os(SOURCE_ENV).filter(|value| !value.is_empty()) {
            Some(dir) => Source::Local(PathBuf::from(dir)),
            None => Source::Github,
        }
    }

    pub fn latest(&self) -> Result<Release, String> {
        match self {
            Source::Github => {
                let body = get(LATEST_URL, Some("application/vnd.github+json"))?
                    .body_mut()
                    .read_to_string()
                    .map_err(|e| format!("Couldn't read GitHub's answer: {e}"))?;
                parse_release(&body)
            }
            Source::Local(dir) => {
                let entries = std::fs::read_dir(dir)
                    .map_err(|e| format!("Couldn't read the local source {}: {e}", dir.display()))?;
                let assets = entries
                    .flatten()
                    .filter(|entry| entry.path().is_file())
                    .map(|entry| Asset {
                        name: entry.file_name().to_string_lossy().into_owned(),
                        url: None,
                    })
                    .collect();
                let version = std::fs::read_to_string(dir.join("VERSION"))
                    .map(|text| version_from_tag(&text))
                    .ok()
                    .filter(|text| !text.is_empty())
                    .unwrap_or_else(|| "local".to_string());
                Ok(Release { version, assets })
            }
        }
    }

    pub fn fetch(
        &self,
        release: &Release,
        name: &str,
        on_progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>, String> {
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .ok_or_else(|| format!("The latest release has no {name} file."))?;
        match (self, &asset.url) {
            (Source::Local(dir), _) => {
                let bytes = std::fs::read(dir.join(name)).map_err(|e| format!("Couldn't read {name}: {e}"))?;
                on_progress(bytes.len() as u64, Some(bytes.len() as u64));
                Ok(bytes)
            }
            (Source::Github, Some(url)) => download(url, name, on_progress),
            (Source::Github, None) => Err(format!("The latest release gives no download link for {name}.")),
        }
    }
}

fn download(url: &str, name: &str, on_progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<Vec<u8>, String> {
    let mut response = get(url, None)?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    if total.is_some_and(|size| size > MAX_DOWNLOAD) {
        return Err(format!("{name} is larger than the installer will download."));
    }
    let mut reader = response.body_mut().as_reader().take(MAX_DOWNLOAD + 1);
    let mut bytes = Vec::with_capacity(total.unwrap_or(0) as usize);
    let mut buffer = vec![0u8; CHUNK];
    loop {
        let read = reader.read(&mut buffer).map_err(|e| format!("The download of {name} was interrupted: {e}"))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() as u64 > MAX_DOWNLOAD {
            return Err(format!("{name} is larger than the installer will download."));
        }
        on_progress(bytes.len() as u64, total);
    }
    if let Some(expected) = total {
        if bytes.len() as u64 != expected {
            return Err(format!("The download of {name} ended early."));
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "tag_name": "v0.2.0",
        "name": "linkgate 0.2.0",
        "assets": [
            {"name": "linkgate.exe", "browser_download_url": "https://example.com/linkgate.exe", "size": 10},
            {"name": "SHA256SUMS", "browser_download_url": "https://example.com/SHA256SUMS", "size": 5}
        ]
    }"#;

    #[test]
    fn parses_a_release_answer() {
        let release = parse_release(SAMPLE).unwrap();
        assert_eq!(release.version, "0.2.0");
        assert!(release.has("linkgate.exe") && release.has("SHA256SUMS"));
        assert!(!release.has("linkgate-setup.exe"));
        assert_eq!(release.assets[0].url.as_deref(), Some("https://example.com/linkgate.exe"));
    }

    #[test]
    fn a_release_without_assets_still_parses() {
        let release = parse_release(r#"{"tag_name": "v1.0.0"}"#).unwrap();
        assert!(release.assets.is_empty());
    }

    #[test]
    fn rejects_answers_that_are_not_releases() {
        assert!(parse_release("{}").is_err());
        assert!(parse_release("<html>").is_err());
    }

    #[test]
    fn strips_the_tag_prefix() {
        assert_eq!(version_from_tag("v1.2.3\n"), "1.2.3");
        assert_eq!(version_from_tag("1.2.3"), "1.2.3");
    }

    #[test]
    fn local_source_lists_files_and_reads_the_version() {
        let dir = std::env::temp_dir().join(format!("linkgate-setup-source-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("linkgate.exe"), b"MZ-fake").unwrap();
        std::fs::write(dir.join("VERSION"), "v9.9.9\n").unwrap();
        let source = Source::Local(dir.clone());
        let release = source.latest().unwrap();
        assert_eq!(release.version, "9.9.9");
        let mut seen = 0;
        let bytes = source.fetch(&release, "linkgate.exe", &mut |done, _| seen = done).unwrap();
        assert_eq!(bytes, b"MZ-fake");
        assert_eq!(seen, 7);
        assert!(source.fetch(&release, "missing.exe", &mut |_, _| {}).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn local_source_without_a_version_file_is_called_local() {
        let dir = std::env::temp_dir().join(format!("linkgate-setup-source2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(Source::Local(dir.clone()).latest().unwrap().version, "local");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_local_source_is_an_error() {
        assert!(Source::Local(PathBuf::from(r"C:\definitely\not\here")).latest().is_err());
    }
}
