# linkgate-setup

The installer for linkgate. It downloads `linkgate.exe` from the latest GitHub release, checks it against the SHA256 digest GitHub publishes for the file, and sets up the parts you tick.

It is a Tauri 2 app in the same pnpm workspace as linkgate, with its own `src-tauri/` crate. Nothing in it is shared with the main app at build time except the WSL files in `../linux/`, which it embeds.

## What it does

| Step | Detail |
| --- | --- |
| Download | Reads `releases/latest`, downloads `linkgate.exe`, verifies its SHA256 against the `digest` in the release answer and checks the `MZ` header, then replaces `%LOCALAPPDATA%\Programs\linkgate\linkgate.exe` through a `.new` file. If the exe is in use, it moves the old one aside instead. |
| Installer copy | Copies itself to `linkgate-setup.exe` next to the app, so the app's update button can run it. When it runs from that copy, it fetches a newer `linkgate-setup.exe` from the release and verifies it. |
| Windows Settings | Registers an uninstall entry under `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\linkgate`. |
| Start menu | Always adds `linkgate.lnk`, which opens linkgate's settings. |
| Desktop | Optional shortcut to the same target. |
| WSL | Optional, per distribution from `wsl.exe -l -q`. Writes `~/.local/bin/linkgate-open` and `~/.local/share/applications/linkgate.desktop` and runs `xdg-mime default`, as `scripts/install.sh` does. The exe path inside the distribution comes from `wslpath -u` run there. |
| Missing packages | When the window opens, it probes each distribution with `wsl.exe -d <distro> --exec sh -s` for `xdg-mime`, `python3`, `bash`, `awk` and a package manager (`apt-get`, `dnf`, `pacman`, `zypper` or `apk`, in that order). A distribution that lacks `xdg-utils`, `python3`, `bash` or `awk` shows the reason and an "Install it for me" box that is ticked by default, with the exact command. Nothing is installed unless that box is ticked and the distribution is ticked. The install runs as root through `wsl.exe -d <distro> -u root --exec`, non-interactively and without recommended extras. If the first attempt fails, the installer refreshes the package lists once (`apt-get update`, `pacman -Sy`, `zypper refresh` or `apk update`) and retries. `pacman -Sy` refreshes the lists without upgrading the system, which Arch Linux documents as risky when run alone, so it only runs after a failure. A failure shows the last lines of the package manager's output, and the WSL setup for that distribution is skipped. |
| `BROWSER` | Optional. Appends `export BROWSER="$HOME/.local/bin/linkgate-open"` to `~/.zshenv` or `~/.profile` once. The `x-www-browser` alternative needs sudo, so the window shows the command and never runs it. |
| VS Code | Optional, for `Code` and `Code - Insiders` when their settings folder exists. Backs up `settings.json` to `settings.json.linkgate-backup` once, then sets `workbench.externalBrowser` and `workbench.browser.openLocalhostLinks`. The file is edited in place, so comments and formatting stay. |
| Blocklists | Starts `linkgate.exe --update-lists` detached. |

What it set up is recorded in `%LOCALAPPDATA%\linkgate\install.json`, including the VS Code values it replaced and, per distribution, the packages it installed. A package that was already there is never recorded. Unticking a part on a later run leaves that part as it is.

## Modes

| Launch | Window |
| --- | --- |
| `linkgate-setup.exe` | Install, or update when an install record exists |
| `linkgate-setup.exe --update` | Update, with the previous choices ticked |
| `linkgate-setup.exe --uninstall` | Uninstall |

Uninstalling removes the program, the shortcuts, the registry entry and the WSL files for each recorded distribution. It keeps the blocklists and linkgate's settings unless you tick the box. For a distribution where the installer added packages, it offers an unticked "Remove" box, and removes only the recorded packages with the package manager's remove command, never a purge. It touches VS Code only when you tick the box, and then restores the values recorded at install. A key that you changed since the install is left alone. Files that are in use, including the running installer, are removed a moment after the window closes.

## Build and test

From the repository root, in WSL:

```bash
pnpm build:setup
pnpm test:setup
```

`build:setup` cross-compiles with `cargo-xwin` to `installer/src-tauri/target/x86_64-pc-windows-msvc/release/linkgate-setup.exe`. `test:setup` builds the Rust tests for Windows and runs them through interop. On a Windows runner, `pnpm --filter linkgate-setup exec tauri build --no-bundle` builds the frontend and the exe at `installer/src-tauri/target/release/linkgate-setup.exe`. Use the Tauri CLI there, because a plain `cargo build` produces an exe that looks for the dev server. `cargo test` in `installer/src-tauri/` runs the tests once `pnpm --filter linkgate-setup build` has created `installer/dist`.

