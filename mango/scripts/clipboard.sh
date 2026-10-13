#!/usr/bin/env bash
# Clipboard history manager via cliphist and rofi

if ! command -v cliphist >/dev/null 2>&1; then
    notify-send "Clipboard" "cliphist kurulu değil! Kurmak için: sudo dnf install -y cliphist"
    exit 1
fi

SELECTED=$(cliphist list | rofi -dmenu -p "❯" -theme-str 'entry { placeholder: "Search clipboard..."; } listview { lines: 8; }')

if [ -n "$SELECTED" ]; then
    echo "$SELECTED" | cliphist decode | wl-copy
fi
