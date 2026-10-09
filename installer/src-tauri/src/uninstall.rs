use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::fsutil;
use crate::install::{self, Context, EXTERNAL_BROWSER, OPEN_LOCALHOST};
use crate::jsonc;
use crate::logging;
use crate::progress::{Event, Status, StepPlan, Steps, Summary};
use crate::registry;
use crate::shortcut;
use crate::state::{self, InstallRecord, VsCodeRecord};
use crate::wsl;

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct UninstallChoices {
    #[serde(default)]
    pub restore_vscode: bool,
    #[serde(default)]
    pub remove_data: bool,
}

fn step(id: impl Into<String>, label: impl Into<String>) -> StepPlan {
    StepPlan {
        id: id.into(),
        label: label.into(),
    }
}

fn plan(ctx: &Context, record: &InstallRecord, choices: &UninstallChoices) -> Vec<StepPlan> {
    let mut steps = Vec::new();
    for item in &record.wsl {
        steps.push(step(format!("wsl:{}", item.distro), format!("Remove the WSL setup from {}", item.distro)));
    }
    if choices.restore_vscode {
        for item in &record.vscode {
            let label = ctx
                .locations
                .vscode
                .iter()
                .find(|target| target.id == item.id)
                .map_or_else(|| item.id.clone(), |target| target.label.to_string());
            steps.push(step(format!("vscode:{}", item.id), format!("Restore {label} settings")));
        }
    }
    steps.push(step("shortcuts", "Remove the shortcuts"));
    steps.push(step("register", "Remove linkgate from Windows Settings"));
    steps.push(step("app", "Remove linkgate"));
    if choices.remove_data {
        steps.push(step("data", "Delete the blocklists and settings"));
    }
    steps.push(step("record", "Forget this install"));
    steps
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn restore_vscode(ctx: &Context, item: &VsCodeRecord) -> Result<Option<String>, String> {
    let path = PathBuf::from(&item.settings_path);
    if !path.exists() {
        return Ok(Some("The settings file is gone, so there was nothing to restore.".into()));
    }
    let (text, bom) = install::read_settings(&path)?;
    let ours = install::windows_string(&ctx.locations.exe_path());
    let current = jsonc::read_values(&text, &[EXTERNAL_BROWSER, OPEN_LOCALHOST])?;
    let mut to_set: Vec<(&str, Value)> = Vec::new();
    let mut to_remove: Vec<&str> = Vec::new();
    let mut left: Vec<&str> = Vec::new();
    for (key, still_ours) in [
        (EXTERNAL_BROWSER, current.get(EXTERNAL_BROWSER) == Some(&Value::String(ours.clone()))),
        (OPEN_LOCALHOST, current.get(OPEN_LOCALHOST) == Some(&Value::Bool(false))),
    ] {
        if !current.contains_key(key) {
            continue;
        }
        if !still_ours {
            left.push(key);
            continue;
        }
        match item.previous.get(key) {
            Some(value) => to_set.push((key, value.clone())),
            None => to_remove.push(key),
        }
    }
    let mut updated = text.clone();
    if !to_remove.is_empty() {
        updated = jsonc::remove_keys(&updated, &to_remove)?;
    }
    if !to_set.is_empty() {
        updated = jsonc::set_values(&updated, &to_set)?;
    }
    if updated != text {
        install::write_settings(&path, &updated, bom)?;
    }
    Ok(if left.is_empty() {
        None
    } else {
        Some(format!("{} was changed since the install, so it was left as it is.", left.join(" and ")))
    })
}

fn schedule_cleanup(paths: &[PathBuf], folders: &[PathBuf], install_dir: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    let deletes: String = paths
        .iter()
        .map(|path| format!(" & del /f /q \"{}\"", path.display()))
        .collect();
    let folder_removals: String = folders
        .iter()
        .map(|folder| format!(" & rmdir /s /q \"{}\"", folder.display()))
        .collect();
    let line = format!(
        "/C ping -n 3 127.0.0.1 >nul{deletes}{folder_removals} & rmdir \"{}\"",
        install_dir.display()
    );
    Command::new("cmd.exe")
        .raw_arg(line)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x0800_0008)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Couldn't schedule the removal of files still in use: {e}"))
}

