use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::state::PackageRecord;

pub const XDG_PACKAGE: &str = "xdg-utils";
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const UPDATE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const FAILURE_TAIL_LINES: usize = 4;

pub const PROBE_SCRIPT: &str = "\
if command -v xdg-mime >/dev/null 2>&1; then echo xdg=1; else echo xdg=0; fi
if command -v python3 >/dev/null 2>&1; then echo python=1; else echo python=0; fi
for manager in apt-get dnf pacman zypper apk; do
  if command -v \"$manager\" >/dev/null 2>&1; then echo \"manager=$manager\"; break; fi
done
";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Manager {
    Apt,
    Dnf,
    Pacman,
    Zypper,
    Apk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Install,
    Remove,
}

impl Manager {
    pub fn id(self) -> &'static str {
        match self {
            Manager::Apt => "apt-get",
            Manager::Dnf => "dnf",
            Manager::Pacman => "pacman",
            Manager::Zypper => "zypper",
            Manager::Apk => "apk",
        }
    }

    pub fn from_id(id: &str) -> Option<Manager> {
        [Manager::Apt, Manager::Dnf, Manager::Pacman, Manager::Zypper, Manager::Apk]
            .into_iter()
            .find(|manager| manager.id() == id)
    }

    fn python_package(self) -> &'static str {
        match self {
            Manager::Pacman => "python",
            _ => "python3",
        }
    }

    fn words(self, action: Action, packages: &[String]) -> Vec<String> {
        let head: &[&str] = match (self, action) {
            (Manager::Apt, Action::Install) => &["apt-get", "install", "-y", "--no-install-recommends"],
            (Manager::Apt, Action::Remove) => &["apt-get", "remove", "-y"],
            (Manager::Dnf, Action::Install) => &["dnf", "install", "-y", "--setopt=install_weak_deps=False"],
            (Manager::Dnf, Action::Remove) => &["dnf", "remove", "-y"],
            (Manager::Pacman, Action::Install) => &["pacman", "-S", "--noconfirm", "--needed"],
            (Manager::Pacman, Action::Remove) => &["pacman", "-R", "--noconfirm"],
            (Manager::Zypper, Action::Install) => &["zypper", "--non-interactive", "install", "--no-recommends"],
            (Manager::Zypper, Action::Remove) => &["zypper", "--non-interactive", "remove"],
            (Manager::Apk, Action::Install) => &["apk", "add", "--no-cache"],
            (Manager::Apk, Action::Remove) => &["apk", "del"],
        };
        head.iter().map(|word| word.to_string()).chain(packages.iter().cloned()).collect()
    }

    fn argv(self, words: Vec<String>) -> Vec<String> {
        match self {
            Manager::Apt => ["env", "DEBIAN_FRONTEND=noninteractive"]
                .iter()
                .map(|word| word.to_string())
                .chain(words)
                .collect(),
            _ => words,
        }
    }

    pub fn install_words(self, packages: &[String]) -> Vec<String> {
        self.words(Action::Install, packages)
    }

    pub fn remove_words(self, packages: &[String]) -> Vec<String> {
        self.words(Action::Remove, packages)
    }

    fn update_argv(self) -> Option<Vec<String>> {
        match self {
            Manager::Apt => Some(self.argv(vec!["apt-get".into(), "update".into(), "-q".into()])),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Probe {
    pub xdg: bool,
    pub python: bool,
    pub manager: Option<Manager>,
}

pub fn parse_probe(output: &str) -> Option<Probe> {
    let mut probe = Probe::default();
    let mut saw_xdg = false;
    for line in output.lines().map(str::trim) {
        match line.split_once('=') {
            Some(("xdg", value)) => {
                saw_xdg = true;
                probe.xdg = value == "1";
            }
            Some(("python", value)) => probe.python = value == "1",
            Some(("manager", value)) => probe.manager = Manager::from_id(value),
            _ => {}
        }
    }
    saw_xdg.then_some(probe)
}

pub fn missing_names(probe: &Probe) -> Vec<&'static str> {
    let mut names = Vec::new();
    if !probe.xdg {
        names.push(XDG_PACKAGE);
    }
    if !probe.python {
        names.push("python3");
    }
    names
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub manager: Manager,
    pub packages: Vec<String>,
}

pub fn plan(probe: &Probe) -> Option<Plan> {
    let manager = probe.manager?;
    let mut packages = Vec::new();
    if !probe.xdg {
        packages.push(XDG_PACKAGE.to_string());
    }
    if !probe.python {
        packages.push(manager.python_package().to_string());
    }
    (!packages.is_empty()).then_some(Plan { manager, packages })
}

pub fn describe_missing(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [XDG_PACKAGE] => "xdg-utils isn't installed, so links opened there can't reach linkgate.".into(),
        ["python3"] => "python3 isn't installed, and linkgate-open needs it.".into(),
        _ => format!("{} aren't installed, so linkgate can't work there.", names.join(" and ")),
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub label: String,
    pub command: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Readiness {
    pub issue: Option<String>,
    pub offer: Option<Offer>,
    pub manual: Option<String>,
    pub probe_error: Option<String>,
}

pub fn readiness(result: Result<Probe, String>) -> Readiness {
    let probe = match result {
        Ok(probe) => probe,
        Err(error) => {
            return Readiness {
                probe_error: Some(error),
                ..Readiness::default()
            }
        }
    };
    let missing = missing_names(&probe);
    if missing.is_empty() {
        return Readiness::default();
    }
    let issue = Some(describe_missing(&missing));
    match plan(&probe) {
        Some(plan) => Readiness {
            issue,
            offer: Some(Offer {
                label: format!("Install {} for me", plan.packages.join(" and ")),
                command: format!("{} (runs as root)", plan.manager.install_words(&plan.packages).join(" ")),
            }),
            ..Readiness::default()
        },
        None => Readiness {
            issue,
            manual: Some(
                "No supported package manager was found there. Install them yourself, then run this again.".into(),
            ),
            ..Readiness::default()
        },
    }
}

pub fn failure_tail(output: &str) -> String {
    let lines: Vec<&str> = output.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
    let start = lines.len().saturating_sub(FAILURE_TAIL_LINES);
    lines[start..].join(" ")
}

fn explain_failure(what: &str, distro: &str, output: &str) -> String {
    let tail = failure_tail(output);
    if tail.is_empty() {
        format!("Couldn't {what} in {distro}.")
    } else {
        format!("Couldn't {what} in {distro}: {tail}")
    }
}

pub trait Runner {
    fn probe(&self, distro: &str) -> Result<Probe, String>;
    fn run_root(&self, distro: &str, argv: &[String], timeout: Duration) -> Result<String, String>;
}

#[derive(Debug, PartialEq)]
pub enum InstallOutcome {
    AlreadyPresent,
    Installed(PackageRecord),
}

pub fn ensure(runner: &dyn Runner, distro: &str) -> Result<InstallOutcome, String> {
    let probe = runner.probe(distro).map_err(|e| format!("Couldn't check {distro}: {e}"))?;
    let missing = missing_names(&probe);
    if missing.is_empty() {
        return Ok(InstallOutcome::AlreadyPresent);
    }
    let Some(plan) = plan(&probe) else {
        return Err(format!(
            "{distro} has no package manager linkgate knows, so install {} there yourself.",
            missing.join(" and ")
        ));
    };
    let install = plan.manager.argv(plan.manager.install_words(&plan.packages));
    let first = runner.run_root(distro, &install, INSTALL_TIMEOUT);
    if let Err(first_error) = first {
        let Some(update) = plan.manager.update_argv() else {
            return Err(explain_failure("install the packages", distro, &first_error));
        };
        runner
            .run_root(distro, &update, UPDATE_TIMEOUT)
            .map_err(|e| explain_failure("refresh the package lists", distro, &e))?;
        runner
            .run_root(distro, &install, INSTALL_TIMEOUT)
            .map_err(|e| explain_failure("install the packages", distro, &e))?;
    }
    Ok(InstallOutcome::Installed(PackageRecord {
        manager: plan.manager.id().to_string(),
        packages: plan.packages,
    }))
}

pub fn remove(runner: &dyn Runner, distro: &str, record: &PackageRecord) -> Result<(), String> {
    let manager = Manager::from_id(&record.manager)
        .ok_or_else(|| format!("The recorded package manager {:?} isn't one linkgate knows.", record.manager))?;
    let argv = manager.argv(manager.remove_words(&record.packages));
    runner
        .run_root(distro, &argv, INSTALL_TIMEOUT)
        .map(|_| ())
        .map_err(|e| explain_failure("remove the packages", distro, &e))
}

pub fn merge_records(existing: Option<&PackageRecord>, added: Option<PackageRecord>) -> Option<PackageRecord> {
    match (existing, added) {
        (Some(old), Some(new)) if old.manager == new.manager => {
            let mut packages = old.packages.clone();
            for package in new.packages {
                if !packages.contains(&package) {
                    packages.push(package);
                }
            }
            Some(PackageRecord {
                manager: new.manager,
                packages,
            })
        }
        (_, Some(new)) => Some(new),
        (Some(old), None) => Some(old.clone()),
        (None, None) => None,
    }
}

pub fn valid_shim(path: &str) -> bool {
    !path.is_empty() && path.starts_with('/') && path.chars().all(|c| c.is_ascii_alphanumeric() || "/_.-".contains(c))
}

pub struct WslRunner {
    pub shim: Option<String>,
}

fn capture(mut command: Command, stdin: Option<&str>, timeout: Duration) -> Result<(Option<i32>, String), String> {
    use std::io::{Read, Write};
    use std::os::windows::process::CommandExt;

    command
        .creation_flags(0x0800_0000)
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| format!("Couldn't start wsl.exe: {e}"))?;
    let writer = stdin.and_then(|text| {
        let mut pipe = child.stdin.take()?;
        let payload = text.to_string();
        Some(std::thread::spawn(move || {
            let _ = pipe.write_all(payload.as_bytes());
        }))
    });
    let mut stdout = child.stdout.take().ok_or("Couldn't read from wsl.exe.")?;
    let mut stderr = child.stderr.take().ok_or("Couldn't read from wsl.exe.")?;
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });
    let deadline = Instant::now() + timeout;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("It didn't finish within {} minutes.", timeout.as_secs().div_ceil(60)));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(error) => return Err(format!("wsl.exe failed: {error}")),
        }
    };
    if let Some(writer) = writer {
        let _ = writer.join();
    }
    let out = out.join().unwrap_or_default();
    let err = err.join().unwrap_or_default();
    let text = format!("{}\n{}", crate::distros::decode(&out), crate::distros::decode(&err));
    Ok((code, text))
}

