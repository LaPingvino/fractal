#!/bin/bash
# The AppImage of the spaces work (F7): the debian:testing .deb (built first by
# packaging/debian/build-deb.sh) unpacked into an AppDir, then linuxdeploy with
# its GTK plugin (GTK 4) bundles the libraries.
#   packaging/appimage/build-appimage.sh <the .deb> <out-dir>
# Known limits, said plainly: it is built on Debian testing, so it needs a
# glibc at least that new (older distributions: use the Flatpak); glycin's
# image loaders and the GStreamer plugins come from the host system, as
# linuxdeploy-plugin-gtk doesn't bundle them.
set -euo pipefail
deb=$1 out=${2:-$PWD}
apt-get install -y --no-install-recommends file wget librsvg2-dev librsvg2-bin
# linuxdeploy-plugin-gtk copies GTK's module folder (<libdir>/gtk-4.0); Debian's
# GTK 4.24 has it only with an optional module package — the GStreamer media
# backend where there is one, and the folder made either way
apt-get install -y --no-install-recommends libgtk-4-media-gstreamer || true
mkdir -p "$(pkg-config --variable=libdir gtk4)/gtk-4.0"
work=$(mktemp -d)
dpkg-deb -x "$deb" "$work/AppDir"
# appimagetool refuses a 0700 AppDir (mktemp's) and wants a PNG icon at its
# top level named after the desktop file's Icon=
rsvg-convert -w 256 -h 256 "$work/AppDir/usr/share/icons/hicolor/scalable/apps/org.gnome.Fractal.svg" -o "$work/AppDir/org.gnome.Fractal.png"
chmod -R a+rX "$work"
cd "$work"
wget -q https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
wget -q https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh
chmod +x linuxdeploy-x86_64.AppImage linuxdeploy-plugin-gtk.sh
export APPIMAGE_EXTRACT_AND_RUN=1 DEPLOY_GTK_VERSION=4 ARCH=x86_64 LDAI_OUTPUT=fractal-spaces-x86_64.AppImage
./linuxdeploy-x86_64.AppImage --appdir AppDir --plugin gtk --output appimage \
  --desktop-file AppDir/usr/share/applications/org.gnome.Fractal.desktop \
  --icon-file AppDir/usr/share/icons/hicolor/scalable/apps/org.gnome.Fractal.svg
mv fractal-spaces-x86_64.AppImage "$out/"
ls -l "$out/fractal-spaces-x86_64.AppImage"
