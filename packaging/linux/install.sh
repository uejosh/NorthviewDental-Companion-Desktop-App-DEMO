#!/bin/sh
# Northview Office per-user installer for Linux. Run from the unpacked folder: ./install.sh
set -e
APP_ID="SturlDigital.NorthviewOffice"
SRC="$(cd "$(dirname "$0")" && pwd)"
DEST="${XDG_DATA_HOME:-$HOME/.local/share}"

mkdir -p "$DEST/northview-office" "$DEST/applications" \
         "$DEST/icons/hicolor/512x512/apps" "$DEST/icons/hicolor/256x256/apps"
install -m 755 "$SRC/northview-office" "$DEST/northview-office/northview-office"
install -m 644 "$SRC/icon-512.png" "$DEST/icons/hicolor/512x512/apps/$APP_ID.png"
install -m 644 "$SRC/icon.png" "$DEST/icons/hicolor/256x256/apps/$APP_ID.png"

# Wayland desktops match a window to its launcher by app ID, so the file name must be the app ID.
cat > "$DEST/applications/$APP_ID.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Northview Office
Comment=Northview Dental front-desk app
Exec=$DEST/northview-office/northview-office
Icon=$APP_ID
StartupWMClass=$APP_ID
Categories=Office;
Terminal=false
EOF

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DEST/applications" || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t "$DEST/icons/hicolor" || true
echo "Northview Office installed. Find it in your applications menu."
