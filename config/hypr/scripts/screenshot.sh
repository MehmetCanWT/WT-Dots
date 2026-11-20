#!/usr/bin/env bash
set -euo pipefail

# Fallback screenshot script used only when the HyprCapture plugin is not loaded.
if ! command -v grim >/dev/null 2>&1 || ! command -v slurp >/dev/null 2>&1; then
    notify-send -u critical "Screenshot Failed" "HyprCapture is not loaded and grim/slurp are not installed."
    exit 1
fi

dir="$HOME/Pictures/Screenshots"
mkdir -p "$dir"
file="$dir/$(date +%Y%m%d_%H%M%S).png"

mode="${1:-region}"
if [ "$mode" = "full" ]; then
    grim "$file"
else
    region="$(slurp)" || exit 0
    [ -z "$region" ] && exit 0
    grim -g "$region" "$file"
fi

wl-copy < "$file"
notify-send "Screenshot Saved" "$file" -i "$file"
