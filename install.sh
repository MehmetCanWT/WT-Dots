#!/usr/bin/env bash
set -euo pipefail

# ===================================================================
# WT-Dots Setup & Installation Script (KDE Plasma 6 & Hyprland)
# ===================================================================

TARGET_MODE="${1:-auto}"
if [ "$TARGET_MODE" = "auto" ]; then
    if [ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ] || [ "${XDG_CURRENT_DESKTOP:-}" = "Hyprland" ]; then
        TARGET_MODE="--hyprland"
    else
        TARGET_MODE="--kde"
    fi
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || echo "")"
if [ -z "$SCRIPT_DIR" ] || [ ! -f "$SCRIPT_DIR/WT-Rust/Cargo.toml" ]; then
    TARGET_DIR="${WT_DOTS_DIR:-$HOME/WT-Dots}"
    echo "==> Cloning WT-Dots repository to $TARGET_DIR..."
    mkdir -p "$(dirname "$TARGET_DIR")"
    if [ -d "$TARGET_DIR/.git" ]; then
        git -C "$TARGET_DIR" pull --ff-only || true
    else
        git clone https://github.com/MehmetCanWT/WT-Dots.git "$TARGET_DIR"
    fi
    exec "$TARGET_DIR/install.sh" "$TARGET_MODE"
fi

echo "==> Setting up WT-Dots ($TARGET_MODE) from: $SCRIPT_DIR"

mkdir -p "$HOME/.local/bin" "$HOME/.config"
for dir in "/home/linuxbrew/.linuxbrew/bin" "$HOME/.cargo/bin" "$HOME/.local/bin"; do
    if [ -d "$dir" ] && [[ ":$PATH:" != *":$dir:"* ]]; then
        export PATH="$dir:$PATH"
    fi
done

if ! command -v cargo >/dev/null 2>&1; then
    echo "==> Cargo not found. Installing Rust toolchain via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    export PATH="$HOME/.cargo/bin:$PATH"
fi

# Sync bundled wallpapers
if [ -d "$HOME/Pictures" ] && [ ! -d "$HOME/Resimler" ]; then
    WP_DEST="$HOME/Pictures/Wallpapers"
else
    WP_DEST="$HOME/Resimler/Wallpapers"
fi
mkdir -p "$WP_DEST"
if [ -d "$SCRIPT_DIR/Wallpapers" ]; then
    echo "==> Copying bundled wallpapers to $WP_DEST..."
    cp -n "$SCRIPT_DIR/Wallpapers/"* "$WP_DEST/" 2>/dev/null || true
fi

# Install bundled fonts (0xProto Nerd Font)
if [ -d "$SCRIPT_DIR/fonts" ]; then
    echo "==> Installing bundled fonts to ~/.local/share/fonts..."
    mkdir -p "$HOME/.local/share/fonts"
    cp -rf "$SCRIPT_DIR/fonts/"* "$HOME/.local/share/fonts/"
    command -v fc-cache >/dev/null 2>&1 && fc-cache -f || true
fi

# Build wt-rust
echo "==> Building wt-rust in release mode..."
cargo build --release --manifest-path "$SCRIPT_DIR/WT-Rust/Cargo.toml"
install -Dm755 "$SCRIPT_DIR/WT-Rust/target/release/wt-rust" "$HOME/.local/bin/wt-rust"
echo "==> Installed binary to $HOME/.local/bin/wt-rust"

link_config() {
    local src="$1"
    local dst="$2"

    if [ -e "$dst" ] && [ ! -L "$dst" ]; then
        echo "==> Backing up existing $dst to ${dst}.bak"
        mv "$dst" "${dst}.bak"
    fi
    ln -sfn "$src" "$dst"
    echo "==> Linked $dst -> $src"
}

link_config "$SCRIPT_DIR/rustfetch" "$HOME/.config/rustfetch"

if [ "$TARGET_MODE" = "--hyprland" ]; then
    echo "==> Linking Hyprland configs..."
    link_config "$SCRIPT_DIR/hyprland/hypr"                     "$HOME/.config/hypr"
    link_config "$SCRIPT_DIR/hyprland/waybar"                   "$HOME/.config/waybar"
    link_config "$SCRIPT_DIR/hyprland/fuzzel"                   "$HOME/.config/fuzzel"
    link_config "$SCRIPT_DIR/hyprland/mako"                     "$HOME/.config/mako"
    link_config "$SCRIPT_DIR/hyprland/ghostty"                  "$HOME/.config/ghostty"
    link_config "$SCRIPT_DIR/hyprland/helium-browser-flags.conf" "$HOME/.config/helium-browser-flags.conf"
    mkdir -p "$HOME/.config/gtk-3.0" "$HOME/.config/gtk-4.0"
    cp -f "$SCRIPT_DIR/hyprland/gtk-3.0/settings.ini" "$HOME/.config/gtk-3.0/settings.ini"
    cp -f "$SCRIPT_DIR/hyprland/gtk-4.0/settings.ini" "$HOME/.config/gtk-4.0/settings.ini"
    command -v gsettings >/dev/null 2>&1 && gsettings set org.gnome.desktop.interface gtk-enable-primary-paste true || true
    command -v xdg-settings >/dev/null 2>&1 && [ -f /usr/share/applications/helium.desktop ] && xdg-settings set default-web-browser helium.desktop || true
else
    echo "==> Linking KDE Plasma 6 configs..."
    link_config "$SCRIPT_DIR/kde/keybinds.conf" "$HOME/.config/keybinds.conf"

    if [ -d "$SCRIPT_DIR/kde/widgets/wt-btop-monitor" ]; then
        mkdir -p "$HOME/.local/share/plasma/plasmoids"
        if command -v kpackagetool6 >/dev/null 2>&1; then
            kpackagetool6 --type Plasma/Applet --upgrade "$SCRIPT_DIR/kde/widgets/wt-btop-monitor" 2>/dev/null || \
            kpackagetool6 --type Plasma/Applet --install "$SCRIPT_DIR/kde/widgets/wt-btop-monitor" 2>/dev/null || true
        fi
    fi

    "$HOME/.local/bin/wt-rust" keybinds sync
fi

echo "==> WT-Dots installation complete!"
