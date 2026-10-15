#!/usr/bin/env bash
# Dropdown System Stats Menu for Waybar (aligned to top-left below CPU/GPU/RAM)

ROFI_THEME='window { location: north west; anchor: north west; y-offset: 36px; x-offset: 8px; width: 400px; border-radius: 10px; } mainbox { children: [ message, listview ]; } message { padding: 8px 12px; } listview { lines: 8; require-input: false; }'

# CPU
CPU_LOAD=$(top -bn1 | grep "Cpu(s)" | awk '{print 100 - $8}')
CPU_TEMP=$(sensors 2>/dev/null | awk '/Package id 0:/ {print $4}' | tr -d '+')

# RAM
RAM_INFO=$(free -h | awk '/Mem:/ {printf "%s / %s (%%%d)", $3, $2, ($3/$2)*100}')

# GPU
IFS=', ' read -r GPU_UTIL GPU_TEMP GPU_USED GPU_TOTAL < <(nvidia-smi --query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total --format=csv,noheader,nounits 2>/dev/null)

# Disk
DISK_ROOT=$(df -h / | awk 'NR==2 {printf "%s / %s (%s)", $3, $2, $5}')

MENU_ITEMS=""
MENU_ITEMS+="  İşlemci (CPU): %${CPU_LOAD:-0} ${CPU_TEMP:-}\n"
if [ -n "$GPU_UTIL" ]; then
    MENU_ITEMS+="󰢮  GPU (RTX 3050): %${GPU_UTIL} [${GPU_TEMP}°C] (${GPU_USED}M/${GPU_TOTAL}M)\n"
fi
MENU_ITEMS+="󰍛  Bellek (RAM): ${RAM_INFO}\n"
MENU_ITEMS+="󰋊  Depolama (/): ${DISK_ROOT}\n"
MENU_ITEMS+="────────────────────────────\n"
MENU_ITEMS+="📊  KDE Sistem Monitörünü Aç\n"
MENU_ITEMS+="⚙   KDE Sistem Ayarları\n"

CHOSEN=$(printf "%b" "$MENU_ITEMS" | \
  rofi -dmenu -click-to-exit -theme-str "$ROFI_THEME" -mesg "Sistem Performans Özeti")

case "$CHOSEN" in
    *"Sistem Monitörünü Aç"*)
        plasma-systemmonitor 2>/dev/null &
        ;;
    *"Sistem Ayarları"*)
        systemsettings 2>/dev/null &
        ;;
esac
