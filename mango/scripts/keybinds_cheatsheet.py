#!/usr/bin/env python3
import sys
import subprocess

CATEGORIES = {
    "🚀 System & Launcher": [
        ("Alt + Space", "Application Launcher (Rofi)"),
        ("Alt + Enter", "Open Terminal (Kitty)"),
        ("Super + E", "Open File Manager (Dolphin)"),
        ("Super + V", "Clipboard History (Cliphist)"),
        ("Super + W", "Wallpaper & Theme Picker"),
        ("Super + K", "Keybindings Cheatsheet"),
        ("Super + R", "Reload MangoWM Configuration"),
        ("Super + M", "Quit MangoWM (Exit Session)"),
        ("Print", "Screenshot Full Screen (Grim)"),
        ("Shift + Print", "Screenshot Selected Area (Slurp)"),
    ],
    "🪟 Window Controls": [
        ("Alt + Q", "Close Focused Window"),
        ("Alt + \\", "Maximize Window in Workspace (Keep Bar)"),
        ("Alt + A", "Toggle Floating Window"),
        ("Alt + C", "Center Floating Window"),
        ("Alt + Middle Click", "Fullscreen Window (Entire Screen)"),
        ("Super + D", "Show Desktop (Toggle Windows)"),
        ("Alt + Tab", "Quick Switch / Jump Window"),
        ("Super + Tab", "Focus Next Window"),
        ("Alt + Arrows", "Focus Window Directionally"),
        ("Super + Shift + Arrows", "Move Window Directionally"),
        ("Super + Left Click", "Drag / Move Window"),
        ("Super + Right Click", "Resize Window with Mouse"),
    ],
    "📐 Sizing & Layout": [
        ("Alt + X", "Cycle Window Width Ratio (50% / 80% / 100%)"),
        ("Alt + E", "Full Width Ratio (100%)"),
        ("Ctrl + Alt + Arrows", "Resize Floating Window (50px)"),
        ("Ctrl + Shift + Arrows", "Move Floating Window (50px)"),
        ("Super + N", "Cycle Layout (Tile / Scroller / Dwindle)"),
        ("Alt + Shift + X", "Increase Window Gaps"),
        ("Alt + Shift + Z", "Decrease Window Gaps"),
        ("Alt + Shift + R", "Toggle Gaps On/Off"),
    ],
    "🏷️ Workspaces (Tags)": [
        ("Alt + 1..9", "Switch to Workspace 1-9"),
        ("Ctrl + 1..9", "Send Window to Workspace 1-9"),
        ("Super + Left / Right", "Switch to Prev / Next Workspace"),
        ("Ctrl + Super + Left / Right", "Move Window to Prev / Next Workspace"),
    ],
    "🔊 Audio & Media": [
        ("Volume Up", "Increase Volume (+5%)"),
        ("Volume Down", "Decrease Volume (-5%)"),
        ("Mute", "Toggle Audio Mute"),
        ("Mangobar Audio Module", "Scroll: Volume | Click: Audio Menu"),
        ("Mangobar Backlight", "Scroll: Brightness | Click: Brightness Menu"),
    ],
    "🎮 Gaming & Overlays": [
        ("Super + L", "Toggle Game Mode (Locks Compositor Keys)"),
        ("Right Shift + F12", "Toggle MangoHud Performance Overlay"),
        ("Steam Launch Options", "mangohud %command%"),
    ]
}

def rofi_menu(prompt, items):
    input_str = "\n".join(items)
    res = subprocess.run(
        ["rofi", "-dmenu", "-p", prompt, "-theme-str", "listview { lines: 8; require-input: false; } window { width: 620px; }"],
        input=input_str,
        text=True,
        capture_output=True
    )
    return res.stdout.strip()

def main():
    while True:
        # Build main menu: 6 categories at top, followed by all shortcuts for search
        menu_items = list(CATEGORIES.keys())
        for cat, items in CATEGORIES.items():
            for key, desc in items:
                menu_items.append(f"{key:<24} ❯ {desc}")

        choice = rofi_menu("Keybinds ❯ ", menu_items)
        if not choice:
            break

        # If user picked a category, open category sub-menu
        if choice in CATEGORIES:
            cat_items = ["⬅ Back"]
            for key, desc in CATEGORIES[choice]:
                cat_items.append(f"{key:<24} ❯ {desc}")
            sub_choice = rofi_menu(f"{choice.split()[-1]} ❯ ", cat_items)
            if sub_choice == "⬅ Back":
                continue
            else:
                break
        else:
            # User selected a specific shortcut or searched directly
            break

if __name__ == "__main__":
    main()
