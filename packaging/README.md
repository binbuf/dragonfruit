# Packaging and release assets

This directory owns the packaging metadata. The release path is:

```text
scripts/stage-install.sh   one install tree (session, portal, QML, binaries)
        │
        ├── scripts/package.sh --formats tar    portable bundle
        ├── scripts/package.sh --formats deb    Debian binary package
        ├── scripts/package.sh --formats rpm    Fedora binary RPM
        └── scripts/package.sh --formats arch   Arch binary package
```

The binary packages are built natively on the oldest supported release of each
family (Debian 13, Fedora 43, Arch) against that release's own Qt. A binary
built against the oldest Qt runs on newer releases of the same family, and the
declared dependency version is derived from the build host. No Qt or system
library is bundled inside a distro package; the package declares its native
dependencies and lets the distro resolve them.

## Local builds

```bash
make stage                    # dist/stage/usr/... for inspection
make dist                     # tar.gz + the deb/rpm this host can build
make dist FORMATS=tar         # portable bundle only
make dist FORMATS=tar,deb     # no rpmbuild on this host
make dist FORMATS=tar,rpm     # no dpkg-deb on this host
make dist FORMATS=tar,arch    # on an Arch host (needs bsdtar + zstd)
make dist VERSION=0.2.0       # override the git-derived version
```

`make dist` detects `dpkg-deb` and `rpmbuild`; `FORMATS=` overrides the
detection. The release workflow passes explicit formats, so a missing tool is
a hard failure there.

Or call the script directly:

```bash
scripts/package.sh --version 0.2.0 --output dist --formats tar,deb,rpm,arch
```

Outputs:

```text
dist/dragonfruit-<version>-linux-<arch>.tar.gz
dist/dragonfruit-desktop_<version>_<deb-arch>.deb
dist/dragonfruit-desktop-<version>-1.<arch>.rpm
dist/dragonfruit-desktop-<version>-1-<arch>.pkg.tar.zst
dist/SHA256SUMS
```

The DEB needs `dpkg-deb` (`dpkg-dev` on Debian/Ubuntu). The RPM needs
`rpmbuild` (`rpm-build` on Fedora, `rpm` on Debian/Ubuntu). The Arch package
needs `bsdtar` (`libarchive`) and `zstd`. Use `--formats tar` on a host without
those tools.

## What the assets do

- Install everything under `/usr` in the `dragonfruit` namespace.
- Install the Wayland session entry, systemd user units, portal discovery
  files, first-party apps, QML modules, icons, translations, and wallpaper.
- Never remove another desktop, preselect a display manager, arm autologin,
  or change default applications.
- Never run an interactive installer from a package scriptlet. The package
  scriptlets only refresh desktop/icon caches.
- Declare native dependencies; do not bundle Qt or system libraries. The QML
  modules the shell imports at startup are hard `Depends`/`Requires`, not
  recommendations, because a minimal install cannot load the shell without
  them.
- Build each binary package on the oldest supported release of its family
  against that release's own Qt. The declared Qt version comes from the build
  host (`pkg-config --modversion Qt6Core`), never a hard-coded value.

The portable tar's `install.sh`/`uninstall.sh` follow the same rules. The
uninstaller removes exactly the paths in the staged manifest and keeps user
settings under `~/.config/dragonfruit/` and `~/.local/state/dragonfruit/`.

## Release CI

[`.github/workflows/release.yml`](../.github/workflows/release.yml) runs on
`v*` tags (and manual dispatch) and publishes:

- portable `tar.gz` + `deb` (Debian 13 build)
- binary `rpm` (Fedora 43 build)
- binary `pkg.tar.zst` (Arch build)
- source `tar.gz` (`git archive`)
- `SHA256SUMS`

Each package job installs its own output in the same container and fails if
the session entry is missing or an ELF binary has an unresolved shared
library. That is the dependency check: if the declared Qt floor is wrong for
the build release, the job fails instead of publishing an unusable asset.

## Scope

These are **early-testing binary assets**, not official distro packages:

- The binary assets target current stable releases with Qt 6.8 or newer
  (Debian 13, Fedora 43+, Ubuntu 25.04+, Arch). Older LTS releases
  (Ubuntu 24.04, Debian 12) and non-packaged distributions use the portable
  bundle.
- T164 still owns Fedora review, source RPMs/vendored crates, and COPR.
- T165 still owns Debian policy, `debian/` source packaging, and clean-VM
  validation.
- T259 verifies package names and licenses against the actual archives.
- An Arch `PKGBUILD`/AUR entry is a later official-packaging task; the release
  asset is a binary `pkg.tar.zst` installable with `pacman -U`.

Flatpak is intentionally not produced for the desktop. A compositor/session
needs DRM, logind, udev, systemd user units, and a host portal backend; those
are outside the Flatpak app sandbox. Flatpak can be considered later for
individual first-party apps, not for the desktop itself.

The canonical optional-dependency markers remain
[`optional-services.json`](optional-services.json).
