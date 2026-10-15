#!/usr/bin/env bash
# Quick Brightness Selector via Rofi and brightnessctl with toggle support

MENU_TAG="brightness"
PID_TAG_FILE="/tmp/active_rofi_menu"

# Singleton & Toggle handling
if pgrep -x rofi >/dev/null; then
    CURRENT_MENU=$(cat "$PID_TAG_FILE" 2>/dev/null)
    pkill -x rofi
    if [ "$CURRENT_MENU" = "$MENU_TAG" ]; then
        rm -f "$PID_TAG_FILE"
        exit 0
    fi
fi

echo "$MENU_TAG" > "$PID_TAG_FILE"
trap 'rm -f "$PID_TAG_FILE"' EXIT

ROFI_THEME='* { font: "0xProto Nerd Font 11"; } window { location: north east; anchor: north east; y-offset: 38px; x-offset: -20px; width: 300px; border: 2px; border-color: #567ae6; border-radius: 12px; } mainbox { children: [ listview ]; } listview { lines: 6; require-input: false; }'

CHOICE=$(printf "100%%  󰃠  Maksimum Parlaklık\n80%%   󰃠  Yüksek Parlaklık\n60%%   󰃟  Dengeli (Varsayılan)\n40%%   󰃞  Standart Parlaklık\n20%%   󰃝  Loş Işık\n5%%    󰃚  Gece Modu (Minimum)" | \
  rofi -dmenu -click-to-exit -theme-str "$ROFI_THEME")

case "$CHOICE" in
  100%*) brightnessctl set 100% ;;
  80%*)  brightnessctl set 80%  ;;
  60%*)  brightnessctl set 60%  ;;
  40%*)  brightnessctl set 40%  ;;
  20%*)  brightnessctl set 20%  ;;
  5%*)   brightnessctl set 5%   ;;
esac
