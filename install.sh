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

# Ensure Homebrew (Bazzite default), Cargo, and ~/.local/bin are in PATH
mkdir -p "$HOME/.local/bin"
for dir in "/home/linuxbrew/.linuxbrew/bin" "$HOME/.cargo/bin" "$HOME/.local/bin"; do
    if [ -d "$dir" ] && [[ ":$PATH:" != *":$dir:"* ]]; then
        export PATH="$dir:$PATH"
    fi
done

# 1. Detect OS (Bazzite / Fedora Atomic vs mutable Fedora) & install missing dependencies
IS_OSTREE=0
if [ -f /run/ostree-booted ] || command -v rpm-ostree >/dev/null 2>&1; then
    IS_OSTREE=1
fi

install_dependencies() {
    if [ "$IS_OSTREE" -eq 1 ]; then
        echo "==> Detected Bazzite / Fedora Atomic (ostree) environment"

        # Prefer Homebrew on Bazzite for userspace CLI tools like cliphist
        if command -v brew >/dev/null 2>&1; then
            if ! command -v cliphist >/dev/null 2>&1; then
                echo "==> Installing cliphist via Homebrew..."
                brew install cliphist || true
            fi
        fi

        # Collect any missing host/Wayland packages for live rpm-ostree layering
        local missing_pkgs=()
        command -v cc >/dev/null 2>&1 || missing_pkgs+=("gcc")
        command -v waybar >/dev/null 2>&1 || missing_pkgs+=("waybar")
        command -v rofi >/dev/null 2>&1 || missing_pkgs+=("rofi-wayland")
        command -v wl-paste >/dev/null 2>&1 || missing_pkgs+=("wl-clipboard")
        command -v brightnessctl >/dev/null 2>&1 || missing_pkgs+=("brightnessctl")
        command -v cliphist >/dev/null 2>&1 || missing_pkgs+=("cliphist")

        if [ "${#missing_pkgs[@]}" -gt 0 ]; then
            echo "==> Installing missing system packages on Bazzite via rpm-ostree (--apply-live): ${missing_pkgs[*]}"
            sudo rpm-ostree install --apply-live --idempotent -y "${missing_pkgs[@]}" || \
                echo "Warning: rpm-ostree live apply had non-zero status; a reboot may be required if packages were layered."
        fi
    elif command -v dnf >/dev/null 2>&1; then
        local missing_pkgs=()
        command -v cc >/dev/null 2>&1 || missing_pkgs+=("gcc")
        command -v waybar >/dev/null 2>&1 || missing_pkgs+=("waybar")
        command -v rofi >/dev/null 2>&1 || missing_pkgs+=("rofi-wayland")
        command -v wl-paste >/dev/null 2>&1 || missing_pkgs+=("wl-clipboard")
        command -v brightnessctl >/dev/null 2>&1 || missing_pkgs+=("brightnessctl")
        command -v cliphist >/dev/null 2>&1 || missing_pkgs+=("cliphist")

        if [ "${#missing_pkgs[@]}" -gt 0 ]; then
            echo "==> Installing missing packages via dnf: ${missing_pkgs[*]}"
            sudo dnf install -y "${missing_pkgs[@]}"
        fi
    fi

    # Install Rust/Cargo via rustup in userspace if not present (ideal for Bazzite immutable root)
    if ! command -v cargo >/dev/null 2>&1; then
        echo "==> Cargo not found. Installing Rust toolchain via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
}

install_dependencies

# Ensure Wallpapers directory exists (supports both Turkish and English XDG locales on Bazzite)
if [ -d "$HOME/Pictures" ] && [ ! -d "$HOME/Resimler" ]; then
    mkdir -p "$HOME/Pictures/Wallpapers"
else
    mkdir -p "$HOME/Resimler/Wallpapers"
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
