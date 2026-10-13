#!/usr/bin/env bash
# Clipboard history manager via cliphist and rofi

# Ensure systemd watcher or background fallback is active
if ! pgrep -f "wl-paste.*cliphist store" >/dev/null; then
    systemctl --user start cliphist-text.service cliphist-image.service 2>/dev/null || {
        wl-paste --type text --watch cliphist store &
        wl-paste --type image --watch cliphist store &
    }
fi

SELECTED=$(cliphist list | rofi -dmenu -p "❯" -theme-str 'entry { placeholder: "Search clipboard..."; } listview { lines: 8; require-input: false; } window { width: 620px; }')

if [ -n "$SELECTED" ]; then
    TMP_DATA=$(cliphist decode <<< "$SELECTED")
    MIME=$(file -b --mime-type - <<< "$TMP_DATA")
    if [[ "$MIME" == image/* ]]; then
        cliphist decode <<< "$SELECTED" | wl-copy --type "$MIME"
    else
        cliphist decode <<< "$SELECTED" | wl-copy
    fi
fi
