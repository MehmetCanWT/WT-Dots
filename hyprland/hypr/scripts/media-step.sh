#!/usr/bin/env bash
set -euo pipefail

# Snap brightness and master volume strictly in 5% multiples (0..100 volume, 5..100 brightness) without OSD popups.

TARGET="${1:-}"
DIR="${2:-}"

step_five() {
    local cur="$1"
    local dir="$2"
    local min_val="$3"
    local next

    if [ "$dir" = "up" ]; then
        next=$(( (cur / 5 + 1) * 5 ))
        [ "$next" -gt 100 ] && next=100
    else
        if [ $((cur % 5)) -eq 0 ]; then
            next=$((cur - 5))
        else
            next=$(( (cur / 5) * 5 ))
        fi
        [ "$next" -lt "$min_val" ] && next="$min_val"
    fi
    echo "$next"
}

case "$TARGET" in
    brightness)
        [ "$DIR" = "up" ] || [ "$DIR" = "down" ] || exit 1
        cur_raw=$(brightnessctl get)
        max_raw=$(brightnessctl max)
        cur_pct=$(( (cur_raw * 100 + max_raw / 2) / max_raw ))
        next_pct=$(step_five "$cur_pct" "$DIR" 5)
        brightnessctl -q set "${next_pct}%"
        ;;
    volume)
        [ "$DIR" = "up" ] || [ "$DIR" = "down" ] || exit 1
        vol_line=$(wpctl get-volume @DEFAULT_AUDIO_SINK@)
        cur_pct=$(printf '%s\n' "$vol_line" | awk '{printf "%d", ($2 * 100) + 0.5}')
        next_pct=$(step_five "$cur_pct" "$DIR" 0)
        wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ "${next_pct}%"
        ;;
    mute)
        wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle
        ;;
    *)
        echo "Usage: $0 <brightness|volume|mute> [up|down]" >&2
        exit 1
        ;;
esac
