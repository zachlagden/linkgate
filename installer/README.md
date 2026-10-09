# linkgate-setup

The installer for linkgate. It downloads `linkgate.exe` from the latest GitHub release, checks it against the release's `SHA256SUMS`, and sets up the parts you tick.

It is a Tauri 2 app in the same pnpm workspace as linkgate, with its own `src-tauri/` crate. Nothing in it is shared with the main app at build time except the WSL files in `../linux/`, which it embeds.

## What it does

| Step | Detail |
| --- | --- |
| Download | Reads `releases/latest`, downloads `linkgate.exe` and `SHA256SUMS`, verifies the SHA256 and the `MZ` header, then replaces `%LOCALAPPDATA%\Programs\linkgate\linkgate.exe` through a `.new` file. If the exe is in use, it moves the old one aside instead. |
| Installer copy | Copies itself to `linkgate-setup.exe` next to the app, so the app's update button can run it. When it runs from that copy, it fetches a newer `linkgate-setup.exe` from the release and verifies it. |
| Windows Settings | Registers an uninstall entry under `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\linkgate`. |
| Start menu | Always adds `linkgate.lnk`, which opens linkgate's settings. |
| Desktop | Optional shortcut to the same target. |
| WSL | Optional, per distribution from `wsl.exe -l -q`. Writes `~/.local/bin/linkgate-open` and `~/.local/share/applications/linkgate.desktop` and runs `xdg-mime default`, as `scripts/install.sh` does. The exe path inside the distribution comes from `wslpath -u` run there. |
| `BROWSER` | Optional. Appends `export BROWSER="$HOME/.local/bin/linkgate-open"` to `~/.zshenv` or `~/.profile` once. The `x-www-browser` alternative needs sudo, so the window shows the command and never runs it. |
| VS Code | Optional, for `Code` and `Code - Insiders` when their settings folder exists. Backs up `settings.json` to `settings.json.linkgate-backup` once, then sets `workbench.externalBrowser` and `workbench.browser.openLocalhostLinks`. The file is edited in place, so comments and formatting stay. |
| Blocklists | Starts `linkgate.exe --update-lists` detached. |

What it set up is recorded in `%LOCALAPPDATA%\linkgate\install.json`, including the VS Code values it replaced. Unticking a part on a later run leaves that part as it is.

## Modes

| Launch | Window |
| --- | --- |
| `linkgate-setup.exe` | Install, or update when an install record exists |
| `linkgate-setup.exe --update` | Update, with the previous choices ticked |
| `linkgate-setup.exe --uninstall` | Uninstall |

Uninstalling removes the program, the shortcuts, the registry entry and the WSL files for each recorded distribution. It keeps the blocklists and linkgate's settings unless you tick the box. It touches VS Code only when you tick the box, and then restores the values recorded at install. A key that you changed since the install is left alone. Files that are in use, including the running installer, are removed a moment after the window closes.

## Build and test

From the repository root, in WSL:

```bash
pnpm build:setup
pnpm test:setup
```

`build:setup` cross-compiles with `cargo-xwin` to `installer/src-tauri/target/x86_64-pc-windows-msvc/release/linkgate-setup.exe`. `test:setup` builds the Rust tests for Windows and runs them through interop. On a Windows runner, the same result comes from `pnpm build` in `installer/`, then `cargo build --release` and `cargo test` in `installer/src-tauri/`.

One test runs the generated shell script in a real WSL distribution and is ignored by default:

```bash
LINKGATE_SETUP_TEST_DISTRO=Ubuntu WSLENV=LINKGATE_SETUP_TEST_DISTRO bash installer/scripts/test-windows.sh --ignored
```

It runs with `HOME` pointed at a scratch folder, so it doesn't touch the distribution's real files.

## Testing hooks

| Variable | Effect |
| --- | --- |
| `LINKGATE_SETUP_SOURCE` | A folder holding `linkgate.exe`, `SHA256SUMS` and optionally `linkgate-setup.exe` and `VERSION`. The installer reads them instead of GitHub. |
| `LINKGATE_SETUP_SANDBOX` | A folder that replaces the install directory, data and settings folders, Start menu, desktop, VS Code settings, and the registry key. WSL steps run with `HOME` set to a scratch folder inside the distribution. |

With both set, `linkgate-setup.exe` runs the whole install without touching the real setup.

`linkgate-setup.exe --headless <job.json>` runs a job without a window and is meant for tests. The job is JSON:

```json
{
  "mode": "install",
  "install": { "desktopShortcut": true, "wslDistros": [], "browserEnv": "off", "vscode": ["code"] },
  "resultPath": "C:\\Temp\\result.json"
}
```

`mode` is `install` or `uninstall`. An uninstall job takes `"uninstall": { "restoreVscode": false, "removeData": false }`. The result file holds `ok`, an `error` for a fatal failure, and the per-step `summary`. The exit code is 0 when every step worked, 1 when a step failed, and 2 when the job couldn't be read.

## Layout

```
src-tauri/src/
├── main.rs        # modes and window start-up
├── commands.rs    # what the window calls
├── install.rs     # the install and update steps
├── uninstall.rs   # the uninstall steps
├── source.rs      # GitHub release or local folder
├── checksums.rs   # SHA256SUMS parsing and verification
├── fsutil.rs      # atomic replace and in-use handling
├── wsl.rs         # shell script generation and running
├── distros.rs     # wsl.exe -l -q decoding
├── jsonc.rs       # comment-preserving settings.json edits
├── shortcut.rs    # .lnk files
├── registry.rs    # the uninstall entry
├── state.rs       # install.json
├── paths.rs       # real and sandboxed locations
├── progress.rs    # step events
└── headless.rs    # the test-only job runner
src/               # React window: options, progress, done, uninstall
```
