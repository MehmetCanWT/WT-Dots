#!/usr/bin/env bash
# Quick Brightness Selector via fuzzel and brightnessctl

CURR=$(brightnessctl -m | awk -F, '{print $4}')

OPTS=$(printf "100%%  󰃠  Maximum Brightness\n80%%   󰃠  High Brightness\n60%%   󰃟  Balanced (Default)\n40%%   󰃞  Standard Brightness\n20%%   󰃝  Dim Light\n5%%    󰃚  Night Mode (Minimum)")

CHOICE=$(echo "$OPTS" | rofi -dmenu -p "󰃠 Brightness ($CURR) ")

case "$CHOICE" in
  100%*) brightnessctl set 100% ;;
  80%*)  brightnessctl set 80%  ;;
  60%*)  brightnessctl set 60%  ;;
  40%*)  brightnessctl set 40%  ;;
  20%*)  brightnessctl set 20%  ;;
  5%*)   brightnessctl set 5%   ;;
esac
