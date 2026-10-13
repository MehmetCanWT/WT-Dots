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


FALLBACK_ICONS = {
    "steam": "steam,applications-games",
    "prism_launcher": "prism_launcher,applications-games",
    "sunshine": "dev.lizardbyte.app.Sunshine,applications-games",
    "antigravity": "antigravity,utilities-terminal,applications-development",
    "nvim": "nvim,neovim,text-editor",
    "kitty": "kitty,utilities-terminal",
    "btop": "btop,utilities-terminal,system-monitor",
    "firefox": "firefox,web-browser",
    "obsidian": "obsidian,text-editor",
    "vlc": "vlc,multimedia-player",
    "bitchord": "bitchord,audio-player",
    "localsend": "localsend,network-workgroup",
    "libreoffice-writer": "libreoffice-writer,x-office-document",
    "dolphin": "org.kde.dolphin,system-file-manager",
    "wallpaper-picker": "preferences-desktop-wallpaper,preferences-desktop",
    "nvidia-settings": "nvidia-settings,preferences-system",
    "openrgb": "openrgb,preferences-system"
}

def resolve_icon(icon_str, bid):
    if not icon_str:
        clean_key = bid.replace(".desktop", "").lower()
        return FALLBACK_ICONS.get(clean_key, "application-x-executable")
    
    # Check if absolute path without extension exists as .png
    if icon_str.startswith("/") and not (icon_str.endswith(".png") or icon_str.endswith(".svg")):
        if os.path.exists(icon_str + ".png"):
            return icon_str + ".png"
    if os.path.exists(icon_str):
        return icon_str
    
    clean_name = os.path.basename(icon_str).replace(".png", "").replace(".svg", "")
    return f"{icon_str},{clean_name}"

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
                        "icon": icon,
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
    target_lower = target.lower()
    for bid, app in apps.items():
        if target_lower == app["name"].lower() or target_lower == bid.lower() or target_lower in app["name"].lower():
            subprocess.Popen(["gtk-launch", bid], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            return
    try:
        subprocess.Popen(target, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except Exception:
        pass

def main():
    apps = load_apps()

    custom_system = {
        "wallpaper-picker.desktop": {
            "name": "Wallpaper Picker",
            "exec": "python3 ~/.config/wallpaper-picker/picker.py",
            "icon": "preferences-desktop-wallpaper",
            "cats": {"Settings"},
            "id": "wallpaper-picker.desktop",
            "file": ""
        }
    }
    apps.update(custom_system)

    category_map = {
        "Games": [],
        "Apps": [],
        "Development": [],
        "System": []
    }

    for bid, app in apps.items():
        name = app["name"]
        cats = app["cats"]
        bid_low = bid.lower()
        if "game" in bid_low or "steam" in bid_low or "prism" in bid_low or "sunshine" in bid_low or "Game" in cats:
            category_map["Games"].append((name, bid))
        elif "Development" in cats or "IDE" in cats or "TerminalEmulator" in cats or name in ["Antigravity 2.0", "Neovim", "kitty", "btop++"]:
            category_map["Development"].append((name, bid))
        elif "Settings" in cats or "System" in cats or "FileManager" in cats or name in ["Dolphin", "Wallpaper Picker", "OpenRGB", "NVIDIA X Server Settings"]:
            category_map["System"].append((name, bid))
        else:
            category_map["Apps"].append((name, bid))

    for cat in category_map:
        category_map[cat].sort(key=lambda x: x[0].lower())

    cat_keys = list(category_map.keys())

    # Put 4 categories first (occupies initial 4 lines, no icons)
    main_menu_lines = list(cat_keys)

    # Followed by all apps with their icons for instant fuzzy search
    for bid, app in sorted(apps.items(), key=lambda x: x[1]["name"].lower()):
        app_icon = resolve_icon(app.get("icon", ""), bid)
        main_menu_lines.append(f"{app['name']}\x00icon\x1f{app_icon}")

    main_menu_input = "\n".join(main_menu_lines)

    # Level 1: Launcher
    res = subprocess.run(
        ["fuzzel", "--dmenu", "--lines=4", "--prompt=> "],
        input=main_menu_input,
        text=True,
        capture_output=True
    )

    choice = res.stdout.strip()
    if not choice:
        return

    # If user selected an exact category
    if choice in cat_keys:
        items = category_map[choice]
        if not items:
            return
        item_lines = []
        for name, bid in items:
            app_icon = resolve_icon(apps[bid].get("icon", ""), bid)
            item_lines.append(f"{name}\x00icon\x1f{app_icon}")

        lines_count = min(len(item_lines), 4)
        sub_res = subprocess.run(
            ["fuzzel", "--dmenu", f"--lines={lines_count}", f"--prompt={choice} > "],
            input="\n".join(item_lines),
            text=True,
            capture_output=True
        )
        selected_app = sub_res.stdout.strip()
        if selected_app:
            launch(selected_app, apps)
    else:
        # User selected an application directly or typed a search query
        launch(choice, apps)

if __name__ == "__main__":
    main()
