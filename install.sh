#!/usr/bin/env bash
# WT-Dots Automated Installer
# Supported: Fedora Linux & Arch Linux (x86_64)

set -euo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
BLUE="\033[0;34m"
YELLOW="\033[1;33m"
RESET="\033[0m"

log() { printf "${BLUE}[WT-Dots]${RESET} %s\n" "$*"; }
ok()  { printf "${GREEN}[✓]${RESET} %s\n" "$*"; }
warn(){ printf "${YELLOW}[!]${RESET} %s\n" "$*"; }

# Detect Distribution
DISTRO="unknown"
if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO="${ID:-unknown}"
fi

REPO_URL="https://github.com/MehmetCanWT/WT-Dots.git"
TARGET_DIR="${HOME}/Code/WT-Dots"

# 1. Clone or locate repository
if [ ! -d "${TARGET_DIR}/.git" ]; then
    log "Cloning repository to ${TARGET_DIR}..."
    mkdir -p "${HOME}/Code"
    git clone "${REPO_URL}" "${TARGET_DIR}"
fi

cd "${TARGET_DIR}"

# 2. Install Dependencies based on Distribution
log "Detected OS: ${DISTRO}. Installing system packages..."

if [[ "$DISTRO" =~ ^(fedora|rhel|centos)$ ]]; then
    # Enable rofi-calc Copr
    sudo dnf copr enable -y skoved/rofi-calc 2>/dev/null || true

    FEDORA_PKGS=(
        rofi
        rofi-calc
        cliphist
        mako
        swaybg
        kitty
        mangohud
        mangohud.i686
        grim
        slurp
        brightnessctl
        wireplumber
        python3-pyside6
        qt6-qtdeclarative
        python3-pillow
        ffmpeg
        xdg-utils
    )
    sudo dnf install -y "${FEDORA_PKGS[@]}"
    ok "Fedora packages installed."

    BUILD_DEPS=(meson ninja-build git cjson-devel wayland-protocols-devel fcft-devel tllist-devel pulseaudio-libs-devel)

elif [[ "$DISTRO" =~ ^(arch|archcraft|endeavouros|manjaro)$ ]]; then
    ARCH_PKGS=(
        base-devel
        git
        rofi
        rofi-calc
        cliphist
        mako
        swaybg
        kitty
        mangohud
        lib32-mangohud
        grim
        slurp
        brightnessctl
        wireplumber
        python-pyside6
        qt6-declarative
        python-pillow
        ffmpeg
        xdg-utils
    )
    sudo pacman -S --needed --noconfirm "${ARCH_PKGS[@]}"
    ok "Arch Linux packages installed."

    BUILD_DEPS=(meson ninja git cjson wayland-protocols fcft tllist libpulse)
else
    warn "Distribution ($DISTRO) not directly recognized. Attempting fallback or manual package installation."
    BUILD_DEPS=()
fi

