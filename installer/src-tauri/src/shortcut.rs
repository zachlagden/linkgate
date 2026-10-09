use std::path::Path;

use windows::core::{Interface, HSTRING};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

const DESCRIPTION: &str = "Choose a browser for links and change linkgate settings";

fn init_com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
}

fn describe(error: windows::core::Error) -> String {
    error.message()
}

pub fn create(shortcut: &Path, target: &Path) -> Result<(), String> {
    if let Some(parent) = shortcut.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Couldn't create {}: {e}", parent.display()))?;
    }
    init_com();
    let working_dir = target.parent().unwrap_or(target);
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("Couldn't start the Windows shortcut service: {}", describe(e)))?;
        link.SetPath(&HSTRING::from(target.as_os_str()))
            .map_err(|e| format!("Couldn't set the shortcut target: {}", describe(e)))?;
        link.SetWorkingDirectory(&HSTRING::from(working_dir.as_os_str()))
            .map_err(|e| format!("Couldn't set the shortcut folder: {}", describe(e)))?;
        link.SetDescription(&HSTRING::from(DESCRIPTION))
            .map_err(|e| format!("Couldn't set the shortcut description: {}", describe(e)))?;
        link.SetIconLocation(&HSTRING::from(target.as_os_str()), 0)
            .map_err(|e| format!("Couldn't set the shortcut icon: {}", describe(e)))?;
        let file: IPersistFile = link
            .cast()
            .map_err(|e| format!("Couldn't save the shortcut: {}", describe(e)))?;
        file.Save(&HSTRING::from(shortcut.as_os_str()), true)
            .map_err(|e| format!("Couldn't save {}: {}", shortcut.display(), describe(e)))
    }
}

pub fn remove(shortcut: &Path) -> Result<(), String> {
    match std::fs::remove_file(shortcut) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Couldn't remove {}: {error}", shortcut.display())),
    }
}

#[cfg(test)]
pub fn target_of(shortcut: &Path) -> Option<String> {
    use windows::Win32::Foundation::MAX_PATH;
    use windows::Win32::UI::Shell::SLGP_RAWPATH;

    init_com();
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let file: IPersistFile = link.cast().ok()?;
        file.Load(&HSTRING::from(shortcut.as_os_str()), windows::Win32::System::Com::STGM(0)).ok()?;
        let mut buffer = [0u16; MAX_PATH as usize];
        link.GetPath(&mut buffer, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32).ok()?;
        let end = buffer.iter().position(|&unit| unit == 0).unwrap_or(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..end]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_shortcut_that_points_at_the_target() {
        let dir = std::env::temp_dir().join(format!("linkgate-setup-lnk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("linkgate.exe");
        std::fs::write(&target, b"MZ").unwrap();
        let shortcut = dir.join("nested").join("linkgate.lnk");
        create(&shortcut, &target).unwrap();
        assert!(shortcut.is_file());
        let resolved = target_of(&shortcut).expect("shortcut should resolve");
        assert!(resolved.eq_ignore_ascii_case(&target.to_string_lossy()), "{resolved}");
        remove(&shortcut).unwrap();
        assert!(!shortcut.exists());
        remove(&shortcut).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
