# Licensing Policy

This document records the Dragonfruit licensing decision. It is a
**blocker for the first public release** (see
[../.docs/design/14-risks.md](../.docs/design/14-risks.md)): nothing is
distributed publicly until the rules below are reviewed and signed off.

## Decision

| Component | License | Why |
|---|---|---|
| `compositor/` (Rust + Smithay) | MIT OR Apache-2.0 | Smithay is MIT; dual licensing keeps options open and matches Rust-ecosystem norms |
| `services/` (settingsd, menu-broker, app-index, session) | MIT OR Apache-2.0 | Independent daemons; permissive so third parties may interoperate without GPL concerns |
| `portal/` (xdg-desktop-portal-dragonfruit) | MIT OR Apache-2.0 | Must be freely consumable by any sandboxed app |
| `tools/` (dev tooling) | MIT OR Apache-2.0 | Developer ergonomics |
| `protocols/df-ipc/` (shared IPC crate) | MIT OR Apache-2.0 | Same reason as the XMLs below |
| `protocols/*.xml` (private Wayland protocols) | **MIT** | Any third party — other compositors, toolkits, client libraries — may implement them freely; a copyleft protocol XML would poison independent implementations |
| `design-system/` (QML module `Dragonfruit`) | LGPL-3.0-or-later | Qt-compatible; first-party GPL code links it, and the LGPL keeps the door open for constrained third-party reuse |
| `shell/` (menu bar, Dock, Control Center, notifications, screenshot) | GPL-3.0-or-later | The desktop's identity; copyleft by choice |
| `apps/` (Settings, Files) | GPL-3.0-or-later | Same as the shell |

## What may link what

```text
apps (GPL) ────────────► design-system (LGPL)      OK: GPL may link LGPL
shell (GPL) ───────────► design-system (LGPL)      OK
services (MIT/Apache) ─► df-ipc (MIT/Apache)       OK
compositor (MIT/Apache) ─► df-ipc, Smithay (MIT)   OK
third-party code ──────► protocol XMLs (MIT)       OK: no obligations beyond the notice
```

Rules of thumb:

- **GPL code may depend on LGPL/MIT/Apache code**; the combination
  remains GPL where the combination is a derived work (statically
  linking the design system into an app makes that app GPL anyway,
  which is our intent for first-party software).
- **Permissively licensed code (services, df-ipc) never links GPL/LGPL
  code** — the dependency arrows above are the only legal direction.
  `df-ipc` in particular must stay dependency-free of everything but
  the standard library, so that any process can embed the contracts.
- **Qt is (L)GPL-licensed or commercial.** We use the open (L)GPL Qt
  under the terms above; no Dragonfruit component may obligate a Qt
  commercial license.

## Qt commercial decision trigger

From [../.docs/design/14-risks.md](../.docs/design/14-risks.md): if the
project ever needs a **statically linked, non-GPL Qt distribution, or
Qt modules only available under a commercial license**, that is a
deliberate project-level decision — it changes distribution terms and
cost. Until then: Qt via the system/distro open-source packages only.
Record any such decision here if it is ever made.

## Mechanical enforcement

- Every source file carries an `SPDX-License-Identifier` header.
- Every crate/package directory carries a `LICENSE` file.
- Full license texts live in [../LICENSES/](../LICENSES/).
- The `protocol_xmls_match_lockstep_version` test in
  `protocols/df-ipc` fails the build if a protocol XML loses its MIT
  SPDX marker.
- New files: match the license of their directory; when in doubt, ask
  before committing — mixing licenses across the arrows above is a
  review blocker.

## Contribution terms

Contributions to any component are accepted under the terms of that
component's license table above (the inbound = outbound model). By
submitting a patch you license it under the license(s) already applied
to the files you touched. The MIT/Apache-2.0 crates accept dual-licensed
contributions only.
