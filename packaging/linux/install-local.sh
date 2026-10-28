#!/usr/bin/env bash
set -euo pipefail

APP_ID="io.github.Franck-Demongin.Trimbox"
APP_NAME="Trimbox"
ICON_NAME="$APP_ID"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ASSETS_DIR="$PROJECT_ROOT/assets/icons"

if [ ! -d "$ASSETS_DIR" ]; then
  echo "Répertoire d'icônes introuvable : $ASSETS_DIR" >&2
  exit 1
fi

if command -v trimbox-gui >/dev/null 2>&1; then
  TRIMBOX_BIN="$(command -v trimbox-gui)"
else
  if [ -x "$HOME/.cargo/bin/trimbox-gui" ]; then
    TRIMBOX_BIN="$HOME/.cargo/bin/trimbox-gui"
    mkdir -p "$HOME/.local/bin"
    ln -sfn "$TRIMBOX_BIN" "$HOME/.local/bin/trimbox-gui"
  else
    echo "Binaire trimbox-gui introuvable. Installez-le d'abord avec cargo install --features gui." >&2
    exit 1
  fi
fi

DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
APPS_DIR="$DATA_HOME/applications"
ICONS_ROOT="$DATA_HOME/icons/hicolor"

for size in 16 32 48 64 128 256 512; do
  target_dir="$ICONS_ROOT/${size}x${size}/apps"
  mkdir -p "$target_dir"
  install -m 0644 "$ASSETS_DIR/trimbox-${size}.png" "$target_dir/${ICON_NAME}.png"
done

mkdir -p "$ICONS_ROOT/scalable/apps"
install -m 0644 "$ASSETS_DIR/trimbox.svg" "$ICONS_ROOT/scalable/apps/${ICON_NAME}.svg"

mkdir -p "$APPS_DIR"
installed_desktop="$APPS_DIR/${APP_ID}.desktop"
template="$PROJECT_ROOT/packaging/linux/${APP_ID}.desktop"

sed \
  -e "s|^TryExec=.*|TryExec=${TRIMBOX_BIN}|" \
  -e "s|^Exec=.*|Exec=${TRIMBOX_BIN}|" \
  "$template" > "$installed_desktop"
chmod 0644 "$installed_desktop"

# Remove stale files left by the previous "trimbox" icon/desktop naming.
# They are not used anymore and can keep GNOME associated with the wrong icon.
rm -f "$APPS_DIR/trimbox.desktop"
for size in 16 32 48 64 128 256 512; do
  rm -f "$ICONS_ROOT/${size}x${size}/apps/trimbox.png"
done
rm -f "$ICONS_ROOT/scalable/apps/trimbox.svg"

if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$installed_desktop"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$ICONS_ROOT" >/dev/null 2>&1 || true
fi

echo "Installation terminée."
echo "Identifiant : $APP_ID"
echo "Bureau : $installed_desktop"
echo "Icônes : $ICONS_ROOT"
