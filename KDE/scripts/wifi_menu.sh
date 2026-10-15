#!/usr/bin/env bash
# Dropdown WiFi Menu for Waybar (aligned to top-right below Waybar)

ROFI_THEME='window { location: north east; anchor: north east; y-offset: 36px; x-offset: -90px; width: 380px; border-radius: 10px; } mainbox { children: [ message, listview ]; } message { padding: 8px 12px; } listview { lines: 10; require-input: false; }'

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
        CLEAN_SSID=$(echo "$CHOSEN" | sed -E 's/^[●○][[:space:]]*//; s/^[󰤨󰤥󰤢󰤟][[:space:]]*//; s/[[:space:]]*//; s/[[:space:]]*\(%[0-9]+\)//; s/^[[:space:]]*//; s/[[:space:]]*$//')
        if [ -n "$CLEAN_SSID" ]; then
            # Attempt to connect directly (works if password is saved)
            OUT=$(nmcli dev wifi connect "$CLEAN_SSID" 2>&1)
            if echo "$OUT" | grep -qi "Secrets were required"; then
                # Prompt for password via rofi
                PASS=$(rofi -dmenu -password -click-to-exit -p "Şifre" -theme-str 'window { location: north east; anchor: north east; y-offset: 36px; x-offset: -90px; width: 340px; border-radius: 10px; } mainbox { children: [ inputbar ]; }' -mesg "Ağ: $CLEAN_SSID")
                if [ -n "$PASS" ]; then
                    nmcli dev wifi connect "$CLEAN_SSID" password "$PASS"
                fi
            fi
        fi
        ;;
esac
