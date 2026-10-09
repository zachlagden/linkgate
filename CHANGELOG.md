# Changelog

## [Unreleased]

### Added

- A once-a-day check for a newer release, shown in settings as "Update available" with an Update button that runs the installer. A settings switch turns the check off.
- A settings switch that turns the blocklist check off. With it off, linkgate looks up no links, downloads no lists and shows no list warning.
- Project documentation for open source use: contributing guide, security policy, issue and pull request templates, CI and Dependabot configuration.
- MIT licence and package metadata.
- Screenshot, troubleshooting, uninstall and file locations in the README.
- `xdg-open` on a PDF, PNG, JPEG, GIF, WebP or SVG file shows the picker, with the WSL path converted to its Windows `file:` link.
- Network file links open in Firefox, which needs five slashes for `wsl.localhost` paths.
- `linkgate-open` accepts `file://` links to WSL paths and percent-encodes special characters in converted paths.

## [0.1.0]

### Added

- Link picker window with a custom title bar, a 10 second countdown and placement on the monitor under the cursor.
- Browser detection from the Windows registry, with icons, the default browser marked and a settings page to hide browsers.
- Colour-coded link anatomy with warnings for plain http, credentials before `@`, punycode, IP addresses and unusual ports.
- Blocklist lookups against the malicious, suspicious and tracking lists, with a confirmation step on a match and a daily background update.
- WSL handler and install script.
