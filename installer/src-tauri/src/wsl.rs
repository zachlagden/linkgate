use serde::{Deserialize, Serialize};

const OPEN_TEMPLATE: &str = include_str!("../../../linux/linkgate-open");
const DESKTOP_TEMPLATE: &str = include_str!("../../../linux/linkgate.desktop");
const MIME_TYPES: [&str; 9] = [
    "x-scheme-handler/http",
    "x-scheme-handler/https",
    "text/html",
    "application/pdf",
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/svg+xml",
];

pub const BROWSER_LINE: &str = r#"export BROWSER="$HOME/.local/bin/linkgate-open""#;
pub const ALTERNATIVES_COMMAND: &str =
    r#"sudo update-alternatives --install /usr/bin/x-www-browser x-www-browser "$HOME/.local/bin/linkgate-open" 500"#;

pub const EXIT_NO_XDG: i32 = 3;
pub const EXIT_NO_PYTHON: i32 = 4;
pub const EXIT_NO_WSLPATH: i32 = 5;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum BrowserEnv {
    #[default]
    Off,
    Zshenv,
    Profile,
}

impl BrowserEnv {
    pub fn file_name(self) -> Option<&'static str> {
        match self {
            BrowserEnv::Off => None,
            BrowserEnv::Zshenv => Some(".zshenv"),
            BrowserEnv::Profile => Some(".profile"),
        }
    }
}

fn single_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

fn with_newline(text: &str) -> String {
    let unix = text.replace("\r\n", "\n");
    if unix.ends_with('\n') {
        unix
    } else {
        format!("{unix}\n")
    }
}

pub fn install_script(windows_exe: &str, env: BrowserEnv) -> String {
    let mut script = String::new();
    script.push_str(
        r#"set -eu
command -v xdg-mime >/dev/null 2>&1 || { echo "xdg-utils is not installed" >&2; exit @NO_XDG@; }
command -v python3 >/dev/null 2>&1 || { echo "python3 is not installed" >&2; exit @NO_PYTHON@; }
exe=$(wslpath -u @EXE@) || exit @NO_WSLPATH@
[ -n "$exe" ] || exit @NO_WSLPATH@
mkdir -p "$HOME"
bin_dir="$HOME/.local/bin"
apps_dir="$HOME/.local/share/applications"
config_dir="${XDG_CONFIG_HOME:-$HOME/.config}"
mkdir -p "$bin_dir" "$apps_dir" "$config_dir"
escape() { printf '%s' "$1" | sed -e 's/[\\|&]/\\&/g'; }
open_path="$bin_dir/linkgate-open"
desktop_path="$apps_dir/linkgate.desktop"
cat > "$open_path.new" <<'LINKGATE_OPEN_EOF'
"#,
    );
    script.push_str(&with_newline(OPEN_TEMPLATE));
    script.push_str(
        r#"LINKGATE_OPEN_EOF
sed "s|__LINKGATE_EXE__|$(escape "$exe")|" "$open_path.new" > "$open_path"
rm -f "$open_path.new"
chmod +x "$open_path"
cat > "$desktop_path.new" <<'LINKGATE_DESKTOP_EOF'
"#,
    );
    script.push_str(&with_newline(DESKTOP_TEMPLATE));
    script.push_str(
        r#"LINKGATE_DESKTOP_EOF
sed "s|__LINKGATE_OPEN__|$(escape "$open_path")|" "$desktop_path.new" > "$desktop_path"
rm -f "$desktop_path.new"
xdg-mime default linkgate.desktop @MIME@
"#,
    );
    if let Some(file) = env.file_name() {
        script.push_str(&format!(
            "env_file=\"$HOME/{file}\"\nline={}\ntouch \"$env_file\"\ngrep -qxF \"$line\" \"$env_file\" || printf '%s\\n' \"$line\" >> \"$env_file\"\n",
            single_quoted(BROWSER_LINE)
        ));
    }
    script
        .replace("@NO_XDG@", &EXIT_NO_XDG.to_string())
        .replace("@NO_PYTHON@", &EXIT_NO_PYTHON.to_string())
        .replace("@NO_WSLPATH@", &EXIT_NO_WSLPATH.to_string())
        .replace("@EXE@", &single_quoted(windows_exe))
        .replace("@MIME@", &MIME_TYPES.join(" "))
}

