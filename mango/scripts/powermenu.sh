#!/usr/bin/env bash
# Power Menu via Rofi

SHUTDOWN="  Kapat"
REBOOT="󰜉  Yeniden Başlat"
LOCK="  Kilitle"
SUSPEND="󰤄  Askıya Al"
LOGOUT="󰍃  Çıkış Yap"

CHOSEN=$(printf "%s\n%s\n%s\n%s\n%s" \
  "$SHUTDOWN" \
  "$REBOOT" \
  "$LOCK" \
  "$SUSPEND" \
  "$LOGOUT" | rofi -dmenu -theme-str 'mainbox { children: [ listview ]; } listview { lines: 5; } window { width: 320px; }')

case "$CHOSEN" in
  "$SHUTDOWN")
    systemctl poweroff
    ;;
  "$REBOOT")
    systemctl reboot
    ;;
  "$LOCK")
    loginctl lock-session
    ;;
  "$SUSPEND")
    systemctl suspend
    ;;
  "$LOGOUT")
    mmsg dispatch quit
    ;;
esac
