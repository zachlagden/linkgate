use std::path::{Path, PathBuf};

pub const SANDBOX_ENV: &str = "LINKGATE_SETUP_SANDBOX";
pub const SOURCE_ENV: &str = "LINKGATE_SETUP_SOURCE";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\linkgate";
const SANDBOX_KEY_ROOT: &str = r"Software\LinkgateSetupSandbox";
const SANDBOX_WSL_HOME: &str = "/tmp/linkgate-setup-sandbox";

#[derive(Clone, Debug)]
pub struct VsCodeTarget {
    pub id: &'static str,
    pub label: &'static str,
    pub settings_path: PathBuf,
}

impl VsCodeTarget {
    pub fn available(&self) -> bool {
        self.settings_path.parent().is_some_and(Path::is_dir)
    }
}

#[derive(Clone, Debug)]
pub struct Locations {
    pub install_dir: PathBuf,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub start_menu_dir: PathBuf,
    pub desktop_dir: PathBuf,
    pub vscode: Vec<VsCodeTarget>,
    pub uninstall_key: String,
    pub wsl_home: Option<String>,
    pub child_local_app_data: Option<PathBuf>,
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3))
}

fn vscode_targets(roaming: &Path) -> Vec<VsCodeTarget> {
    vec![
        VsCodeTarget {
            id: "code",
            label: "Visual Studio Code",
            settings_path: roaming.join("Code").join("User").join("settings.json"),
        },
        VsCodeTarget {
            id: "insiders",
            label: "Visual Studio Code Insiders",
            settings_path: roaming.join("Code - Insiders").join("User").join("settings.json"),
        },
    ]
}

fn env_dir(key: &str) -> Result<PathBuf, String> {
    std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| format!("Windows didn't provide %{key}%, so the install location can't be found."))
}

fn known_folder(id: &windows::core::GUID) -> Option<PathBuf> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DEFAULT};

    unsafe {
        let raw = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
        let path = raw.to_string().ok();
        CoTaskMemFree(Some(raw.0 as *const _));
        path.filter(|p| !p.is_empty()).map(PathBuf::from)
    }
}

impl Locations {
    pub fn detect() -> Result<Locations, String> {
        match std::env::var_os(SANDBOX_ENV).filter(|value| !value.is_empty()) {
            Some(root) => Ok(Locations::sandboxed(Path::new(&root))),
            None => Locations::real(),
        }
    }

    pub fn real() -> Result<Locations, String> {
        use windows::Win32::UI::Shell::{FOLDERID_Desktop, FOLDERID_Programs};

        let local = env_dir("LOCALAPPDATA")?;
        let roaming = env_dir("APPDATA")?;
        let start_menu_dir = known_folder(&FOLDERID_Programs)
            .unwrap_or_else(|| roaming.join("Microsoft").join("Windows").join("Start Menu").join("Programs"));
        let desktop_dir = known_folder(&FOLDERID_Desktop)
            .or_else(|| env_dir("USERPROFILE").ok().map(|home| home.join("Desktop")))
            .ok_or("Windows didn't say where the desktop folder is.")?;
        Ok(Locations {
            install_dir: local.join("Programs").join("linkgate"),
            data_dir: local.join("linkgate"),
            config_dir: roaming.join("linkgate"),
            start_menu_dir,
            desktop_dir,
            vscode: vscode_targets(&roaming),
            uninstall_key: UNINSTALL_KEY.to_string(),
            wsl_home: None,
            child_local_app_data: None,
        })
    }

    pub fn sandboxed(root: &Path) -> Locations {
        let name = format!("{:016x}", fnv1a(root.to_string_lossy().to_lowercase().as_bytes()));
        let local = root.join("LocalAppData");
        let roaming = root.join("AppData");
        Locations {
            install_dir: local.join("Programs").join("linkgate"),
            data_dir: local.join("linkgate"),
            config_dir: roaming.join("linkgate"),
            start_menu_dir: root.join("StartMenu"),
            desktop_dir: root.join("Desktop"),
            vscode: vscode_targets(&roaming),
            uninstall_key: format!(r"{SANDBOX_KEY_ROOT}\{name}"),
            wsl_home: Some(SANDBOX_WSL_HOME.to_string()),
            child_local_app_data: Some(local),
        }
    }

    pub fn exe_path(&self) -> PathBuf {
        self.install_dir.join("linkgate.exe")
    }

    pub fn setup_path(&self) -> PathBuf {
        self.install_dir.join("linkgate-setup.exe")
    }

    pub fn record_path(&self) -> PathBuf {
        self.data_dir.join("install.json")
    }

    pub fn start_menu_shortcut(&self) -> PathBuf {
        self.start_menu_dir.join("linkgate.lnk")
    }

    pub fn desktop_shortcut(&self) -> PathBuf {
        self.desktop_dir.join("linkgate.lnk")
    }

    pub fn is_sandboxed(&self) -> bool {
        self.wsl_home.is_some()
    }
}
