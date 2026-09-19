# Dragonfruit Licensing

Dragonfruit uses different licenses per component class, by design
(the full policy, including what may link what and the Qt commercial
trigger, is in [docs/licensing.md](docs/licensing.md)):

| Component | License |
|---|---|
| `compositor/`, `portal/`, `services/`, `tools/`, `protocols/df-ipc/` | MIT OR Apache-2.0 |
| `protocols/*.xml` (private Wayland protocols) | MIT |
| `design-system/` (QML module `Dragonfruit`) | LGPL-3.0-or-later |
| `shell/`, `apps/` | GPL-3.0-or-later |

Full license texts live in [LICENSES/](LICENSES/). Every source file
carries an `SPDX-License-Identifier` header; every crate/package
directory carries a `LICENSE` file.

Copyright (c) 2026 Dragonfruit contributors.
