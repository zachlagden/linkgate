use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::checksums;
use crate::fsutil;
use crate::jsonc;
use crate::logging;
use crate::packages::{self, InstallOutcome, WslRunner};
use crate::paths::{Locations, VsCodeTarget};
use crate::progress::{Event, StepPlan, Steps, Summary};
use crate::registry;
use crate::shortcut;
use crate::source::{Release, Source};
use crate::state::{self, InstallRecord, VsCodeRecord, WslRecord};
use crate::wsl::{self, BrowserEnv};

pub const EXTERNAL_BROWSER: &str = "workbench.externalBrowser";
pub const OPEN_LOCALHOST: &str = "workbench.browser.openLocalhostLinks";
pub const BACKUP_SUFFIX: &str = ".linkgate-backup";
const EXE: &str = "linkgate.exe";
const SETUP: &str = "linkgate-setup.exe";
const SUMS: &str = "SHA256SUMS";

pub struct Context<'a> {
    pub locations: &'a Locations,
    pub source: &'a Source,
    pub current_exe: &'a Path,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstallChoices {
    #[serde(default)]
    pub desktop_shortcut: bool,
    #[serde(default)]
    pub wsl_distros: Vec<String>,
    #[serde(default)]
    pub install_packages: Vec<String>,
    #[serde(default)]
    pub browser_env: BrowserEnv,
    #[serde(default)]
    pub vscode: Vec<String>,
}

struct Fetched {
    release: Release,
    exe: Vec<u8>,
    sums: HashMap<String, String>,
}

pub fn windows_string(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

fn step(id: impl Into<String>, label: impl Into<String>) -> StepPlan {
    StepPlan {
        id: id.into(),
        label: label.into(),
    }
}

fn vscode_label(locations: &Locations, id: &str) -> String {
    locations
        .vscode
        .iter()
        .find(|target| target.id == id)
        .map_or_else(|| id.to_string(), |target| target.label.to_string())
}

fn plan(locations: &Locations, choices: &InstallChoices) -> Vec<StepPlan> {
    let mut steps = vec![
        step("download", "Download the latest linkgate"),
        step("app", "Install linkgate"),
        step("setupcopy", "Keep the installer for updates"),
        step("register", "Add linkgate to Windows Settings"),
        step("startmenu", "Add a Start menu shortcut"),
    ];
    if choices.desktop_shortcut {
        steps.push(step("desktop", "Add a desktop shortcut"));
    }
    for distro in &choices.wsl_distros {
        if choices.install_packages.contains(distro) {
            steps.push(step(
                format!("packages:{distro}"),
                format!("Install {} in {distro}", packages::XDG_PACKAGE),
            ));
        }
        steps.push(step(format!("wsl:{distro}"), format!("Set up WSL: {distro}")));
    }
    for id in &choices.vscode {
        steps.push(step(
            format!("vscode:{id}"),
            format!("Update {} settings", vscode_label(locations, id)),
        ));
    }
    steps.push(step("blocklists", "Start the blocklist download"));
    steps.push(step("record", "Save what was installed"));
    steps
}

fn fetch(ctx: &Context, steps: &mut Steps) -> Result<Fetched, String> {
    let release = ctx.source.latest()?;
    for name in [EXE, SUMS] {
        if !release.has(name) {
            return Err(format!("Release {} has no {name} file, so it can't be installed.", release.version));
        }
    }
    let sums_bytes = ctx.source.fetch(&release, SUMS, &mut |_, _| {})?;
    let sums = checksums::parse(&String::from_utf8_lossy(&sums_bytes));
    let exe = ctx.source.fetch(&release, EXE, &mut |done, total| steps.download(done, total))?;
    checksums::verify(EXE, &exe, &sums)?;
    if !exe.starts_with(b"MZ") {
        return Err("The downloaded linkgate.exe isn't a Windows program, so it was not installed.".into());
    }
    Ok(Fetched { release, exe, sums })
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn keep_setup_copy(ctx: &Context, fetched: &Fetched) -> Result<Option<String>, String> {
    let target = ctx.locations.setup_path();
    if !same_file(ctx.current_exe, &target) {
        let bytes = std::fs::read(ctx.current_exe)
            .map_err(|e| format!("Couldn't read the running installer to copy it: {e}"))?;
        fsutil::replace_file(&target, &bytes)?;
        return Ok(None);
    }
    if !fetched.release.has(SETUP) {
        return Ok(Some("This release has no newer installer, so the installed one was kept.".into()));
    }
    let bytes = ctx.source.fetch(&fetched.release, SETUP, &mut |_, _| {})?;
    checksums::verify(SETUP, &bytes, &fetched.sums)?;
    fsutil::replace_file(&target, &bytes)?;
    Ok(Some("The installer was refreshed from the release.".into()))
}

pub fn valid_distro(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('-') && !name.chars().any(char::is_control)
}

fn package_runner(ctx: &Context) -> Result<WslRunner, String> {
    if ctx.locations.is_sandboxed() && ctx.locations.package_shim.is_none() {
        return Err("Package installs are turned off in test mode.".into());
    }
    Ok(WslRunner {
        shim: ctx.locations.package_shim.clone(),
    })
}

fn install_packages(ctx: &Context, distro: &str) -> Result<InstallOutcome, String> {
    if !valid_distro(distro) {
        return Err(format!("{distro:?} isn't a usable WSL distribution name."));
    }
    let runner = package_runner(ctx)?;
    packages::ensure(&runner, distro)
}

fn set_up_wsl(ctx: &Context, distro: &str, env: BrowserEnv) -> Result<Option<String>, String> {
    if !valid_distro(distro) {
        return Err(format!("{distro:?} isn't a usable WSL distribution name."));
    }
    let exe = windows_string(&ctx.locations.exe_path());
    wsl::run(distro, &wsl::install_script(&exe, env), ctx.locations.wsl_home.as_deref())?;
    Ok(match env {
        BrowserEnv::Off => None,
        _ => Some("BROWSER is set for new shells.".into()),
    })
}

fn strip_bom(text: String) -> (String, bool) {
    match text.strip_prefix('\u{feff}') {
        Some(rest) => (rest.to_string(), true),
        None => (text, false),
    }
}

pub fn read_settings(path: &Path) -> Result<(String, bool), String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(strip_bom(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok((String::new(), false)),
        Err(error) => Err(format!("Couldn't read {}: {error}", path.display())),
    }
}

pub fn write_settings(path: &Path, text: &str, bom: bool) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Couldn't create {}: {e}", parent.display()))?;
    }
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".linkgate-tmp");
    let staged = path.with_file_name(name);
    let body = if bom { format!("\u{feff}{text}") } else { text.to_string() };
    std::fs::write(&staged, body).map_err(|e| format!("Couldn't write {}: {e}", staged.display()))?;
    std::fs::rename(&staged, path).map_err(|e| {
        let _ = std::fs::remove_file(&staged);
        format!("Couldn't replace {}: {e}", path.display())
    })
}

