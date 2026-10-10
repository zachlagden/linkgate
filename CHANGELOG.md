# Changelog

## [Unreleased]

### Added

- The installer checks each WSL distribution for `xdg-utils` and `python3`, and offers to install what is missing with `apt-get`, `dnf`, `pacman`, `zypper` or `apk`, as root and only when you leave the box ticked. Uninstalling offers to remove only the packages the installer added.

## [0.2.0]

### Added

- `linkgate-setup.exe`, an installer that downloads the latest release, checks it against `SHA256SUMS` and installs it. It adds a Start menu shortcut and an optional desktop shortcut, sets up the WSL handler for each distribution you tick, and can set the VS Code link settings. Running it again updates, and Windows Settings can uninstall it.
- A release workflow that builds `linkgate.exe` and `linkgate-setup.exe` on a Windows runner and publishes them with `SHA256SUMS` and build provenance attestations when a `vX.Y.Z` tag is pushed.
- A workflow that replaces `linkgate-setup.exe` on the latest release when the installer or the WSL files change, without a new version.
- CI builds, lints and tests the installer.
- A setting for how long the window waits before closing, from 3 to 60 seconds, or never. The default stays at 10 seconds.
- Drag the browsers in settings into the order you want. The picker and the `1` to `9` keys follow that order, and `Alt+Up` and `Alt+Down` on a handle move a browser from the keyboard.
- A once-a-day check for a newer release, shown in settings as "Update available" with an Update button that runs the installer. A settings switch turns the check off.
- A settings switch that turns the blocklist check off. With it off, linkgate looks up no links, downloads no lists and shows no list warning.
- Project documentation for open source use: contributing guide, security policy, issue and pull request templates, CI and Dependabot configuration.
- MIT licence and package metadata.
- Screenshot, troubleshooting, uninstall and file locations in the README.
- `xdg-open` on a PDF, PNG, JPEG, GIF, WebP or SVG file shows the picker, with the WSL path converted to its Windows `file:` link.
- Network file links open in Firefox, which needs five slashes for `wsl.localhost` paths.
- `linkgate-open` accepts `file://` links to WSL paths and percent-encodes special characters in converted paths.

### Fixed

- A `settings.json` that starts with a byte order mark, as PowerShell 5 and Notepad write it, loads instead of resetting every setting.

## [0.1.0]

### Added

- Link picker window with a custom title bar, a 10 second countdown and placement on the monitor under the cursor.
- Browser detection from the Windows registry, with icons, the default browser marked and a settings page to hide browsers.
- Colour-coded link anatomy with warnings for plain http, credentials before `@`, punycode, IP addresses and unusual ports.
- Blocklist lookups against the malicious, suspicious and tracking lists, with a confirmation step on a match and a daily background update.
- WSL handler and install script.
