#!/usr/bin/env bash
set -euo pipefail

APP_ID="io.github.Franck-Demongin.Trimbox"
ICON_NAME="$APP_ID"

DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
APPS_DIR="$DATA_HOME/applications"
ICONS_ROOT="$DATA_HOME/icons/hicolor"

rm -f "$HOME/.local/bin/trimbox-gui"
rm -f "$APPS_DIR/${APP_ID}.desktop"

for size in 16 32 48 64 128 256 512; do
  rm -f "$ICONS_ROOT/${size}x${size}/apps/${ICON_NAME}.png"
done
rm -f "$ICONS_ROOT/scalable/apps/${ICON_NAME}.svg"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$ICONS_ROOT" >/dev/null 2>&1 || true
fi

echo "Trimbox désinstallé localement."