fn backup_settings(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(BACKUP_SUFFIX);
    let backup = path.with_file_name(name);
    if backup.exists() {
        return Ok(());
    }
    std::fs::copy(path, &backup)
        .map(|_| ())
        .map_err(|e| format!("Couldn't back up {}: {e}", path.display()))
}

fn point_vscode_at_linkgate(
    target: &VsCodeTarget,
    exe: &Path,
    existing: Option<&VsCodeRecord>,
) -> Result<VsCodeRecord, String> {
    let (text, bom) = read_settings(&target.settings_path)?;
    let exe_path = windows_string(exe);
    let mut previous = match existing {
        Some(record) => record.previous.clone(),
        None => jsonc::read_values(&text, &[EXTERNAL_BROWSER, OPEN_LOCALHOST])?,
    };
    previous.retain(|key, value| !(key == EXTERNAL_BROWSER && value == &Value::String(exe_path.clone())));
    backup_settings(&target.settings_path)?;
    let updated = jsonc::set_values(&text, &[(EXTERNAL_BROWSER, json!(exe_path)), (OPEN_LOCALHOST, json!(false))])?;
    write_settings(&target.settings_path, &updated, bom)?;
    Ok(VsCodeRecord {
        id: target.id.to_string(),
        settings_path: target.settings_path.to_string_lossy().into_owned(),
        previous,
    })
}

fn set_up_vscode(ctx: &Context, id: &str, record: &InstallRecord) -> Result<VsCodeRecord, String> {
    let target = ctx
        .locations
        .vscode
        .iter()
        .find(|target| target.id == id)
        .ok_or_else(|| format!("{id} isn't a VS Code edition this installer knows."))?;
    let existing = record.vscode.iter().find(|r| r.id == id);
    point_vscode_at_linkgate(target, &ctx.locations.exe_path(), existing)
}

fn start_blocklist_download(locations: &Locations) -> Result<Option<String>, String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    let mut command = Command::new(locations.exe_path());
    command
        .arg("--update-lists")
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
        .map(|_| None)
        .map_err(|e| format!("linkgate was installed, but the blocklist download didn't start: {e}"))
}

fn upsert<T>(items: &mut Vec<T>, item: T, same: impl Fn(&T) -> bool) {
    match items.iter().position(same) {
        Some(index) => items[index] = item,
        None => items.push(item),
    }
}

