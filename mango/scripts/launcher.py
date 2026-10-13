#!/usr/bin/env python3
import os
import sys
import glob
import subprocess

DESKTOP_DIRS = [
    os.path.expanduser("~/.local/share/applications"),
    "/usr/share/applications"
]

CURRENT_DESKTOP = os.environ.get("XDG_CURRENT_DESKTOP", "mango").lower()

def load_apps():
    apps = {}
    seen = set()
    for d in DESKTOP_DIRS:
        for f in glob.glob(os.path.join(d, "*.desktop")):
            bid = os.path.basename(f)
            if bid in seen:
                continue
            seen.add(bid)
            try:
                with open(f, "r", encoding="utf-8", errors="ignore") as fp:
                    lines = fp.read().splitlines()
                in_entry = False
                nodisplay = False
                name, exec_cmd, icon, cats = "", "", "", []
                only_show, not_show = [], []
                for line in lines:
                    line = line.strip()
                    if line == "[Desktop Entry]":
                        in_entry = True
                    elif line.startswith("[") and line.endswith("]"):
                        in_entry = False
                    if not in_entry:
                        continue
                    if line.startswith("NoDisplay=") and line.split("=")[1].strip().lower() == "true":
                        nodisplay = True
                        break
                    elif line.startswith("Name=") and not name:
                        name = line.split("=", 1)[1].strip()
                    elif line.startswith("Exec=") and not exec_cmd:
                        exec_cmd = line.split("=", 1)[1].strip()
                    elif line.startswith("Icon=") and not icon:
                        icon = line.split("=", 1)[1].strip()
                    elif line.startswith("Categories=") and not cats:
                        cats = [c.strip() for c in line.split("=", 1)[1].split(";") if c.strip()]
                    elif line.startswith("OnlyShowIn="):
                        only_show = [x.lower() for x in line.split("=", 1)[1].split(";") if x.strip()]
                    elif line.startswith("NotShowIn="):
                        not_show = [x.lower() for x in line.split("=", 1)[1].split(";") if x.strip()]
                if nodisplay:
                    continue
                if only_show and CURRENT_DESKTOP not in only_show and "all" not in only_show:
                    continue
                if not_show and (CURRENT_DESKTOP in not_show or "all" in not_show):
                    continue
                if name and exec_cmd:
                    apps[bid] = {
                        "name": name,
                        "exec": exec_cmd,
                        "cats": set(cats),
                        "id": bid,
                        "file": f
                    }
            except Exception:
                pass
    return apps

def launch(target, apps):
    if not target:
        return
    # If desktop id or matching app name
    target_lower = target.lower()
    for bid, app in apps.items():
        if target_lower == app["name"].lower() or target_lower == bid.lower() or target_lower in app["name"].lower():
            subprocess.Popen(["gtk-launch", bid], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            return
    # Otherwise try running directly
    try:
        subprocess.Popen(target, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except Exception:
        pass

def main():
    apps = load_apps()

    # Add custom entries if not present
    custom_system = {
        "wallpaper-picker.desktop": {
            "name": "Wallpaper Picker",
            "exec": "python3 ~/.config/wallpaper-picker/picker.py",
            "cats": {"Settings"},
            "id": "wallpaper-picker.desktop",
            "file": ""
        }
    }
    apps.update(custom_system)

    category_map = {
        "🎮 Games": [],
        "📱 Apps": [],
        "💻 Development": [],
        "⚙️ System": []
    }

    for bid, app in apps.items():
        name = app["name"]
        cats = app["cats"]
        bid_low = bid.lower()
        if "game" in bid_low or "steam" in bid_low or "prism" in bid_low or "sunshine" in bid_low or "Game" in cats:
            category_map["🎮 Games"].append((name, bid))
        elif "Development" in cats or "IDE" in cats or "TerminalEmulator" in cats or name in ["Antigravity 2.0", "Neovim", "kitty", "btop++"]:
            category_map["💻 Development"].append((name, bid))
        elif "Settings" in cats or "System" in cats or "FileManager" in cats or name in ["Dolphin", "Wallpaper Picker", "OpenRGB", "NVIDIA X Server Settings"]:
            category_map["⚙️ System"].append((name, bid))
        else:
            category_map["📱 Apps"].append((name, bid))

    # Sort items within each category
    for cat in category_map:
        category_map[cat].sort(key=lambda x: x[0].lower())

    cat_keys = list(category_map.keys())
    cat_menu_input = "\n".join(cat_keys)

    # Level 1: Category selection
    res = subprocess.run(
        ["fuzzel", "--dmenu", "--lines=4", "--prompt=Category > "],
        input=cat_menu_input,
        text=True,
        capture_output=True
    )

    choice = res.stdout.strip()
    if not choice:
        return

    # Check if choice is one of the 4 categories
    matched_cat = None
    for k in cat_keys:
        if choice == k or choice.lower() in k.lower():
            matched_cat = k
            break

    if matched_cat:
        items = category_map[matched_cat]
        if not items:
            return
        item_names = [name for name, _ in items]
        lines_count = min(len(item_names), 4)
        sub_res = subprocess.run(
            ["fuzzel", "--dmenu", f"--lines={lines_count}", f"--prompt={matched_cat.split()[-1]} > "],
            input="\n".join(item_names),
            text=True,
            capture_output=True
        )
        selected_app = sub_res.stdout.strip()
        if selected_app:
            launch(selected_app, apps)
    else:
        # User typed an app name directly on level 1
        launch(choice, apps)

if __name__ == "__main__":
    main()
