# linkgate

A small Windows window that asks where a link should go. When a program in WSL tries to open a link or a PDF or image file, linkgate appears in the middle of whichever monitor the mouse is on and offers every installed Windows browser plus "Copy link". Pressing a button runs that action and closes the window. After 10 seconds with no choice, it closes and does nothing.

It also helps spot fake links:

- The domain is shown large, with the subdomain dimmed, so `paypal.com.account-check.io` reads as `account-check.io`.
- The full link is split into colours: scheme, subdomain, domain, port, path, query keys, query values and fragment.
- It flags plain `http`, text before an `@` (which is not the site), punycode domains that imitate other letters, raw IP addresses and unusual ports.
- It checks the domain and its parent domains against the malicious, suspicious and tracking lists from [Pi-hole Optimized Blocklists](https://github.com/zachlagden/Pi-hole-Optimized-Blocklists). A match adds a confirmation step and never blocks the link.

## How it works

```
program in WSL
  → xdg-open / $BROWSER / x-www-browser
  → ~/.local/bin/linkgate-open          (linux/linkgate-open)
  → linkgate.exe <url>                  (through WSL interop)
  → window on the monitor under the cursor
```

`linkgate-open` turns a WSL path, or a `file://` link to one, into the matching Windows link before it starts `linkgate.exe`. `/tmp/a.pdf` becomes `file://wsl.localhost/<distro>/tmp/a.pdf`, and `/mnt/c/...` becomes `file:///C:/...`. The handler is registered for PDF, PNG, JPEG, GIF, WebP and SVG files as well as `http`, `https` and HTML, so `xdg-open` on those files shows the picker.

`linkgate.exe` is a Tauri 2 app. The Rust side parses the link, looks up the blocklists, reads installed browsers from `StartMenuInternet` in the registry, extracts their icons and launches the chosen one. The window is React and Tailwind, with a custom title bar and no Windows frame.

Blocklists are downloaded to `%LOCALAPPDATA%\linkgate\lists` and converted to a sorted index that is memory-mapped and binary-searched, so a lookup doesn't load the 2 million domain list into memory. When linkgate starts and the last check is more than 24 hours old, it starts a detached `linkgate.exe --update-lists` process that fetches only the lists whose ETag changed.

Settings live in `%APPDATA%\linkgate\settings.json`. Run `linkgate.exe` with no link, or use the gear in the title bar, to choose which browsers appear and to check the lists by hand. Errors go to `%LOCALAPPDATA%\linkgate\linkgate.log` as JSON lines. Links are never logged.

### Keyboard

| Key | Action |
|---|---|
| `1` to `9` | Open in that browser |
| `C` | Copy the link |
| `Esc` | Close, or go back from a confirmation or settings |
| `Enter` | Confirm, on the blocklist confirmation |

## Requirements

- Windows 11 with WebView2, and WSL with interop enabled (`[interop] enabled` must not be `false` in `/etc/wsl.conf`).
- For building in WSL: Rust with the `x86_64-pc-windows-msvc` target, `cargo-xwin`, `clang`, `lld`, `llvm`, Node 22 and pnpm.

```bash
rustup target add x86_64-pc-windows-msvc
cargo install --locked cargo-xwin
sudo apt install clang lld llvm
```

`cargo-xwin` downloads the MSVC CRT and Windows SDK on first use, which means accepting Microsoft's licence for them.

## Install

```bash
pnpm install
pnpm install:windows
```

This builds `linkgate.exe`, copies it to `%LOCALAPPDATA%\Programs\linkgate`, installs `~/.local/bin/linkgate-open` and a `linkgate.desktop` handler, sets it as the default for `http`, `https`, `text/html`, `application/pdf` and the image types above, and downloads the blocklists. Two settings are left to you:

```bash
echo 'export BROWSER="$HOME/.local/bin/linkgate-open"' >> ~/.zshenv
sudo update-alternatives --install /usr/bin/x-www-browser x-www-browser "$HOME/.local/bin/linkgate-open" 500
```

## Development

| Command | What it does |
|---|---|
| `pnpm build` | Type-checks and builds the frontend into `dist/` |
| `pnpm build:windows` | Builds `linkgate.exe` with `cargo-xwin` |
| `pnpm test:rust` | Builds the Rust tests for Windows and runs them through interop |
| `pnpm install:windows` | Builds and installs, as above |

`scripts/install.sh --skip-build` reinstalls the last build.