One test runs the generated shell script in a real WSL distribution and is ignored by default:

```bash
LINKGATE_SETUP_TEST_DISTRO=Ubuntu WSLENV=LINKGATE_SETUP_TEST_DISTRO bash installer/scripts/test-windows.sh --ignored
```

It runs with `HOME` pointed at a scratch folder, so it doesn't touch the distribution's real files.

Two more ignored tests cover the package offer. The first only reads: it prints what the probe finds in a real distribution.

```bash
LINKGATE_SETUP_TEST_DISTRO=Ubuntu WSLENV=LINKGATE_SETUP_TEST_DISTRO bash installer/scripts/test-windows.sh --ignored the_probe_reads_a_real_distro --nocapture
```

The second runs the install and removal flows in a distribution that has no `xdg-utils`. It puts a fake `apt-get` that only logs its arguments in a temporary folder inside the distribution and puts that folder first on the command's `PATH`, so no real package is installed or removed. It then checks the logged commands and the record in `install.json`, and deletes the folder:

```bash
LINKGATE_SETUP_TEST_DISTRO=<distro without xdg-utils> WSLENV=LINKGATE_SETUP_TEST_DISTRO bash installer/scripts/test-windows.sh --ignored the_package_flow_runs_through_a_fake_package_manager_in_a_real_distro
```

## Releases

`linkgate-setup.exe` is published as a release asset next to `linkgate.exe`. A `vX.Y.Z` tag builds both in `.github/workflows/release.yml`. A push to `main` that changes `installer/` or `../linux/` rebuilds the installer and replaces it on the latest release in `.github/workflows/installer.yml`. [docs/installer-design.md](../docs/installer-design.md) explains why.

## Testing hooks

| Variable | Effect |
| --- | --- |
| `LINKGATE_SETUP_SOURCE` | A folder holding `linkgate.exe` and optionally `linkgate-setup.exe` and `VERSION`. The installer reads them instead of GitHub. A `digests.json` in the folder maps each file name to its `sha256:<hex>` digest, as GitHub's release answer does. Without it, a `SHA256SUMS` file in `sha256sum` format is read instead, like the fallback for older releases. |
| `LINKGATE_SETUP_SANDBOX` | A folder that replaces the install directory, data and settings folders, Start menu, desktop, VS Code settings, and the registry key. WSL steps run with `HOME` set to a scratch folder inside the distribution. Package installs and removals are refused in this mode, because they change the real distribution. |
| `LINKGATE_SETUP_PACKAGE_SHIM` | Only with the sandbox. An absolute path inside the distribution, made of letters, digits and `/_.-`. Package commands then run as your normal user with that folder first on `PATH`, so a fake package manager there records its arguments instead of installing anything. |

With the first two set, `linkgate-setup.exe` runs the whole install without touching the real setup.

`linkgate-setup.exe --headless <job.json>` runs a job without a window and is meant for tests. The job is JSON:

```json
{
  "mode": "install",
  "install": { "desktopShortcut": true, "wslDistros": [], "browserEnv": "off", "vscode": ["code"] },
  "resultPath": "C:\\Temp\\result.json"
}
```

`mode` is `install` or `uninstall`. An install job takes `"installPackages": ["<distro>"]` to consent to installing missing packages in a ticked distribution. An uninstall job takes `"uninstall": { "restoreVscode": false, "removeData": false, "removePackages": [] }`, where `removePackages` lists the distributions to remove recorded packages from. The result file holds `ok`, an `error` for a fatal failure, and the per-step `summary`. The exit code is 0 when every step worked, 1 when a step failed, and 2 when the job couldn't be read.

## Layout

```
src-tauri/src/
├── main.rs        # modes and window start-up
├── commands.rs    # what the window calls
├── install.rs     # the install and update steps
├── uninstall.rs   # the uninstall steps
├── source.rs      # GitHub release or local folder
├── checksums.rs   # digest parsing, SHA256SUMS parsing and verification
├── fsutil.rs      # atomic replace and in-use handling
├── wsl.rs         # shell script generation and running
├── distros.rs     # wsl.exe -l -q decoding
├── packages.rs    # distribution probe, package manager commands, install and removal
├── jsonc.rs       # comment-preserving settings.json edits
├── shortcut.rs    # .lnk files
├── registry.rs    # the uninstall entry
├── state.rs       # install.json
├── paths.rs       # real and sandboxed locations
├── progress.rs    # step events
└── headless.rs    # the test-only job runner
src/               # React window: options, progress, done, uninstall
```
