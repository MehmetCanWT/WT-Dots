#!/usr/bin/env bash
set -euo pipefail

# ===================================================================
# WT-Dots Setup & Installation Script
# ===================================================================

# If running directly from curl without cloned directory, bootstrap it
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || echo "")"
if [ -z "$SCRIPT_DIR" ] || [ ! -f "$SCRIPT_DIR/WT-Rust/Cargo.toml" ]; then
    TARGET_DIR="${WT_DOTS_DIR:-$HOME/Code/WT-Dots}"
    echo "==> Cloning WT-Dots repository to $TARGET_DIR..."
    mkdir -p "$(dirname "$TARGET_DIR")"
    if [ -d "$TARGET_DIR/.git" ]; then
        git -C "$TARGET_DIR" pull --ff-only || true
    else
        git clone https://github.com/MehmetCanWT/WT-Dots.git "$TARGET_DIR"
    fi
    exec "$TARGET_DIR/install.sh" "$@"
fi

echo "==> Setting up WT-Dots from: $SCRIPT_DIR"

# 1. Check prerequisites
if ! command -v cargo >/dev/null 2>&1; then
    echo "Error: Cargo/Rust is not installed. Please install Rust via 'curl --proto =https --tlsv1.2 -sSf https://sh.rustup.rs | sh' or dnf install rust cargo." >&2
    exit 1
fi

# Ensure ~/.local/bin exists and is in PATH
mkdir -p "$HOME/.local/bin"
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    export PATH="$HOME/.local/bin:$PATH"
fi

# 2. Build WT-Rust companion binary in release mode
echo "==> Building wt-rust in release mode..."
cargo build --release --manifest-path "$SCRIPT_DIR/WT-Rust/Cargo.toml"
install -Dm755 "$SCRIPT_DIR/WT-Rust/target/release/wt-rust" "$HOME/.local/bin/wt-rust"
echo "==> Installed binary to $HOME/.local/bin/wt-rust"

# 3. Symlink configuration directories & files
mkdir -p "$HOME/.config"

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

link_config "$SCRIPT_DIR/waybar" "$HOME/.config/waybar"
link_config "$SCRIPT_DIR/rofi" "$HOME/.config/rofi"
link_config "$SCRIPT_DIR/keybinds.conf" "$HOME/.config/keybinds.conf"

# 4. Configure Spectacle (auto-save to folder + clipboard, never open viewer window)
echo "==> Configuring Spectacle screenshot settings..."
if command -v kwriteconfig6 >/dev/null 2>&1; then
    kwriteconfig6 --file spectaclerc --group General --key autoSaveImage true
    kwriteconfig6 --file spectaclerc --group General --key clipboardGroup PostScreenshotCopyImage
    kwriteconfig6 --file spectaclerc --group General --key quitAfterSaveCopyExport true
else
    SPECTACLE_RC="$HOME/.config/spectaclerc"
    if [ ! -f "$SPECTACLE_RC" ]; then
        cat << 'EOF' > "$SPECTACLE_RC"
[General]
autoSaveImage=true
clipboardGroup=PostScreenshotCopyImage
quitAfterSaveCopyExport=true
EOF
    fi
fi

# 5. Install & Enable User systemd services (Waybar + Cliphist)
echo "==> Configuring systemd user services..."
mkdir -p "$HOME/.config/systemd/user"

for unit in waybar.service cliphist-text.service cliphist-image.service clipboard-sync-primary.service; do
    if [ -f "$SCRIPT_DIR/systemd/$unit" ]; then
        cp "$SCRIPT_DIR/systemd/$unit" "$HOME/.config/systemd/user/$unit"
    fi
done

if command -v systemctl >/dev/null 2>&1; then
    systemctl --user daemon-reload || true
    systemctl --user enable waybar.service cliphist-text.service cliphist-image.service clipboard-sync-primary.service || true
    systemctl --user restart waybar.service cliphist-text.service cliphist-image.service clipboard-sync-primary.service || true
fi

# 6. Sync declarative keybindings into KDE Plasma 6
echo "==> Synchronizing keybindings with KDE Plasma 6..."
"$HOME/.local/bin/wt-rust" keybinds sync

echo "==> WT-Dots installation complete!"