fn webview_folders(locations: &crate::paths::Locations, choices: &UninstallChoices) -> Vec<PathBuf> {
    let Some(root) = locations.data_dir.parent() else {
        return Vec::new();
    };
    let mut folders = vec![root.join("uk.zachlagden.linkgate.setup")];
    if choices.remove_data {
        folders.push(root.join("uk.zachlagden.linkgate"));
    }
    folders.into_iter().filter(|folder| folder.exists()).collect()
}

fn remove_app(ctx: &Context, choices: &UninstallChoices) -> Result<Option<String>, String> {
    let locations = ctx.locations;
    let mut pending: Vec<PathBuf> = Vec::new();
    let mut failure: Option<String> = None;
    for target in [locations.exe_path(), locations.setup_path()] {
        if same_file(ctx.current_exe, &target) {
            pending.push(target);
            continue;
        }
        match fsutil::remove_file_or_defer(&target) {
            Ok(Some(left)) => pending.push(left),
            Ok(None) => {}
            Err(error) => failure = failure.or(Some(error)),
        }
    }
    for leftover in ["linkgate.exe.new", "linkgate.exe.old", "linkgate-setup.exe.new", "linkgate-setup.exe.old"] {
        let _ = std::fs::remove_file(locations.install_dir.join(leftover));
    }
    let _ = std::fs::remove_dir(&locations.install_dir);
    if let Some(error) = failure {
        return Err(error);
    }
    let folders = webview_folders(locations, choices);
    if pending.is_empty() && folders.is_empty() {
        return Ok(None);
    }
    schedule_cleanup(&pending, &folders, &locations.install_dir)?;
    Ok(Some("The last files are removed a moment after this window closes.".into()))
}

fn remove_dir(path: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Couldn't delete {}: {error}. Close linkgate and try again.", path.display())),
    }
}

pub fn run(ctx: &Context, choices: &UninstallChoices, report: &mut dyn FnMut(Event)) -> Result<Summary, String> {
    let locations = ctx.locations;
    let recorded = state::load(&locations.record_path());
    let version = recorded.as_ref().map(|r| r.version.clone()).unwrap_or_default();
    let record = recorded.clone().unwrap_or_default();
    let mut steps = Steps::new(report, plan(ctx, &record, choices));

    for item in &record.wsl {
        let id = format!("wsl:{}", item.distro);
        steps.start(&id);
        let script = wsl::uninstall_script(item.browser_env_file.as_deref());
        let outcome = wsl::run(&item.distro, &script, locations.wsl_home.as_deref()).map(|_| None);
        steps.finish(&id, outcome);
    }

    if choices.restore_vscode {
        for item in &record.vscode {
            let id = format!("vscode:{}", item.id);
            steps.start(&id);
            steps.finish(&id, restore_vscode(ctx, item));
        }
    }

    steps.start("shortcuts");
    let mut outcome = Ok(None);
    if recorded.is_none() || record.start_menu_shortcut {
        outcome = shortcut::remove(&locations.start_menu_shortcut()).map(|_| None);
    }
    if outcome.is_ok() && record.desktop_shortcut {
        outcome = shortcut::remove(&locations.desktop_shortcut()).map(|_| None);
    }
    steps.finish("shortcuts", outcome);

    steps.start("register");
    steps.finish("register", registry::remove(locations).map(|_| None));

    steps.start("app");
    steps.finish("app", remove_app(ctx, choices));

    if choices.remove_data {
        steps.start("data");
        let outcome = remove_dir(&locations.data_dir).and_then(|_| remove_dir(&locations.config_dir)).map(|_| None);
        steps.finish("data", outcome);
    }

    steps.start("record");
    let outcome = match std::fs::remove_file(locations.record_path()) {
        Ok(()) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Couldn't remove {}: {error}", locations.record_path().display())),
    };
    steps.finish("record", outcome);

    let summary = steps.into_summary(version);
    if !choices.remove_data {
        for result in summary.steps.iter().filter(|r| r.status == Status::Failed) {
            logging::write(
                &locations.data_dir,
                "error",
                &format!("uninstall_step_failed:{}", result.id),
                result.message.as_deref().unwrap_or(""),
            );
        }
    }
    Ok(summary)
}
