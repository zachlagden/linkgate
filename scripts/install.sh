#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
build=1
for arg in "$@"; do
  case $arg in
    --skip-build) build=0 ;;
    *) echo "unknown option: $arg" >&2; exit 2 ;;
  esac
done

if [ "$build" = 1 ]; then
  (cd "$root" && pnpm install --frozen-lockfile && pnpm build:windows)
fi

built="$root/src-tauri/target/x86_64-pc-windows-msvc/release/linkgate.exe"
[ -f "$built" ] || { echo "no build found at $built" >&2; exit 1; }

cmd=/mnt/c/Windows/System32/cmd.exe
local_app_data=$(wslpath -u "$(cd /mnt/c && "$cmd" /d /c 'echo %LOCALAPPDATA%' | tr -d '\r')")
install_dir="$local_app_data/Programs/linkgate"
exe="$install_dir/linkgate.exe"

mkdir -p "$install_dir"
cp "$built" "$exe.new"
mv -f "$exe.new" "$exe"

bin_dir="$HOME/.local/bin"
apps_dir="$HOME/.local/share/applications"
mkdir -p "$bin_dir" "$apps_dir"

sed "s|__LINKGATE_EXE__|$exe|" "$root/linux/linkgate-open" > "$bin_dir/linkgate-open"
chmod +x "$bin_dir/linkgate-open"
sed "s|__LINKGATE_OPEN__|$bin_dir/linkgate-open|" "$root/linux/linkgate.desktop" > "$apps_dir/linkgate.desktop"
xdg-mime default linkgate.desktop x-scheme-handler/http x-scheme-handler/https text/html

setsid "$exe" --update-lists </dev/null >/dev/null 2>&1 &

echo "Installed $exe"
echo "Handler: $bin_dir/linkgate-open"
echo "Set BROWSER=$bin_dir/linkgate-open in your shell environment if it isn't already."
