use std::io::Write;
use std::path::Path;

pub fn write(data_dir: &Path, level: &str, event: &str, detail: &str) {
    if std::fs::create_dir_all(data_dir).is_err() {
        return;
    }
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = serde_json::json!({ "ts": seconds, "level": level, "event": event, "detail": detail });
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_dir.join("setup.log"))
    {
        let _ = writeln!(file, "{line}");
    }
}
