# WT-Dots 🥭

Elegant, minimal, and responsive dotfiles for **MangoWM (0.18)** on Wayland, tailored for Linux with Turkish locale directory structures (`~/Resimler/Wallpapers`, `~/Resimler/Screenshots`, `~/İndirilenler`).

---

## ✨ Features

- **Modular Configuration**: Clean separation of concerns:
  - `system.conf` - Input hardware, mouse acceleration profile (flat 1:1), 144Hz monitor settings, layouts, and autostart.
  - `theme.conf` - Colors, borders, curves, gaps, and window animations.
  - `keybinds.conf` - Shortcuts, media controls, and mouse bindings.
  - `gamemode.toml` - Inhibit desktop keybindings while playing games (with `Alt+Tab` preserved).
- **Native Floating 3-Island Status Bar (`mangobar`)**: Custom-built solid minimalist status bar directly connected to MangoWM via native IPC:
  - **Left**: Power menu trigger button (``) and active workspace tags.
  - **Center**: Clean time clock.
  - **Right**: System tray, network, audio volume, and battery status.
- **Power Menu (``)**: Quick action launcher via `fuzzel --dmenu` with options to Shutdown, Reboot, Suspend, and Logout.
- **Cinematic Wallpaper Picker (`Super+W`)**:
  - Fullscreen fluid 3D-feel cover-flow carousel built with PySide6 & QML.
  - **Singleton protection**: prevents opening multiple instances concurrently.
  - **Auto-hides `mangobar`** during wallpaper selection for an immersive view.
  - Minimalist aesthetic: purely focused on visual cards, navigation arrows, and dynamic color capsules.
  - **On-demand theming**: smoothly applies new palettes and wallpapers upon selection commit (`Enter`), leaving browsing fast and lightweight.
  - Real-time dominant color palette extraction (k-means quantization with PIL).
  - Instantly syncs colors across MangoWM borders, `mangobar`, `fuzzel`, `mako`, and `kitty`.
  - Supports image and video wallpapers.
  - Interactive controls via **A / D**, **← / →**, mouse scroll, or on-screen arrows.
- **Application Launcher (`fuzzel`)**: Dark themed, curved borders, with full desktop filtering (`XDG_CURRENT_DESKTOP`) to keep foreign desktop environment entries hidden.
- **Keybindings Cheatsheet (`Super+K`)**: Instant searchable popup cheatsheet listing all active keybindings.
- **Show / Hide Desktop (`Super+D`)**: Toggle all active windows to view wallpaper cleanly.
- **Game Mode (`Super+L`)**: Toggles a dedicated keymode submap that disables compositor shortcuts for pure gaming inputs, while keeping `Alt+Tab` enabled.
- **Dolphin Dark Theme**: Dark mode support via `~/.config/kdeglobals` and BreezeDark.

---

## ⌨️ Essential Keybindings

| Shortcut | Action |
|---|---|
| `Alt + Space` | Open Application Launcher (`fuzzel`) |
| `Alt + Return` | Open Terminal (`kitty`) |
| `Alt + Q` | Close Active Window |
| `Super + W` | Open Wallpaper Carousel Picker (Singleton, hides bar) |
| `Super + K` | Keybindings Cheatsheet Popup |
| `Super + D` | Show / Hide Desktop (Toggle all windows) |
| `Alt + Tab` | Switch Window / Jump (Works in Game Mode) |
| `Super + L` | Toggle Game Mode (inhibit / restore shortcuts) |
| `Alt + Middle Click` | Toggle Fullscreen |
| `Super + R` | Reload MangoWM Configuration |
| `Super + M` | Exit MangoWM Session |
| `Ctrl + [1-9]` | Switch to Tag 1-9 |
| `Alt + [1-9]` | Move Window to Tag 1-9 |
| `Print` | Fullscreen Screenshot ➔ `~/Resimler/Screenshots/` & Clipboard |
| `Shift + Print` | Selected Region Screenshot ➔ `~/Resimler/Screenshots/` & Clipboard |

---

## 📁 Repository Structure

```
WT-Dots/
├── mango/
│   ├── config.conf       # Master loader (sources sub-configs)
│   ├── system.conf       # Hardware, displays, rules & autostart
│   ├── theme.conf        # Borders, animations & theme colors
│   ├── keybinds.conf     # Hotkeys and mouse bindings
│   ├── gamemode.toml     # Gaming submap (preserves Alt+Tab)
│   └── scripts/
│       ├── keybinds_cheatsheet.sh  # Interactive cheatsheet
│       ├── powermenu.sh            # Minimal fuzzel-powered power menu
│       └── toggle_desktop.py       # Desktop minimize/restore toggle
├── mangobar/
│   ├── config.jsonc      # Status bar modules (3-island floating setup)
│   └── style.css         # Solid dark theme CSS
├── fuzzel/
│   └── fuzzel.ini        # Application launcher configuration
├── mako/
│   └── config            # Notification daemon style
├── wallpaper-picker/
│   ├── picker.py         # PySide6 wallpaper carousel application (Singleton)
│   ├── picker.qml        # QML 3D coverflow UI
│   └── theme.py          # Dynamic palette & theme generator
├── environment.d/
│   └── 10-theme.conf     # System-wide dark mode environment variables
├── gtk-3.0/
│   └── settings.ini      # GTK 3 Breeze-Dark theme settings
└── gtk-4.0/
    └── settings.ini      # GTK 4 Breeze-Dark theme settings
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
mkdir -p ~/Resimler/Wallpapers ~/Resimler/Screenshots
```

---

## 📜 License

MIT License. Designed with care for clean, distraction-free Linux desktop workflows.
