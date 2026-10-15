#!/usr/bin/env bash
# Quick Brightness Selector via Rofi and brightnessctl

CHOICE=$(printf "100%%  󰃠  Maksimum Parlaklık\n80%%   󰃠  Yüksek Parlaklık\n60%%   󰃟  Dengeli (Varsayılan)\n40%%   󰃞  Standart Parlaklık\n20%%   󰃝  Loş Işık\n5%%    󰃚  Gece Modu (Minimum)" | \
  rofi -dmenu -click-to-exit -theme-str 'window { location: north east; anchor: north east; y-offset: 36px; x-offset: -20px; width: 300px; border-radius: 10px; } mainbox { children: [ listview ]; } listview { lines: 6; require-input: false; }')

case "$CHOICE" in
  100%*) brightnessctl set 100% ;;
  80%*)  brightnessctl set 80%  ;;
  60%*)  brightnessctl set 60%  ;;
  40%*)  brightnessctl set 40%  ;;
  20%*)  brightnessctl set 20%  ;;
  5%*)   brightnessctl set 5%   ;;
esac
