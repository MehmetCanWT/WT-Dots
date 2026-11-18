#!/usr/bin/env bash
set -euo pipefail

if pgrep -x rofi >/dev/null 2>&1; then
    pkill -x rofi
    exit 0
fi

thumb_dir="${XDG_CACHE_HOME:-$HOME/.cache}/cliphist-thumbs"
mkdir -p "$thumb_dir"

selection="$(
    cliphist list | while IFS=$'\t' read -r id txt; do
        if [[ "$txt" =~ ^\[\[\ binary\ data\ (.*)\ \]\]$ ]]; then
            info="${BASH_REMATCH[1]}"
            thumb="$thumb_dir/${id}.img"
            if [ ! -s "$thumb" ]; then
                cliphist decode "$id" > "$thumb" 2>/dev/null || true
            fi
            printf '%s\t%s\0icon\x1f%s\n' "$id" "Image ($info)" "$thumb"
        else
            printf '%s\t%s\n' "$id" "$txt"
        fi
    done | rofi -dmenu -i -show-icons \
        -display-columns 2 \
        -display-column-separator $'\t' \
        -theme-str 'window { width: 38em; } entry { placeholder: "Clipboard..."; } element-icon { size: 3.2em; }'
)" || exit 0

[ -z "$selection" ] && exit 0

id="${selection%%$'\t'*}"
cliphist decode "$id" | wl-copy
