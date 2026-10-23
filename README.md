# WT-Dots

Modular dotfiles for **KDE Plasma 6 (Wayland)** featuring an ultra-fast hardware-accelerated companion helper & wallpaper selector written in **Pure Rust** (`wt-rust`), a native BTOP-style system monitor Plasmoid widget, and custom **RustFetch** system info configs.

---

## Architecture & Features

- **KDE Plasma 6 Native Taskbar & BTOP Widget (`wt-btop-monitor`)**:
  - Native Plasma 6 plasmoid applet with compact taskbar meters (CPU, RAM, GPU) and full expandable pop-up panel.
  - Sub-millisecond stats collection powered directly by `wt-rust btop-stats` in native machine code (no runtime overhead, minimal CPU/battery impact).
  - Dynamic color theme adaptation reacting live to wallpaper palette changes.
  - Bilingual localization: Automatically detects system language (Turkish or English).
  - Accurate metrics: Per-core CPU load, temperature, frequency, RAM & SWAP, disk usage (Bazzite atomic ostree & standard Linux support), NVIDIA GPU telemetry, real-time network transfer rates, and top active processes.

- **`wt-rust` (`WT-Rust`)**:
  - Single-binary Rust companion replacing external shell/Python scripts:
  - `wt-rust wallpaper [select|apply <path>]`: Hardware-accelerated (`eframe`/`egui` + Wayland + Glow) fullscreen wallpaper carousel selector with smooth crossfade and cached thumbnails.
  - `wt-rust theme [get <path> | apply [path]]`: Dominant HSL palette extraction and full-system theme synchronization across KDE Plasma 6 (.colors scheme & window decorations), GTK 3 & GTK 4, and Kitty terminal (`current-theme.conf`).
  - `wt-rust keybinds [sync|watch]`: Declarative keybinding sync from `keybinds.conf` directly into KDE Plasma 6 global shortcuts.
  - `wt-rust btop-stats`: High-performance JSON system telemetry generator for the Plasma monitor widget.

- **RustFetch (`rustfetch`)**:
  - Pre-configured `config.toml` for blazing fast terminal system info.
  - Bundled custom high-resolution logos in `rustfetch/logos/` with dynamic color extraction.

---

## Directory Structure

```
WT-Dots/
├── install.sh                  # One-shot idempotent installer & config linker
├── keybinds.conf               # Declarative KDE Plasma 6 global shortcuts
├── rustfetch/                  # RustFetch terminal info configuration
│   ├── config.toml
│   └── logos/                  # Custom logos & color configurations
├── widgets/                    # Native Plasma 6 widgets
│   ├── wt-btop-monitor/        # QML source files
│   └── wt-btop-monitor.plasmoid# Packaged Plasma 6 applet
├── WT-Rust/                    # Pure Rust companion binary & GUI selector
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── btop.rs
│       ├── keybinds.rs
│       ├── theme.rs
│       └── wallpaper.rs
├── Wallpapers/                 # Bundled wallpaper collection
└── README.md
```

---

## Installation & Setup

### Automated Setup

Run the installer from within the cloned repository:

```bash
./install.sh
```

The script will:
1. Verify and install any missing system dependencies.
2. Build `wt-rust` in release mode and install it to `~/.local/bin/wt-rust`.
3. Symlink `keybinds.conf` and `rustfetch` into `~/.config/`.
4. Copy bundled wallpapers to your pictures directory.
5. Install and register the `wt-btop-monitor` Plasma 6 widget.
6. Synchronize declarative keybindings with KDE.

---

## License

MIT
