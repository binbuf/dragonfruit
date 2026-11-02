# Licensing Policy

This document records the Dragonfruit licensing decision. It is a
**blocker for the first public release** (see
[../docs/design/14-risks.md](../docs/design/14-risks.md)): nothing is
distributed publicly until the rules below are reviewed and signed off.

## Decision

Dragonfruit is licensed under the **MIT License**, repo-wide. One license
covers every component, so there are no cross-component linking
restrictions to track.

| Component | License |
|---|---|
| `compositor/` (Rust + Smithay) | MIT |
| `services/` (settingsd, menu-broker, app-index, session) | MIT |
| `portal/` (xdg-desktop-portal-dragonfruit) | MIT |
| `tools/` (dev tooling) | MIT |
| `protocols/df-ipc/` (shared IPC crate) | MIT |
| `protocols/*.xml` (private Wayland protocols) | MIT |
| `design-system/` (QML module `Dragonfruit`) | MIT |
| `shell/` (menu bar, Dock, Control Center, notifications, screenshot) | MIT |
| `apps/` (Settings, Files) | MIT |
| `assets/fonts/Inter/` (system font, embedded via `libs/system-font/`) | SIL OFL 1.1 |
| `assets/icons/tabler/` (line-glyph set and pinned filled marks) | MIT |
| `assets/icons/phosphor/` (filled Settings-tile glyphs) | MIT |
| `assets/cursors/capitaine-cursors/` (system cursor theme) | LGPL-3.0-or-later |
| `assets/graphics/wallpapers/Default.jpg` (shipped default wallpaper) | MIT (original project asset) |

Rationale: MIT is permissive, maximises third-party adoption and
interoperability, and keeps Dragonfruit consumable by any project —
other compositors, toolkits, client libraries, and sandboxed apps alike.
It retains credit through the copyright notice that every redistribution
must preserve. Name/logo credit is handled separately; see "Trademarks".

## Linking rules

Because all first-party code is MIT, there are no license-direction
constraints between components. Two rules still matter:

- **`df-ipc` must stay dependency-free of everything but the standard
  library**, so that any process can embed the contracts.
- **No first-party component may pull in GPL/AGPL code.** A copyleft
  dependency would relicense the combined work and defeat the permissive
  choice; adding one is a review blocker. Separable LGPL data assets are
  allowed when they are loaded at runtime, not linked into first-party code,
  and their license/source notice ships with the distribution (the Capitaine
  cursor theme below is the current case).

## Third-party dependencies

- **Rust crates.** The whole dependency graph is permissively licensed
  (MIT, Apache-2.0, BSD-2-Clause/BSD-3-Clause, ISC, Zlib, Unlicense).
  The exact set is pinned in `Cargo.lock`; see [NOTICE](../NOTICE).
- **Qt is (L)GPL-licensed or commercial.** We use the open (L)GPL Qt
  under its terms, linked dynamically from the system/distro packages.
  MIT first-party code linking LGPL-3.0 Qt is compatible, and no
  Dragonfruit component may obligate a Qt commercial license.
- **System libraries** (libdrm, libinput, libseat, xkbcommon, wayland,
  systemd, NetworkManager, PipeWire, …) keep their own licenses; see
  [NOTICE](../NOTICE).
- **Fonts.** Inter is bundled under the SIL OFL 1.1 (see
  [../LICENSES/OFL-1.1.txt](../LICENSES/OFL-1.1.txt)); it has no Reserved
  Font Name. OFL condition 2 requires the copyright notice and license text
  to accompany every copy of the Font Software, so any binary distribution
  (packages, images) must include `LICENSES/OFL-1.1.txt` or the equivalent
  notice — a packaging acceptance item for T-16.9/T-16.10.
- **Icons.** Tabler Icons 3.50.0 (`v3.50.0`, commit
  `6904efd245391ad396df17654738f52f6da8c40e`) is bundled under the MIT license
  (see [../LICENSES/Tabler.txt](../LICENSES/Tabler.txt)) as the line-glyph
  set and the pinned filled app marks. The Settings category tiles use
  Phosphor 2.0.8 (MIT, [../LICENSES/Phosphor.txt](../LICENSES/Phosphor.txt))
  because the tile needs a solid near-white glyph. Both sets are vendored as
  data, not linked code; the asset READMEs record the upstream version, commit,
  source path, and the exact pinned files, and `make check-tabler` /
  `make check-phosphor` fail if a registry or a glyph reference drifts. No
  Apple artwork is vendored or reproduced (see
  [../docs/design/14-risks.md](design/14-risks.md)).
