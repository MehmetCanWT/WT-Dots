#!/usr/bin/env bash
# Keybindings Cheatsheet for MangoWM displayed via fuzzel

cat << 'EOF' | fuzzel --dmenu --prompt "Keybinds ❯ " --width 65 --lines 20
[ SYSTEM & LAUNCHER ]
Alt + Return           ❯ Open Terminal (kitty)
Alt + Space            ❯ Application Launcher (fuzzel)
Super + E              ❯ Open File Manager (dolphin)
Super + W              ❯ Wallpaper Picker & Theme Selector
Super + K              ❯ Keybindings Cheatsheet
Super + L              ❯ Toggle Game Mode (Disables compositor keys)
Super + M              ❯ Exit MangoWM
Super + R              ❯ Reload MangoWM Config

[ WINDOW NAVIGATION & MANAGEMENT ]
Alt + Q                ❯ Close Focused Window
Alt + Tab              ❯ Toggle Jump / Switch Window (Works in Game Mode)
Super + Tab            ❯ Focus Next Window
Alt + ← / → / ↑ / ↓    ❯ Focus Window Directionally
Super + Shift + Arrows ❯ Move Focused Window
Alt + Middle Click     ❯ Toggle Fullscreen

[ WINDOW STATES & LAYOUT ]
Super + D              ❯ Show / Hide Desktop (Toggle All Windows)
Alt + \                ❯ Toggle Floating Window (Üste Al)
Alt + C                ❯ Center Floating Window (Ortaya Al)
Super + Left Click     ❯ Move Window (Sürükle)
Super + Right Click    ❯ Resize Window (Boyutlandır)
Alt + A                ❯ Toggle Maximize Screen
Alt + Shift + F        ❯ Toggle Fake Fullscreen
Alt + Z                ❯ Toggle Scratchpad
Super + N              ❯ Cycle Layout (Tile, Scroller, Dwindle, Monocle)
Alt + E                ❯ Scroller Preset 1.0 (Full Width)
Alt + X                ❯ Scroller Switch Proportion Preset

[ WORKSPACES (TAGS) ]
Alt + 1..9             ❯ Switch to Workspace 1-9
Ctrl + 1..9            ❯ Move Window to Workspace 1-9
Super + ← / →          ❯ Switch to Previous / Next Workspace
Ctrl + Super + ← / →   ❯ Move Window to Previous / Next Workspace

[ SCREENSHOTS ]
Print                  ❯ Screenshot (Full Screen) ➔ ~/Resimler/Screenshots & Clipboard
Shift + Print          ❯ Screenshot (Select Area) ➔ ~/Resimler/Screenshots & Clipboard

[ AUDIO & MULTIMEDIA ]
XF86AudioRaiseVolume   ❯ Increase Volume (+5%)
XF86AudioLowerVolume   ❯ Decrease Volume (-5%)
XF86AudioMute          ❯ Toggle Mute
EOF
