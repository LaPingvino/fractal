# Fractal with spaces — made for screen reader users

This is a build of [Fractal](https://gitlab.gnome.org/World/fractal), the GNOME Matrix client, with
support for Matrix spaces, built first of all for people who use their computer through a screen reader.

## Who this is for, and why

It was made for a blind friend of the author who uses Matrix with a screen reader (Orca on the Linux
desktop). For him, spaces are not a nice extra: they are how large communities are organised. Without them,
every room he has joined arrives as one long flat list that a screen reader reads out item by item, and
finding the rooms of one community — or the rooms it suggests joining — means listening through everything
else first. Few Matrix clients work well with a screen reader on the Linux desktop, and Fractal, being a GTK
app that speaks to Orca through the desktop's accessibility layer, is one of the better starting points. What
it lacked was spaces.

What this build changes, with the screen reader in mind:

- **Spaces in the sidebar**, with each space's rooms grouped under it, so a community can be reached in a few
  steps instead of searched for in the full list.
- **Every space icon is named**: the sidebar's space icons carry their space's name as a tooltip and
  label, and the purely decorative space icon is hidden from the screen reader, so it no longer
  announces an unnamed image.
- **Unread counts that add up**: a space shows the unread messages of all its rooms, including rooms in
  sub-spaces, so you can tell from the space alone whether anything needs attention.
- **A space page** that says what it is: the space's name in the header, a settings button described for the
  screen reader ("View rooms and join suggested rooms in this space"), the rooms of the space listed with a
  clear way to open each one.
- **Joining suggested rooms with a real button**: instead of rows that have to be activated in a way you would
  have to know about, each suggested room has a Join button, which says "Joined" once it worked and comes
  back if it didn't — so the result is announced, not guessed.
- **Behind the scenes**: one central record of which rooms belong to which spaces, so the sidebar doesn't
  stall while a big account loads (the "back" button used to freeze), with 24 tests.

Every change to this branch is judged first by how it reads to a screen reader, not only by how it looks. A
full pass through each screen with Orca and the AT-SPI tree is under way; until it's done, the list above is
what the code does, not yet a guarantee of how every screen sounds. If something is announced badly, missing a name, or can't be reached with
the keyboard, that is a bug here — please report it on this repository's issues.

### Part of a wider accessibility effort

This branch is one piece of a longer effort by the same author to make free software usable with a screen
reader:

- **[Wally](https://github.com/LaPingvino/wally)**, an accessible Matrix web client (a fork of Cinny): ARIA
  roles and live regions throughout, keyboard shortcuts for navigation and for moving through unread rooms,
  screen-reader labels on every login and settings form, notification sounds, and a tree-view room list.
- **Screen readers for the [Fyne](https://fyne.io) toolkit** (Go), on every desktop, in a
  [fork of Fyne](https://github.com/LaPingvino/fyne/tree/linux-a11y): one shared accessibility bridge, with
  AT-SPI2 for Orca on Linux (through [atspi](https://github.com/LaPingvino/atspi), written for it in pure
  Go), UI Automation on Windows for screen readers and braille displays, and NSAccessibility for VoiceOver on
  macOS. With it come accessible names (set by the application, or worked out automatically), a sensible
  reading order, and keyboard use where it was missing — tab bars, Escape for dialogs and pop-ups.
  **[Accolade](https://github.com/LaPingvino/accolade)**, a screenplay editor, is the first application built
  on it, with a test in its CI that reads Accolade on Windows the way a screen reader does. Smaller fixes from
  the same work are already part of Fyne itself.

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

## Why this is not in Fractal itself

Because this work was made with an AI assistant (Claude), it can't go into Fractal. Fractal's contribution
guide says:

> Contributions must not include content generated by large language models or other probabilistic tools
> like ChatGPT, Claude, and Copilot.

That stance was taken in the early days of AI-assisted programming, as a complete ban on code, documentation,
issues and artwork alike, and for reasons that were understandable then: how the training data was gathered,
the energy the models use, worries about quality and about copyright. As things stand, this fork is all we
can do — and I've had to tell my blind friend, who this was made for, that the work can't be upstreamed. Ever,
unless that stance changes.

That is a pity, because this is exactly where AI helps most: with the things a developer is not good at.
Linus Torvalds says it about his own hobby project
[AudioNoise](https://github.com/torvalds/AudioNoise): "the python visualizer tool has been basically written by
vibe-coding. I know more about analog filters — and that's not saying much — than I do about python. […]
I cut out the middle-man — me — and just used Google Antigravity to do the audio sample visualizer." That was
in January 2026, with the models of that moment; they have become a good deal better since. For me
that is accessibility, and things like documentation: work that matters a great deal to the people who need
it, and that rarely gets done at all. The Linux kernel accepts AI-assisted contributions under its normal
rules, and in July 2026 Torvalds put it plainly on the kernel mailing list: "AI is a tool, just like other
tools we use. And it's clearly a useful one."
([Phoronix](https://www.phoronix.com/news/Linux-Is-Not-Anti-AI),
[Simon Willison](https://simonwillison.net/2026/Jul/16/linus-torvalds/))

My own view: most code ever written is slop, by people as much as by machines. AI-written code has one thing
going for it — like code run through `go fmt`, it tends to follow the standards by default, which makes it
easier to read and to check. The work here was reviewed as any change should be, and its commits say openly
how they were made (`Co-Authored-By: Claude`). It is kept small, in Fractal's own style, and rebased onto
Fractal's main every night, so that it is ready the day the door opens.

— Joop Kiefte

The source is on the `spaces` branch of this repository.
