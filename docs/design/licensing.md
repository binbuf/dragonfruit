# Licensing Policy

This document records the Dragonfruit licensing decision. It is a
**blocker for the first public release** (see
[14-risks.md](14-risks.md)): nothing is
distributed publicly until the rules below are reviewed and signed off.

## Decision

Dragonfruit is licensed under the **GNU General Public License, version 3
only** (**GPL-3.0-only**), repo-wide. One license covers every
first-party component, so there are no cross-component linking
restrictions to track beyond GPL-3.0-only compatibility.

| Component | License |
|---|---|
| `compositor/` (Rust + Smithay) | GPL-3.0-only |
| `services/` (settingsd, menu-broker, app-index, session) | GPL-3.0-only |
| `portal/` (xdg-desktop-portal-dragonfruit) | GPL-3.0-only |
| `tools/` (dev tooling) | GPL-3.0-only |
| `protocols/df-ipc/` (shared IPC crate) | GPL-3.0-only |
| `protocols/*.xml` (private Wayland protocols) | GPL-3.0-only |
| `design-system/` (QML module `Dragonfruit`) | GPL-3.0-only |
| `shell/` (menu bar, Dock, Quick Settings, notifications, screenshot) | GPL-3.0-only |
| `apps/` (Settings, Files) | GPL-3.0-only |
| `assets/fonts/Inter/` (system font, embedded via `libs/system-font/`) | SIL OFL 1.1 |
| `assets/icons/tabler/` (line-glyph set and pinned filled marks) | MIT |
| `assets/icons/phosphor/` (filled Settings-tile glyphs) | MIT |
| `assets/icons/brand/` (Dragonfruit and Tux OS-menu marks) | GPL-3.0-only (original project assets) |
| `assets/icons/whitesur/` (colored place/app artwork and the Files app icon) | GPL-3.0-only |
| `assets/cursors/capitaine-cursors/` (system cursor theme) | LGPL-3.0-or-later |
| `assets/graphics/wallpapers/` (`Default.jpg`, `Lagoon.jpg`, `Midnight.jpg`; shipped built-in wallpapers) | GPL-3.0-only (original project assets) |

Rationale: GPL-3.0-only keeps the desktop and every improvement to it free
software. Anyone who distributes a modified Dragonfruit — a fork, a
downstream distro build, or an appliance — must publish the corresponding
source under the same terms, so upstream fixes and features cannot be taken
proprietary. The copyleft also matches the environment the desktop already
lives in: Qt is used under the (L)GPL and WhiteSur already ships under
GPL-3.0-only. Credit is retained through the copyright notices, and the
license text must accompany every distribution. Name/logo credit is handled
separately; see "Trademarks".

Every binary distribution must ship the GPL-3.0 text and make the
corresponding source available under GPL-3.0-only. The packaging and release
assets include [LICENSES/GPL-3.0-only.txt](../../LICENSES/GPL-3.0-only.txt);
releases are built from the tagged public repository.

## Linking rules

Because all first-party code is GPL-3.0-only, dependencies and linked
components must be GPL-3.0-only compatible. Two rules matter:

- **`df-ipc` must stay dependency-free of everything but the standard
  library**, so that any process can embed the contracts.
- **No first-party component may pull in AGPL, GPL-2.0-only, or other
  GPL-3.0-incompatible code.** Such a dependency could not be distributed as
  part of the combined work; adding one is a review blocker. Permissive
  dependencies (MIT, Apache-2.0, BSD, ISC, Zlib, Unlicense) remain compatible
  and preferred for the dependency graph. Separable LGPL data assets are
  allowed when they are loaded at runtime, not linked into first-party code,
  and their license/source notice ships with the distribution (the Capitaine
  cursor theme below is the current case).

## Third-party dependencies

- **Rust crates.** The whole dependency graph is permissively licensed
  (MIT, Apache-2.0, BSD-2-Clause/BSD-3-Clause, ISC, Zlib, Unlicense).
  The exact set is pinned in `Cargo.lock`; see [NOTICE](../../NOTICE).
- **Qt is (L)GPL-licensed or commercial.** We use the open (L)GPL Qt
  under its terms, linked dynamically from the system/distro packages.
  GPL-3.0-only first-party code linking LGPL-3.0 Qt is compatible, and no
  Dragonfruit component may obligate a Qt commercial license.
- **System libraries** (libdrm, libinput, libseat, xkbcommon, wayland,
  systemd, NetworkManager, PipeWire, …) keep their own licenses; see
  [NOTICE](../../NOTICE).
- **Fonts.** Inter is bundled under the SIL OFL 1.1 (see
  [../../LICENSES/OFL-1.1.txt](../../LICENSES/OFL-1.1.txt)); it has no Reserved
  Font Name. OFL condition 2 requires the copyright notice and license text
  to accompany every copy of the Font Software, so any binary distribution
  (packages, images) must include `LICENSES/OFL-1.1.txt` or the equivalent
  notice — a packaging acceptance item for T-16.9/T-16.10.
