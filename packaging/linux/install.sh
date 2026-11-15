#!/bin/sh
# Northview Office per-user installer for Linux.
# Works from the unpacked package (./install.sh) or from the source checkout
# (packaging/linux/install.sh, after `cargo build --release`).
set -e
APP_ID="SturlDigital.NorthviewOffice"
SRC="$(cd "$(dirname "$0")" && pwd)"
DEST="${XDG_DATA_HOME:-$HOME/.local/share}"

if [ -f "$SRC/northview-office" ]; then
    # unpacked package: everything sits next to this script
    BIN="$SRC/northview-office"
    ICON_256="$SRC/icon.png"
    ICON_512="$SRC/icon-512.png"
else
    # source checkout: packaging/linux/ -> project root
    ROOT="$(cd "$SRC/../.." && pwd)"
    BIN="$ROOT/target/release/nvdental-office-app"
    ICON_256="$ROOT/assets/icon.png"
    ICON_512="$ROOT/assets/icon-512.png"
    if [ ! -f "$BIN" ]; then
        echo "No release build found at $BIN" >&2
        echo "Build it first:  cd \"$ROOT\" && cargo build --release" >&2
        exit 1
    fi
fi
for f in "$ICON_256" "$ICON_512"; do
    [ -f "$f" ] || { echo "Missing icon: $f" >&2; exit 1; }
done

mkdir -p "$DEST/northview-office" "$DEST/applications" \
         "$DEST/icons/hicolor/512x512/apps" "$DEST/icons/hicolor/256x256/apps"
install -m 755 "$BIN" "$DEST/northview-office/northview-office"
install -m 644 "$ICON_512" "$DEST/icons/hicolor/512x512/apps/$APP_ID.png"
install -m 644 "$ICON_256" "$DEST/icons/hicolor/256x256/apps/$APP_ID.png"

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