pub fn run(ctx: &Context, choices: &InstallChoices, report: &mut dyn FnMut(Event)) -> Result<Summary, String> {
    let locations = ctx.locations;
    let mut steps = Steps::new(report, plan(locations, choices));
    let mut record = state::load(&locations.record_path()).unwrap_or_default();

    steps.start("download");
    let fetched = match fetch(ctx, &mut steps) {
        Ok(fetched) => {
            steps.finish("download", Ok(Some(format!("Version {}", fetched.release.version))));
            fetched
        }
        Err(error) => {
            steps.finish("download", Err(error.clone()));
            logging::write(&locations.data_dir, "error", "download_failed", &error);
            return Err(error);
        }
    };

    steps.start("app");
    if let Err(error) = fsutil::replace_file(&locations.exe_path(), &fetched.exe) {
        steps.finish("app", Err(error.clone()));
        logging::write(&locations.data_dir, "error", "install_failed", &error);
        return Err(error);
    }
    steps.finish("app", Ok(None));
    record.version = fetched.release.version.clone();
    record.install_dir = windows_string(&locations.install_dir);

    steps.start("setupcopy");
    let outcome = keep_setup_copy(ctx, &fetched);
    steps.finish("setupcopy", outcome);

    steps.start("register");
    let outcome = registry::write(locations, &registry::Entry { version: &fetched.release.version }).map(|_| None);
    steps.finish("register", outcome);

    steps.start("startmenu");
    let made = steps.finish(
        "startmenu",
        shortcut::create(&locations.start_menu_shortcut(), &locations.exe_path()).map(|_| None),
    );
    record.start_menu_shortcut = record.start_menu_shortcut || made;

    if choices.desktop_shortcut {
        steps.start("desktop");
        let made = steps.finish(
            "desktop",
            shortcut::create(&locations.desktop_shortcut(), &locations.exe_path()).map(|_| None),
        );
        record.desktop_shortcut = record.desktop_shortcut || made;
    }

    for distro in &choices.wsl_distros {
        let existing = record.wsl.iter().find(|r| &r.distro == distro).cloned();
        let mut added = None;
        let mut packages_ready = true;
        if choices.install_packages.contains(distro) {
            let id = format!("packages:{distro}");
            steps.start(&id);
            match install_packages(ctx, distro) {
                Ok(InstallOutcome::Installed(done)) => {
                    steps.finish(&id, Ok(Some(format!("Installed {}.", done.packages.join(" and ")))));
                    added = Some(done);
                }
                Ok(InstallOutcome::AlreadyPresent) => {
                    steps.finish(&id, Ok(Some("Already installed, so nothing was changed.".into())));
                }
                Err(error) => {
                    steps.finish(&id, Err(error));
                    packages_ready = false;
                }
            }
        }
        let id = format!("wsl:{distro}");
        steps.start(&id);
        let done = if packages_ready {
            steps.finish(&id, set_up_wsl(ctx, distro, choices.browser_env))
        } else {
            steps.finish(&id, Err(format!("Skipped, because {distro} is still missing packages linkgate needs.")))
        };
        let installed_packages =
            packages::merge_records(existing.as_ref().and_then(|r| r.installed_packages.as_ref()), added.clone());
        if done || added.is_some() {
            let browser_env_file = if done {
                choices.browser_env.file_name().map(str::to_string)
            } else {
                existing.and_then(|r| r.browser_env_file)
            };
            let item = WslRecord {
                distro: distro.clone(),
                browser_env_file,
                installed_packages,
            };
            upsert(&mut record.wsl, item, |r| &r.distro == distro);
        }
    }

    for vscode_id in &choices.vscode {
        let id = format!("vscode:{vscode_id}");
        steps.start(&id);
        match set_up_vscode(ctx, vscode_id, &record) {
            Ok(item) => {
                steps.finish(&id, Ok(Some("Reload VS Code for the change to apply.".into())));
                upsert(&mut record.vscode, item, |r| &r.id == vscode_id);
            }
            Err(error) => {
                steps.finish(&id, Err(error));
            }
        }
    }

    steps.start("blocklists");
    steps.finish("blocklists", start_blocklist_download(locations));

    steps.start("record");
    steps.finish("record", state::save(&locations.record_path(), &record).map(|_| None));

    let summary = steps.into_summary(fetched.release.version);
    for result in summary.steps.iter().filter(|r| r.status == crate::progress::Status::Failed) {
        logging::write(
            &locations.data_dir,
            "error",
            &format!("step_failed:{}", result.id),
            result.message.as_deref().unwrap_or(""),
        );
    }
    Ok(summary)
}
