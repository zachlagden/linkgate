use std::path::{Path, PathBuf};
use std::time::Duration;

const ATTEMPTS: u32 = 4;
const WAIT: Duration = Duration::from_millis(300);

fn sibling(target: &Path, suffix: &str) -> Result<PathBuf, String> {
    let name = target
        .file_name()
        .ok_or_else(|| format!("{} has no file name.", target.display()))?
        .to_string_lossy();
    Ok(target.with_file_name(format!("{name}{suffix}")))
}

fn file_label(target: &Path) -> String {
    target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| target.display().to_string())
}

pub fn replace_file(target: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Couldn't create {}: {e}", parent.display()))?;
    }
    let staged = sibling(target, ".new")?;
    let aside = sibling(target, ".old")?;
    let _ = std::fs::remove_file(&aside);
    std::fs::write(&staged, bytes).map_err(|e| format!("Couldn't write {}: {e}", staged.display()))?;

    let mut last_error = String::new();
    for attempt in 0..ATTEMPTS {
        match std::fs::rename(&staged, target) {
            Ok(()) => return Ok(()),
            Err(error) => last_error = error.to_string(),
        }
        if attempt + 1 < ATTEMPTS {
            std::thread::sleep(WAIT);
        }
    }

    if target.exists() {
        match std::fs::rename(target, &aside) {
            Ok(()) => {
                if let Err(error) = std::fs::rename(&staged, target) {
                    let _ = std::fs::rename(&aside, target);
                    let _ = std::fs::remove_file(&staged);
                    return Err(format!("Couldn't put the new {} in place: {error}", file_label(target)));
                }
                let _ = std::fs::remove_file(&aside);
                return Ok(());
            }
            Err(error) => last_error = error.to_string(),
        }
    }
    let _ = std::fs::remove_file(&staged);
    Err(format!(
        "{} is in use and can't be replaced. Close linkgate and run the installer again. ({last_error})",
        file_label(target)
    ))
}

pub fn remove_file_or_defer(target: &Path) -> Result<Option<PathBuf>, String> {
    if !target.exists() {
        return Ok(None);
    }
    let mut last_error = String::new();
    for attempt in 0..ATTEMPTS {
        match std::fs::remove_file(target) {
            Ok(()) => return Ok(None),
            Err(error) => last_error = error.to_string(),
        }
        if attempt + 1 < ATTEMPTS {
            std::thread::sleep(WAIT);
        }
    }
    let aside = sibling(target, ".old")?;
    let _ = std::fs::remove_file(&aside);
    match std::fs::rename(target, &aside) {
        Ok(()) => Ok(Some(aside)),
        Err(_) => Err(format!(
            "{} is in use and can't be removed. Close linkgate and try again. ({last_error})",
            file_label(target)
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("linkgate-setup-fs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn creates_the_directory_and_writes_a_new_file() {
        let dir = temp("new");
        let target = dir.join("nested").join("linkgate.exe");
        replace_file(&target, b"one").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"one");
        assert!(!dir.join("nested").join("linkgate.exe.new").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn replaces_an_existing_file_and_leaves_no_scraps() {
        let dir = temp("replace");
        let target = dir.join("linkgate.exe");
        std::fs::write(&target, b"old").unwrap();
        replace_file(&target, b"new").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_locked_file_fails_with_a_plain_message_and_keeps_the_old_one() {
        let dir = temp("locked");
        let target = dir.join("linkgate.exe");
        std::fs::write(&target, b"old").unwrap();
        let lock = std::fs::OpenOptions::new().read(true).share_mode(0).open(&target).unwrap();
        let error = replace_file(&target, b"new").unwrap_err();
        assert!(error.contains("in use"), "{error}");
        drop(lock);
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        assert!(!dir.join("linkgate.exe.new").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn removes_a_file_or_reports_that_it_was_already_gone() {
        let dir = temp("remove");
        let target = dir.join("a.exe");
        std::fs::write(&target, b"x").unwrap();
        assert_eq!(remove_file_or_defer(&target).unwrap(), None);
        assert!(!target.exists());
        assert_eq!(remove_file_or_defer(&target).unwrap(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
