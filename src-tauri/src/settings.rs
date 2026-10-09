use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub hidden: Vec<String>,
    #[serde(default)]
    pub seen: Vec<String>,
    #[serde(default)]
    pub order: Vec<String>,
    #[serde(default = "enabled_by_default")]
    pub blocklist_enabled: bool,
    #[serde(default = "enabled_by_default")]
    pub update_check_enabled: bool,
}

fn enabled_by_default() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hidden: Vec::new(),
            seen: Vec::new(),
            order: Vec::new(),
            blocklist_enabled: true,
            update_check_enabled: true,
        }
    }
}

fn path() -> std::path::PathBuf {
    paths::config_dir().join("settings.json")
}

pub fn load() -> Settings {
    std::fs::read(path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save(settings: &Settings) -> Result<(), String> {
    std::fs::create_dir_all(paths::config_dir()).map_err(|e| e.to_string())?;
    let json = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path(), json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocklist_is_on_when_the_field_is_missing() {
        let settings: Settings = serde_json::from_str(r#"{"hidden":["a"],"seen":["a"]}"#).unwrap();
        assert!(settings.blocklist_enabled);
    }

    #[test]
    fn blocklist_is_on_for_an_empty_file() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(settings.blocklist_enabled);
    }

    #[test]
    fn blocklist_off_survives_a_round_trip() {
        let settings = Settings {
            blocklist_enabled: false,
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let loaded: Settings = serde_json::from_str(&json).unwrap();
        assert!(!loaded.blocklist_enabled);
    }

    #[test]
    fn update_check_is_on_when_the_field_is_missing() {
        let settings: Settings = serde_json::from_str(r#"{"hidden":[],"seen":[],"blocklistEnabled":false}"#).unwrap();
        assert!(settings.update_check_enabled);
        assert!(!settings.blocklist_enabled);
    }

    #[test]
    fn update_check_off_survives_a_round_trip() {
        let settings = Settings {
            update_check_enabled: false,
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let loaded: Settings = serde_json::from_str(&json).unwrap();
        assert!(!loaded.update_check_enabled);
        assert!(loaded.blocklist_enabled);
    }

    #[test]
    fn order_is_empty_when_the_field_is_missing() {
        let settings: Settings = serde_json::from_str(r#"{"hidden":[],"seen":[]}"#).unwrap();
        assert!(settings.order.is_empty());
    }

    #[test]
    fn order_survives_a_round_trip() {
        let settings = Settings {
            order: vec!["firefox".into(), "chrome".into()],
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let loaded: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.order, vec!["firefox".to_string(), "chrome".to_string()]);
    }

    #[test]
    fn default_has_the_blocklist_on() {
        assert!(Settings::default().blocklist_enabled);
        assert!(Settings::default().update_check_enabled);
    }
}
