# WT-Dots (Hyprland)

Personal **Hyprland 0.57+ (Native Lua)** dotfiles featuring **WT-Rust**, a pure-Rust companion suite for dynamic wallpaper theming, control center popups, and live synced lyrics.

## Directory Structure

```text
WT-Dots/
├── config/                 # Maps 1:1 to ~/.config/
│   ├── fontconfig/         # System-wide 0xProto Nerd Font defaults
│   ├── ghostty/            # Ghostty terminal config & dynamic theme
│   ├── gtk-3.0/            # GTK 3 settings
│   ├── gtk-4.0/            # GTK 4 settings
│   ├── hypr/               # Hyprland Lua config, hyprlock, hypridle, hyprpaper & scripts
│   │   └── configs/        # Modular Lua files (monitors, env, autostart, appearance, input, animations, keybinds, rules, theme)
│   ├── mako/               # Mako notification daemon config & dynamic colors
│   ├── rofi/               # HyDE-inspired Rofi launcher & dynamic theme
│   ├── rustfetch/          # Custom system fetch config & logos
│   └── waybar/             # HANCORE-inspired Waybar layout, style & dynamic colors
├── scripts/
│   └── game-run            # NVIDIA PRIME + Vulkan offload launcher for hybrid laptops
├── assets/
│   ├── fonts/              # Bundled 0xProto Nerd Font
│   └── wallpapers/         # Bundled desktop wallpapers
├── WT-Rust/                # Pure-Rust companion CLI (wt-rust)
└── install.sh              # Master setup & symlink script
```

## `wt-rust` Commands

- `wt-rust wallpaper [select|next|random]` — Pick/cycle wallpapers and regenerate `waybar`, `rofi`, `mako`, `ghostty`, and `hyprland` colors.
- `wt-rust theme <image>` — Extract Material/dominant colors from an image and apply them across all desktop configs.
- `wt-rust popup [wifi|bluetooth|audio|network|power|calendar|music]` — Open native `eframe`/`egui` control center popups.
- `wt-rust music` — Output JSON status with live synced lyrics (LRCLIB + NetEase) for Waybar.
- `wt-rust btop-stats` — Output live CPU/RAM/GPU telemetry JSON for Waybar.

## Installation

```bash
git clone https://github.com/MehmetCanWT/WT-Dots.git ~/WT-Dots
cd ~/WT-Dots && ./install.sh
```

## Keybinds (`SUPER` = Windows Key)

- `SUPER + Space` — Rofi Application Launcher
- `SUPER + Return` — Ghostty Terminal
- `SUPER + E` — Dolphin File Manager
- `SUPER + B` — Zen Browser
- `SUPER + W` — Wallpaper Picker
- `SUPER + Q` — Close Active Window
- `SUPER + V` — Toggle Floating
- `SUPER + F` — Toggle Fullscreen
- `SUPER + L` — Lock Screen (`hyprlock`)
- `Print` / `Shift + Print` — Region / Fullscreen Screenshot