- **Icons.** Tabler Icons 3.50.0 (`v3.50.0`, commit
  `6904efd245391ad396df17654738f52f6da8c40e`) is bundled under the MIT license
  (see [../../LICENSES/Tabler.txt](../../LICENSES/Tabler.txt)) as the line-glyph
  set and the pinned filled app marks. The Settings category tiles use
  Phosphor 2.0.8 (MIT, [../../LICENSES/Phosphor.txt](../../LICENSES/Phosphor.txt))
  because the tile needs a solid near-white glyph. Both sets are vendored as
  data, not linked code; the asset READMEs record the upstream version, commit,
  source path, and the exact pinned files, and `make check-tabler` /
  `make check-phosphor` fail if a registry or a glyph reference drifts. No
  Apple artwork is vendored or reproduced (see
  [14-risks.md](14-risks.md)).
- **WhiteSur place/app artwork.** WhiteSur `2026-09-10` (commit
  `73d8040da51a9ed74e47c7366e7e9ff437601a5c`) is bundled as a separable
  **GPL-3.0-only** data asset under `assets/icons/whitesur/`: colored folder,
  home/desktop, Trash, computer/volume, Files, and Applications-launcher SVGs.
  It is loaded by QML as files, never linked into first-party code, and can be
  replaced independently. Its copyleft terms now match the first-party tree;
  every binary distribution must ship the GPL-3.0 text, the provenance record
  in
  [`assets/icons/whitesur/README.md`](../../assets/icons/whitesur/README.md), and
  the corresponding source. See
  [ADR 0251](adr/0251-whitesur-place-icons.md). The artwork is
  third-party open work, not Apple artwork.
- **Cursor theme.** Capitaine cursors is bundled as a separable data asset
  under the LGPL-3.0-or-later (see
  [../../assets/cursors/capitaine-cursors/COPYING](../../assets/cursors/capitaine-cursors/COPYING)
  and [../../LICENSES/LGPL-3.0-or-later.txt](../../LICENSES/LGPL-3.0-or-later.txt)).
  It is not linked into first-party code: the compositor loads the XCursor
  files at runtime, the theme directory can be replaced independently, and
  the upstream source is identified in
  [../../assets/cursors/capitaine-cursors/README.md](../../assets/cursors/capitaine-cursors/README.md).
  Any binary distribution must ship that notice and license text, and must
  make the corresponding upstream source available under the LGPL terms.

## Asset manifest

Every shipped icon, font, cursor, and wallpaper — plus the runtime wallpaper
provider and the not-yet-shipped sound and illustration categories — has a
source/license row in
[asset-manifest.json](asset-manifest.json). The manifest names
the path, upstream source, version, license, license text, notice, and install
location. `make check-assets` validates it and fails if a file under `assets/`
is unlisted. No asset with an unknown license may ship.

## Optional services and packaging markers

The chosen optional services are separate processes (or dynamically linked
libraries) with the notices in [NOTICE](../../NOTICE). They are never hard
dependencies: the core desktop installs, starts, and works with the required
host stack only. The canonical marker table is
[packaging/optional-services.json](../../packaging/optional-services.json),
shared by the Fedora and Debian packages:

- `Recommends` for the preferred backend or strongly expected service (restic,
  GNOME Online Accounts, Orca, speech-dispatcher, ibus/fcitx5, PackageKit,
  Flatpak, CUPS, SANE, colord, fprintd, accountsservice, and the distro's
  firewall adapter).
- `Suggests` for a useful alternate or future delegate (Tracker/Baloo/Recoll,
  Pika Backup/Timeshift, espeak-ng, LocalSend, GNOME Software/Discover, snapd,
  and the non-native firewall adapter).
- Never `Requires`/`Depends` for an optional service. Package names and
  licenses are verified against the actual archive at packaging time (T259).

The eight naming/licensing decisions are recorded in
[ADR 0229](adr/0229-naming-and-optional-service-decisions.md). Display
managers are session-entry integrations, not dependencies; GDM is the reference
and SDDM the secondary tested path, with LightDM and greetd supported.

## Icon asset governance

Icons are third-party data assets, not linked code. Every vendored icon set
follows the same credit package:

- **Pinned source.** The asset README records the upstream repository, the
  exact release tag and commit, the upstream directory/file naming, and the
  license. The files themselves are copied unchanged (flattened with a
  `-filled` suffix where the set has a second weight), so a future update is a
  reviewable file swap plus a registry regeneration.
- **License text and notice.** The set ships its own `LICENSE` beside the
  assets, a copy in `LICENSES/`, a row in [NOTICE](../../NOTICE), and a row in the
  component table above. MIT sets need no special distribution obligation
  beyond preserving the notice; a new copyleft icon set must be recorded as an
  explicitly separable data asset, as WhiteSur is in ADR 0251, and every
  binary distribution must ship the GPL-3.0 text and corresponding source.
