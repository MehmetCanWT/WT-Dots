#!/usr/bin/env bash
set -euo pipefail

state_file="${XDG_RUNTIME_DIR:-/tmp}/hypr_show_desktop_ws"
cur=$(hyprctl activeworkspace -j | jq -r '.id')

if [ "$cur" = "99" ]; then
    prev=$(cat "$state_file" 2>/dev/null || echo "1")
    if [ -z "$prev" ] || [ "$prev" = "99" ]; then
        prev="1"
    fi
    hyprctl dispatch "hl.dsp.focus({ workspace = $prev })" >/dev/null
else
    echo "$cur" > "$state_file"
    hyprctl dispatch "hl.dsp.focus({ workspace = 99 })" >/dev/null
fi
