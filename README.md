# WT-Dots

Wayland desktop configuration for **MangoWM (0.18)** on Fedora Linux. Includes dynamic color extraction from wallpapers, a standalone PySide6 carousel picker, companion status bar (`mangobar`), and native application bindings.

---

## Quick Install

Run this in your terminal:

```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/MehmetCanWT/WT-Dots/main/install.sh)"
```

---

## Default Software Stack

| Role | Software | Notes |
|---|---|---|
| **Compositor** | [MangoWM](https://github.com/DreamMaoMao/mango) (0.18) | Tiling Wayland compositor with layout support (Tile, Scroller, Dwindle) |
| **Status Bar** | [mangobar](https://github.com/DreamMaoMao/mangobar) | Native layer-shell bar (IPC-connected workspaces, clock, audio, backlight, tray) |
| **Terminal** | [Kitty](https://sw.kovidgoyal.net/kitty/) | 16 ANSI colors auto-generated from wallpaper, 0xProto font |
| **File Manager** | [Dolphin](https://apps.kde.org/dolphin/) | Fast Qt6 file manager, split view, wallpaper accent color integration |
| **App Launcher** | [Fuzzel](https://codeberg.org/dnkl/fuzzel) | Curved borders, `hide-before-typing=yes`, max 4 results |
| **Notifications**| [Mako](https://github.com/emersion/mako) | Border & background colors synced with active palette |
| **Wallpaper & Theme** | Custom PySide6 / QML | Fullscreen 3D coverflow picker (`Super + W`), 64-color quantization engine |
| **Screenshots** | `grim` + `slurp` + `wl-copy` | Saves directly to `~/Resimler/Screenshots/` and copies to clipboard |
| **Backlight** | `brightnessctl` | Scroll on bar to adjust; click to open preset menu |
| **Audio** | WirePlumber (`wpctl`) | Scroll on bar for volume; click to select default speaker/mic |

---

## Keybindings

### Applications & System
| Key | Action |
|---|---|
| `Alt + Return` | Open Terminal (`kitty`) |
| `Alt + Space` | Application Launcher (`fuzzel`) |
| `Super + E` | Open File Manager (`dolphin`) |
| `Super + W` | Open Wallpaper Carousel Picker |
| `Super + K` | Keybindings Cheatsheet Popup |
| `Super + L` | Toggle Game Mode (inhibits compositor keys, keeps `Alt+Tab`) |
| `Super + D` | Toggle Show / Hide Desktop |
| `Super + R` | Reload MangoWM Configuration |
| `Super + M` | Exit MangoWM |

### Window Management
| Key | Action |
|---|---|
| `Alt + Q` | Close focused window |
| `Alt + Tab` | Switch window / Jump |
| `Alt + \` | Toggle floating window |
| `Alt + A` | Toggle maximize screen |
| `Alt + Middle Click` | Toggle fullscreen |
| `Alt + ← / → / ↑ / ↓` | Move focus directionally |
| `Super + Shift + Arrows` | Move window directionally |
| `Super + N` | Switch layout mode (Tile, Scroller, Dwindle) |

### Workspaces (Tags)
| Key | Action |
|---|---|
| `Alt + 1..9` | Switch to workspace 1–9 |
| `Ctrl + 1..9` | Move window to workspace 1–9 |
| `Super + ← / →` | Previous / Next workspace |

### Media & Utility
| Key | Action |
|---|---|
| `Print` | Fullscreen screenshot ➔ `~/Resimler/Screenshots/` & clipboard |
| `Shift + Print` | Area screenshot ➔ `~/Resimler/Screenshots/` & clipboard |
| `Volume Keys` | Volume ±5% / Mute toggle |

---

## Status Bar Controls

The status bar (`mangobar`) provides quick mouse and menu controls:

- **Power Button (``)**: Left click opens power menu (Shutdown, Reboot, Suspend, Logout).
- **Workspaces (`1..9`)**: Left click to switch, right click to toggle.
- **Audio Module (``)**:
  - Scroll wheel: Volume up / down.
  - Right click: Mute toggle.
  - Left / Middle click: Audio device menu (select default speaker or microphone).
- **Backlight Module (`󰃠`)**:
  - Scroll wheel: Brightness ±5%.
  - Left / Middle click: Brightness preset menu (100%, 80%, 60%, 40%, 20%, 5%).

---

## Repository Layout

```
WT-Dots/
├── install.sh                  # One-line installer script
├── mango/
│   ├── config.conf             # Master loader
│   ├── system.conf             # Hardware, displays, rules & autostart
│   ├── theme.conf              # Borders, animations & theme colors
│   ├── keybinds.conf           # Keyboard & mouse shortcuts
│   ├── gamemode.toml           # Gaming submap
│   └── scripts/
│       ├── audio_menu.py       # WirePlumber device selector
│       ├── brightness_menu.sh  # Brightness preset menu
│       ├── keybinds_cheatsheet.sh # Interactive cheatsheet
│       ├── powermenu.sh        # System power actions
│       └── toggle_desktop.py   # Desktop toggle script
├── mangobar/
│   ├── config.jsonc            # Bar modules configuration
│   └── style.css               # Bar CSS styles
├── fuzzel/
│   └── fuzzel.ini              # Launcher configuration (filter-desktop=yes)
├── mako/
│   └── config                  # Notification daemon style
├── kitty/
│   └── kitty.conf              # Terminal configuration
├── wallpaper-picker/
│   ├── picker.py               # Singleton wallpaper carousel application
│   ├── picker.qml              # 3D coverflow QML interface
│   └── theme.py                # 64-color quantization & palette generator
├── environment.d/
│   └── 10-theme.conf           # System-wide dark mode variables
├── gtk-3.0/
│   ├── gtk.css                 # Dynamic GTK3 accent colors
│   └── settings.ini            # GTK3 dark theme settings
└── gtk-4.0/
    ├── gtk.css                 # Libadwaita @define-color overrides (Nautilus)
    └── settings.ini            # GTK4 dark theme settings
```

---

## Manual Dependencies (Fedora)

If you prefer to install packages manually:

```bash
sudo dnf install -y fuzzel mako swaybg kitty nautilus grim slurp \
  brightnessctl wireplumber python3-pyside6 qt6-qtdeclarative \
  python3-pillow ffmpeg
```

Deploy configs:
```bash
cp -r mango mangobar fuzzel mako kitty wallpaper-picker environment.d gtk-3.0 gtk-4.0 ~/.config/
chmod +x ~/.config/mango/scripts/* ~/.config/wallpaper-picker/*.py
```

---

## License

MIT
