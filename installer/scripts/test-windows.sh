#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root/src-tauri"
cargo xwin test --target x86_64-pc-windows-msvc --no-run
binary=$(find target/x86_64-pc-windows-msvc/debug/deps -name 'linkgate_setup-*.exe' -printf '%T@ %p\n' | sort -rn | head -1 | cut -d' ' -f2-)
temp_dir=$(wslpath -u "$(cd /mnt/c && /mnt/c/Windows/System32/cmd.exe /d /c 'echo %TEMP%' | tr -d '\r')")
copy="$temp_dir/linkgate-setup-tests.exe"
cp "$binary" "$copy"
trap 'rm -f "$copy"' EXIT
"$copy" "$@"