pub fn uninstall_script(env_file: Option<&str>) -> String {
    let mut script = String::from(
        r#"set -eu
rm -f "$HOME/.local/bin/linkgate-open" "$HOME/.local/share/applications/linkgate.desktop"
mime_file="${XDG_CONFIG_HOME:-$HOME/.config}/mimeapps.list"
if [ -f "$mime_file" ]; then sed -i '/=linkgate\.desktop$/d' "$mime_file"; fi
"#,
    );
    if let Some(file) = env_file {
        script.push_str(&format!(
            "if [ -f \"$HOME/{file}\" ]; then sed -i -e '/^export BROWSER=\"\\$HOME\\/\\.local\\/bin\\/linkgate-open\"$/d' \"$HOME/{file}\"; fi\n"
        ));
    }
    script
}

pub fn explain_exit(code: Option<i32>, stderr: &str, distro: &str) -> String {
    match code {
        Some(EXIT_NO_XDG) => format!(
            "{distro} has no xdg-utils. Tick the option to install it, or install xdg-utils in {distro} yourself, then try again."
        ),
        Some(EXIT_NO_PYTHON) => format!("{distro} has no python3, which linkgate-open needs. Install it, then try again."),
        Some(EXIT_NO_WSLPATH) => format!("{distro} couldn't convert the Windows path to a Linux path with wslpath."),
        _ => {
            let detail = stderr.trim();
            if detail.is_empty() {
                format!("Setting up {distro} failed with exit code {}.", code.map_or("unknown".to_string(), |c| c.to_string()))
            } else {
                format!("Setting up {distro} failed: {detail}")
            }
        }
    }
}

