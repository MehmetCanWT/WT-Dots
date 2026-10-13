#!/usr/bin/env bash
# Update checker and notifier for MangoWM (Fedora & Arch)

sleep 10

PKG_COUNT=0
PKG_NAME="DNF"
UPGRADE_CMD="sudo dnf upgrade"

if command -v checkupdates >/dev/null 2>&1; then
    PKG_COUNT=$(checkupdates 2>/dev/null | wc -l)
    PKG_NAME="Pacman"
    UPGRADE_CMD="sudo pacman -Syu"
elif command -v dnf >/dev/null 2>&1; then
    PKG_COUNT=$(dnf check-update -q 2>/dev/null | grep -Ev "^(Upgrades|$)" | wc -l)
    PKG_NAME="DNF"
    UPGRADE_CMD="sudo dnf upgrade"
fi
PKG_COUNT=${PKG_COUNT:-0}

FLATPAK_COUNT=0
if command -v flatpak >/dev/null 2>&1; then
    FLATPAK_COUNT=$(flatpak remote-ls --updates 2>/dev/null | wc -l)
    FLATPAK_COUNT=${FLATPAK_COUNT:-0}
fi

TOTAL=$(( PKG_COUNT + FLATPAK_COUNT ))
if [ "$TOTAL" -gt 0 ]; then
    MSG=""
    [ "$PKG_COUNT" -gt 0 ] && MSG+="${PKG_COUNT} ${PKG_NAME} güncellemesi"
    if [ "$FLATPAK_COUNT" -gt 0 ]; then
        [ -n "$MSG" ] && MSG+=" ve "
        MSG+="${FLATPAK_COUNT} Flatpak güncellemesi"
        UPGRADE_CMD+=" && flatpak update"
    fi
    MSG+=" mevcut.\n\nGüncellemek için:\n${UPGRADE_CMD}"
    notify-send -i software-update-available -u normal "Sistem Güncellemeleri" "$MSG"
fi
