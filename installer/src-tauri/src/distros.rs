use serde::Serialize;

const INTERNAL: [&str; 2] = ["docker-desktop", "docker-desktop-data"];

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Distro {
    pub name: String,
    pub is_default: bool,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct WslStatus {
    pub available: bool,
    pub distros: Vec<Distro>,
    pub note: Option<String>,
}

pub fn decode(bytes: &[u8]) -> String {
    let utf16 = bytes.starts_with(&[0xFF, 0xFE]) || (bytes.len() >= 2 && bytes[1] == 0);
    let text = if utf16 {
        let body = bytes.strip_prefix(&[0xFF, 0xFE]).unwrap_or(bytes);
        let units: Vec<u16> = body.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    text.trim_start_matches('\u{feff}').replace('\0', "")
}

pub fn parse_names(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .filter(|name| !INTERNAL.iter().any(|internal| name.eq_ignore_ascii_case(internal)))
        .map(str::to_string)
        .collect()
}

pub fn mark_default(names: Vec<String>, default: Option<&str>) -> Vec<Distro> {
    let chosen = default
        .and_then(|wanted| names.iter().position(|name| name.eq_ignore_ascii_case(wanted)))
        .or(if names.is_empty() { None } else { Some(0) });
    names
        .into_iter()
        .enumerate()
        .map(|(index, name)| Distro {
            name,
            is_default: Some(index) == chosen,
        })
        .collect()
}

#[cfg(windows)]
pub fn detect() -> WslStatus {
    use std::os::windows::process::CommandExt;

    let output = std::process::Command::new("wsl.exe")
        .args(["-l", "-q"])
        .creation_flags(0x0800_0000)
        .output();
    let output = match output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return WslStatus {
                available: false,
                distros: Vec::new(),
                note: Some("WSL isn't installed on this PC.".into()),
            }
        }
        Err(error) => {
            return WslStatus {
                available: false,
                distros: Vec::new(),
                note: Some(format!("Couldn't run wsl.exe: {error}")),
            }
        }
    };
    let names = parse_names(&decode(&output.stdout));
    if names.is_empty() {
        return WslStatus {
            available: true,
            distros: Vec::new(),
            note: Some("WSL has no distributions installed.".into()),
        };
    }
    WslStatus {
        available: true,
        distros: mark_default(names, registry_default().as_deref()),
        note: None,
    }
}

#[cfg(windows)]
fn registry_default() -> Option<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let lxss = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Lxss")
        .ok()?;
    let guid: String = lxss.get_value("DefaultDistribution").ok()?;
    lxss.open_subkey(guid).ok()?.get_value("DistributionName").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16le(text: &str, bom: bool) -> Vec<u8> {
        let mut bytes = if bom { vec![0xFF, 0xFE] } else { Vec::new() };
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn decodes_utf16_with_and_without_a_bom() {
        for bom in [true, false] {
            let bytes = utf16le("Ubuntu\r\nDebian\r\n", bom);
            assert_eq!(parse_names(&decode(&bytes)), vec!["Ubuntu", "Debian"]);
        }
    }

    #[test]
    fn decodes_plain_utf8_output() {
        assert_eq!(parse_names(&decode(b"Ubuntu-24.04\nArch\n")), vec!["Ubuntu-24.04", "Arch"]);
    }

    #[test]
    fn skips_docker_distros_and_blank_lines() {
        let bytes = utf16le("docker-desktop\r\n\r\nUbuntu\r\nDocker-Desktop-Data\r\n", true);
        assert_eq!(parse_names(&decode(&bytes)), vec!["Ubuntu"]);
    }

    #[test]
    fn empty_output_has_no_distros() {
        assert!(parse_names(&decode(&[])).is_empty());
        assert!(parse_names(&decode(&utf16le("\r\n", true))).is_empty());
    }

    #[test]
    fn marks_the_registry_default_or_falls_back_to_the_first() {
        let names = vec!["Debian".to_string(), "Ubuntu".to_string()];
        let marked = mark_default(names.clone(), Some("ubuntu"));
        assert!(!marked[0].is_default && marked[1].is_default);
        let marked = mark_default(names.clone(), Some("Missing"));
        assert!(marked[0].is_default && !marked[1].is_default);
        let marked = mark_default(names, None);
        assert!(marked[0].is_default);
        assert!(mark_default(Vec::new(), None).is_empty());
    }
}
