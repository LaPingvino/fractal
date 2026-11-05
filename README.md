# Fractal with spaces

This is a build of [Fractal](https://gitlab.gnome.org/World/fractal), the GNOME Matrix client, with
extra support for Matrix spaces: spaces in the sidebar, a space page, and work on how both read with a
screen reader.

It installs next to the regular Fractal as "Fractal (Development)", so both can be used.

## Install

You need Flatpak with Flathub. Most distributions have it; if not, see https://flathub.org/setup.

1. Download `fractal-spaces.flatpak` from the release page:
   https://github.com/LaPingvino/fractal/releases/tag/spaces-latest
2. In a terminal, in the folder where you saved it, run:

       flatpak install --user fractal-spaces.flatpak

   Flatpak asks to also install the GNOME runtime from Flathub; answer yes.
3. Start it from your applications menu as "Fractal (Development)", or with:

       flatpak run org.gnome.Fractal.Devel

### On Arch Linux, as a native package

Download `fractal-spaces-x86_64.pkg.tar.zst` from the same release page and run:

    sudo pacman -U fractal-spaces-x86_64.pkg.tar.zst

It replaces the regular `fractal` package (both use the same program name). To build it yourself:
`packaging/arch/PKGBUILD` on this branch, with `makepkg -si`.

## Update

Download the file again and run the same install command. A new build is made every night and whenever
the work changes.

## Remove

    flatpak uninstall --user org.gnome.Fractal.Devel

## About this build

The spaces work was written with the help of an AI assistant (Claude). Fractal's maintainers do not accept
AI-assisted contributions, so it is not part of Fractal itself and is not offered to them. The source is on
the `spaces` branch of this repository.
