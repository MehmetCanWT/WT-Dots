# WT-Dots

Wayland desktop configuration for **MangoWM (0.18)** on **Fedora Linux** & **Arch Linux**. Includes dynamic wallpaper color extraction, a standalone PySide6 carousel picker, companion status bar (`mangobar`), clipboard history with screenshot support, Steam-style MangoHud performance overlay, and native application shortcuts.

---

## Quick Install

### Fedora Linux
```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/MehmetCanWT/WT-Dots/main/install.sh)"
```

### Arch Linux
```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/MehmetCanWT/WT-Dots/main/install.sh)"
```

### Manual Installation
```bash
git clone https://github.com/MehmetCanWT/WT-Dots.git ~/Code/WT-Dots
cd ~/Code/WT-Dots
chmod +x install.sh
./install.sh
```

---

## Default Software Stack

| Role | Software | Notes |
|---|---|---|
| **Compositor** | [MangoWM](https://github.com/DreamMaoMao/mango) (0.18) | Tiling Wayland compositor with layout support (Tile, Scroller, Dwindle) |
| **Status Bar** | [mangobar](https://github.com/DreamMaoMao/mangobar) | Native layer-shell bar (workspaces, clock, audio, backlight, tray) |
| **Terminal** | [Kitty](https://sw.kovidgoyal.net/kitty/) | 16 ANSI colors auto-generated from wallpaper, 0xProto Nerd Font |
| **File Manager** | [Dolphin](https://apps.kde.org/dolphin/) | Fast Qt6 file manager, split view, wallpaper accent color integration |
| **App Launcher** | [Rofi](https://github.com/davatorium/rofi) | Centered popup, instant search, calculator integration (`rofi-calc`) |
| **Clipboard** | [cliphist](https://github.com/sentriz/cliphist) + `wl-clipboard` | Persistent text and screenshot image history (`Super + V`) |
| **Notifications**| [Mako](https://github.com/emersion/mako) | Border & background colors synced with active palette |
| **Wallpaper & Theme** | Custom PySide6 / QML | Fullscreen 3D coverflow picker (`Super + W`), 64-color quantization engine |
| **Game Overlay** | [MangoHud](https://github.com/flightlessmango/MangoHud) | Steam Deck-style horizontal performance bar (FPS, RTX 3050 stats, RAM) |
| **Screenshots** | `grim` + `slurp` + `wl-copy` | Fullscreen (`Print`) and region (`Shift + Print`) copied to clipboard & saved |
| **Backlight** | `brightnessctl` | Scroll on bar to adjust; click to open preset menu |
| **Audio** | WirePlumber (`wpctl`) | Multi-level interactive speaker & microphone switcher |

---

## Keybindings

### Applications & System
| Key | Action |
|---|---|
| `Alt + Return` | Open Terminal (`kitty`) |
| `Alt + Space` | Application Launcher & Calculator (`rofi`) |
| `Super + E` | Open File Manager (`dolphin`) |
| `Super + V` | Clipboard History (`cliphist` + `rofi`, text & screenshots) |
| `Super + W` | Open Wallpaper Carousel Picker |
| `Super + K` | Keybindings Cheatsheet Modal (`rofi`) |
| `Super + L` | Toggle Game Mode (inhibits compositor keys, keeps `Alt+Tab`) |
| `Super + D` | Toggle Show / Hide Desktop |
| `Super + R` | Reload MangoWM Configuration |
| `Super + M` | Exit MangoWM |

### Window Management
| Key | Action |
|---|---|
| `Alt + Q` | Close focused window |
| `Alt + Tab` | Switch window / Jump |
| `Alt + \` | Maximize window in workspace (preserves top bar) |
| `Alt + A` | Toggle floating window |
| `Alt + C` | Center floating window |
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
| `Right Shift + F12` | Toggle MangoHud Performance Overlay |
| `Volume Keys` | Volume ±5% / Mute toggle |

---

## License

MIT
