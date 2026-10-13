#!/usr/bin/env bash
# Power Menu via Rofi

CHOSEN=$(printf "  Kapat\n󰜉  Yeniden Başlat\n  Kilitle\n󰤄  Askıya Al\n󰍃  Çıkış Yap" | \
  rofi -dmenu -theme-str 'mainbox { children: [ listview ]; } listview { lines: 5; } window { width: 320px; }')

case "$CHOSEN" in
  *Kapat*)          systemctl poweroff ;;
  *Yeniden\ Başlat*) systemctl reboot ;;
  *Kilitle*)        loginctl lock-session ;;
  *Askıya\ Al*)     systemctl suspend ;;
  *Çıkış\ Yap*)     mmsg dispatch quit ;;
esac
