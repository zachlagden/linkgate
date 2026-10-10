# Installer and updates

Status: implemented. This note records how the installer, the release process and the update check fit together, and why. The installer's own commands, modes and test hooks are in [installer/README.md](../installer/README.md).

Before the installer, installing linkgate meant cloning the repository and cross-compiling it in WSL. The installer downloads a finished build, sets up WSL and VS Code, and a release workflow supplies those builds.

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
| `SHA256SUMS` | Checksums for the other two, in `sha256sum` format |

The installer reads the latest release through the GitHub API and downloads `linkgate.exe` and `SHA256SUMS` from it. A failed check for a newer version counts as "no update" and is never an error.

## Release workflow

`.github/workflows/release.yml` runs when a tag matching `vX.Y.Z` is pushed.

1. A Windows job checks that the tag matches the version in `package.json`, both `Cargo.toml` files and both `tauri.conf.json` files, then builds `linkgate.exe` and `linkgate-setup.exe` with `tauri build --no-bundle`, writes `SHA256SUMS` and uploads the three files as a workflow artifact.
2. A publish job attests the two exes with `actions/attest`, takes the release notes from the matching `## [X.Y.Z]` section of `CHANGELOG.md`, and creates the release with `gh release create`. It fails if that section is missing.

On a pull request that changes `release.yml`, only the build job runs. That checks the workflow without publishing.

Tags are the only thing that creates a version. Changing the installer never does.

## Installer refresh workflow

The installer is a thin bootstrap. It fetches the app instead of containing it, so most installer changes are independent of app versions.

`.github/workflows/installer.yml` runs on pushes to `main` that touch `installer/`, `linux/` or the workflow itself, and on manual dispatch. It rebuilds `linkgate-setup.exe`. A second job then finds the latest release, replaces the `linkgate-setup.exe` line in its `SHA256SUMS`, attests the new exe, and uploads both files with `gh release upload --clobber`. With no release yet, it ends with a notice. The second job runs only on `main`, so a manual dispatch from another branch can't replace the release's installer.

The refreshed installer takes its version from `main`, which can be ahead of the release it is attached to. The installer reads the app from the release, not from its own version, so that doesn't change what it installs.

Both workflows write release files inside one concurrency group, `release-assets`, so they never write at once. GitHub keeps only one waiting run per group and cancels an older waiting run when a newer one arrives, so a release run that is waiting behind a refresh can be cancelled by a second refresh. Re-run the release if that happens.

This relies on release assets being replaceable. Do not enable GitHub's immutable releases setting on this repository.

## What the installer does

The installer is a small window, built like the picker with Tauri and React. It shows a tickbox for each optional part and does nothing for a part left unticked.

1. Download `linkgate.exe` from the latest release, verify its SHA256, and copy it to `%LOCALAPPDATA%\Programs\linkgate`. An existing install is replaced through a `.new` file.
2. Copy itself to `%LOCALAPPDATA%\Programs\linkgate\linkgate-setup.exe`, so the app's update button and the uninstall entry can run it. When it runs from that copy, it fetches and verifies a newer `linkgate-setup.exe` from the release.
3. Create a Start menu shortcut, `linkgate`, that starts `linkgate.exe` with no link. That opens the settings page. Register the uninstall entry in Windows Settings.
4. Optionally create a desktop shortcut to the same target.
5. Optionally set up WSL. The installer lists distributions with `wsl.exe -l -q`, skips Docker's internal ones, and ticks the default. For each ticked distribution, it writes `~/.local/bin/linkgate-open` and `~/.local/share/applications/linkgate.desktop` and registers the handler with `xdg-mime` for `http`, `https`, `text/html`, `application/pdf` and the common image types. The path to `linkgate.exe` inside the distribution comes from `wslpath -u` run there. Before that, it probes each distribution for `xdg-mime`, `python3` and a package manager, and offers to install what is missing as root through `wsl.exe -u root`, only with the tickbox ticked. The `BROWSER` export is a separate tickbox. The `x-www-browser` alternative needs `sudo`, so the installer shows the command and never runs it.
6. Optionally set `workbench.externalBrowser` and `workbench.browser.openLocalhostLinks` in VS Code's user `settings.json`, after backing up the file. The installer looks for `%APPDATA%\Code\User\settings.json` and `%APPDATA%\Code - Insiders\User\settings.json`, offers each one it finds, and edits the file in place so comments and formatting survive.
7. Start the first blocklist download.

The WSL handler files live in the installer, not in `linkgate.exe`, so a fix to them ships through the installer refresh workflow.

What the installer set up is recorded in `%LOCALAPPDATA%\linkgate\install.json`.

## Updating and uninstalling

Running `linkgate-setup.exe` again updates the app, then offers the same options with the current choices ticked. `--update` opens the same window.

Uninstalling removes the exe, the shortcuts, the Windows Settings entry and the WSL files the installer wrote. It leaves the blocklists and settings unless the user asks to remove them. It offers to remove packages from a distribution only when the installer added them. It changes VS Code only when the user ticks that box, and then restores the values it replaced.

## Update checks

`linkgate.exe` checks the latest release once a day when it starts, in the same way it checks the blocklists. A detached `linkgate.exe --check-update` process writes the result to `%LOCALAPPDATA%\linkgate\update.json`. When a newer version exists, settings shows "Update available" with a button that runs `linkgate-setup.exe --update`, or opens the releases page when that file is missing. A toggle in settings turns the check off, because it is a second network request beyond the blocklist download.

## Signing

The installer ships unsigned. Windows SmartScreen shows "Windows protected your PC" the first time it runs, and the README says so. Users can verify the download against `SHA256SUMS` and the build attestation, or build from source. Applying to SignPath Foundation for free open source signing is a later step.
