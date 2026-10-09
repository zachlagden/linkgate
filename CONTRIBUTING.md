# Contributing

Bug reports, ideas and pull requests are welcome.

## Reporting a bug or suggesting a change

Open an issue with the matching template. For a bug, include your Windows version, your WSL distro, the linkgate version and the last lines of `%LOCALAPPDATA%\linkgate\linkgate.log`. Replace private links with an `example.com` link. For a security problem, follow [SECURITY.md](SECURITY.md) instead.

## Development setup

linkgate is built and tested from WSL. The [README](README.md#requirements) lists the tools to install.

```bash
git clone https://github.com/zachlagden/linkgate.git
cd linkgate
pnpm install
```

| Command | What it does |
|---|---|
| `pnpm dev` | Runs the Vite dev server for the frontend alone |
| `pnpm build` | Type-checks and builds the frontend |
| `pnpm test:rust` | Builds the Rust tests for Windows and runs them through interop |
| `pnpm install:windows` | Builds `linkgate.exe` and installs it on your machine |

Before opening a pull request, run `pnpm build` and `pnpm test:rust`. For Rust changes, `cargo xwin clippy --target x86_64-pc-windows-msvc --all-targets -- -D warnings` from `src-tauri/` must also pass.

## Making a change

1. Fork the repository and create a branch named `<type>/<short-description>`, such as `feature/edge-profiles` or `fix/unc-links`.
2. Keep the change to one purpose. Match the naming and structure of the code around it.
3. Add a Rust test for parsing, matching or path-conversion behaviour you change.
4. Update the README for user-visible changes and add a line under `Unreleased` in the CHANGELOG.
5. Commit with [Conventional Commits](https://www.conventionalcommits.org/), in the imperative mood and at most 72 characters: `fix(link): keep the port of an IPv6 host`.
6. Open a pull request with a conventional-commit title and a body that says what changed and why.

## Project layout

The README's [Project structure](README.md#project-structure) section maps each directory.

## Licence

By contributing, you agree that your contribution is licensed under the [MIT Licence](LICENCE).
