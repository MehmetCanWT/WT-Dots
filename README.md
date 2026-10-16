# WT-Dots

Modular dotfiles for **KDE Plasma 6 (Wayland)** featuring a floating 3-island **Waybar**, a fast companion helper & wallpaper selector written in **100% Pure Rust** (`wt-rust`), and a matching **Rofi** launcher.

---

## Architecture & Features

- **Waybar (KDE Plasma 6)**:
  - 3-island floating pill layout (`#111116` dark background with dynamic accent border and shadow).
  - Desktop & fullscreen friendly (`layer: "background"`, `exclusive: false`) — stays visible on `Meta+D` (Show Desktop) while windows smoothly cover the bar when maximized.
  - Left island: CPU utilization + core temp, GPU utilization + temp (`wt-rust gpu`), RAM usage, and active workspace dots (`● ○`).
  - Center island: Day & time clock.
  - Right island: System tray, audio volume (`wt-rust volume`, capped at 100%), backlight (`wt-rust brightness`, 1% hard floor + KDE DBus sync), and battery/power profile status (`wt-rust power`).
- **`wt-rust` (`WT-Rust`)**:
  - Zero-bloat, single-binary Rust companion replacing all external shell/Python scripts:
  - `wt-rust wallpaper [select|apply <path>]`: Hardware-accelerated (`eframe`/`egui` + Wayland + Glow) zero-text fullscreen stepped-carousel wallpaper selector with centered 4-color theme swatch preview, smooth background crossfade, and multi-core RAM/disk caching (`~/.cache/wt-rust/`).
  - `wt-rust theme [get <path> | apply [path]]`: Fast (<1ms) dominant HSL palette extraction and full-system theme synchronization across KDE Plasma 6 (desktop accent & window decorations), GTK 3 & GTK 4, Kitty terminal (`current-theme.conf` with 16-color ANSI harmony), Waybar (`theme.css`), Rofi (`theme.rasi`), and Fastfetch logo accent.
  - `wt-rust keybinds [watch]`: Syncs `~/.config/keybinds.conf` into KDE Plasma 6 global shortcuts (`kglobalshortcutsrc`) and cleans up removed bindings automatically.
  - `wt-rust gpu`: NVIDIA GPU utilization and temperature JSON output for Waybar.
  - `wt-rust workspaces [next|prev]`: KWin `VirtualDesktopManager` DBus status and desktop switching.
  - `wt-rust brightness [toggle|inc|dec]`: KDE PowerManagement DBus + sysfs backlight control with a strict 1% minimum floor.
  - `wt-rust volume [inc|dec|mute|status]`: PipeWire (`wpctl`) volume control strictly capped at 100%.
  - `wt-rust power [status|next|prev|set|get]`: Battery telemetry and KDE / `power-profiles-daemon` profile switching.
- **Rofi**:
  - Centered modern dark launcher (`drun` + `calc`) styled with dynamic accent colors.

---

## Directory Structure

```
WT-Dots/
├── waybar/
│   ├── config.jsonc    # Waybar layout & module bindings
│   ├── style.css       # 3-island CSS styling & color variables
│   └── theme.css       # Dynamic theme override palette
├── WT-Rust/            # Pure Rust companion binary & GUI wallpaper selector
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src/
│       ├── main.rs
│       ├── audio.rs
│       ├── keybinds.rs
│       ├── power.rs
│       ├── theme.rs
│       ├── wallpaper.rs
│       └── waybar.rs
├── rofi/
│   ├── config.rasi     # Rofi settings & keybindings
│   └── theme.rasi      # Custom Rofi theme (dynamically updated by wt-rust)
├── keybinds.conf       # Declarative KDE Plasma 6 global shortcuts
├── road.md             # Project roadmap & engineering log
└── README.md
```

---

## Requirements

- **KDE Plasma 6** on Wayland
- **Waybar**
- **Rust & Cargo** (`rustc`, `cargo`)
- **Rofi** (`rofi-wayland`, `rofi-calc`)
- **brightnessctl** (fallback for backlight control)
- **wireplumber** (`wpctl` for PipeWire audio)
- **NVIDIA Proprietary Drivers** (`nvidia-smi` for GPU metrics)
- **Font**: `0xProto Nerd Font`

---

## Installation & Setup

### 1. Build and Install `wt-rust`

```bash
cd WT-Rust
cargo build --release
install -Dm755 target/release/wt-rust ~/.local/bin/wt-rust
```

Make sure `~/.local/bin` is in your `$PATH`.

### 2. Sync Configuration Files

```bash
# Waybar
mkdir -p ~/.config/waybar
cp -r waybar/* ~/.config/waybar/

# Rofi
mkdir -p ~/.config/rofi
cp -r rofi/* ~/.config/rofi/

# Keybindings
cp keybinds.conf ~/.config/keybinds.conf
wt-rust keybinds
```

### 3. Waybar Autostart (systemd user service)

```bash
mkdir -p ~/.config/systemd/user
cat << 'EOF' > ~/.config/systemd/user/waybar.service
[Unit]
Description=Waybar
PartOf=graphical-session.target
After=graphical-session.target

[Service]
ExecStart=/usr/bin/waybar
Restart=on-failure

[Install]
WantedBy=graphical-session.target
EOF

systemctl --user daemon-reload
systemctl --user enable --now waybar.service
```

---

## License

MIT
