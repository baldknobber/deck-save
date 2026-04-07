#!/usr/bin/env bash
# DeckSave — Flatpak installer for Steam Deck / Linux
#
# Usage:
#   curl -sL https://raw.githubusercontent.com/baldknobber/deck-save/master/scripts/deck-install-flatpak.sh | bash
#
# What it does:
#   1. Fetches the latest .flatpak from GitHub Releases
#   2. Installs it via flatpak (bundles webkit2gtk — works on SteamOS)

set -euo pipefail

REPO="baldknobber/deck-save"
APP_ID="com.baldknobber.decksave"

echo "=== DeckSave Flatpak Installer ==="
echo ""

# 1. Ensure Flathub runtime is available
echo "Ensuring GNOME runtime is available..."
flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user -y flathub org.gnome.Platform//46 2>/dev/null || true

# 2. Get latest release tag
echo "Checking latest release..."
LATEST=$(curl -sI "https://github.com/$REPO/releases/latest" | grep -i '^location:' | sed 's|.*/tag/||' | tr -d '\r\n')

if [ -z "$LATEST" ]; then
    echo "ERROR: Could not determine latest release."
    echo "Check: https://github.com/$REPO/releases"
    exit 1
fi

echo "Latest release: $LATEST"

# 3. Find the .flatpak asset URL
FLATPAK_URL=$(curl -sL "https://api.github.com/repos/$REPO/releases/tags/$LATEST" \
    | grep -o '"browser_download_url": *"[^"]*\.flatpak"' \
    | head -1 \
    | sed 's/"browser_download_url": *"//' \
    | tr -d '"')

if [ -z "$FLATPAK_URL" ]; then
    echo "ERROR: No .flatpak found in release $LATEST."
    echo "The release may still be building. Check:"
    echo "  https://github.com/$REPO/actions"
    exit 1
fi

echo "Downloading: $(basename "$FLATPAK_URL")"

# 4. Download and install
TMPFILE=$(mktemp /tmp/DeckSave.XXXXXX.flatpak)
curl -sL "$FLATPAK_URL" -o "$TMPFILE"
flatpak install --user -y "$TMPFILE"
rm -f "$TMPFILE"

echo ""
echo "=== Install Complete ==="
echo ""
echo "Run DeckSave:"
echo "  flatpak run $APP_ID"
echo ""
echo "Add to Gaming Mode:"
echo "  DeckSave should appear in your app menu automatically."
echo "  Or: Steam → Games → Add a Non-Steam Game → find DeckSave"
echo ""
echo "To update later, just run this script again."