# 3. Build & Install mangobar if missing
if ! command -v mangobar >/dev/null 2>&1; then
    log "mangobar not found. Building from source..."
    if [ ${#BUILD_DEPS[@]} -gt 0 ]; then
        if [[ "$DISTRO" =~ ^(fedora|rhel|centos)$ ]]; then
            sudo dnf install -y "${BUILD_DEPS[@]}"
        elif [[ "$DISTRO" =~ ^(arch|archcraft|endeavouros|manjaro)$ ]]; then
            sudo pacman -S --needed --noconfirm "${BUILD_DEPS[@]}"
        fi
    fi

    MANGOBAR_SRC="/tmp/mangobar-build"
    rm -rf "${MANGOBAR_SRC}"
    git clone --depth 1 https://github.com/DreamMaoMao/mangobar.git "${MANGOBAR_SRC}"
    meson setup "${MANGOBAR_SRC}/build" "${MANGOBAR_SRC}" --prefix="${HOME}/.local"
    ninja -C "${MANGOBAR_SRC}/build" install
    rm -rf "${MANGOBAR_SRC}"
    ok "mangobar built and installed to ~/.local/bin/mangobar."
fi

# 4. Deploy Configuration Files
log "Deploying dotfiles to ~/.config..."
mkdir -p "${HOME}/.config"

for dir in mango mangobar rofi mako kitty wallpaper-picker environment.d gtk-3.0 gtk-4.0 xdg-desktop-portal MangoHud; do
    mkdir -p "${HOME}/.config/${dir}"
    cp -r "${TARGET_DIR}/${dir}/"* "${HOME}/.config/${dir}/"
done

chmod +x "${HOME}/.config/mango/scripts/"* "${HOME}/.config/wallpaper-picker/"*.py 2>/dev/null || true
ok "Configurations deployed."

# 4.1 Deploy local binaries
if [ -d "${TARGET_DIR}/local-bin" ]; then
    mkdir -p "${HOME}/.local/bin"
    cp -r "${TARGET_DIR}/local-bin/"* "${HOME}/.local/bin/"
    chmod +x "${HOME}/.local/bin/"* 2>/dev/null || true
fi

# 5. User Directories, Hide Discover & File Association
log "Setting up directories and desktop entries..."
mkdir -p "${HOME}/Resimler/Wallpapers" "${HOME}/Resimler/Screenshots" "${HOME}/İndirilenler" "${HOME}/.local/share/applications"

# Ensure Discover (Keşfet) is hidden from Alt+Space launcher
cat << 'EOF' > "${HOME}/.local/share/applications/org.kde.discover.desktop"
[Desktop Entry]
Type=Application
Name=Discover
NoDisplay=true
EOF

# 6. Setup and Start Systemd User Services for Cliphist
log "Configuring clipboard daemon systemd services..."
mkdir -p "${HOME}/.config/systemd/user"

cat << 'EOF' > "${HOME}/.config/systemd/user/cliphist-text.service"
[Unit]
Description=Cliphist Text Clipboard Watcher
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/bin/wl-paste --type text --watch /usr/bin/cliphist store
Restart=always
RestartSec=2

[Install]
WantedBy=default.target
EOF

cat << 'EOF' > "${HOME}/.config/systemd/user/cliphist-image.service"
[Unit]
Description=Cliphist Image Clipboard Watcher
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/bin/wl-paste --type image --watch /usr/bin/cliphist store
Restart=always
RestartSec=2

[Install]
WantedBy=default.target
EOF

if command -v systemctl >/dev/null 2>&1; then
    systemctl --user daemon-reload 2>/dev/null || true
    systemctl --user enable --now cliphist-text.service cliphist-image.service 2>/dev/null || true
fi
ok "Clipboard watcher services configured and started."

if command -v xdg-mime >/dev/null 2>&1; then
    xdg-mime default org.kde.dolphin.desktop inode/directory 2>/dev/null || true
fi

if command -v gsettings >/dev/null 2>&1; then
    gsettings set org.gnome.desktop.interface font-name "Cantarell 10" 2>/dev/null || true
    gsettings set org.gnome.desktop.interface document-font-name "Cantarell 10" 2>/dev/null || true
    gsettings set org.gnome.desktop.interface monospace-font-name "0xProto Nerd Font 10" 2>/dev/null || true
fi

# 7. Apply Environment Variables to DBus
if command -v dbus-update-activation-environment >/dev/null 2>&1; then
    dbus-update-activation-environment --systemd \
        WAYLAND_DISPLAY \
        XDG_CURRENT_DESKTOP \
        XDG_SESSION_TYPE \
        QT_QPA_PLATFORMTHEME=kde \
        QT_STYLE_OVERRIDE=Breeze-Dark 2>/dev/null || true
fi

# 8. Initial Theme Restore
if [ -f "${HOME}/.config/wallpaper-picker/theme.py" ]; then
    log "Generating initial theme palette..."
    python3 "${HOME}/.config/wallpaper-picker/theme.py" --restore 2>/dev/null || true
fi

printf "\n${GREEN}${BOLD}✓ WT-Dots installation complete!${RESET}\n"
printf "Log into MangoWM or reload with ${BOLD}Super + R${RESET}.\n"
