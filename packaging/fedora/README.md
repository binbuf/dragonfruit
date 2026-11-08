# Fedora packaging

Lands in T164 (Fedora packaging and CI; still OPEN). Until then this
directory anchors the monorepo layout from
[docs/design/01-architecture.md](../../docs/design/01-architecture.md).

## Release asset

[`dragonfruit.spec`](dragonfruit.spec) is the **binary** RPM spec used by
`.github/workflows/release.yml`. It packages the staged install tree produced
by `scripts/stage-install.sh`; it is not the Fedora review/COPR source spec.
Build it locally with:

```bash
make dist FORMATS=rpm
```

T164 still owns the source RPM, vendored crates, Fedora review, and COPR.

The Fedora package will carry the pinned toolchain expectations from
the repository root: Rust per `rust-toolchain.toml`, Qt 6.11 per the
top-level `CMakeLists.txt`, Smithay `=0.7.0` per the root `Cargo.toml`.

## Optional dependency markers (T220)

The canonical marker table is
[`../optional-services.json`](../optional-services.json), shared with Debian.
RPM dependency fields follow its `fedora` column:

- `Recommends:` for the preferred backend or strongly expected service
  (restic, GNOME Online Accounts, Orca, speech-dispatcher, ibus/fcitx5,
  PackageKit, Flatpak, CUPS, SANE, colord, fprintd, accountsservice, and
  firewalld).
- `Suggests:` for a useful alternate or future delegate (Tracker/Baloo/Recoll,
  Pika Backup/Timeshift, espeak-ng, LocalSend, GNOME Software/Discover, snapd,
  ufw).
- Never `Requires:` for an optional service. The desktop must install, start,
  and work with the required host stack only; every optional feature has an
  absence path.
- Do not require a display manager. Ship the `dragonfruit.desktop` session
  entry and support GDM, SDDM, LightDM, and greetd through the per-DM adapter
  (ADR 0173).
- GPL/LGPL services remain separate processes or dynamically linked libraries
  with the notices in `NOTICE` and `docs/licensing.md`. No GPL code is copied
  into the MIT packages.

Package names are verified against the actual Fedora archive at packaging time
(T259); the JSON records the expected name and the marker. LocalSend is not
packaged in Fedora 44 today, so its `Suggests` applies only when a package
exists; the feature is omitted in 0.2.0.
