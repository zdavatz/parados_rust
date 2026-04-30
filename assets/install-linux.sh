#!/usr/bin/env bash
# Install Parados into the user's desktop environment.
#
# Drops the binary into ~/.local/bin, the .desktop file into
# ~/.local/share/applications and the icon into the standard hicolor
# theme directory.  Run from inside the unpacked release archive.

set -euo pipefail

bin_dir="$HOME/.local/bin"
apps_dir="$HOME/.local/share/applications"
icons_dir="$HOME/.local/share/icons/hicolor/512x512/apps"

mkdir -p "$bin_dir" "$apps_dir" "$icons_dir"

cp -v parados            "$bin_dir/"
cp -v parados.desktop    "$apps_dir/"
cp -v icon.png           "$icons_dir/parados.png"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$apps_dir" 2>/dev/null || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo
echo "Installed to $bin_dir."
echo "Make sure $bin_dir is on your PATH:  export PATH=\"\$HOME/.local/bin:\$PATH\""
echo "Launch from your application menu or run: parados"
