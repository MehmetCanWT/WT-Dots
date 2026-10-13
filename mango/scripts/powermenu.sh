#!/usr/bin/env bash
# Minimal Power Menu via fuzzel

SHUTDOWN="  Kapat"
REBOOT="󰜉  Yeniden Başlat"
SUSPEND="󰤄  Askıya Al"
LOGOUT="󰍃  Çıkış Yap"

CHOSEN=$(printf "%s\n%s\n%s\n%s" \
  "$SHUTDOWN" \
  "$REBOOT" \
  "$SUSPEND" \
  "$LOGOUT" | rofi -dmenu -p "  ")

case "$CHOSEN" in
  "$SHUTDOWN")
    systemctl poweroff
    ;;
  "$REBOOT")
    systemctl reboot
    ;;
  "$SUSPEND")
    systemctl suspend
    ;;
  "$LOGOUT")
    mmsg dispatch quit
    ;;
esac
