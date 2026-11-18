# WT-Dots (Hyprland)

Personal **Hyprland 0.57+ (Native Lua)** dotfiles featuring **WT-Rust**, a pure-Rust companion suite for dynamic wallpaper theming, control center popups, and live synced lyrics.

## Structure

- `hyprland/hypr/` — Hyprland Lua config (`hyprland.lua`, `configs/keybinds.lua`, `configs/rules.lua`, `configs/theme.lua`), `hyprpaper.conf`, `hypridle.conf`, `hyprlock.conf`, and helper scripts (`screenshot.sh`, `show-desktop.sh`, `media-step.sh`).
- `hyprland/waybar/` — HANCORE `V1.9c`-inspired Waybar layout (`config.jsonc`, `style.css`, dynamic `colors.css`).
- `hyprland/rofi/` — HyDE `style_10`-inspired Rofi Spotlight launcher with pre-scaled live wallpaper header (`config.rasi`, dynamic `theme.rasi`).
- `hyprland/mako/` — Mako notification daemon configuration (`config`, dynamic `colors`).
- `hyprland/ghostty/` — Ghostty terminal config and dynamic theme.
- `hyprland/bin/game-run` — NVIDIA PRIME + Vulkan offload launcher for hybrid iGPU/dGPU laptops.
- `WT-Rust/` — Pure-Rust companion (`wt-rust`):
  - `wallpaper select` — GPU-accelerated wallpaper carousel (`eframe`/`egui`).
  - `theme apply <img_path>` — Extracts palette and syncs Hyprland, Waybar, Rofi, Mako, Hyprlock, Ghostty, Kitty, and GTK3/GTK4.
  - `popup <volume|wifi|bluetooth|power>` — Top-right/top-left Bento control center popups.
  - `music --waybar` — Event-driven MPRIS + LRCLIB synced lyrics with character-level glow wave.
  - `btop-stats --waybar` — Lightweight CPU/RAM telemetry for Waybar.
- `fonts/0xProto/` — Bundled `0xProto Nerd Font`.
- `Wallpapers/` — Bundled desktop wallpapers.

## Installation

```bash
git clone https://github.com/MehmetCanWT/WT-Dots.git ~/WT-Dots
cd ~/WT-Dots && ./install.sh
```

## Keybindings (`ALT` as main modifier)

| Keybind | Action |
| :--- | :--- |
| `Alt + Space` | Rofi app launcher |
| `Alt + Enter` | Ghostty terminal |
| `Alt + E` | Dolphin file manager |
| `Alt + B` | Zen Browser |
| `Alt + W` | `wt-rust` wallpaper selector |
| `Alt + D` | Show desktop toggle |
| `Alt + V` / `Alt + F` | Toggle floating / Fullscreen |
| `Alt + Q` | Close active window |
| `Alt + L` / `Alt + M` | Lock (`hyprlock`) / Graceful exit (`hyprshutdown`) |
| `Print` / `Shift + Print` | Region / Full screenshot (`~/Pictures/Screenshots` + clipboard) |
