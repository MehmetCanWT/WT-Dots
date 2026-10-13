#!/usr/bin/env bash
# Update checker and notifier for Fedora & MangoWM

# Wait for desktop session to be ready
sleep 10

# Count DNF updates
DNF_COUNT=$(dnf check-update -q 2>/dev/null | grep -v "^Upgrades" | grep -v "^$" | wc -l)
DNF_COUNT=${DNF_COUNT:-0}

# Count Flatpak updates (if flatpak installed)
FLATPAK_COUNT=0
if command -v flatpak >/dev/null 2>&1; then
    FLATPAK_COUNT=$(flatpak remote-ls --updates 2>/dev/null | wc -l)
    FLATPAK_COUNT=${FLATPAK_COUNT:-0}
fi

TOTAL=$(( DNF_COUNT + FLATPAK_COUNT ))

if [ "$TOTAL" -gt 0 ]; then
    MSG=""
    [ "$DNF_COUNT" -gt 0 ] && MSG+="${DNF_COUNT} DNF sistemi güncellemesi"
    if [ "$FLATPAK_COUNT" -gt 0 ]; then
        [ -n "$MSG" ] && MSG+=" ve "
        MSG+="${FLATPAK_COUNT} Flatpak uygulama güncellemesi"
    fi
    MSG+=" mevcut.\n\nGüncellemek için:\nsudo dnf upgrade && flatpak update"

    notify-send -i software-update-available -u normal "Sistem Güncellemeleri" "$MSG"
fi
