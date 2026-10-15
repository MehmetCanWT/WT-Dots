# WT-Dots

Modular dotfiles for **KDE Plasma 6 (Wayland)** featuring a floating 3-island **Waybar**, a fast companion helper written in **Rust** (`wb-helper`), and a matching **Rofi** launcher.

---

## Architecture & Features

- **Waybar (KDE Plasma 6)**:
  - 3-island floating pill layout (`#111116` dark background with subtle border and shadow).
  - Maximized & fullscreen friendly (`layer: "bottom"`, `exclusive: false`) — windows smoothly cover the bar without ghost struts.
  - Left island: CPU utilization + core temp, GPU utilization + temp (via `wb-helper`), RAM usage, and active workspace dots (`● ○`).
  - Center island: Day & time clock.
  - Right island: System tray, audio volume (click to mute, scroll to adjust), backlight (click to toggle 50%/100%, scroll to adjust), and battery status.
- **`wb-helper` (Rust Helper)**:
  - High-performance, low-overhead binary replacing slow shell/python scripts.
  - `wb-helper gpu`: NVIDIA GPU utilization and temperature querying.
  - `wb-helper workspaces [next|prev]`: KWin `VirtualDesktopManager` DBus reader with UTF-8 JSON output for Waybar, plus desktop switching.
  - `wb-helper brightness toggle`: Instant 50% / 100% brightness stepping.
- **Rofi**:
  - Centered modern dark launcher styled with custom theme.

---

## Directory Structure

```
WT-Dots/
├── waybar/
│   ├── config.jsonc    # Waybar layout & module bindings
│   ├── style.css       # 3-island CSS styling & color variables
│   └── theme.css       # Dynamic theme override palette
├── wb-helper/          # Companion Rust helper source
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src/
│       └── main.rs
├── rofi/
│   ├── config.rasi     # Rofi settings & keybindings
│   └── theme.rasi      # Custom Rofi theme
└── README.md
```

---

## Requirements

- **KDE Plasma 6** on Wayland
- **Waybar**
- **Rust & Cargo** (`rustc`, `cargo`)
- **Rofi** (`rofi-wayland`)
- **brightnessctl** (for backlight control)
- **wireplumber** (`wpctl` for PipeWire audio)
- **NVIDIA Proprietary Drivers** (`nvidia-smi` for GPU metrics)
- **Font**: `0xProto Nerd Font`

---

## Installation & Setup

### 1. Build and Install `wb-helper`

```bash
cd wb-helper
cargo build --release
install -Dm755 target/release/wb-helper ~/.local/bin/wb-helper
```

Make sure `~/.local/bin` is in your `$PATH`.

### 2. Link Configuration Files

```bash
# Waybar
mkdir -p ~/.config/waybar
cp -r waybar/* ~/.config/waybar/

# Rofi
mkdir -p ~/.config/rofi
cp -r rofi/* ~/.config/rofi/
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
