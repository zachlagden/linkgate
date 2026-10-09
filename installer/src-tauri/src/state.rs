use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WslRecord {
    pub distro: String,
    #[serde(default)]
    pub browser_env_file: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VsCodeRecord {
    pub id: String,
    pub settings_path: String,
    #[serde(default)]
    pub previous: BTreeMap<String, Value>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstallRecord {
    pub version: String,
    pub install_dir: String,
    #[serde(default)]
    pub start_menu_shortcut: bool,
    #[serde(default)]
    pub desktop_shortcut: bool,
    #[serde(default)]
    pub wsl: Vec<WslRecord>,
    #[serde(default)]
    pub vscode: Vec<VsCodeRecord>,
}

pub fn load(path: &Path) -> Option<InstallRecord> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

pub fn save(path: &Path, record: &InstallRecord) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Couldn't create {}: {e}", parent.display()))?;
    }
    let json = serde_json::to_vec_pretty(record).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| format!("Couldn't write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("linkgate-setup-state-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn round_trips_a_full_record() {
        let dir = temp("round");
        let path = dir.join("install.json");
        let mut previous = BTreeMap::new();
        previous.insert("workbench.externalBrowser".to_string(), Value::String("edge".into()));
        let record = InstallRecord {
            version: "0.2.0".into(),
            install_dir: r"C:\Users\Alex\AppData\Local\Programs\linkgate".into(),
            start_menu_shortcut: true,
            desktop_shortcut: false,
            wsl: vec![WslRecord {
                distro: "Ubuntu".into(),
                browser_env_file: Some(".zshenv".into()),
            }],
            vscode: vec![VsCodeRecord {
                id: "code".into(),
                settings_path: r"C:\Users\Alex\AppData\Roaming\Code\User\settings.json".into(),
                previous,
            }],
        };
        save(&path, &record).unwrap();
        assert_eq!(load(&path), Some(record));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_or_broken_files_load_as_none() {
        let dir = temp("broken");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(load(&dir.join("absent.json")), None);
        std::fs::write(dir.join("broken.json"), "{ not json").unwrap();
        assert_eq!(load(&dir.join("broken.json")), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn older_records_with_fewer_fields_still_load() {
        let dir = temp("old");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("install.json"), r#"{"version":"0.1.0","installDir":"C:\\x"}"#).unwrap();
        let record = load(&dir.join("install.json")).unwrap();
        assert!(record.wsl.is_empty() && record.vscode.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