impl Runner for WslRunner {
    fn probe(&self, distro: &str) -> Result<Probe, String> {
        let mut command = Command::new("wsl.exe");
        command.args(["-d", distro, "--exec", "sh", "-s"]);
        let (code, output) = capture(command, Some(PROBE_SCRIPT), PROBE_TIMEOUT)?;
        parse_probe(&output).ok_or_else(|| {
            let detail = failure_tail(&output);
            if detail.is_empty() {
                format!("wsl.exe exited with code {}.", code.map_or("unknown".to_string(), |c| c.to_string()))
            } else {
                detail
            }
        })
    }

    fn run_root(&self, distro: &str, argv: &[String], timeout: Duration) -> Result<String, String> {
        let mut command = Command::new("wsl.exe");
        command.args(["-d", distro]);
        match &self.shim {
            Some(shim) => {
                if !valid_shim(shim) {
                    return Err(format!("{shim:?} isn't a usable folder for the test package shim."));
                }
                command
                    .arg("--exec")
                    .arg("env")
                    .arg(format!("PATH={shim}:/usr/local/bin:/usr/bin:/bin"));
            }
            None => {
                command.args(["-u", "root", "--exec"]);
            }
        }
        command.args(argv);
        let (code, output) = capture(command, None, timeout)?;
        if code == Some(0) {
            Ok(output)
        } else {
            Err(if output.trim().is_empty() {
                format!("The command exited with code {}.", code.map_or("unknown".to_string(), |c| c.to_string()))
            } else {
                output
            })
        }
    }
}

