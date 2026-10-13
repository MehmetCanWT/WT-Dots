#!/usr/bin/env bash
# WT-Dots Automated Installer
# Supported: Fedora Linux (x86_64)

set -euo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
BLUE="\033[0;34m"
YELLOW="\033[1;33m"
RESET="\033[0m"

log() { printf "${BLUE}[WT-Dots]${RESET} %s\n" "$*"; }
ok()  { printf "${GREEN}[✓]${RESET} %s\n" "$*"; }
warn(){ printf "${YELLOW}[!]${RESET} %s\n" "$*"; }

REPO_URL="https://github.com/MehmetCanWT/WT-Dots.git"
TARGET_DIR="${HOME}/Code/WT-Dots"

# 1. Clone or locate repository
if [ ! -d "${TARGET_DIR}/.git" ]; then
    log "Cloning repository to ${TARGET_DIR}..."
    mkdir -p "${HOME}/Code"
    git clone "${REPO_URL}" "${TARGET_DIR}"
fi

cd "${TARGET_DIR}"

# 2. Install Dependencies
log "Checking and installing system packages..."
PKGS=(
    fuzzel
    mako
    swaybg
    kitty
    nautilus
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

if command -v pkcon >/dev/null 2>&1; then
    pkcon install -y "${PKGS[@]}" || sudo dnf install -y "${PKGS[@]}"
elif command -v dnf >/dev/null 2>&1; then
    sudo dnf install -y "${PKGS[@]}"
fi
ok "System packages installed."

# 3. Build & Install mangobar if missing
if ! command -v mangobar >/dev/null 2>&1; then
    log "mangobar not found. Building from source..."
    BUILD_DEPS=(meson ninja-build git cjson-devel wayland-protocols-devel fcft-devel tllist-devel pulseaudio-libs-devel)
    if command -v pkcon >/dev/null 2>&1; then
        pkcon install -y "${BUILD_DEPS[@]}" || sudo dnf install -y "${BUILD_DEPS[@]}"
    else
        sudo dnf install -y "${BUILD_DEPS[@]}"
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

deploy_dir() {
    local src="$1"
    local dest="${HOME}/.config/$2"
    mkdir -p "${dest}"
    cp -r "${src}/"* "${dest}/"
}

deploy_dir "${TARGET_DIR}/mango" "mango"
deploy_dir "${TARGET_DIR}/mangobar" "mangobar"
deploy_dir "${TARGET_DIR}/fuzzel" "fuzzel"
deploy_dir "${TARGET_DIR}/mako" "mako"
deploy_dir "${TARGET_DIR}/kitty" "kitty"
deploy_dir "${TARGET_DIR}/wallpaper-picker" "wallpaper-picker"
deploy_dir "${TARGET_DIR}/environment.d" "environment.d"
deploy_dir "${TARGET_DIR}/gtk-3.0" "gtk-3.0"
deploy_dir "${TARGET_DIR}/gtk-4.0" "gtk-4.0"
deploy_dir "${TARGET_DIR}/xdg-desktop-portal" "xdg-desktop-portal"

chmod +x "${HOME}/.config/mango/scripts/"* "${HOME}/.config/wallpaper-picker/"*.py 2>/dev/null || true
ok "Configurations deployed."

# 5. User Directories & File Association
log "Setting up directories and default apps..."
mkdir -p "${HOME}/Resimler/Wallpapers" "${HOME}/Resimler/Screenshots" "${HOME}/İndirilenler"

if command -v xdg-mime >/dev/null 2>&1; then
    xdg-mime default org.gnome.Nautilus.desktop inode/directory
fi

# 6. Apply Environment Variables to DBus
if command -v dbus-update-activation-environment >/dev/null 2>&1; then
    dbus-update-activation-environment --systemd \
        WAYLAND_DISPLAY \
        XDG_CURRENT_DESKTOP \
        XDG_SESSION_TYPE \
        QT_QPA_PLATFORMTHEME=kde \
        QT_STYLE_OVERRIDE=Breeze-Dark || true
fi

# 7. Initial Theme Restore
if [ -f "${HOME}/.config/wallpaper-picker/theme.py" ]; then
    log "Generating initial theme palette..."
    python3 "${HOME}/.config/wallpaper-picker/theme.py" --restore 2>/dev/null || true
fi

printf "\n${GREEN}${BOLD}✓ WT-Dots installation complete!${RESET}\n"
printf "Log into MangoWM or reload with ${BOLD}Super + R${RESET}.\n"
