#!/usr/bin/env bash
# Dropdown WiFi Menu for Waybar (Rofi with dark pill theme and toggle support)

MENU_TAG="wifi"
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

ROFI_THEME='* { font: "0xProto Nerd Font 11"; } window { location: north east; anchor: north east; y-offset: 38px; x-offset: -80px; width: 380px; border: 2px; border-color: #567ae6; border-radius: 12px; } mainbox { children: [ message, listview ]; } message { padding: 6px 10px; } listview { lines: 10; require-input: false; }'

# Get WiFi Radio Status
WIFI_STATUS=$(nmcli radio wifi)

if [ "$WIFI_STATUS" = "disabled" ]; then
    CHOICE=$(printf "󰤨  WiFi'ı Aç\n⚙  KDE Ağ Ayarları" | \
      rofi -dmenu -click-to-exit -p "WiFi" -theme-str "$ROFI_THEME" -mesg "WiFi şu anda kapalı")
    case "$CHOICE" in
        *"WiFi'ı Aç"*) nmcli radio wifi on ;;
        *"Ağ Ayarları"*) kcmshell6 kcm_networkmanagement 2>/dev/null & ;;
    esac
    exit 0
fi

# Fetch active connection
ACTIVE_SSID=$(nmcli -t -f ACTIVE,SSID dev wifi | grep '^yes:' | cut -d: -f2)

# Scan WiFi networks
WIFI_LIST=$(nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY dev wifi list --rescan no 2>/dev/null | awk -F: '
NF>=4 && $2!="" && !seen[$2]++ {
    in_use = ($1 == "*") ? "● [BAĞLI] " : "○ "
    lock = ($4 ~ /WPA|WEP/) ? " " : ""
    sig = $3 + 0
    if (sig >= 75) icon = "󰤨 "
    else if (sig >= 50) icon = "󰤥 "
    else if (sig >= 25) icon = "󰤢 "
    else icon = "󰤟 "
    printf "%s%s%s%s (%%%d)\n", in_use, icon, $2, lock, sig
}')

MENU_ITEMS=""
if [ -n "$ACTIVE_SSID" ]; then
    MENU_ITEMS="󰤮  Bağlantıyı Kes (${ACTIVE_SSID})\n"
fi
MENU_ITEMS+="󰑐  Ağları Yeniden Tara\n"
MENU_ITEMS+="󰤮  WiFi'ı Kapat\n"
MENU_ITEMS+="⚙  KDE Ağ Ayarları\n"
MENU_ITEMS+="────────────────────────────\n"
MENU_ITEMS+="$WIFI_LIST"

CHOSEN=$(printf "%b" "$MENU_ITEMS" | \
  rofi -dmenu -click-to-exit -theme-str "$ROFI_THEME" -mesg "Aktif: ${ACTIVE_SSID:-Bağlantı Yok}")

if [ -z "$CHOSEN" ] || [ "$CHOSEN" = "────────────────────────────" ]; then
    exit 0
fi

case "$CHOSEN" in
    *"WiFi'ı Kapat"*)
        nmcli radio wifi off
        ;;
    *"Bağlantıyı Kes"*)
        nmcli con down id "$ACTIVE_SSID" 2>/dev/null || nmcli dev disconnect wlp0s20f3 2>/dev/null
        ;;
    *"Ağları Yeniden Tara"*)
        nmcli dev wifi list --rescan yes >/dev/null 2>&1
        exec "$0"
        ;;
    *"KDE Ağ Ayarları"*)
        kcmshell6 kcm_networkmanagement 2>/dev/null &
        ;;
    *)
        # Extract SSID: remove status dot and signal icons
        CLEAN_SSID=$(echo "$CHOSEN" | sed -E 's/^[●○][[:space:]]*//; s/^\[BAĞLI\][[:space:]]*//; s/^[󰤨󰤥󰤢󰤟][[:space:]]*//; s/[[:space:]]*//; s/[[:space:]]*\(%[0-9]+\)//; s/^[[:space:]]*//; s/[[:space:]]*$//')
        if [ -n "$CLEAN_SSID" ]; then
            OUT=$(nmcli dev wifi connect "$CLEAN_SSID" 2>&1)
            if echo "$OUT" | grep -qi "Secrets were required"; then
                PASS=$(rofi -dmenu -password -click-to-exit -p "Şifre" -theme-str '* { font: "0xProto Nerd Font 11"; } window { location: north east; anchor: north east; y-offset: 38px; x-offset: -80px; width: 340px; border: 2px; border-color: #567ae6; border-radius: 12px; } mainbox { children: [ inputbar ]; }' -mesg "Ağ: $CLEAN_SSID")
                if [ -n "$PASS" ]; then
                    nmcli dev wifi connect "$CLEAN_SSID" password "$PASS"
                fi
            fi
        fi
        ;;
esac
