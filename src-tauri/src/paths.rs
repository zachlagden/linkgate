use std::path::PathBuf;

fn env_dir(key: &str) -> PathBuf {
    std::env::var_os(key)
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

pub fn data_dir() -> PathBuf {
    env_dir("LOCALAPPDATA").join("linkgate")
}

pub fn config_dir() -> PathBuf {
    env_dir("APPDATA").join("linkgate")
}

pub fn lists_dir() -> PathBuf {
    data_dir().join("lists")
}

pub fn icons_dir() -> PathBuf {
    data_dir().join("icons")
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
