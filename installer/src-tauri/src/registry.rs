use winreg::enums::HKEY_CURRENT_USER;
#[cfg(test)]
use winreg::enums::KEY_READ;
use winreg::RegKey;

use crate::paths::Locations;

pub struct Entry<'a> {
    pub version: &'a str,
}

pub fn write(locations: &Locations, entry: &Entry) -> Result<(), String> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(&locations.uninstall_key)
        .map_err(|e| format!("Couldn't register linkgate in Windows Settings: {e}"))?;
    let install_dir = locations.install_dir.to_string_lossy().into_owned();
    let exe = locations.exe_path().to_string_lossy().into_owned();
    let setup = locations.setup_path().to_string_lossy().into_owned();
    let strings = [
        ("DisplayName", "linkgate".to_string()),
        ("DisplayVersion", entry.version.to_string()),
        ("Publisher", "Zach Lagden".to_string()),
        ("InstallLocation", install_dir),
        ("DisplayIcon", exe),
        ("UninstallString", format!("\"{setup}\" --uninstall")),
        ("URLInfoAbout", "https://github.com/zachlagden/linkgate".to_string()),
    ];
    for (name, value) in strings {
        key.set_value(name, &value)
            .map_err(|e| format!("Couldn't write {name} to the registry: {e}"))?;
    }
    for name in ["NoModify", "NoRepair"] {
        key.set_value(name, &1u32)
            .map_err(|e| format!("Couldn't write {name} to the registry: {e}"))?;
    }
    Ok(())
}

pub fn remove(locations: &Locations) -> Result<(), String> {
    match RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&locations.uninstall_key) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Couldn't remove linkgate from Windows Settings: {error}")),
    }
}

#[cfg(test)]
pub fn read_value(locations: &Locations, name: &str) -> Option<String> {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(&locations.uninstall_key, KEY_READ)
        .ok()?
        .get_value(name)
        .ok()
}

#[cfg(test)]
pub fn exists(locations: &Locations) -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(&locations.uninstall_key, KEY_READ)
        .is_ok()
}