#[cfg(windows)]
pub fn run(distro: &str, script: &str, home: Option<&str>) -> Result<String, String> {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    let mut command = Command::new("wsl.exe");
    command.args(["-d", distro, "--exec"]);
    if let Some(home) = home {
        command.arg("env").arg(format!("HOME={home}"));
    }
    command
        .args(["bash", "-s"])
        .creation_flags(0x0800_0000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|e| format!("Couldn't start wsl.exe: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("Couldn't open a pipe to wsl.exe.")?;
    let payload = script.to_string();
    let writer = std::thread::spawn(move || {
        let result = stdin.write_all(payload.as_bytes());
        drop(stdin);
        result
    });
    let output = child.wait_with_output().map_err(|e| format!("wsl.exe failed: {e}"))?;
    let _ = writer.join();
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let stderr = crate::distros::decode(&output.stderr);
    Err(explain_exit(output.status.code(), &stderr, distro))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = r"C:\Users\Alex\AppData\Local\Programs\linkgate\linkgate.exe";

    #[test]
    fn install_script_resolves_the_exe_path_inside_the_distro() {
        let script = install_script(EXE, BrowserEnv::Off);
        assert!(script.contains(&format!("wslpath -u '{EXE}'")));
        assert!(!script.contains("/mnt/c"));
        assert!(!script.contains("@EXE@") && !script.contains("@MIME@") && !script.contains("@NO_"));
    }

    #[test]
    fn install_script_embeds_both_templates_and_every_mime_type() {
        let script = install_script(EXE, BrowserEnv::Off);
        assert!(script.contains("__LINKGATE_EXE__"));
        assert!(script.contains("__LINKGATE_OPEN__"));
        assert!(script.contains("<<'LINKGATE_OPEN_EOF'\n#!/usr/bin/env bash"));
        for mime in MIME_TYPES {
            assert!(script.contains(mime), "{mime}");
        }
        assert_eq!(script.matches("\nLINKGATE_OPEN_EOF\n").count(), 1);
        assert_eq!(script.matches("\nLINKGATE_DESKTOP_EOF\n").count(), 1);
    }

    #[test]
    fn embedded_templates_never_carry_carriage_returns() {
        assert_eq!(with_newline("a\r\nb\r\n"), "a\nb\n");
        assert_eq!(with_newline("a\r\nb"), "a\nb\n");
        assert!(!install_script(EXE, BrowserEnv::Zshenv).contains('\r'));
    }

    #[test]
    fn install_script_quotes_awkward_paths() {
        let script = install_script(r"C:\Users\O'Neil & Sons\linkgate.exe", BrowserEnv::Off);
        assert!(script.contains(r"wslpath -u 'C:\Users\O'\''Neil & Sons\linkgate.exe'"));
    }

    #[test]
    fn browser_env_is_appended_only_when_chosen() {
        assert!(!install_script(EXE, BrowserEnv::Off).contains("env_file"));
        let zsh = install_script(EXE, BrowserEnv::Zshenv);
        assert!(zsh.contains("env_file=\"$HOME/.zshenv\""));
        assert!(zsh.contains("grep -qxF"));
        assert!(install_script(EXE, BrowserEnv::Profile).contains("$HOME/.profile"));
    }

    #[test]
    fn install_script_never_asks_for_sudo() {
        let script = install_script(EXE, BrowserEnv::Zshenv);
        assert!(!script.contains("sudo"));
        assert!(!script.contains("update-alternatives"));
    }

    #[test]
    fn uninstall_script_removes_files_and_only_our_env_line() {
        let script = uninstall_script(Some(".zshenv"));
        assert!(script.contains("rm -f \"$HOME/.local/bin/linkgate-open\""));
        assert!(script.contains("=linkgate\\.desktop$/d"));
        assert!(script.contains("$HOME/.zshenv"));
        assert!(!uninstall_script(None).contains(".zshenv"));
    }

    #[test]
    fn exit_codes_have_plain_messages() {
        assert!(explain_exit(Some(EXIT_NO_XDG), "", "Ubuntu").contains("xdg-utils"));
        assert!(explain_exit(Some(EXIT_NO_PYTHON), "", "Ubuntu").contains("python3"));
        assert!(explain_exit(Some(1), "boom\n", "Ubuntu").contains("boom"));
        assert!(explain_exit(None, "", "Ubuntu").contains("unknown"));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "runs against a real WSL distro, set LINKGATE_SETUP_TEST_DISTRO"]
    fn script_installs_and_removes_under_a_scratch_home() {
        let distro = std::env::var("LINKGATE_SETUP_TEST_DISTRO").expect("LINKGATE_SETUP_TEST_DISTRO");
        let home = format!("/tmp/linkgate-setup-test-{}", std::process::id());
        run(&distro, &install_script(EXE, BrowserEnv::Zshenv), Some(&home)).unwrap();
        run(&distro, &install_script(EXE, BrowserEnv::Zshenv), Some(&home)).unwrap();
        let check = format!(
            "set -eu; test -x {home}/.local/bin/linkgate-open; grep -c linkgate-open {home}/.zshenv; grep -q '^exe=\"/mnt/c/' {home}/.local/bin/linkgate-open; grep -q 'Exec={home}/.local/bin/linkgate-open %u' {home}/.local/share/applications/linkgate.desktop; xdg-mime query default application/pdf; xdg-mime query default x-scheme-handler/https"
        );
        let output = run(&distro, &check, Some(&home)).unwrap();
        assert_eq!(output.lines().collect::<Vec<_>>(), vec!["1", "linkgate.desktop", "linkgate.desktop"]);
        run(&distro, &uninstall_script(Some(".zshenv")), Some(&home)).unwrap();
        let gone = format!("set -eu; test ! -e {home}/.local/bin/linkgate-open; test ! -e {home}/.local/share/applications/linkgate.desktop; ! grep -q linkgate {home}/.zshenv; rm -rf {home}");
        run(&distro, &gone, Some(&home)).unwrap();
    }
}
