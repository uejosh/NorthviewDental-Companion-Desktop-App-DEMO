#!/bin/sh
# Removes only the files install.sh created. Settings in ~/.local/share/NorthviewOffice are kept.
APP_ID="SturlDigital.NorthviewOffice"
DEST="${XDG_DATA_HOME:-$HOME/.local/share}"

rm -f "$DEST/northview-office/northview-office" \
      "$DEST/applications/$APP_ID.desktop" \
      "$DEST/icons/hicolor/512x512/apps/$APP_ID.png" \
      "$DEST/icons/hicolor/256x256/apps/$APP_ID.png"
rmdir "$DEST/northview-office" 2>/dev/null || true
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DEST/applications" || true
echo "Northview Office removed."
