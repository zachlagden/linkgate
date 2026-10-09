use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State, WebviewWindow};

use crate::distros::{self, WslStatus};
use crate::install::{self, Context, InstallChoices};
use crate::paths::Locations;
use crate::progress::{Event, Summary};
use crate::source::Source;
use crate::state;
use crate::uninstall::{self, UninstallChoices};
use crate::wsl::{self, BrowserEnv};

pub const PROGRESS_EVENT: &str = "setup://progress";

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Install,
    Update,
    Uninstall,
}

pub struct AppState {
    pub mode: Mode,
    pub locations: Result<Locations, String>,
    pub source: Source,
    pub current_exe: PathBuf,
}

impl AppState {
    fn locations(&self) -> Result<&Locations, String> {
        self.locations.as_ref().map_err(Clone::clone)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VsCodeView {
    id: &'static str,
    label: &'static str,
    path: String,
    available: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Previous {
    desktop_shortcut: bool,
    wsl: Vec<String>,
    vscode: Vec<String>,
    browser_env: BrowserEnv,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialState {
    mode: Mode,
    setup_version: &'static str,
    installed_version: Option<String>,
    install_dir: String,
    sandboxed: bool,
    local_source: bool,
    wsl: WslStatus,
    vscode: Vec<VsCodeView>,
    previous: Option<Previous>,
    browser_line: &'static str,
    alternatives_command: &'static str,
}

#[derive(Serialize)]
pub struct Latest {
    version: String,
}

fn browser_env_of(file: Option<&str>) -> BrowserEnv {
    match file {
        Some(".zshenv") => BrowserEnv::Zshenv,
        Some(".profile") => BrowserEnv::Profile,
        _ => BrowserEnv::Off,
    }
}

#[tauri::command]
pub async fn initial_state(state: State<'_, AppState>) -> Result<InitialState, String> {
    let wsl_status = tauri::async_runtime::spawn_blocking(distros::detect)
        .await
        .map_err(|e| e.to_string())?;
    let locations = state.locations()?;
    let record = state::load(&locations.record_path());
    let previous = record.as_ref().map(|record| Previous {
        desktop_shortcut: record.desktop_shortcut,
        wsl: record.wsl.iter().map(|item| item.distro.clone()).collect(),
        vscode: record.vscode.iter().map(|item| item.id.clone()).collect(),
        browser_env: browser_env_of(record.wsl.iter().find_map(|item| item.browser_env_file.as_deref())),
    });
    Ok(InitialState {
        mode: state.mode,
        setup_version: env!("CARGO_PKG_VERSION"),
        installed_version: record.map(|record| record.version).filter(|version| !version.is_empty()),
        install_dir: install::windows_string(&locations.install_dir),
        sandboxed: locations.is_sandboxed(),
        local_source: matches!(state.source, Source::Local(_)),
        wsl: wsl_status,
        vscode: locations
            .vscode
            .iter()
            .map(|target| VsCodeView {
                id: target.id,
                label: target.label,
                path: install::windows_string(&target.settings_path),
                available: target.available(),
            })
            .collect(),
        previous,
        browser_line: wsl::BROWSER_LINE,
        alternatives_command: wsl::ALTERNATIVES_COMMAND,
    })
}

#[tauri::command]
pub async fn check_latest(state: State<'_, AppState>) -> Result<Latest, String> {
    let source = state.source.clone();
    let release = tauri::async_runtime::spawn_blocking(move || source.latest())
        .await
        .map_err(|e| e.to_string())??;
    Ok(Latest { version: release.version })
}

#[tauri::command]
pub async fn run_install(
    app: AppHandle,
    state: State<'_, AppState>,
    choices: InstallChoices,
) -> Result<Summary, String> {
    let locations = state.locations()?.clone();
    let source = state.source.clone();
    let current_exe = state.current_exe.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let ctx = Context {
            locations: &locations,
            source: &source,
            current_exe: &current_exe,
        };
        let mut report = |event: Event| {
            let _ = app.emit(PROGRESS_EVENT, event);
        };
        install::run(&ctx, &choices, &mut report)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn run_uninstall(
    app: AppHandle,
    state: State<'_, AppState>,
    choices: UninstallChoices,
) -> Result<Summary, String> {
    let locations = state.locations()?.clone();
    let source = state.source.clone();
    let current_exe = state.current_exe.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let ctx = Context {
            locations: &locations,
            source: &source,
            current_exe: &current_exe,
        };
        let mut report = |event: Event| {
            let _ = app.emit(PROGRESS_EVENT, event);
        };
        uninstall::run(&ctx, &choices, &mut report)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn copy_text(text: String) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(text))
        .map_err(|e| format!("Couldn't copy to the clipboard: {e}"))
}

#[tauri::command]
pub fn open_linkgate(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    let locations = state.locations()?;
    let exe = locations.exe_path();
    let mut command = Command::new(&exe);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x0800_0008);
    if let Some(local) = &locations.child_local_app_data {
        command.env("LOCALAPPDATA", local);
        if let Some(roaming) = locations.config_dir.parent() {
            command.env("APPDATA", roaming);
        }
    }
    command
        .spawn()
        .map_err(|e| format!("Couldn't start {}: {e}", exe.display()))?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn present(window: WebviewWindow) -> Result<(), String> {
    crate::border::paint(&window);
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn dismiss(app: AppHandle) {
    app.exit(0);
}