- **One registry per set.** `scripts/gen-tabler-glyphs.py` and
  `scripts/gen-phosphor-glyphs.py` extract the path data into generated QML
  singletons; `--check` fails the build if the committed registry is stale.
  `scripts/check-tabler-icons.py` and `scripts/check-phosphor-icons.py` fail if
  a QML source names a glyph that is not vendored, including the legacy
  `Icon.qml` alias map and the dynamic service vocabulary.
- **Current split.** Tabler 3.50.0 is the line-glyph set (Files, menu bar,
  Dock, shell chrome, and the `Icon.qml` facade). Phosphor 2.0.8 is the filled
  Settings-tile set. WhiteSur is the colored place/app artwork: folder,
  home/desktop, Trash, computer/volume, Files, and Applications-launcher icons.
  The Settings app artwork uses Tabler's filled mark; the Files app artwork is
  a byte-for-byte copy of WhiteSur's `file-manager.svg`. Tabler and Phosphor
  are MIT; WhiteSur is GPL-3.0-only and separable (ADR 0251).
- **No Apple artwork.** The original-assets rule in
  [14-risks.md](14-risks.md) still applies: third-party open icon sets
  are allowed; Apple's assets and pixel-copied proprietary artwork are not.

## Qt commercial decision trigger

From [14-risks.md](14-risks.md): if the
project ever needs a **statically linked, non-LGPL Qt distribution, or
Qt modules only available under a commercial license**, that is a
deliberate project-level decision — it changes distribution terms and
cost. Until then: Qt via the system/distro open-source packages only.
Record any such decision here if it is ever made.

## Fetched third-party content (wallpaper)

Two different things share this surface (ADRs
[0055](adr/0055-online-wallpaper-content-provider.md) /
[0094](adr/0094-bundled-default-wallpaper-and-lazy-cache.md)):

- The **shipped built-in wallpapers**
  (`assets/graphics/wallpapers/Default.jpg`, `Lagoon.jpg`, and `Midnight.jpg`)
  are our own original project assets. They **are** bundled and installed to
  the share path; they are listed in the asset table above and covered by
  `NOTICE` like the rest of the tree.
- The **Featured pictures** are fetched at runtime from Wikimedia Commons and
  are **not** bundled in the package. This is separate from the "original
  assets only" rule, which forbids reproducing Apple's artwork: Commons
  Featured Pictures are third-party works under their own licenses (public
  domain, CC BY, CC BY-SA, and FAL have all appeared in the six categories we
  use). Consequences:

  - No fetched image is ever shipped in a package artifact; the cache is user
    data under `$XDG_CACHE_HOME/dragonfruit/`. The only images in a package are
    the three shipped original built-ins (`Default.jpg`, `Lagoon.jpg`, and
    `Midnight.jpg`).
  - Attribution (`Artist` + license name/link + file page) is displayed for the
    current fetched wallpaper and the tiles, and is not optional.
  - Share-alike (CC BY-SA) and FAL images are never represented as original
    Dragonfruit work. The provider stores the license fields verbatim and the
    pane links to the file page; it never re-labels a fetched work as
    Dragonfruit's own.
  - The only new dependency is the HTTP client
    [`ureq`](https://github.com/algesten/ureq) 3.4.2 (MIT OR Apache-2.0,
    rustls + gzip; see `services/wallpaperd/Cargo.toml`); the "no GPL/AGPL
    dependency" rule still applies to every future provider as well.

The mechanical checks live beside the code:
`services/session/tests/session_entry.rs` asserts `Default.jpg` is installed
byte-identically to the stable share path,
`services/wallpaperd/tests/read_path.rs` asserts the cache holds only fetched
bytes and never the bundled asset, and
`docs/captures/t18-absence-matrix.md` records the reviewed state matrix.

## Trademarks

The GPL-3.0-only license grants copyright permissions only; it does not grant
rights to the Dragonfruit name or logo. Projects that want to use the
name or branding should ask first.

## Mechanical enforcement

- Every source file carries an `SPDX-License-Identifier: GPL-3.0-only` header.
- Every crate/package directory carries a `LICENSE` file.
- Full license text lives in [../../LICENSES/](../../LICENSES/).
- The `protocol_xmls_match_lockstep_version` test in
  `protocols/df-ipc` fails the build if a protocol XML loses its
  GPL-3.0-only SPDX marker.
- `make check-tabler` runs `gen-tabler-glyphs.py --check` (the registry must
  match the SVG directory), `gen-app-icons.py --check` (the first-party app
  artwork must match the pinned filled glyphs), and
  `scripts/check-tabler-icons.py` (every literal/dynamic glyph reference must
  resolve, including the `Icon.qml` compatibility map).
- `make check-phosphor` remains the equivalent gate for the filled Settings
  tile/app-mark set.
- `make check-assets` validates the WhiteSur provenance row, its license text,
  and the copied Files app icon.
- New files: carry the GPL-3.0-only SPDX header. Adding a GPL-3.0-incompatible
  dependency or changing a component's license is a review blocker.

## Contribution terms

Contributions are accepted under the GPL-3.0-only license (inbound =
outbound). By submitting a patch you license it to the project under
GPL-3.0-only; do not submit code you cannot license that way.
