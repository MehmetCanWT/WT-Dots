#!/usr/bin/env bash
set -euo pipefail

# ===================================================================
# WT-Dots Setup & Installation Script
# ===================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
echo "==> Setting up WT-Dots from: $SCRIPT_DIR"

# 1. Build WT-Rust binary in release mode
echo "==> Building wt-rust in release mode..."
cargo build --release --manifest-path "$SCRIPT_DIR/WT-Rust/Cargo.toml"

mkdir -p "$HOME/.local/bin"
cp "$SCRIPT_DIR/WT-Rust/target/release/wt-rust" "$HOME/.local/bin/wt-rust"
chmod +x "$HOME/.local/bin/wt-rust"
echo "==> Installed binary to $HOME/.local/bin/wt-rust"

# 2. Symlink configuration directories & files
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

# 3. Configure Spectacle (auto-save to folder + clipboard, never open viewer window)
echo "==> Configuring Spectacle screenshot settings..."
if command -v kwriteconfig6 >/dev/null 2>&1; then
    kwriteconfig6 --file spectaclerc --group General --key autoSaveImage true
    kwriteconfig6 --file spectaclerc --group General --key clipboardGroup PostScreenshotCopyImage
    kwriteconfig6 --file spectaclerc --group General --key quitAfterSaveCopyExport true
else
    mkdir -p "$HOME/.config"
    SPECTACLE_RC="$HOME/.config/spectaclerc"
    if [ ! -f "$SPECTACLE_RC" ]; then
        echo -e "[General]\nautoSaveImage=true\nclipboardGroup=PostScreenshotCopyImage\nquitAfterSaveCopyExport=true" > "$SPECTACLE_RC"
    fi
fi

# 4. Install & Enable Waybar systemd service
echo "==> Configuring systemd user service for Waybar..."
mkdir -p "$HOME/.config/systemd/user"
cp "$SCRIPT_DIR/systemd/waybar.service" "$HOME/.config/systemd/user/waybar.service"
systemctl --user daemon-reload
systemctl --user enable waybar.service
systemctl --user restart waybar.service || pkill -SIGUSR2 waybar || true

# 5. Sync keybindings (Meta+W)
echo "==> Synchronizing keybindings..."
"$HOME/.local/bin/wt-rust" keybinds sync

echo "==> WT-Dots installation complete!"