- **Cursor theme.** Capitaine cursors is bundled as a separable data asset
  under the LGPL-3.0-or-later (see
  [../assets/cursors/capitaine-cursors/COPYING](../assets/cursors/capitaine-cursors/COPYING)
  and [../LICENSES/LGPL-3.0-or-later.txt](../LICENSES/LGPL-3.0-or-later.txt)).
  It is not linked into first-party code: the compositor loads the XCursor
  files at runtime, the theme directory can be replaced independently, and
  the upstream source is identified in
  [../assets/cursors/capitaine-cursors/README.md](../assets/cursors/capitaine-cursors/README.md).
  Any binary distribution must ship that notice and license text, and must
  make the corresponding upstream source available under the LGPL terms.

## Icon asset governance

Icons are third-party data assets, not linked code. Every vendored icon set
follows the same credit package:

- **Pinned source.** The asset README records the upstream repository, the
  exact release tag and commit, the upstream directory/file naming, and the
  license. The files themselves are copied unchanged (flattened with a
  `-filled` suffix where the set has a second weight), so a future update is a
  reviewable file swap plus a registry regeneration.
- **License text and notice.** The set ships its own `LICENSE` beside the
  assets, a copy in `LICENSES/`, a row in [NOTICE](../NOTICE), and a row in the
  component table above. MIT sets need no special distribution obligation
  beyond preserving the notice; a future copyleft icon set would be a review
  blocker under the linking rules.
- **One registry per set.** `scripts/gen-tabler-glyphs.py` and
  `scripts/gen-phosphor-glyphs.py` extract the path data into generated QML
  singletons; `--check` fails the build if the committed registry is stale.
  `scripts/check-tabler-icons.py` and `scripts/check-phosphor-icons.py` fail if
  a QML source names a glyph that is not vendored, including the legacy
  `Icon.qml` alias map and the dynamic service vocabulary.
- **Current split.** Tabler 3.50.0 is the line-glyph set (Files, menu bar,
  Dock, shell chrome, and the `Icon.qml` facade). Phosphor 2.0.8 is the filled
  Settings-tile set. The first-party app artwork uses Tabler's filled marks.
  Both are MIT.
- **No Apple artwork.** The original-assets rule in
  [14-risks.md](design/14-risks.md) still applies: third-party open icon sets
  are allowed; Apple's assets and pixel-copied proprietary artwork are not.

## Qt commercial decision trigger

From [../docs/design/14-risks.md](../docs/design/14-risks.md): if the
project ever needs a **statically linked, non-LGPL Qt distribution, or
Qt modules only available under a commercial license**, that is a
deliberate project-level decision — it changes distribution terms and
cost. Until then: Qt via the system/distro open-source packages only.
Record any such decision here if it is ever made.

## Fetched third-party content (wallpaper)

Two different things share this surface (ADRs
[0055](design/adr/0055-online-wallpaper-content-provider.md) /
[0094](design/adr/0094-bundled-default-wallpaper-and-lazy-cache.md)):

- The **shipped default wallpaper**
  (`assets/graphics/wallpapers/Default.jpg`) is our own original project asset.
  It **is** bundled and installed to the share path; it is listed in the asset
  table above and covered by `NOTICE` like the rest of the tree.
- The **Featured pictures** are fetched at runtime from Wikimedia Commons and
  are **not** bundled in the package. This is separate from the "original
  assets only" rule, which forbids reproducing Apple's artwork: Commons
  Featured Pictures are third-party works under their own licenses (public
  domain, CC BY, CC BY-SA, and FAL have all appeared in the six categories we
  use). Consequences:

  - No fetched image is ever shipped in a package artifact; the cache is user
    data under `$XDG_CACHE_HOME/dragonfruit/`. The only image in a package is
    the shipped original `Default.jpg`.
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

The MIT license grants copyright permissions only; it does not grant
rights to the Dragonfruit name or logo. Projects that want to use the
name or branding should ask first.

## Mechanical enforcement

- Every source file carries an `SPDX-License-Identifier: MIT` header.
- Every crate/package directory carries a `LICENSE` file.
- Full license text lives in [../LICENSES/](../LICENSES/).
- The `protocol_xmls_match_lockstep_version` test in
  `protocols/df-ipc` fails the build if a protocol XML loses its MIT
  SPDX marker.
- `make check-tabler` runs `gen-tabler-glyphs.py --check` (the registry must
  match the SVG directory), `gen-app-icons.py --check` (the first-party app
  artwork must match the pinned filled glyphs), and
  `scripts/check-tabler-icons.py` (every literal/dynamic glyph reference must
  resolve, including the `Icon.qml` compatibility map).
- `make check-phosphor` remains the equivalent gate for the filled Settings
  tile/app-mark set.
- New files: carry the MIT SPDX header. Adding a copyleft dependency or
  changing a component's license is a review blocker.

## Contribution terms

Contributions are accepted under the MIT License (inbound = outbound).
By submitting a patch you license it to the project under MIT; do not
submit code you cannot license that way.
