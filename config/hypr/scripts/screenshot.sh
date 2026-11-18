#!/usr/bin/env bash
set -euo pipefail

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
