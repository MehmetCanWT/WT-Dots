#!/usr/bin/env bash
# Quick Brightness Selector via fuzzel and brightnessctl

CURR=$(brightnessctl -m | awk -F, '{print $4}')

OPTS=$(printf "100%%  󰃠  Maksimum Parlaklık\n80%%   󰃠  Yüksek Parlaklık\n60%%   󰃟  Dengeli (Varsayılan)\n40%%   󰃞  Standart Parlaklık\n20%%   󰃝  Loş Işık\n5%%    󰃚  Gece Modu (Minimum)")

CHOICE=$(echo "$OPTS" | fuzzel --dmenu --prompt "󰃠 Parlaklık ($CURR) > " --lines 6 --width 28)

case "$CHOICE" in
  100%*) brightnessctl set 100% ;;
  80%*)  brightnessctl set 80%  ;;
  60%*)  brightnessctl set 60%  ;;
  40%*)  brightnessctl set 40%  ;;
  20%*)  brightnessctl set 20%  ;;
  5%*)   brightnessctl set 5%   ;;
esac
