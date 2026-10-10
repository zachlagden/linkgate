<div align="center">

<img src="assets/icon.svg" alt="" width="88" />

# linkgate

**A link picker for WSL. Choose which Windows browser opens a link, and see where it really goes first.**

[![License](https://img.shields.io/github/license/zachlagden/linkgate?style=flat-square)](LICENCE)
[![Stars](https://img.shields.io/github/stars/zachlagden/linkgate?style=flat-square)](https://github.com/zachlagden/linkgate/stargazers)
[![Last Commit](https://img.shields.io/github/last-commit/zachlagden/linkgate?style=flat-square)](https://github.com/zachlagden/linkgate/commits/main)
[![Issues](https://img.shields.io/github/issues/zachlagden/linkgate?style=flat-square)](https://github.com/zachlagden/linkgate/issues)
[![CI](https://img.shields.io/github/actions/workflow/status/zachlagden/linkgate/ci.yml?branch=main&style=flat-square&label=CI)](https://github.com/zachlagden/linkgate/actions/workflows/ci.yml)

![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-1.85+-000000?style=flat-square&logo=rust&logoColor=white)
![React](https://img.shields.io/badge/React-19-61DAFB?style=flat-square&logo=react&logoColor=black)
![Windows](https://img.shields.io/badge/Windows-11-0078D4?style=flat-square&logo=windows11&logoColor=white)
![pnpm](https://img.shields.io/badge/pnpm-managed-F69220?style=flat-square&logo=pnpm&logoColor=white)

[Install](#install) · [Use it from VS Code](#use-it-from-vs-code) · [How it works](#how-it-works) · [Report Bug](https://github.com/zachlagden/linkgate/issues) · [Request Feature](https://github.com/zachlagden/linkgate/issues)

<img src="docs/screenshot.png" alt="The linkgate window showing a suspicious link split into colours, with Firefox, Chrome and Copy link as actions" width="482" />

</div>

---

## What is linkgate?

When a program in WSL runs `xdg-open https://...`, it normally opens a browser without asking. linkgate sits in that path. A small window appears in the middle of the monitor your mouse is on, shows the link broken into its parts, and offers every Windows browser you have installed plus "Copy link". Pressing a button runs that action and closes the window. After 10 seconds with no choice, the window closes and nothing happens. You can change that time in settings, or turn the timeout off.

It also opens PDFs and images from WSL. `xdg-open ~/report.pdf` turns the Linux path into the Windows one (`file://wsl.localhost/Ubuntu/home/...`) and passes it to the browser you pick.

It is a Tauri 2 app. The Rust side handles parsing, blocklist lookups, browser detection and launching. The window is React and Tailwind.

---

## Features

| Feature | Description |
| --- | --- |
| Browser choice | Lists every browser registered under `StartMenuInternet` in the Windows registry, with its icon, and marks your default. Hide the ones you don't want and drag the rest into the order you like in settings. |
| Link anatomy | Shows the registrable domain large and the subdomain dimmed, so `paypal.com.account-check.io` reads as `account-check.io`. The full link is coloured by part: scheme, subdomain, domain, port, path, query keys, query values and fragment. |
| Warnings | Flags plain `http`, text before an `@` (which is not the site), punycode domains that imitate other letters, raw IP addresses and unusual ports. |
| Blocklist check | Checks the domain and its parent domains against the malicious, suspicious and tracking lists from [Pi-hole Optimized Blocklists](https://github.com/zachlagden/Pi-hole-Optimized-Blocklists). A match adds a confirmation step and never blocks the link. A switch in settings turns the check off. |
| Files from WSL | Converts WSL paths and `file://` links to Windows links for PDF, PNG, JPEG, GIF, WebP and SVG files. |
| Timeout | The window closes by itself after 3 to 60 seconds, 10 by default, or stays open until you choose if you set it to never. A bar under the title shows the time left. |
| Keyboard first | `1` to `9` open in a browser in the order you set, `C` copies the link, `Esc` closes. |
| Installer | `linkgate-setup.exe` downloads and verifies the latest release, adds Start menu and desktop shortcuts, and sets up WSL and VS Code. Each part is optional. |
| Private | Links are never logged. linkgate makes two kinds of network request, the blocklist download and a once-a-day check for a new release, and each has a switch in settings. The installer talks to GitHub only while it runs. |

---

## Install

### With the installer

1. Download `linkgate-setup.exe` from the [latest release](https://github.com/zachlagden/linkgate/releases/latest).
2. Run it. Windows may show "Windows protected your PC", which [the next section](#windows-smartscreen) explains.
3. Tick what you want in the window and press Install.

The installer needs Windows 11 with WebView2. WSL setup also needs WSL with interop enabled (`[interop] enabled` must not be `false` in `/etc/wsl.conf`; `appendWindowsPath = false` is fine) and `xdg-utils` and `python3` inside each distribution you tick. If either is missing, the installer offers to install it for you.

| Option | Default | What it does |
| --- | --- | --- |
| Start menu shortcut | Always | Adds `linkgate` to the Start menu. It opens the settings page. |
| Desktop shortcut | Off | Adds the same shortcut to the desktop. |
| WSL distributions | The default distribution ticked | For each ticked distribution, installs `~/.local/bin/linkgate-open` and a `linkgate.desktop` handler, and makes it the default for `http`, `https`, `text/html`, `application/pdf` and the common image types. |
| Install missing packages | On, shown only when something is missing | When a ticked distribution has no `xdg-utils` or `python3`, the window says so and shows the exact package manager command. With the box ticked, the installer runs it as root through `wsl.exe -u root`, so no password is needed. It supports `apt-get`, `dnf`, `pacman`, `zypper` and `apk`. Untick the box to install them yourself. |
| `BROWSER` variable | Off | Adds `export BROWSER="$HOME/.local/bin/linkgate-open"` to `~/.zshenv` or `~/.profile` in each ticked distribution. |
| VS Code | Off | For `Code` and `Code - Insiders`, when found, sets the two [VS Code settings](#use-it-from-vs-code) after backing up `settings.json` to `settings.json.linkgate-backup`. |

Whatever you tick, the installer downloads `linkgate.exe` from the latest release, checks it against the release's `SHA256SUMS`, installs it to `%LOCALAPPDATA%\Programs\linkgate`, adds linkgate to Windows Settings under Apps, and starts the first blocklist download. It also keeps a copy of itself, `linkgate-setup.exe`, in that folder.

One WSL setting needs `sudo`, so the installer shows the command and never runs it:

```bash
sudo update-alternatives --install /usr/bin/x-www-browser x-www-browser "$HOME/.local/bin/linkgate-open" 500
```

Test it:

```bash
xdg-open https://example.com
```

To update, press Update in linkgate's settings when it offers one, or run `linkgate-setup.exe` again. Each run shows your earlier choices ticked. Unticking an option on a later run leaves that part as it is. Uninstalling removes it.

#### Windows SmartScreen

The installer isn't code signed, so on first run Windows shows "Windows protected your PC". Choose "More info", then "Run anyway". Signing certificates cost money, and the source is here to read and build yourself.

To check a download, compare its hash with the release's `SHA256SUMS`:

```powershell
Get-FileHash .\linkgate-setup.exe -Algorithm SHA256
```

```bash
sha256sum --check --ignore-missing SHA256SUMS
```

`linkgate.exe` and `linkgate-setup.exe` each carry a build provenance attestation from GitHub Actions. With the [GitHub CLI](https://cli.github.com/):

```bash
gh attestation verify linkgate-setup.exe --repo zachlagden/linkgate
```

### Build from source

You need Rust with the `x86_64-pc-windows-msvc` target, `cargo-xwin`, `clang`, `lld` and `llvm`, Node 22 and pnpm, in WSL with interop enabled.

```bash
rustup target add x86_64-pc-windows-msvc
cargo install --locked cargo-xwin
sudo apt install clang lld llvm
```

`cargo-xwin` downloads the MSVC CRT and Windows SDK on first use, which means accepting Microsoft's licence for them.

```bash
git clone https://github.com/zachlagden/linkgate.git
cd linkgate
pnpm install
pnpm install:windows
```

This builds `linkgate.exe`, copies it to `%LOCALAPPDATA%\Programs\linkgate`, installs `~/.local/bin/linkgate-open` and a `linkgate.desktop` handler in the distribution you run it from, and sets that handler as the default for `http`, `https`, `text/html`, `application/pdf` and the common image types. It then downloads the blocklists. It adds no Start menu shortcut and no Windows Settings entry, and it leaves `BROWSER` and `x-www-browser` to you, as described above:

```bash
echo 'export BROWSER="$HOME/.local/bin/linkgate-open"' >> ~/.zshenv
```

`scripts/install.sh --skip-build` reinstalls the last build without rebuilding. `pnpm build:setup` builds the installer with `cargo-xwin` to `installer/src-tauri/target/x86_64-pc-windows-msvc/release/linkgate-setup.exe`.

### Uninstall

Open Windows Settings, then Apps, then Installed apps, find linkgate and choose Uninstall. Or run:

```powershell
& "$env:LOCALAPPDATA\Programs\linkgate\linkgate-setup.exe" --uninstall
```

Uninstalling removes `linkgate.exe`, the shortcuts, the Windows Settings entry, and for each distribution the installer set up, the handler files, the `linkgate.desktop` default entries and any `BROWSER` line the installer added. It keeps linkgate's settings and blocklists unless you tick the box to delete them. It offers to remove `xdg-utils` or `python3` from a distribution only if the installer added them, and you have to tick that box too, because other programs there may use them. It changes VS Code only if you tick that box, and then restores the values it replaced. A setting you changed since the install stays.

It doesn't remove the `x-www-browser` alternative, because that needed `sudo`:

```bash
sudo update-alternatives --remove x-www-browser "$HOME/.local/bin/linkgate-open"
```

If you installed with `pnpm install:windows`, remove the files by hand instead:

```bash
rm -rf "/mnt/c/Users/<you>/AppData/Local/Programs/linkgate" "/mnt/c/Users/<you>/AppData/Local/linkgate" "/mnt/c/Users/<you>/AppData/Roaming/linkgate"
rm -f ~/.local/bin/linkgate-open ~/.local/share/applications/linkgate.desktop
```

Then remove the `BROWSER` line from your shell profile and pick a new default for the file types with `xdg-mime default <app>.desktop x-scheme-handler/https`.

---

## Use it from VS Code

VS Code for Windows opens links without going through WSL, so it needs two settings in its own `settings.json`. The [installer](#with-the-installer) can set them for you when you tick VS Code. To set them by hand, open `settings.json` with `Ctrl+Shift+P`, then `Preferences: Open User Settings (JSON)`, and add:

```json
{
  "workbench.externalBrowser": "C:\\Users\\<you>\\AppData\\Local\\Programs\\linkgate\\linkgate.exe",
  "workbench.browser.openLocalhostLinks": false
}
```

- `workbench.externalBrowser` makes VS Code start `linkgate.exe` with the URL for every `http` and `https` link, whether it comes from the editor, the terminal or an extension.
- `workbench.browser.openLocalhostLinks` stops VS Code opening `localhost`, `127.0.0.1` and `0.0.0.0` links in its Integrated Browser. Without it, those links skip linkgate.

Replace `<you>` with your Windows user name, and use double backslashes because the file is JSON. Reload the window afterwards with `Ctrl+Shift+P`, then `Developer: Reload Window`.

---

## Usage

| Key | Action |
|---|---|
| `1` to `9` | Open in that browser, counting down the list as you ordered it |
| `C` | Copy the link |
| `Esc` | Close, or go back from a confirmation or settings |
| `Enter` | Confirm, on the blocklist confirmation |

Run `linkgate.exe` with no link, or click the gear in the title bar, to open settings. There you choose which browsers appear, set how long the window waits, turn the blocklist check and the update check on or off, check the blocklists by hand, and start an update when one is available.

### Browser order

The browsers in settings are listed in the order the picker shows them, and the numbers `1` to `9` follow that order. Drag a row by its handle to move it, or focus the handle with the keyboard and press `Alt+Up` or `Alt+Down`. Press `Esc` during a drag to cancel it. Hidden browsers keep their place in the list but take no number. A browser you haven't placed yet, such as one you install later, goes at the end of the list.

### Timeout

The bar under the title bar counts down the time you have to choose. It pauses while the mouse is over the window, and it starts again from the full time when you return from settings or when a blocklist confirmation appears. In settings, "Close automatically after" sets the time from 3 to 60 seconds. Turn on "Never" to remove the timeout, so the window stays until you choose an action or press `Esc`. A new value applies from the next link you open.

### Warnings

| Warning | Meaning |
| --- | --- |
| Not encrypted | The link uses `http`, so anyone on the network can read or change the page. |
| Credentials | Text before an `@` is a username, not the site. `https://paypal.com@evil.example` goes to `evil.example`. |
| Punycode | The domain contains letters from another alphabet that can look like Latin ones. The window shows the decoded form. |
| IP address | The link goes to a raw IP address instead of a domain name. |
| Port | The link uses a port other than the scheme's default. |

---

## How it works

```
program in WSL
  │  xdg-open / $BROWSER / x-www-browser
  ▼
~/.local/bin/linkgate-open          (linux/linkgate-open)
  │  converts WSL paths to Windows links
  ▼
linkgate.exe <url>                  (through WSL interop)
  │  parses the link, checks the blocklists, reads browsers from the registry
  ▼
picker window on the monitor under the cursor
  │  you choose
  ▼
browser.exe <url>                   (or the clipboard)
```

`linkgate-open` is a shell script. It checks that WSL interop is on, converts an existing path or `file://` link to the Windows form, and starts `linkgate.exe` detached so the calling program doesn't wait for your choice.

Firefox reads a WSL file only as `file://///wsl.localhost/<distro>/...`, with five slashes, and shows a blank page for the standard two-slash form. linkgate shows the standard form in the window and converts it to the five-slash form when it starts any browser. Chrome opens that form too.

### Blocklists

Blocklists are downloaded to `%LOCALAPPDATA%\linkgate\lists` and converted to a sorted index that is memory-mapped and binary-searched, so a lookup doesn't load the 2 million domain list into memory. When linkgate starts and the last check is more than 24 hours old, it starts a detached `linkgate.exe --update-lists` process that fetches only the lists whose ETag changed.

Turn off "Check links against the blocklists" in settings and linkgate stops looking links up and stops downloading lists. The lists already on disk stay there, and the check resumes when you turn the switch back on.

### Updates

When linkgate starts and the last check is more than 24 hours old, it starts a detached `linkgate.exe --check-update` process. That process asks the GitHub API for the latest release of this repository, with the `linkgate/<version>` user agent and nothing else, and writes the result to `%LOCALAPPDATA%\linkgate\update.json`. If the release is newer than the running version, settings shows "Update available" with an Update button. The button runs `linkgate-setup.exe --update` from the folder `linkgate.exe` is in, or opens the releases page when that file isn't there. A failed check is logged and treated as no update.

Turn off "Check for updates" in settings and linkgate makes no release check. With both switches off it makes no network requests at all.

### Where files live

| Path | Contents |
| --- | --- |
| `%LOCALAPPDATA%\Programs\linkgate\linkgate.exe` | The app |
| `%LOCALAPPDATA%\Programs\linkgate\linkgate-setup.exe` | The installer's copy of itself, used for updates and uninstalling |
| `%LOCALAPPDATA%\linkgate\install.json` | What the installer set up, so it can update and uninstall it |
| `%APPDATA%\Microsoft\Windows\Start Menu\Programs\linkgate.lnk` | The Start menu shortcut |
| `%APPDATA%\linkgate\settings.json` | Hidden browsers, their order, the timeout and the blocklist and update switches |
| `%LOCALAPPDATA%\linkgate\lists\` | Blocklists and their index |
| `%LOCALAPPDATA%\linkgate\icons\` | Cached browser icons |
| `%LOCALAPPDATA%\linkgate\update.json` | The latest release found by the last update check |
| `%LOCALAPPDATA%\linkgate\linkgate.log` | Errors as JSON lines. Links are never logged. |
| `~/.local/bin/linkgate-open` | The WSL handler |
| `~/.local/share/applications/linkgate.desktop` | The desktop entry that registers it |

---

## Tech Stack

| Layer | Technology |
| --- | --- |
| App shell | Tauri 2 |
| Backend | Rust, `url`, `idna`, `psl`, `memmap2`, `winreg`, `ureq` |
| Frontend | React 19, TypeScript, Tailwind CSS 4, Vite |
| Installer | A second Tauri 2 app in a pnpm workspace: Rust, `jsonc-parser`, `winreg`, `ureq` and React |
| WSL handler | Bash |
| Builds | `cargo-xwin` from WSL, native MSVC on GitHub Actions |
| Releases | GitHub Actions, with SHA256 checksums and build provenance attestations |

---

## Project Structure

```
src-tauri/src/
├── main.rs         # entry point, --update-lists and --check-update modes
├── commands.rs     # commands the window calls
├── link.rs         # link parsing, domain split and warning signals
├── browsers.rs     # registry detection and launching
├── icons.rs        # browser icon extraction
├── lists/          # blocklist download, index and lookup
├── updates.rs      # release check, version comparison and starting the installer
├── net.rs          # HTTP client shared by the downloads and the release check
├── background.rs   # detached background processes
├── lock.rs         # lock files that stop two background runs overlapping
├── placement.rs    # window placement on the monitor under the cursor
├── order.rs        # the order of the browser list
├── timeout.rs      # the auto-close timeout
├── settings.rs     # settings.json
├── paths.rs        # data and config directories
└── logging.rs      # JSON-lines error log
src/                # React frontend: picker, link anatomy, confirmation, settings
installer/          # linkgate-setup.exe, the installer (see installer/README.md)
linux/              # linkgate-open and the desktop entry
scripts/            # install.sh and the Rust test runner
docs/               # screenshot and the installer design note
.github/workflows/  # CI, release and installer refresh
```

---

## Development

| Command | What it does |
|---|---|
| `pnpm dev` | Runs the Vite dev server for the frontend alone |
| `pnpm build` | Type-checks and builds the frontend into `dist/` |
| `pnpm build:windows` | Builds `linkgate.exe` with `cargo-xwin` |
| `pnpm test:rust` | Builds the Rust tests for Windows and runs them through interop |
| `pnpm install:windows` | Builds and installs without the installer, as above |
| `pnpm build:setup` | Builds `linkgate-setup.exe` with `cargo-xwin` |
| `pnpm test:setup` | Builds the installer's Rust tests for Windows and runs them through interop |

See [CONTRIBUTING.md](CONTRIBUTING.md) for the pull request process and [installer/README.md](installer/README.md) for how the installer works and how to test it without touching your own setup.

### Releases

Pushing a tag named `vX.Y.Z` runs `.github/workflows/release.yml`. It checks that the tag matches the version in `package.json`, both `Cargo.toml` files and both `tauri.conf.json` files, builds `linkgate.exe` and `linkgate-setup.exe` on a Windows runner, writes `SHA256SUMS`, attests the two exes and publishes a release with the notes from the matching `## [X.Y.Z]` section of `CHANGELOG.md`.

A push to `main` that changes `installer/` or `linux/` runs `.github/workflows/installer.yml`. It rebuilds `linkgate-setup.exe` and replaces it, and its line in `SHA256SUMS`, on the latest release without creating a new version. Don't turn on GitHub's immutable releases for this repository, because that blocks replacing release files.

---

## Troubleshooting

<details>
<summary>"WSL interop is disabled" or "linkgate.exe is missing"</summary>

`linkgate-open` prints one of these when it can't start the Windows app. For the first, check that `/etc/wsl.conf` doesn't set `[interop] enabled=false`, then run `wsl --shutdown` from Windows. For the second, run `linkgate-setup.exe` again, or `scripts/install.sh` if you built from source.
</details>

<details>
<summary>The installer says "GitHub has no published linkgate release yet" or "Couldn't reach GitHub"</summary>

The installer downloads from the latest release of this repository. The first message means no release exists yet, so build from source instead. For the second, check your connection and any proxy or firewall that blocks `api.github.com`, then run the installer again.
</details>

<details>
<summary>The installer says a download doesn't match its published checksum</summary>

It didn't install the file. Run the installer again. If the message comes back, open an issue with the version shown in the window.
</details>

<details>
<summary>The installer says linkgate.exe "is in use and can't be replaced"</summary>

Close linkgate, including its settings window, and run the installer again.
</details>

<details>
<summary>The installer says a distribution has no xdg-utils or python3</summary>

The installer offers to install them for you when it finds a supported package manager. If the offer isn't there, or you unticked it, install them inside that distribution, for example `sudo apt install xdg-utils python3`, then run the installer again and tick the distribution. If the install fails, the window shows the last lines of the package manager's output.
</details>

<details>
<summary>A program still opens links without the picker</summary>

Check which handler `xdg-open` uses with `xdg-mime query default x-scheme-handler/https`. It should print `linkgate.desktop`. Programs that read `$BROWSER` need the `BROWSER` option from the installer, or the export from the build-from-source steps. Programs that run on Windows, such as VS Code, need their own setting.
</details>

<details>
<summary>Firefox shows a blank page for a file</summary>

Firefox reads WSL files only as `file://///wsl.localhost/<distro>/...`. linkgate converts to that form when it starts a browser, so a blank page means the build is older than that fix. Update linkgate with the installer, or reinstall with `pnpm install:windows` if you built from source.
</details>

<details>
<summary>The blocklists are out of date</summary>

Open settings and check the lists by hand. The log at `%LOCALAPPDATA%\linkgate\linkgate.log` records download errors.
</details>

---

## Star History

<a href="https://github.com/zachlagden/linkgate/stargazers">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://repo-star-history.zachlagden.uk/svg?repos=zachlagden/linkgate&type=Date&theme=dark&legend=top-left&format=png" />
   <source media="(prefers-color-scheme: light)" srcset="https://repo-star-history.zachlagden.uk/svg?repos=zachlagden/linkgate&type=Date&legend=top-left&format=png" />
   <img alt="Star History Chart for zachlagden/linkgate" src="https://repo-star-history.zachlagden.uk/svg?repos=zachlagden/linkgate&type=Date&legend=top-left&format=png" />
 </picture>
</a>

---

## Support

If this project is useful to you, consider supporting development:

<a href="https://github.com/sponsors/zachlagden">
  <img src="https://img.shields.io/badge/Sponsor_on_GitHub-ea4aaa?style=for-the-badge&logo=github&logoColor=white" alt="Sponsor on GitHub" />
</a>

---

## License

This project is licensed under the MIT License. See the [LICENCE](LICENCE) file for details.

---

<div align="center">

**[Report Bug](https://github.com/zachlagden/linkgate/issues)** · **[Request Feature](https://github.com/zachlagden/linkgate/issues)**

Made by [Zach Lagden](https://github.com/zachlagden)

</div>
