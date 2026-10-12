# WT-Dots 🥭

Elegant, minimal, and responsive dotfiles for **MangoWM (0.18)** on Wayland, tailored for Linux with Turkish locale directory structures (`~/Resimler`, `~/İndirilenler`).

---

## ✨ Features

- **Modular Configuration**: Clean separation of concerns:
  - `system.conf` - Input hardware, mouse acceleration profile (flat 1:1), 144Hz monitor settings, layouts, and autostart.
  - `theme.conf` - Colors, borders, curves, gaps, and window animations.
  - `keybinds.conf` - Shortcuts, media controls, and mouse bindings.
  - `gamemode.toml` - Inhibit desktop keybindings while playing games.
- **Native Status Bar (`mangobar`)**: Custom-built solid minimalist status bar directly connected to MangoWM via native IPC (workspaces, tray, network, audio, battery, clock).
- **Cinematic Wallpaper Picker (`Super+W`)**:
  - Fullscreen fluid 3D-feel cover-flow carousel built with PySide6 & QML.
  - Real-time dominant color palette extraction (k-means quantization with PIL).
  - Instantly syncs colors across MangoWM borders, `mangobar`, `fuzzel`, `mako`, and `kitty`.
  - Supports image and video wallpapers.
  - Interactive controls via **A / D**, **← / →**, mouse scroll, or on-screen arrows.
- **Application Launcher (`fuzzel`)**: Dark themed, curved borders, with full desktop filtering (`XDG_CURRENT_DESKTOP`) to keep foreign desktop environment entries hidden.
- **Game Mode (`Super+F12`)**: Toggles a dedicated keymode submap that disables compositor shortcuts so games receive pure keyboard inputs (including Alt).

---

## ⌨️ Essential Keybindings

| Shortcut | Action |
|---|---|
| `Alt + Space` | Open Application Launcher (`fuzzel`) |
| `Alt + Return` | Open Terminal (`kitty`) |
| `Alt + Q` | Close Active Window |
| `Super + W` | Open Wallpaper Carousel Picker |
| `Super + F12` | Toggle Game Mode (inhibit / restore shortcuts) |
| `Alt + Middle Click` | Toggle Fullscreen |
| `Super + R` | Reload MangoWM Configuration |
| `Super + M` | Exit MangoWM Session |
| `Ctrl + [1-9]` | Switch to Tag 1-9 |
| `Alt + [1-9]` | Move Window to Tag 1-9 |
| `Print` | Copy Fullscreen to Clipboard |
| `Shift + Print` | Copy Selected Region to Clipboard |
| `Ctrl + Print` | Save Region to `~/Resimler/Ekran Görüntüleri/` |

---

## 📁 Repository Structure

```
WT-Dots/
├── mango/
│   ├── config.conf       # Master loader (sources sub-configs)
│   ├── system.conf       # Hardware, displays, rules & autostart
│   ├── theme.conf        # Borders, animations & theme colors
│   ├── keybinds.conf     # Hotkeys and mouse bindings
│   └── gamemode.toml     # Gaming submap
├── mangobar/
│   ├── config.jsonc      # Status bar modules
│   └── style.css         # Solid CSS theme
├── fuzzel/
│   └── fuzzel.ini        # Application launcher configuration
├── mako/
│   └── config            # Notification daemon style
└── wallpaper-picker/
    ├── picker.py         # PySide6 wallpaper carousel application
    ├── picker.qml        # QML 3D coverflow UI
    └── theme.py          # Dynamic palette & theme generator
```

---

## 🚀 Installation & Requirements

### Dependencies (Fedora)

```bash
sudo dnf install -y fuzzel mako swaybg kitty grim slurp \
  python3-pyside6 qt6-qtdeclarative python3-pillow ffmpeg
```

To build `mangobar`:
```bash
sudo dnf install -y meson ninja-build git cjson-devel \
  wayland-protocols-devel fcft-devel tllist-devel pulseaudio-libs-devel
```

### Setup

```bash
# Clone the repository
git clone https://github.com/<your-username>/WT-Dots.git ~/Code/WT-Dots

# Deploy configuration directories
mkdir -p ~/.config
cp -r ~/Code/WT-Dots/mango ~/.config/
cp -r ~/Code/WT-Dots/mangobar ~/.config/
cp -r ~/Code/WT-Dots/fuzzel ~/.config/
cp -r ~/Code/WT-Dots/mako ~/.config/
cp -r ~/Code/WT-Dots/wallpaper-picker ~/.config/

# Place wallpapers into your pictures folder
mkdir -p ~/Resimler/Wallpapers
mkdir -p ~/Resimler/'Ekran Görüntüleri'
```

---

## 🎨 Wallpaper Directory
Place all your wallpapers (PNG, JPG, WEBP, or MP4) in:
```
~/Resimler/Wallpapers/
```
Press `Super + W` to browse, preview dynamic palettes, and hit `Enter` to apply.
