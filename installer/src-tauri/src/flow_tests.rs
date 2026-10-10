use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::checksums::sha256_hex;
use crate::install::{self, Context, InstallChoices, BACKUP_SUFFIX, EXTERNAL_BROWSER, OPEN_LOCALHOST};
use crate::jsonc;
use crate::paths::Locations;
use crate::progress::{Event, Status, Summary};
use crate::registry;
use crate::shortcut;
use crate::source::Source;
use crate::state;
use crate::uninstall::{self, UninstallChoices};
use crate::wsl::BrowserEnv;

const FAKE_EXE: &str = r"C:\Windows\System32\hostname.exe";

struct Sandbox {
    root: PathBuf,
    source_dir: PathBuf,
    locations: Locations,
    source: Source,
}

impl Sandbox {
    fn new(name: &str) -> Sandbox {
        let root = std::env::temp_dir().join(format!("linkgate-setup-flow-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let source_dir = root.join("source");
        std::fs::create_dir_all(&source_dir).unwrap();
        let locations = Locations::sandboxed(&root.join("home"));
        let sandbox = Sandbox {
            source: Source::Local(source_dir.clone()),
            root,
            source_dir,
            locations,
        };
        sandbox.publish("9.9.9", &fake_exe_bytes(), None);
        sandbox
    }

    fn publish(&self, version: &str, exe: &[u8], setup: Option<&[u8]>) {
        std::fs::write(self.source_dir.join("linkgate.exe"), exe).unwrap();
        let mut sums = format!("{}  linkgate.exe\n", sha256_hex(exe));
        match setup {
            Some(bytes) => {
                std::fs::write(self.source_dir.join("linkgate-setup.exe"), bytes).unwrap();
                sums.push_str(&format!("{}  linkgate-setup.exe\n", sha256_hex(bytes)));
            }
            None => {
                let _ = std::fs::remove_file(self.source_dir.join("linkgate-setup.exe"));
            }
        }
        std::fs::write(self.source_dir.join("SHA256SUMS"), sums).unwrap();
        std::fs::write(self.source_dir.join("VERSION"), version).unwrap();
    }

    fn vscode_settings(&self, id: &str) -> PathBuf {
        self.locations.vscode.iter().find(|t| t.id == id).unwrap().settings_path.clone()
    }

    fn write_vscode(&self, id: &str, text: &str) {
        let path = self.vscode_settings(id);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn install(&self, choices: &InstallChoices) -> Result<Summary, String> {
        self.install_as(choices, &std::env::current_exe().unwrap())
    }

    fn install_as(&self, choices: &InstallChoices, current_exe: &Path) -> Result<Summary, String> {
        let ctx = Context {
            locations: &self.locations,
            source: &self.source,
            current_exe,
        };
        install::run(&ctx, choices, &mut |_| {})
    }

    fn uninstall(&self, choices: &UninstallChoices) -> Summary {
        let ctx = Context {
            locations: &self.locations,
            source: &self.source,
            current_exe: &std::env::current_exe().unwrap(),
        };
        uninstall::run(&ctx, choices, &mut |_| {}).unwrap()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = registry::remove(&self.locations);
        for _ in 0..20 {
            if std::fs::remove_dir_all(&self.root).is_ok() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
}

fn fake_exe_bytes() -> Vec<u8> {
    std::fs::read(FAKE_EXE).expect("hostname.exe is part of Windows")
}

fn failed_ids(summary: &Summary) -> Vec<&str> {
    summary
        .steps
        .iter()
        .filter(|step| step.status == Status::Failed)
        .map(|step| step.id.as_str())
        .collect()
}

fn setting(path: &Path, key: &str) -> Option<Value> {
    let text = std::fs::read_to_string(path).unwrap();
    jsonc::read_values(&text, &[key]).unwrap().remove(key)
}

const ORIGINAL_SETTINGS: &str = "{\n  // my settings\n  \"editor.fontSize\": 14,\n  \"workbench.browser.openLocalhostLinks\": true,\n}\n";

#[test]
fn installs_the_app_shortcuts_registry_entry_and_record() {
    let sandbox = Sandbox::new("full");
    sandbox.write_vscode("code", ORIGINAL_SETTINGS);
    let choices = InstallChoices {
        desktop_shortcut: true,
        vscode: vec!["code".into()],
        ..InstallChoices::default()
    };
    let summary = sandbox.install(&choices).unwrap();
    assert_eq!(failed_ids(&summary), Vec::<&str>::new(), "{summary:?}");
    assert_eq!(summary.version, "9.9.9");

    let locations = &sandbox.locations;
    assert_eq!(std::fs::read(locations.exe_path()).unwrap(), fake_exe_bytes());
    assert!(locations.setup_path().is_file());
    for lnk in [locations.start_menu_shortcut(), locations.desktop_shortcut()] {
        let target = shortcut::target_of(&lnk).unwrap();
        assert_eq!(std::fs::canonicalize(&target).unwrap(), std::fs::canonicalize(locations.exe_path()).unwrap());
    }
    assert_eq!(registry::read_value(locations, "DisplayVersion").as_deref(), Some("9.9.9"));
    let uninstall_string = registry::read_value(locations, "UninstallString").unwrap();
    assert!(uninstall_string.ends_with("linkgate-setup.exe\" --uninstall"), "{uninstall_string}");

    let record = state::load(&locations.record_path()).unwrap();
    assert_eq!(record.version, "9.9.9");
    assert!(record.start_menu_shortcut && record.desktop_shortcut);
    assert_eq!(record.vscode.len(), 1);
    assert_eq!(record.vscode[0].previous.get(OPEN_LOCALHOST), Some(&json!(true)));
    assert!(!record.vscode[0].previous.contains_key(EXTERNAL_BROWSER));

    let settings = sandbox.vscode_settings("code");
    assert_eq!(setting(&settings, OPEN_LOCALHOST), Some(json!(false)));
    let expected = install::windows_string(&locations.exe_path());
    assert_eq!(setting(&settings, EXTERNAL_BROWSER), Some(json!(expected)));
    let edited = std::fs::read_to_string(&settings).unwrap();
    assert!(edited.contains("// my settings") && edited.contains("\"editor.fontSize\": 14"));
    let backup = settings.with_file_name(format!("settings.json{BACKUP_SUFFIX}"));
    assert_eq!(std::fs::read_to_string(backup).unwrap(), ORIGINAL_SETTINGS);
}

#[test]
fn only_the_chosen_parts_are_installed() {
    let sandbox = Sandbox::new("minimal");
    sandbox.write_vscode("code", ORIGINAL_SETTINGS);
    let summary = sandbox.install(&InstallChoices::default()).unwrap();
    assert_eq!(failed_ids(&summary), Vec::<&str>::new());
    assert!(sandbox.locations.start_menu_shortcut().is_file());
    assert!(!sandbox.locations.desktop_shortcut().exists());
    assert_eq!(std::fs::read_to_string(sandbox.vscode_settings("code")).unwrap(), ORIGINAL_SETTINGS);
    assert!(summary.steps.iter().all(|step| !step.id.starts_with("wsl:") && !step.id.starts_with("vscode:")));
}

#[test]
fn updating_keeps_the_original_settings_and_the_first_backup() {
    let sandbox = Sandbox::new("update");
    sandbox.write_vscode("code", ORIGINAL_SETTINGS);
    let choices = InstallChoices {
        vscode: vec!["code".into()],
        ..InstallChoices::default()
    };
    sandbox.install(&choices).unwrap();
    sandbox.publish("9.9.10", &fake_exe_bytes(), None);
    let summary = sandbox.install(&choices).unwrap();
    assert_eq!(failed_ids(&summary), Vec::<&str>::new());
    assert_eq!(summary.version, "9.9.10");

    let record = state::load(&sandbox.locations.record_path()).unwrap();
    assert_eq!(record.version, "9.9.10");
    assert_eq!(record.vscode.len(), 1);
    assert_eq!(record.vscode[0].previous.get(OPEN_LOCALHOST), Some(&json!(true)));
    let backup = sandbox.vscode_settings("code").with_file_name(format!("settings.json{BACKUP_SUFFIX}"));
    assert_eq!(std::fs::read_to_string(backup).unwrap(), ORIGINAL_SETTINGS);
}

#[test]
fn a_checksum_mismatch_installs_nothing() {
    let sandbox = Sandbox::new("badsum");
    std::fs::write(sandbox.source_dir.join("SHA256SUMS"), format!("{}  linkgate.exe\n", sha256_hex(b"other"))).unwrap();
    let error = sandbox.install(&InstallChoices::default()).unwrap_err();
    assert!(error.contains("checksum"), "{error}");
    assert!(!sandbox.locations.exe_path().exists());
    assert!(!sandbox.locations.start_menu_shortcut().exists());
    assert!(!registry::exists(&sandbox.locations));
}

#[test]
fn a_release_without_checksums_installs_nothing() {
    let sandbox = Sandbox::new("nosums");
    std::fs::remove_file(sandbox.source_dir.join("SHA256SUMS")).unwrap();
    let error = sandbox.install(&InstallChoices::default()).unwrap_err();
    assert!(error.contains("SHA256SUMS"), "{error}");
    assert!(!sandbox.locations.exe_path().exists());
}

#[test]
fn a_download_that_is_not_a_program_is_refused() {
    let sandbox = Sandbox::new("notexe");
    sandbox.publish("1.0.0", b"<html>not found</html>", None);
    let error = sandbox.install(&InstallChoices::default()).unwrap_err();
    assert!(error.contains("isn't a Windows program"), "{error}");
}

#[test]
fn bad_choices_fail_their_own_step_and_nothing_else() {
    let sandbox = Sandbox::new("badchoices");
    let choices = InstallChoices {
        wsl_distros: vec!["--help".into()],
        vscode: vec!["notepad".into()],
        browser_env: BrowserEnv::Off,
        ..InstallChoices::default()
    };
    let summary = sandbox.install(&choices).unwrap();
    assert_eq!(failed_ids(&summary), vec!["wsl:--help", "vscode:notepad"]);
    let record = state::load(&sandbox.locations.record_path()).unwrap();
    assert!(record.wsl.is_empty() && record.vscode.is_empty());
    assert!(sandbox.locations.exe_path().is_file());
}

#[test]
fn a_broken_settings_file_is_reported_and_left_alone() {
    let sandbox = Sandbox::new("brokenjson");
    sandbox.write_vscode("code", "{ \"a\": ");
    let choices = InstallChoices {
        vscode: vec!["code".into()],
        ..InstallChoices::default()
    };
    let summary = sandbox.install(&choices).unwrap();
    assert_eq!(failed_ids(&summary), vec!["vscode:code"]);
    assert_eq!(std::fs::read_to_string(sandbox.vscode_settings("code")).unwrap(), "{ \"a\": ");
}

#[test]
fn a_new_settings_file_is_created_when_the_folder_exists() {
    let sandbox = Sandbox::new("newsettings");
    let folder = sandbox.vscode_settings("insiders").parent().unwrap().to_path_buf();
    std::fs::create_dir_all(&folder).unwrap();
    let choices = InstallChoices {
        vscode: vec!["insiders".into()],
        ..InstallChoices::default()
    };
    let summary = sandbox.install(&choices).unwrap();
    assert_eq!(failed_ids(&summary), Vec::<&str>::new());
    assert_eq!(setting(&sandbox.vscode_settings("insiders"), OPEN_LOCALHOST), Some(json!(false)));
}

#[test]
fn running_from_the_installed_copy_refreshes_it_from_the_release() {
    let sandbox = Sandbox::new("selfupdate");
    let installed = sandbox.locations.setup_path();
    std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
    std::fs::write(&installed, b"MZ-old-installer").unwrap();
    sandbox.publish("9.9.9", &fake_exe_bytes(), Some(b"MZ-new-installer"));
    let summary = sandbox.install_as(&InstallChoices::default(), &installed).unwrap();
    assert_eq!(failed_ids(&summary), Vec::<&str>::new());
    assert_eq!(std::fs::read(&installed).unwrap(), b"MZ-new-installer");

    sandbox.publish("9.9.9", &fake_exe_bytes(), None);
    let summary = sandbox.install_as(&InstallChoices::default(), &installed).unwrap();
    assert_eq!(failed_ids(&summary), Vec::<&str>::new());
    assert_eq!(std::fs::read(&installed).unwrap(), b"MZ-new-installer");
    let note = summary.steps.iter().find(|s| s.id == "setupcopy").unwrap().message.clone().unwrap();
    assert!(note.contains("no newer installer"), "{note}");
}

#[test]
fn a_release_installer_with_a_wrong_checksum_is_not_used() {
    let sandbox = Sandbox::new("badsetup");
    let installed = sandbox.locations.setup_path();
    std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
    std::fs::write(&installed, b"MZ-old-installer").unwrap();
    sandbox.publish("9.9.9", &fake_exe_bytes(), Some(b"MZ-new-installer"));
    std::fs::write(sandbox.source_dir.join("linkgate-setup.exe"), b"MZ-tampered").unwrap();
    let summary = sandbox.install_as(&InstallChoices::default(), &installed).unwrap();
    assert_eq!(failed_ids(&summary), vec!["setupcopy"]);
    assert_eq!(std::fs::read(&installed).unwrap(), b"MZ-old-installer");
}

#[test]
fn progress_events_cover_the_plan_and_every_step() {
    let sandbox = Sandbox::new("events");
    let ctx = Context {
        locations: &sandbox.locations,
        source: &sandbox.source,
        current_exe: &std::env::current_exe().unwrap(),
    };
    let mut events: Vec<Event> = Vec::new();
    install::run(&ctx, &InstallChoices::default(), &mut |event| events.push(event)).unwrap();
    let Some(Event::Plan { steps }) = events.first() else {
        panic!("the first event should be the plan");
    };
    let planned: Vec<&str> = steps.iter().map(|s| s.id.as_str()).collect();
    for id in &planned {
        let finished = events.iter().any(|e| matches!(e, Event::Step { id: got, status: Status::Done | Status::Failed, .. } if got == id));
        assert!(finished, "{id} never finished");
    }
    assert!(events.iter().any(|e| matches!(e, Event::Download { .. })));
}

#[test]
fn uninstalling_restores_vscode_and_removes_everything_the_installer_made() {
    let sandbox = Sandbox::new("uninstall");
    sandbox.write_vscode("code", ORIGINAL_SETTINGS);
    let choices = InstallChoices {
        desktop_shortcut: true,
        vscode: vec!["code".into()],
        ..InstallChoices::default()
    };
    sandbox.install(&choices).unwrap();

    let summary = sandbox.uninstall(&UninstallChoices {
        restore_vscode: true,
        remove_data: false,
        ..UninstallChoices::default()
    });
    assert_eq!(failed_ids(&summary), Vec::<&str>::new(), "{summary:?}");

    let locations = &sandbox.locations;
    assert!(!locations.exe_path().exists());
    assert!(!locations.setup_path().exists());
    assert!(!locations.start_menu_shortcut().exists());
    assert!(!locations.desktop_shortcut().exists());
    assert!(!registry::exists(locations));
    assert!(!locations.record_path().exists());

    let settings = sandbox.vscode_settings("code");
    assert_eq!(setting(&settings, EXTERNAL_BROWSER), None);
    assert_eq!(setting(&settings, OPEN_LOCALHOST), Some(json!(true)));
    let restored = std::fs::read_to_string(&settings).unwrap();
    assert!(restored.contains("// my settings") && restored.contains("\"editor.fontSize\": 14"));
}

#[test]
fn uninstalling_leaves_vscode_alone_unless_asked() {
    let sandbox = Sandbox::new("keepvscode");
    sandbox.write_vscode("code", ORIGINAL_SETTINGS);
    let choices = InstallChoices {
        vscode: vec!["code".into()],
        ..InstallChoices::default()
    };
    sandbox.install(&choices).unwrap();
    let before = std::fs::read_to_string(sandbox.vscode_settings("code")).unwrap();
    let summary = sandbox.uninstall(&UninstallChoices::default());
    assert_eq!(failed_ids(&summary), Vec::<&str>::new());
    assert_eq!(std::fs::read_to_string(sandbox.vscode_settings("code")).unwrap(), before);
}

#[test]
fn uninstalling_does_not_clobber_settings_changed_since_the_install() {
    let sandbox = Sandbox::new("changed");
    sandbox.write_vscode("code", ORIGINAL_SETTINGS);
    let choices = InstallChoices {
        vscode: vec!["code".into()],
        ..InstallChoices::default()
    };
    sandbox.install(&choices).unwrap();
    let settings = sandbox.vscode_settings("code");
    let text = std::fs::read_to_string(&settings).unwrap();
    let edited = jsonc::set_values(&text, &[(EXTERNAL_BROWSER, json!(r"C:\Browsers\other.exe"))]).unwrap();
    std::fs::write(&settings, edited).unwrap();
    let summary = sandbox.uninstall(&UninstallChoices {
        restore_vscode: true,
        remove_data: false,
        ..UninstallChoices::default()
    });
    assert_eq!(failed_ids(&summary), Vec::<&str>::new());
    assert_eq!(setting(&settings, EXTERNAL_BROWSER), Some(json!(r"C:\Browsers\other.exe")));
    assert_eq!(setting(&settings, OPEN_LOCALHOST), Some(json!(true)));
}

#[test]
fn uninstalling_can_delete_blocklists_and_settings() {
    let sandbox = Sandbox::new("data");
    sandbox.install(&InstallChoices::default()).unwrap();
    std::fs::create_dir_all(sandbox.locations.data_dir.join("lists")).unwrap();
    std::fs::create_dir_all(&sandbox.locations.config_dir).unwrap();
    std::fs::write(sandbox.locations.config_dir.join("settings.json"), "{}").unwrap();

    let kept = sandbox.uninstall(&UninstallChoices::default());
    assert_eq!(failed_ids(&kept), Vec::<&str>::new());
    assert!(sandbox.locations.data_dir.join("lists").is_dir());
    assert!(sandbox.locations.config_dir.join("settings.json").is_file());

    sandbox.install(&InstallChoices::default()).unwrap();
    let removed = sandbox.uninstall(&UninstallChoices {
        restore_vscode: false,
        remove_data: true,
        ..UninstallChoices::default()
    });
    assert_eq!(failed_ids(&removed), Vec::<&str>::new());
    assert!(!sandbox.locations.data_dir.exists());
    assert!(!sandbox.locations.config_dir.exists());
}

#[test]
fn uninstalling_without_a_record_still_cleans_up_what_it_can() {
    let sandbox = Sandbox::new("norecord");
    let summary = sandbox.uninstall(&UninstallChoices::default());
    assert_eq!(failed_ids(&summary), Vec::<&str>::new());
}

#[test]
fn uninstalling_from_the_installed_copy_schedules_its_removal() {
    let sandbox = Sandbox::new("selfremove");
    let installed = sandbox.locations.setup_path();
    std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
    std::fs::write(&installed, b"MZ-installer").unwrap();
    sandbox.install_as(&InstallChoices::default(), &installed).unwrap();
    let ctx = Context {
        locations: &sandbox.locations,
        source: &sandbox.source,
        current_exe: &installed,
    };
    let summary = uninstall::run(&ctx, &UninstallChoices::default(), &mut |_| {}).unwrap();
    assert_eq!(failed_ids(&summary), Vec::<&str>::new());
    for _ in 0..40 {
        if !installed.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    assert!(!installed.exists(), "the installed setup copy should be deleted after a moment");
}

const ABSENT_DISTRO: &str = "Linkgate-Test-Absent";

fn step_message<'a>(summary: &'a Summary, id: &str) -> &'a str {
    summary
        .steps
        .iter()
        .find(|step| step.id == id)
        .and_then(|step| step.message.as_deref())
        .unwrap_or_else(|| panic!("no message for step {id}"))
}

fn package_record_for(distro: &str) -> state::WslRecord {
    state::WslRecord {
        distro: distro.into(),
        browser_env_file: None,
        installed_packages: Some(state::PackageRecord {
            manager: "apt-get".into(),
            packages: vec!["xdg-utils".into()],
        }),
    }
}

#[test]
fn package_installs_are_refused_in_test_mode_and_skip_the_wsl_setup() {
    let sandbox = Sandbox::new("pkgrefused");
    let choices = InstallChoices {
        wsl_distros: vec![ABSENT_DISTRO.into()],
        install_packages: vec![ABSENT_DISTRO.into()],
        ..InstallChoices::default()
    };
    let summary = sandbox.install(&choices).unwrap();
    let packages_id = format!("packages:{ABSENT_DISTRO}");
    let wsl_id = format!("wsl:{ABSENT_DISTRO}");
    assert_eq!(failed_ids(&summary), vec![packages_id.as_str(), wsl_id.as_str()]);
    assert!(step_message(&summary, &packages_id).contains("turned off in test mode"));
    assert!(step_message(&summary, &wsl_id).starts_with("Skipped"));
    let record = state::load(&sandbox.locations.record_path()).unwrap();
    assert!(record.wsl.is_empty());
}

#[test]
fn consent_for_a_distro_that_is_not_ticked_adds_no_step() {
    let sandbox = Sandbox::new("pkgunticked");
    let ctx = Context {
        locations: &sandbox.locations,
        source: &sandbox.source,
        current_exe: &std::env::current_exe().unwrap(),
    };
    let choices = InstallChoices {
        install_packages: vec![ABSENT_DISTRO.into()],
        ..InstallChoices::default()
    };
    let mut events: Vec<Event> = Vec::new();
    install::run(&ctx, &choices, &mut |event| events.push(event)).unwrap();
    let Some(Event::Plan { steps }) = events.first() else {
        panic!("the first event should be the plan");
    };
    assert!(steps.iter().all(|step| !step.id.starts_with("packages:") && !step.id.starts_with("wsl:")));
}

#[test]
fn a_distro_without_consent_gets_no_package_step() {
    let sandbox = Sandbox::new("pkgnoconsent");
    let choices = InstallChoices {
        wsl_distros: vec![ABSENT_DISTRO.into()],
        ..InstallChoices::default()
    };
    let summary = sandbox.install(&choices).unwrap();
    assert!(summary.steps.iter().all(|step| !step.id.starts_with("packages:")));
}

#[test]
fn a_repeated_install_keeps_the_record_of_packages_it_added_earlier() {
    let sandbox = Sandbox::new("pkgkeep");
    sandbox.install(&InstallChoices::default()).unwrap();
    let path = sandbox.locations.record_path();
    let mut record = state::load(&path).unwrap();
    record.wsl.push(package_record_for(ABSENT_DISTRO));
    state::save(&path, &record).unwrap();
    let choices = InstallChoices {
        wsl_distros: vec![ABSENT_DISTRO.into()],
        ..InstallChoices::default()
    };
    sandbox.install(&choices).unwrap();
    let after = state::load(&path).unwrap();
    assert_eq!(after.wsl, vec![package_record_for(ABSENT_DISTRO)]);
}

#[test]
fn uninstalling_offers_package_removal_only_for_what_the_installer_added() {
    let sandbox = Sandbox::new("pkguninstall");
    sandbox.install(&InstallChoices::default()).unwrap();
    let path = sandbox.locations.record_path();
    let mut record = state::load(&path).unwrap();
    record.wsl.push(package_record_for(ABSENT_DISTRO));
    state::save(&path, &record).unwrap();

    let kept = sandbox.uninstall(&UninstallChoices::default());
    assert!(kept.steps.iter().all(|step| !step.id.starts_with("packages:")));

    state::save(&path, &record).unwrap();
    let removed = sandbox.uninstall(&UninstallChoices {
        remove_packages: vec![ABSENT_DISTRO.into()],
        ..UninstallChoices::default()
    });
    let id = format!("packages:{ABSENT_DISTRO}");
    assert!(failed_ids(&removed).contains(&id.as_str()));
    assert!(step_message(&removed, &id).contains("turned off in test mode"));
}

#[test]
fn removal_is_never_offered_for_a_distro_without_a_package_record() {
    let sandbox = Sandbox::new("pkgnorecord");
    sandbox.install(&InstallChoices::default()).unwrap();
    let path = sandbox.locations.record_path();
    let mut record = state::load(&path).unwrap();
    record.wsl.push(state::WslRecord {
        distro: ABSENT_DISTRO.into(),
        browser_env_file: None,
        installed_packages: None,
    });
    state::save(&path, &record).unwrap();
    let removed = sandbox.uninstall(&UninstallChoices {
        remove_packages: vec![ABSENT_DISTRO.into()],
        ..UninstallChoices::default()
    });
    assert!(removed.steps.iter().all(|step| !step.id.starts_with("packages:")));
}

#[test]
#[ignore = "runs against a real WSL distro that has no xdg-utils, set LINKGATE_SETUP_TEST_DISTRO"]
fn the_package_flow_runs_through_a_fake_package_manager_in_a_real_distro() {
    let distro = std::env::var("LINKGATE_SETUP_TEST_DISTRO").expect("LINKGATE_SETUP_TEST_DISTRO");
    let shim = format!("/tmp/linkgate-setup-shim-{}", std::process::id());
    let log = format!("{shim}/log");
    let make = format!(
        "set -eu; mkdir -p {shim}; printf '#!/bin/sh\\necho \"$@\" >> {log}\\n' > {shim}/apt-get; chmod +x {shim}/apt-get"
    );
    crate::wsl::run(&distro, &make, None).unwrap();

    let mut sandbox = Sandbox::new("pkgreal");
    sandbox.locations.package_shim = Some(shim.clone());
    let choices = InstallChoices {
        wsl_distros: vec![distro.clone()],
        install_packages: vec![distro.clone()],
        ..InstallChoices::default()
    };
    let summary = sandbox.install(&choices).unwrap();
    let packages_id = format!("packages:{distro}");
    let status = |id: &str| summary.steps.iter().find(|s| s.id == id).map(|s| s.status);
    assert_eq!(status(&packages_id), Some(Status::Done), "{summary:?}");
    assert_eq!(step_message(&summary, &packages_id), "Installed xdg-utils.");
    let record = state::load(&sandbox.locations.record_path()).unwrap();
    let added = record.wsl.iter().find(|r| r.distro == distro).and_then(|r| r.installed_packages.clone());
    assert_eq!(added.map(|a| a.packages), Some(vec!["xdg-utils".to_string()]));

    let removed = sandbox.uninstall(&UninstallChoices {
        remove_packages: vec![distro.clone()],
        ..UninstallChoices::default()
    });
    let removed_status = removed.steps.iter().find(|s| s.id == packages_id).map(|s| s.status);
    assert_eq!(removed_status, Some(Status::Done), "{removed:?}");

    let log_text = crate::wsl::run(&distro, &format!("cat {log}"), None).unwrap();
    crate::wsl::run(&distro, &format!("rm -rf {shim}"), None).unwrap();
    let lines: Vec<&str> = log_text.lines().collect();
    assert_eq!(
        lines,
        vec!["install -y --no-install-recommends xdg-utils", "remove -y xdg-utils"],
        "{log_text}"
    );
}

#[test]
fn headless_jobs_install_and_uninstall_through_the_environment() {
    let sandbox = Sandbox::new("headless");
    let root = sandbox.root.join("home");
    let job_path = sandbox.root.join("job.json");
    let result_path = sandbox.root.join("result.json");
    std::env::set_var(crate::paths::SANDBOX_ENV, &root);
    std::env::set_var(crate::paths::SOURCE_ENV, &sandbox.source_dir);

    let write_job = |job: Value| std::fs::write(&job_path, serde_json::to_vec(&job).unwrap()).unwrap();
    let read_result = || -> Value { serde_json::from_slice(&std::fs::read(&result_path).unwrap()).unwrap() };

    write_job(json!({ "mode": "install", "install": { "desktopShortcut": true }, "resultPath": result_path }));
    assert_eq!(crate::headless::run(&job_path), 0);
    let result = read_result();
    assert_eq!(result["ok"], json!(true), "{result}");
    assert!(sandbox.locations.exe_path().is_file());
    assert!(sandbox.locations.desktop_shortcut().is_file());

    write_job(json!({ "mode": "uninstall", "uninstall": {}, "resultPath": result_path }));
    assert_eq!(crate::headless::run(&job_path), 0);
    assert_eq!(read_result()["ok"], json!(true));
    assert!(!sandbox.locations.exe_path().exists());

    write_job(json!({ "mode": "sideways", "resultPath": result_path }));
    assert_eq!(crate::headless::run(&job_path), 1);
    assert!(read_result()["error"].as_str().unwrap().contains("sideways"));

    std::fs::write(&job_path, "not json").unwrap();
    assert_eq!(crate::headless::run(&job_path), 2);

    std::env::remove_var(crate::paths::SANDBOX_ENV);
    std::env::remove_var(crate::paths::SOURCE_ENV);
}