pub fn probe_all(names: &[String]) -> Vec<(String, Readiness)> {
    let runner = WslRunner { shim: None };
    std::thread::scope(|scope| {
        let handles: Vec<_> = names
            .iter()
            .map(|name| {
                let runner = &runner;
                scope.spawn(move || {
                    let result = if crate::install::valid_distro(name) {
                        runner.probe(name)
                    } else {
                        Err(format!("{name:?} isn't a usable WSL distribution name."))
                    };
                    (name.clone(), readiness(result))
                })
            })
            .collect();
        handles
            .into_iter()
            .zip(names)
            .map(|(handle, name)| {
                handle
                    .join()
                    .unwrap_or_else(|_| (name.clone(), readiness(Err("The check stopped unexpectedly.".into()))))
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Fake {
        probe: Result<Probe, String>,
        results: RefCell<Vec<Result<String, String>>>,
        calls: RefCell<Vec<Vec<String>>>,
    }

    impl Fake {
        fn new(probe: Result<Probe, String>, results: Vec<Result<String, String>>) -> Fake {
            Fake {
                probe,
                results: RefCell::new(results),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn lines(&self) -> Vec<String> {
            self.calls.borrow().iter().map(|call| call.join(" ")).collect()
        }
    }

    impl Runner for Fake {
        fn probe(&self, _: &str) -> Result<Probe, String> {
            self.probe.clone()
        }

        fn run_root(&self, _: &str, argv: &[String], _: Duration) -> Result<String, String> {
            self.calls.borrow_mut().push(argv.to_vec());
            self.results.borrow_mut().remove(0)
        }
    }

    fn missing_xdg() -> Probe {
        Probe {
            xdg: false,
            python: true,
            manager: Some(Manager::Apt),
        }
    }

    #[test]
    fn parses_probe_output_from_real_distros() {
        let ubuntu = parse_probe("xdg=1\npython=1\nmanager=apt-get\n").unwrap();
        assert_eq!(
            ubuntu,
            Probe {
                xdg: true,
                python: true,
                manager: Some(Manager::Apt)
            }
        );
        let bare = parse_probe("xdg=0\r\npython=1\r\nmanager=apt-get\r\n").unwrap();
        assert!(!bare.xdg && bare.python && bare.manager == Some(Manager::Apt));
        let none = parse_probe("xdg=0\npython=0\n").unwrap();
        assert_eq!(none.manager, None);
        assert_eq!(parse_probe("manager=unknown-pm\nxdg=1\n").unwrap().manager, None);
    }

    #[test]
    fn output_without_the_marker_is_not_a_probe() {
        assert_eq!(parse_probe(""), None);
        assert_eq!(parse_probe("WSL: distro not found\n"), None);
    }

    #[test]
    fn packages_needed_follow_what_is_missing() {
        assert_eq!(plan(&missing_xdg()).unwrap().packages, vec!["xdg-utils"]);
        let both = Probe {
            xdg: false,
            python: false,
            manager: Some(Manager::Pacman),
        };
        assert_eq!(plan(&both).unwrap().packages, vec!["xdg-utils", "python"]);
        let apt_both = Probe {
            manager: Some(Manager::Apt),
            ..both.clone()
        };
        assert_eq!(plan(&apt_both).unwrap().packages, vec!["xdg-utils", "python3"]);
        assert_eq!(
            plan(&Probe {
                xdg: true,
                python: true,
                manager: Some(Manager::Apt)
            }),
            None
        );
        assert_eq!(plan(&Probe { manager: None, ..both }), None);
    }

    #[test]
    fn install_command_lines_are_quiet_and_minimal_for_each_manager() {
        let one = vec!["xdg-utils".to_string()];
        assert_eq!(
            Manager::Apt.install_words(&one).join(" "),
            "apt-get install -y --no-install-recommends xdg-utils"
        );
        assert_eq!(
            Manager::Dnf.install_words(&one).join(" "),
            "dnf install -y --setopt=install_weak_deps=False xdg-utils"
        );
        assert_eq!(Manager::Pacman.install_words(&one).join(" "), "pacman -S --noconfirm --needed xdg-utils");
        assert_eq!(
            Manager::Zypper.install_words(&one).join(" "),
            "zypper --non-interactive install --no-recommends xdg-utils"
        );
        assert_eq!(Manager::Apk.install_words(&one).join(" "), "apk add --no-cache xdg-utils");
    }

    #[test]
    fn remove_command_lines_name_only_the_recorded_packages() {
        let pkgs = vec!["xdg-utils".to_string()];
        assert_eq!(Manager::Apt.remove_words(&pkgs).join(" "), "apt-get remove -y xdg-utils");
        assert_eq!(Manager::Dnf.remove_words(&pkgs).join(" "), "dnf remove -y xdg-utils");
        assert_eq!(Manager::Pacman.remove_words(&pkgs).join(" "), "pacman -R --noconfirm xdg-utils");
        assert_eq!(Manager::Zypper.remove_words(&pkgs).join(" "), "zypper --non-interactive remove xdg-utils");
        assert_eq!(Manager::Apk.remove_words(&pkgs).join(" "), "apk del xdg-utils");
        assert!(!Manager::Apt.remove_words(&pkgs).iter().any(|word| word.contains("purge")));
    }

    #[test]
    fn apt_runs_non_interactively_and_others_run_as_they_are() {
        let argv = Manager::Apt.argv(Manager::Apt.install_words(&["xdg-utils".to_string()]));
        assert_eq!(&argv[..3], ["env", "DEBIAN_FRONTEND=noninteractive", "apt-get"]);
        assert_eq!(Manager::Dnf.argv(vec!["dnf".into()]), vec!["dnf"]);
        assert!(Manager::Apt.update_argv().is_some());
        assert!(Manager::Dnf.update_argv().is_none());
    }

    #[test]
    fn manager_ids_round_trip() {
        for manager in [Manager::Apt, Manager::Dnf, Manager::Pacman, Manager::Zypper, Manager::Apk] {
            assert_eq!(Manager::from_id(manager.id()), Some(manager));
        }
        assert_eq!(Manager::from_id("brew"), None);
    }

    #[test]
    fn readiness_offers_an_install_when_a_manager_exists() {
        let ready = readiness(Ok(missing_xdg()));
        assert!(ready.issue.unwrap().contains("xdg-utils isn't installed"));
        let offer = ready.offer.unwrap();
        assert_eq!(offer.label, "Install xdg-utils for me");
        assert!(offer.command.starts_with("apt-get install -y --no-install-recommends xdg-utils"));
        assert!(offer.command.ends_with("(runs as root)"));
        assert!(ready.manual.is_none() && ready.probe_error.is_none());
    }

    #[test]
    fn readiness_is_empty_when_nothing_is_missing() {
        let probe = Probe {
            xdg: true,
            python: true,
            manager: Some(Manager::Apt),
        };
        assert_eq!(readiness(Ok(probe)), Readiness::default());
    }

    #[test]
    fn readiness_without_a_manager_gives_manual_advice_and_no_offer() {
        let ready = readiness(Ok(Probe {
            xdg: false,
            python: true,
            manager: None,
        }));
        assert!(ready.offer.is_none());
        assert!(ready.manual.unwrap().contains("package manager"));
        assert!(ready.issue.is_some());
    }

    #[test]
    fn readiness_reports_a_failed_probe_without_an_offer() {
        let ready = readiness(Err("distro won't start".into()));
        assert_eq!(ready.probe_error.as_deref(), Some("distro won't start"));
        assert!(ready.offer.is_none() && ready.issue.is_none());
    }

    #[test]
    fn missing_packages_are_described_in_plain_words() {
        assert_eq!(
            describe_missing(&["xdg-utils"]),
            "xdg-utils isn't installed, so links opened there can't reach linkgate."
        );
        assert!(describe_missing(&["python3"]).contains("linkgate-open needs it"));
        assert!(describe_missing(&["xdg-utils", "python3"]).starts_with("xdg-utils and python3 aren't installed"));
        assert_eq!(describe_missing(&[]), "");
    }

    #[test]
    fn ensure_installs_what_is_missing_and_records_it() {
        let fake = Fake::new(Ok(missing_xdg()), vec![Ok(String::new())]);
        let outcome = ensure(&fake, "Debian").unwrap();
        assert_eq!(
            outcome,
            InstallOutcome::Installed(PackageRecord {
                manager: "apt-get".into(),
                packages: vec!["xdg-utils".into()]
            })
        );
        assert_eq!(
            fake.lines(),
            vec!["env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends xdg-utils"]
        );
    }

    #[test]
    fn ensure_does_nothing_when_everything_is_present() {
        let probe = Probe {
            xdg: true,
            python: true,
            manager: Some(Manager::Apt),
        };
        let fake = Fake::new(Ok(probe), vec![]);
        assert_eq!(ensure(&fake, "Ubuntu").unwrap(), InstallOutcome::AlreadyPresent);
        assert!(fake.lines().is_empty());
    }

    #[test]
    fn ensure_refreshes_apt_lists_once_and_retries() {
        let fake = Fake::new(
            Ok(missing_xdg()),
            vec![Err("E: Unable to locate package xdg-utils".into()), Ok(String::new()), Ok(String::new())],
        );
        assert!(matches!(ensure(&fake, "Debian").unwrap(), InstallOutcome::Installed(_)));
        let lines = fake.lines();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("apt-get install"));
        assert!(lines[1].contains("apt-get update"));
        assert!(lines[2].contains("apt-get install"));
    }

    #[test]
    fn ensure_reports_the_last_lines_when_the_retry_fails_too() {
        let fake = Fake::new(
            Ok(missing_xdg()),
            vec![
                Err("first".into()),
                Ok(String::new()),
                Err("Reading package lists...\nE: Unable to locate package xdg-utils\n".into()),
            ],
        );
        let error = ensure(&fake, "Debian").unwrap_err();
        assert!(error.contains("Couldn't install the packages in Debian"));
        assert!(error.contains("Unable to locate package xdg-utils"));
    }

    #[test]
    fn ensure_reports_a_failed_list_refresh_without_retrying() {
        let fake = Fake::new(Ok(missing_xdg()), vec![Err("x".into()), Err("Temporary failure resolving".into())]);
        let error = ensure(&fake, "Debian").unwrap_err();
        assert!(error.contains("refresh the package lists"));
        assert!(error.contains("Temporary failure resolving"));
        assert_eq!(fake.lines().len(), 2);
    }

    #[test]
    fn ensure_does_not_retry_for_managers_without_a_refresh_step() {
        let probe = Probe {
            xdg: false,
            python: true,
            manager: Some(Manager::Dnf),
        };
        let fake = Fake::new(Ok(probe), vec![Err("No match for argument: xdg-utils".into())]);
        let error = ensure(&fake, "Fedora").unwrap_err();
        assert!(error.contains("No match for argument"));
        assert_eq!(fake.lines().len(), 1);
    }

    #[test]
    fn ensure_explains_a_distro_with_no_known_manager() {
        let probe = Probe {
            xdg: false,
            python: true,
            manager: None,
        };
        let fake = Fake::new(Ok(probe), vec![]);
        let error = ensure(&fake, "Weird").unwrap_err();
        assert!(error.contains("no package manager"));
        assert!(fake.lines().is_empty());
    }

    #[test]
    fn ensure_reports_a_distro_that_cannot_be_checked() {
        let fake = Fake::new(Err("won't start".into()), vec![]);
        assert!(ensure(&fake, "Broken").unwrap_err().contains("Couldn't check Broken: won't start"));
    }

    #[test]
    fn remove_runs_only_the_recorded_packages() {
        let fake = Fake::new(Ok(Probe::default()), vec![Ok(String::new())]);
        let record = PackageRecord {
            manager: "apt-get".into(),
            packages: vec!["xdg-utils".into()],
        };
        remove(&fake, "Debian", &record).unwrap();
        assert_eq!(fake.lines(), vec!["env DEBIAN_FRONTEND=noninteractive apt-get remove -y xdg-utils"]);
    }

    #[test]
    fn remove_refuses_an_unknown_recorded_manager() {
        let fake = Fake::new(Ok(Probe::default()), vec![]);
        let record = PackageRecord {
            manager: "brew".into(),
            packages: vec!["x".into()],
        };
        assert!(remove(&fake, "Debian", &record).is_err());
        assert!(fake.lines().is_empty());
    }

    #[test]
    fn failure_tail_keeps_the_last_lines_only() {
        let output = "a\nb\n\nc\nd\ne\nf\n";
        assert_eq!(failure_tail(output), "c d e f");
        assert_eq!(failure_tail("  \n"), "");
    }

    #[test]
    fn records_from_repeated_installs_are_merged_without_duplicates() {
        let old = PackageRecord {
            manager: "apt-get".into(),
            packages: vec!["xdg-utils".into()],
        };
        let added = PackageRecord {
            manager: "apt-get".into(),
            packages: vec!["xdg-utils".into(), "python3".into()],
        };
        let merged = merge_records(Some(&old), Some(added)).unwrap();
        assert_eq!(merged.packages, vec!["xdg-utils", "python3"]);
        assert_eq!(merge_records(Some(&old), None), Some(old.clone()));
        assert_eq!(merge_records(None, None), None);
        let other = PackageRecord {
            manager: "dnf".into(),
            packages: vec!["xdg-utils".into()],
        };
        assert_eq!(merge_records(Some(&old), Some(other.clone())), Some(other));
    }

    #[test]
    fn shim_folders_must_be_plain_absolute_paths() {
        assert!(valid_shim("/tmp/linkgate-shim_1.x"));
        assert!(!valid_shim("tmp/shim"));
        assert!(!valid_shim("/tmp/a b"));
        assert!(!valid_shim("/tmp/$(x)"));
        assert!(!valid_shim(""));
    }

    #[test]
    #[ignore = "runs the read-only probe against real WSL distros, set LINKGATE_SETUP_TEST_DISTRO"]
    fn the_probe_reads_a_real_distro() {
        let distro = std::env::var("LINKGATE_SETUP_TEST_DISTRO").expect("LINKGATE_SETUP_TEST_DISTRO");
        let probe = WslRunner { shim: None }.probe(&distro).unwrap();
        println!("{distro}: {probe:?}");
        assert!(probe.python);
    }
}
