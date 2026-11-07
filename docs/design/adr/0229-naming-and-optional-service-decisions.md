# 0229 — Naming and optional-service decisions

## Status

accepted — T220 records the outcomes of the eight open questions in
[interface-naming-and-licensing.md](../interface-naming-and-licensing.md) §7.
The generic naming policy remains [ADR 0220](0220-generic-naming-and-third-party-substitution.md).

## Context

T220 must land the naming map, asset provenance, and third-party equivalence
before new user-facing strings or assets are added. The policy had a candidate
table but eight unanswered decisions: nearby transfer, search backend, backup
UI, online accounts, accessibility depth, IME, display manager, and firewall.
Leaving them open would let implementation tasks make incompatible or
license-risky choices.

The default position is the safest permissive/optional path: a permissive
backend where one exists, a separate-process delegate where it does not, and
an explicit absence path everywhere.

## Decision

| Question | Decision |
|---|---|
| Nearby transfer | Omit from 0.2.0. LocalSend (Apache-2.0) is the only future candidate; no package dependency, no first-party UI, and no dead Settings row. KDE Connect/Warpinator/Syncthing remain documented alternates only. |
| Search backend | The built-in filename index is the default. Tracker (GPL-2.0+), Baloo (LGPL-2.1+), and Recoll (GPL-2.0+) are optional separate-process backends selected in Settings; the pane never claims content indexing the backend does not provide. |
| Backup UI | restic (BSD-2-Clause) is the preferred backend behind a thin first-party Backup pane. Pika Backup and Timeshift are optional separate-app delegates. No custom backup format, no privileged daemon. |
| Online accounts | Optional GNOME Online Accounts (LGPL-2.1+) adapter over D-Bus. No first-party OAuth implementation. If GOA is absent the Online Accounts pane is hidden, not a dead entry. |
| Accessibility depth | AT-SPI is the bus; Orca (LGPL-2.1+) is the screen-reader delegate; speech-dispatcher plus a synthesizer is the speech delegate; the magnifier is first-party. Shell-chrome AT-SPI export is a T252 release gate (G-19). |
| IME | Support ibus and fcitx5 (both LGPL-2.1+) as separate processes. Detect the running framework, never start both, and do not embed an IME engine. The candidate window is the framework's, with a first-party minimal fallback only if T253 proves it necessary. |
| Display manager | Support all four session entries: GDM, SDDM, LightDM, and greetd, through the per-DM adapter in ADR 0173. GDM is the reference tested path and SDDM the secondary; the desktop does not depend on a display-manager package. An ambiguous host degrades to the greeter. |
| Firewall | firewalld (GPL-2.0+, D-Bus) on the Fedora family and ufw (GPL-3.0, CLI) on Debian/Ubuntu, behind one `FirewallStatus`/`set_enabled`/`open_port` contract. No first-party nftables rule editor. With no supported manager the pane shows "No firewall detected" and no fake toggle. |

All eight are optional or separate-process under the MIT codebase. None is a
hard dependency; the core desktop installs, starts, and works with the required
host stack only.

## Consequences

- The Settings tasks T225/T226/T228/T229a implement these choices without
  reopening them; a later change is a new ADR.
- `packaging/optional-services.json` is the canonical `Recommends`/`Suggests`
  table shared by the Fedora and Debian packages. `NOTICE` mirrors it.
- The first-party surfaces use the generic names in
  [naming-gate.json](../naming-gate.json); `make check-naming` enforces them
  over QML/C++ strings, desktop entries, and packaging metadata.
- Absence paths are testable: no firewall manager, no GOA, no Orca, no IME,
  no restic, and no search backend each degrade one feature, never the session.
- A legal review remains required before the first public release, and package
  names/licenses are verified at packaging time (T259).
