use std::path::{Path, PathBuf};

pub struct Lock {
    path: PathBuf,
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn age(path: &Path) -> Option<u64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(modified.elapsed().map(|d| d.as_secs()).unwrap_or(0))
}

pub fn is_held(path: &Path, stale_secs: u64) -> bool {
    age(path).is_some_and(|age| age < stale_secs)
}

pub fn acquire(path: &Path, stale_secs: u64) -> Option<Lock> {
    if age(path).is_some_and(|age| age >= stale_secs) {
        let _ = std::fs::remove_file(path);
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .ok()
        .map(|_| Lock { path: path.to_path_buf() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("linkgate-lock-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("test.lock")
    }

    #[test]
    fn a_second_acquire_fails_until_the_first_is_dropped() {
        let path = scratch("second");
        let first = acquire(&path, 3600).unwrap();
        assert!(is_held(&path, 3600));
        assert!(acquire(&path, 3600).is_none());
        drop(first);
        assert!(!is_held(&path, 3600));
        assert!(acquire(&path, 3600).is_some());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn a_stale_lock_is_replaced() {
        let path = scratch("stale");
        std::fs::write(&path, "").unwrap();
        assert!(acquire(&path, 0).is_some());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
