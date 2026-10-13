#!/usr/bin/env bash
# Minimal Power Menu via fuzzel

SHUTDOWN="  Power Off"
REBOOT="󰜉  Reboot"
SUSPEND="󰤄  Suspend"
LOGOUT="󰍃  Log Out"

CHOSEN=$(printf "%s\n%s\n%s\n%s" \
  "$SHUTDOWN" \
  "$REBOOT" \
  "$SUSPEND" \
  "$LOGOUT" | rofi -dmenu -p "  Power ")

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
