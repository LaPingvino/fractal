# Fractal with the spaces work, from LaPingvino/fractal (branch spaces), for
# Fedora (GTK ≥ 4.20, libadwaita ≥ 1.8). Built by .github/workflows/fedora.yml
# in a fedora:latest container; BuildRequires as pkgconfig() names, the ones
# Fractal's meson.build asks for.
%global branch spaces
%global debug_package %{nil}

Name:           fractal-spaces
Version:        14.3
Release:        0.git%{?snapdate}%{?dist}
Summary:        Matrix group messaging app, with spaces support (AI-assisted fork branch)
License:        GPL-3.0-only
URL:            https://github.com/LaPingvino/fractal
Source0:        %{url}/archive/refs/heads/%{branch}.tar.gz#/fractal-%{branch}.tar.gz

BuildRequires:  meson
BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  clang
BuildRequires:  git-core
BuildRequires:  blueprint-compiler
BuildRequires:  desktop-file-utils
BuildRequires:  libappstream-glib
BuildRequires:  pkgconfig(glib-2.0) >= 2.82
BuildRequires:  pkgconfig(gtk4) >= 4.20.2
BuildRequires:  pkgconfig(libadwaita-1) >= 1.8.0
BuildRequires:  pkgconfig(gstreamer-1.0)
BuildRequires:  pkgconfig(gstreamer-app-1.0)
BuildRequires:  pkgconfig(gstreamer-base-1.0)
BuildRequires:  pkgconfig(gstreamer-pbutils-1.0)
BuildRequires:  pkgconfig(gstreamer-play-1.0)
BuildRequires:  pkgconfig(gstreamer-video-1.0)
BuildRequires:  pkgconfig(gtksourceview-5)
BuildRequires:  pkgconfig(glycin-2)
BuildRequires:  pkgconfig(glycin-gtk4-2)
BuildRequires:  pkgconfig(libwebp)
BuildRequires:  pkgconfig(shumate-1.0)
BuildRequires:  pkgconfig(sqlite3)
BuildRequires:  pkgconfig(openssl)
BuildRequires:  pkgconfig(libseccomp)
BuildRequires:  pkgconfig(libpipewire-0.3)
BuildRequires:  pkgconfig(lcms2)

Requires:       gst-plugins-good
Provides:       fractal = %{version}-%{release}
Conflicts:      fractal

%description
Fractal is a Matrix messaging app for GNOME written in Rust. This build carries
the spaces work from LaPingvino/fractal (branch spaces), which is AI-assisted
and not offered to the Fractal project.

%prep
%autosetup -n fractal-%{branch}

%build
# As in the Arch package: full debug info with thin LTO and one codegen unit
# runs a CI runner out of memory; line tables keep backtraces readable.
export CARGO_HOME="$PWD/.cargo-home"
export CARGO_PROFILE_RELEASE_DEBUG=line-tables-only
%meson -Dprofile=default
# Fedora's build flags put -Cdebuginfo=2 in RUSTFLAGS, after the profile: line tables there too
export RUSTFLAGS="${RUSTFLAGS//-Cdebuginfo=2/-Cdebuginfo=line-tables-only}"
%meson_build

%install
export CARGO_HOME="$PWD/.cargo-home"
export CARGO_PROFILE_RELEASE_DEBUG=line-tables-only
%set_build_flags
export RUSTFLAGS="${RUSTFLAGS//-Cdebuginfo=2/-Cdebuginfo=line-tables-only}"
%meson_install
# what meson installed, exactly (no guessed paths to go stale)
(cd %{buildroot} && find . \( -type f -o -type l \) | sed 's|^\.||' | sort) > files.list

%files -f files.list
%license LICENSE
