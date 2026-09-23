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
  choice; adding one is a review blocker.

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

## Qt commercial decision trigger

From [../docs/design/14-risks.md](../docs/design/14-risks.md): if the
project ever needs a **statically linked, non-LGPL Qt distribution, or
Qt modules only available under a commercial license**, that is a
deliberate project-level decision — it changes distribution terms and
cost. Until then: Qt via the system/distro open-source packages only.
Record any such decision here if it is ever made.

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
- New files: carry the MIT SPDX header. Adding a copyleft dependency or
  changing a component's license is a review blocker.

## Contribution terms

Contributions are accepted under the MIT License (inbound = outbound).
By submitting a patch you license it to the project under MIT; do not
submit code you cannot license that way.
