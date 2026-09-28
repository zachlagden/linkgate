use std::io::Write;

use crate::paths;

const MAX_LOG_BYTES: u64 = 1_000_000;

pub fn error(event: &str, detail: impl std::fmt::Display) {
    write("error", event, &detail.to_string());
}

pub fn info(event: &str, detail: impl std::fmt::Display) {
    write("info", event, &detail.to_string());
}

fn write(level: &str, event: &str, detail: &str) {
    let dir = paths::data_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join("linkgate.log");
    if std::fs::metadata(&path).map(|m| m.len() > MAX_LOG_BYTES).unwrap_or(false) {
        let _ = std::fs::rename(&path, dir.join("linkgate.old.log"));
    }
    let line = serde_json::json!({
        "ts": paths::now_secs(),
        "level": level,
        "event": event,
        "detail": detail,
    });
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(file, "{line}");
    }
}
