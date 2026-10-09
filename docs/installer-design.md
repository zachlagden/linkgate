# Installer and updates

Status: proposed. Nothing in this document is built yet.

Today, installing linkgate means cloning the repository and building it with a cross-compiling toolchain in WSL. This document describes an installer that downloads a finished build, sets up WSL and VS Code, and a release process that supplies those builds.

## Goals

- Install from one download, with no Rust, Node or `cargo-xwin` on the machine.
- Set up the WSL handler and the VS Code settings, each one optional.
- Add linkgate to the Start menu, and optionally to the desktop, so settings open without finding the exe.
- Update by running the same installer again.
- Verify every download against a published checksum.

## Release assets

Each release carries three assets with fixed names:

| Asset | Purpose |
| --- | --- |
| `linkgate.exe` | The app |
| `linkgate-setup.exe` | The installer |
| `SHA256SUMS` | Checksums for the other two |

The installer always reads the latest release through the GitHub API and downloads `linkgate.exe` from it. A failed check for a newer version counts as "no update" and is never an error.

## Release workflow

A tag matching `vX.Y.Z` starts a workflow on a Windows runner. It builds `linkgate.exe` and `linkgate-setup.exe`, writes `SHA256SUMS`, attaches build provenance with `actions/attest`, and publishes the release with all three assets.

Tags are the only thing that creates a version. Changing the installer never does.

## Installer refresh workflow

The installer is a thin bootstrap. It fetches the app instead of containing it, so most installer changes are independent of app versions.

A second workflow runs on pushes to `main` that touch the installer's directory. It rebuilds `linkgate-setup.exe` and replaces it on the latest release with `gh release upload --clobber`. It uploads a regenerated `SHA256SUMS` in the same step, because a stale checksum file would fail verification.

This relies on release assets being replaceable. Do not enable GitHub's immutable releases setting on this repository.

## What the installer does

The installer is a small window, built like the picker with Tauri and React. It shows a tickbox for each optional part and does nothing for a part left unticked. A user can read the source and build it themselves, as the README describes.

1. Download `linkgate.exe` from the latest release, verify its SHA256, and copy it to `%LOCALAPPDATA%\Programs\linkgate`. An existing install is replaced atomically, as `scripts/install.sh` does today.
2. Create a Start menu shortcut, `linkgate`, that starts `linkgate.exe` with no link. That opens the settings page.
3. Optionally create a desktop shortcut to the same target.
4. Optionally set up WSL. The installer lists distros with `wsl.exe -l -q`, skips Docker's internal ones, and ticks the default distro. For each distro the user ticks, write `~/.local/bin/linkgate-open` and `~/.local/share/applications/linkgate.desktop`, and register the handler with `xdg-mime` for `http`, `https`, `text/html`, `application/pdf` and the common image types. The `BROWSER` export and the `x-www-browser` alternative change files outside the project, so the installer shows them and asks before applying.
5. Optionally set `workbench.externalBrowser` and `workbench.browser.openLocalhostLinks` in VS Code's user `settings.json`, after backing up the file. The installer looks for `%APPDATA%\Code\User\settings.json` and `%APPDATA%\Code - Insiders\User\settings.json`, and offers each one it finds.
6. Start the first blocklist download.

The WSL handler files live in the installer, not in `linkgate.exe`, so a fix to them ships through the installer refresh workflow.

## Updating and uninstalling

Running `linkgate-setup.exe` again updates the app, then offers the same options with the current choices ticked.

An uninstall entry removes the exe, the shortcuts and the WSL files the installer wrote. It leaves the blocklists and settings unless the user asks to remove them, and it restores nothing in VS Code settings without confirmation.

## Update checks

`linkgate.exe` checks the latest release once a day when it starts, in the same way it checks the blocklists. When a newer version exists, settings shows "Update available" with a button that runs `linkgate-setup.exe`. A toggle in settings turns the check off, because it is a second network request beyond the blocklist download. The README's privacy statement changes with it.

## Signing

The installer ships unsigned. Windows SmartScreen shows "Windows protected your PC" the first time it runs, and the README says so. Users can verify the download against `SHA256SUMS` and the build attestation, or build from source. Applying to SignPath Foundation for free open source signing is a later step.
