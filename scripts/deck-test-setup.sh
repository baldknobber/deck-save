#!/usr/bin/env bash
# DeckSave — Steam Deck Testing Script
# Run this on the Steam Deck via SSH or Konsole in Desktop Mode.
#
# Prerequisites:
#   - Steam Deck in Desktop Mode
#   - SSH enabled (Settings → Developer → SSH) or use Konsole directly
#   - DeckSave AppImage downloaded to ~/Downloads/

set -euo pipefail

APPIMAGE_NAME="DeckSave"
APPIMAGE_DIR="$HOME/Applications"
DESKTOP_FILE="$HOME/.local/share/applications/decksave.desktop"
ICON_DIR="$HOME/.local/share/icons"

echo "=== DeckSave Steam Deck Setup ==="

# 1. Find the AppImage
APPIMAGE=$(find "$HOME/Downloads" -maxdepth 1 -name "*DeckSave*.AppImage" -o -name "*deck-save*.AppImage" | head -1)

if [ -z "$APPIMAGE" ]; then
    echo "ERROR: No DeckSave AppImage found in ~/Downloads/"
    echo "Download it from GitHub Releases first."
    exit 1
fi

echo "Found: $APPIMAGE"

# 2. Move to Applications
mkdir -p "$APPIMAGE_DIR"
cp "$APPIMAGE" "$APPIMAGE_DIR/$APPIMAGE_NAME.AppImage"
chmod +x "$APPIMAGE_DIR/$APPIMAGE_NAME.AppImage"
echo "Installed to: $APPIMAGE_DIR/$APPIMAGE_NAME.AppImage"

# 3. Create .desktop file for Gaming Mode
mkdir -p "$(dirname "$DESKTOP_FILE")"
cat > "$DESKTOP_FILE" << EOF
[Desktop Entry]
Name=DeckSave
Comment=Backup and sync game saves across devices
Exec=$APPIMAGE_DIR/$APPIMAGE_NAME.AppImage
Icon=decksave
Type=Application
Categories=Game;Utility;
Keywords=save;backup;sync;steam;deck;
StartupNotify=true
Terminal=false
EOF

echo "Desktop entry created: $DESKTOP_FILE"

# 4. Refresh desktop database
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$HOME/.local/share/applications/" 2>/dev/null || true
fi

echo ""
echo "=== Setup Complete ==="
echo ""
echo "To test in Desktop Mode:"
echo "  $APPIMAGE_DIR/$APPIMAGE_NAME.AppImage"
echo ""
echo "To add to Gaming Mode:"
echo "  1. Open Steam (Desktop Mode)"
echo "  2. Games → Add a Non-Steam Game"
echo "  3. Browse to: $APPIMAGE_DIR/$APPIMAGE_NAME.AppImage"
echo "  4. Add Selected Programs"
echo "  5. Switch to Gaming Mode — DeckSave will appear in your library"
echo ""
echo "Testing checklist:"
echo "  [ ] App launches and shows game list"
echo "  [ ] D-pad navigates between game cards"
echo "  [ ] A button (Enter) opens game detail modal"
echo "  [ ] B button (Escape) closes modals"
echo "  [ ] Bumpers (Tab/Shift+Tab) switch between nav tabs"
echo "  [ ] Scan finds Steam games with Proton save paths"
echo "  [ ] Backup creates zip in backup directory"
echo "  [ ] Restore extracts back to save paths"
echo "  [ ] All touch targets are 48px+ (fingertip friendly)"
echo "  [ ] Text is readable at 1280x800"
echo "  [ ] Sync tab detects Syncthing (if installed)"
