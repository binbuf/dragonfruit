# Debian packaging

Lands in T165 (Debian packaging and CI; still OPEN). Until then this
directory anchors the monorepo layout from
[docs/design/01-architecture.md](../../docs/design/01-architecture.md).

## Release asset

[`control.in`](control.in), [`postinst`](postinst), and [`postrm`](postrm)
are used by `.github/workflows/release.yml` to build a binary DEB from the
staged install tree. Build it locally with:

```bash
make dist FORMATS=deb
```

This is an early binary asset, not a `debian/` source package. T165 still owns
Debian policy, source packaging, and clean-VM validation.

The CMake floor is Qt 6.8, which is the real API floor (`QChronoTimer` in the
shell), not an arbitrary pin. Debian 13 ships Qt 6.8, so the DEB is built
there against the system Qt; Ubuntu 25.04+ inherits it. Ubuntu 24.04 and
Debian 12 ship Qt 6.4 and use the portable bundle. T165 still owns the
`debian/` source package and policy review.

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
