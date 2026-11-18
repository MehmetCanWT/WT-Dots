#!/usr/bin/env bash
set -euo pipefail

# ===================================================================
# WT-Dots Setup & Installation Script (Hyprland)
# ===================================================================

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
    exec "$TARGET_DIR/install.sh" "$@"
fi

echo "==> Setting up WT-Dots (Hyprland) from: $SCRIPT_DIR"

mkdir -p "$HOME/.local/bin" "$HOME/.config"
for dir in "$HOME/.cargo/bin" "$HOME/.local/bin"; do
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
WP_DEST="$HOME/Pictures/Wallpapers"
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

# Install helper binaries
if [ -f "$SCRIPT_DIR/hyprland/bin/game-run" ]; then
    install -Dm755 "$SCRIPT_DIR/hyprland/bin/game-run" "$HOME/.local/bin/game-run"
fi

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

echo "==> Linking Hyprland configs..."
link_config "$SCRIPT_DIR/rustfetch"         "$HOME/.config/rustfetch"
link_config "$SCRIPT_DIR/hyprland/hypr"    "$HOME/.config/hypr"
link_config "$SCRIPT_DIR/hyprland/waybar"  "$HOME/.config/waybar"
link_config "$SCRIPT_DIR/hyprland/rofi"    "$HOME/.config/rofi"
link_config "$SCRIPT_DIR/hyprland/mako"    "$HOME/.config/mako"
link_config "$SCRIPT_DIR/hyprland/ghostty" "$HOME/.config/ghostty"

echo "==> WT-Dots Hyprland installation complete!"
