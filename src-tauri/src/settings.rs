use serde::{Deserialize, Serialize};

use crate::{paths, timeout};

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
    #[serde(default = "enabled_by_default")]
    pub auto_close_enabled: bool,
    #[serde(default = "timeout::default_seconds", deserialize_with = "timeout::deserialize_seconds")]
    pub timeout_seconds: u32,
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
            auto_close_enabled: true,
            timeout_seconds: timeout::DEFAULT_SECONDS,
        }
    }
}

fn path() -> std::path::PathBuf {
    paths::config_dir().join("settings.json")
}

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

fn parse(bytes: &[u8]) -> Option<Settings> {
    serde_json::from_slice(bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes)).ok()
}

pub fn load() -> Settings {
    std::fs::read(path())
        .ok()
        .and_then(|bytes| parse(&bytes))
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
    fn auto_close_is_on_for_10_seconds_when_the_fields_are_missing() {
        let settings: Settings = serde_json::from_str(r#"{"hidden":[],"seen":[]}"#).unwrap();
        assert!(settings.auto_close_enabled);
        assert_eq!(settings.timeout_seconds, 10);
    }

    #[test]
    fn auto_close_is_on_for_10_seconds_in_an_empty_file() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(settings.auto_close_enabled);
        assert_eq!(settings.timeout_seconds, 10);
    }

    #[test]
    fn timeout_below_the_range_loads_as_3() {
        let settings: Settings = serde_json::from_str(r#"{"timeoutSeconds":0}"#).unwrap();
        assert_eq!(settings.timeout_seconds, 3);
    }

    #[test]
    fn timeout_above_the_range_loads_as_60() {
        let settings: Settings = serde_json::from_str(r#"{"timeoutSeconds":9000}"#).unwrap();
        assert_eq!(settings.timeout_seconds, 60);
    }

    #[test]
    fn a_bad_timeout_does_not_discard_the_other_settings() {
        let settings: Settings =
            serde_json::from_str(r#"{"hidden":["a"],"timeoutSeconds":"soon","blocklistEnabled":false}"#).unwrap();
        assert_eq!(settings.timeout_seconds, 10);
        assert_eq!(settings.hidden, vec!["a".to_string()]);
        assert!(!settings.blocklist_enabled);
    }

    #[test]
    fn never_closing_survives_a_round_trip_and_keeps_the_seconds() {
        let settings = Settings {
            auto_close_enabled: false,
            timeout_seconds: 25,
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        let loaded: Settings = serde_json::from_str(&json).unwrap();
        assert!(!loaded.auto_close_enabled);
        assert_eq!(loaded.timeout_seconds, 25);
    }

    #[test]
    fn a_file_with_a_byte_order_mark_still_loads() {
        let mut bytes = UTF8_BOM.to_vec();
        bytes.extend_from_slice(br#"{"timeoutSeconds":25,"blocklistEnabled":false}"#);
        let settings = parse(&bytes).unwrap();
        assert_eq!(settings.timeout_seconds, 25);
        assert!(!settings.blocklist_enabled);
    }

    #[test]
    fn default_has_the_blocklist_on() {
        assert!(Settings::default().blocklist_enabled);
        assert!(Settings::default().update_check_enabled);
        assert!(Settings::default().auto_close_enabled);
        assert_eq!(Settings::default().timeout_seconds, 10);
    }
}
