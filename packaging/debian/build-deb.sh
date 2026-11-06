#!/bin/bash
# The .deb of the spaces work (F7), for Debian testing and Ubuntu's development
# series — the only ones with GTK ≥ 4.20.2, libadwaita ≥ 1.8 and glycin 2
# (Debian 13 "trixie" has GTK 4.18 and libadwaita 1.7: too old). Run as root in
# a debian:testing or ubuntu:devel container from the build branch's checkout:
#   packaging/debian/build-deb.sh <out-dir>
# Not a policy-conformant Debian source package: a snapshot built with meson,
# its runtime Depends from dpkg-shlibdeps, packed with dpkg-deb.
set -euo pipefail
out=${1:-$PWD}
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends \
  ca-certificates curl git build-essential clang pkg-config meson ninja-build \
  blueprint-compiler desktop-file-utils appstream-util gettext dpkg-dev \
  libglib2.0-dev libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev libshumate-dev \
  libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev \
  libglycin-2-dev libglycin-gtk4-2-dev libwebp-dev libsqlite3-dev libssl-dev \
  libseccomp-dev libpipewire-0.3-dev liblcms2-dev

# Rust ≥ 1.96 (Fractal's rust-version) whatever the release ships, and grass
# for the stylesheet (Fractal takes grass or dart-sass; neither is packaged)
export CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup PATH=/opt/cargo/bin:$PATH
curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
cargo install grass --locked

src=$(mktemp -d)
curl -fsSL "https://github.com/${REPO:-LaPingvino/fractal}/archive/refs/heads/spaces.tar.gz" | tar -xz -C "$src" --strip-components=1
cd "$src"
# as in the Arch and Fedora packages: line tables, or the runner runs out of memory
export CARGO_PROFILE_RELEASE_DEBUG=line-tables-only
meson setup _build --prefix=/usr -Dprofile=default --buildtype=release
meson compile -C _build
root=$(mktemp -d)
DESTDIR="$root" meson install -C _build --no-rebuild

version="14.3~git$(date +%Y%m%d)"
arch=$(dpkg --print-architecture)
mkdir -p "$root/DEBIAN" debian
# dpkg-shlibdeps wants a debian/control to sit beside
printf 'Source: fractal-spaces\n\nPackage: fractal-spaces\nArchitecture: any\n' > debian/control
deps=$(dpkg-shlibdeps -O -e"$root/usr/bin/fractal" 2>/dev/null | sed -n 's/^shlibs:Depends=//p')
cat > "$root/DEBIAN/control" <<CTL
Package: fractal-spaces
Version: $version
Architecture: $arch
Maintainer: Joop Kiefte <ikojba@gmail.com>
Depends: $deps, gstreamer1.0-plugins-good, glycin-loaders
Conflicts: fractal
Provides: fractal
Replaces: fractal
Section: net
Priority: optional
Homepage: https://github.com/LaPingvino/fractal
Description: Matrix group messaging app, with spaces support (AI-assisted fork branch)
 Fractal is a Matrix messaging app for GNOME written in Rust. This build
 carries the spaces work from LaPingvino/fractal (branch spaces), which is
 AI-assisted and not offered to the Fractal project.
CTL
. /etc/os-release
dpkg-deb --root-owner-group --build "$root" "$out/fractal-spaces-${ID}-${arch}.deb"
dpkg-deb --info "$out/fractal-spaces-${ID}-${arch}.deb" | sed -n '/Package:/,/Description:/p'
# the check: apt resolves every Depends and installs it
apt-get install -y "$out/fractal-spaces-${ID}-${arch}.deb"
fractal --version || true
