#!/usr/bin/env bash
# DeckSave — One-liner installer for Steam Deck / Linux
#
# Usage:
#   curl -sL https://raw.githubusercontent.com/baldknobber/deck-save/master/scripts/deck-install.sh | bash
#
# What it does:
#   1. Fetches the latest AppImage from GitHub Releases
#   2. Installs to ~/Applications/DeckSave.AppImage
#   3. Creates a .desktop shortcut

set -euo pipefail

REPO="baldknobber/deck-save"
APP_NAME="DeckSave"
INSTALL_DIR="$HOME/Applications"
DESKTOP_DIR="$HOME/.local/share/applications"

echo "=== DeckSave Installer ==="
echo ""

# 1. Get latest release tag
echo "Checking latest release..."
LATEST=$(curl -sI "https://github.com/$REPO/releases/latest" | grep -i '^location:' | sed 's|.*/tag/||' | tr -d '\r\n')

if [ -z "$LATEST" ]; then
    echo "ERROR: Could not determine latest release."
    echo "Check: https://github.com/$REPO/releases"
    exit 1
fi

echo "Latest release: $LATEST"

# 2. Find the AppImage asset URL
APPIMAGE_URL=$(curl -sL "https://api.github.com/repos/$REPO/releases/tags/$LATEST" \
    | grep -o '"browser_download_url": *"[^"]*\.AppImage"' \
    | head -1 \
    | sed 's/"browser_download_url": *"//' \
    | tr -d '"')

if [ -z "$APPIMAGE_URL" ]; then
    echo "ERROR: No AppImage found in release $LATEST."
    echo "The release may still be building. Check:"
    echo "  https://github.com/$REPO/actions"
    exit 1
fi

echo "Downloading: $(basename "$APPIMAGE_URL")"

# 3. Download and install
mkdir -p "$INSTALL_DIR"
curl -sL "$APPIMAGE_URL" -o "$INSTALL_DIR/$APP_NAME.AppImage"
chmod +x "$INSTALL_DIR/$APP_NAME.AppImage"

echo "Installed to: $INSTALL_DIR/$APP_NAME.AppImage"

# 4. Create .desktop shortcut
mkdir -p "$DESKTOP_DIR"
cat > "$DESKTOP_DIR/decksave.desktop" << EOF
[Desktop Entry]
Name=DeckSave
Comment=Backup and sync game saves across devices
Exec=$INSTALL_DIR/$APP_NAME.AppImage
Icon=decksave
Type=Application
Categories=Game;Utility;
Keywords=save;backup;sync;steam;deck;
StartupNotify=true
Terminal=false
EOF

if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
fi

echo ""
echo "=== Install Complete ==="
echo ""
echo "Run DeckSave:"
echo "  $INSTALL_DIR/$APP_NAME.AppImage"
echo ""
echo "Add to Gaming Mode:"
echo "  Steam → Games → Add a Non-Steam Game → Browse to:"
echo "  $INSTALL_DIR/$APP_NAME.AppImage"
echo ""
echo "To update later, just run this script again."
