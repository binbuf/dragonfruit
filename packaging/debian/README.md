# Debian packaging

Lands in T165 (Debian packaging and CI; still OPEN). Until then this
directory only anchors the monorepo layout from
[docs/design/01-architecture.md](../../docs/design/01-architecture.md).

Note for T165: Qt 6.11 availability on Debian vs Fedora is an open
question (T-01 risks); resolve it there, not by lowering the CMake
minimum.

## Optional dependency markers (T220)

The canonical marker table is
[`../optional-services.json`](../optional-services.json), shared with Fedora.
Debian control fields follow its `debian` column:

- `Recommends:` for the preferred backend or strongly expected service
  (restic, GNOME Online Accounts, Orca, speech-dispatcher, ibus/fcitx5,
  PackageKit, Flatpak, CUPS, SANE, colord, fprintd, accountsservice, and the
  distro's firewall adapter).
- `Suggests:` for a useful alternate or future delegate (Tracker/Baloo/Recoll,
  Pika Backup/Timeshift, espeak-ng, LocalSend, GNOME Software/Discover, snapd,
  the non-native firewall adapter).
- Never `Depends:` for an optional service. The desktop must install, start,
  and work with the required host stack only; every optional feature has an
  absence path.
- Do not depend on a display manager. Ship the `dragonfruit.desktop` session
  entry and support GDM, SDDM, LightDM, and greetd through the per-DM adapter
  (ADR 0173).
- GPL/LGPL services remain separate processes or dynamically linked libraries
  with the notices in `NOTICE` and `docs/licensing.md`. No GPL code is copied
  into the MIT packages.

Package names are verified against the actual Debian/Ubuntu archive at
packaging time (T259); the JSON records the expected name and the marker.
