# T-20 — System Service Adapters

| | |
|---|---|
| **Phase** | 4 · System integration |
| **Area** | `services/` / `shell/` shared adapter modules (Rust) |
| **Depends on** | [T-01](01-repo-scaffolding-ci-licensing.md) · [T-02](02-compositor-core.md) (D-Bus in compositor loop is separate; these are session-bus clients) |
| **Blocks** | [T-09](09-menu-bar.md) (status items) · [T-16](16-settings-app.md) (panes) · [T-21](21-control-center.md) · [T-34](34-mvp-vertical-slice-gate.md) (MVP slice A1/A3/A4) · Phase-4 exit |
| **Estimate** | L (per-adapter S) |
| **Design docs** | [07-system-integration.md](../../design/07-system-integration.md) · [08-settings.md](../../design/08-settings.md) |

## Summary

Thin, mockable adapters over the host daemons: NetworkManager, BlueZ,
PipeWire/WirePlumber, UPower, power-profiles-daemon, UDisks2/GIO,
CUPS/SANE, Secret Service, systemd-timedated, accountsservice, logind,
polkit. We replace the **presentation**, never the stack; every adapter
degrades gracefully when its daemon is absent.

## Background

Every subsystem already exposes a D-Bus (or equivalent) API intended exactly
for custom desktop frontends
([07-system-integration.md](../../design/07-system-integration.md)). Adapters
are small modules exposing a stable internal API; **nothing above the
adapter knows which daemon implements it**. Absence of a daemon is a normal
state, not an error.

## MVP slice (for T-34)

The 30-second-loop MVP needs live **menu-bar status items** before Control
Center or Settings breadth: a read-only-plus-basic-toggle subset of
**A1 networking** (current Wi-Fi state + join), **A3 audio** (volume/mute),
and **A4 power** (battery level/charging). These three adapters ship on the
MVP critical path (feeding [T-09](09-menu-bar.md)); the remaining nine, the
Control Center tiles (T-21), and the full Settings panes stay post-gate.
Because the vertical slice lists Wi-Fi/volume/battery explicitly, these are
**not** optional for T-34.

## Scope — the adapter roster

| # | Subsystem | Service | Adapter exposes |
|---|---|---|---|
| A1 | Networking | NetworkManager (libnm) | Device + access-point enumeration; activate/deactivate; create/edit/delete connections; VPN basics |
| A2 | Bluetooth | BlueZ | `org.bluez.Adapter1` + device objects; discovery, pair, connect |
| A3 | Audio | PipeWire + WirePlumber | Graph status; volume/mute per sink/source; default device switching |
| A4 | Power | UPower | Batteries + power devices, charge/state events |
| A5 | Power profiles | power-profiles-daemon | performance/balanced/power-saver **where the service exists** |
| A6 | Storage | UDisks2 + GIO/GVfs | Block devices, mount operations, volume monitor, eject/busy reporting |
| A7 | Printing/scanning | CUPS + SANE | IPP printing via `org.cups.*`/freedesktop print APIs; queue state; SANE device scan |
| A8 | Secrets | Secret Service API | Host keyring reuse — we **never** build a credential store; first-party apps never cache secrets themselves |
| A9 | Date & time | systemd-timedated | Timezone + NTP settings |
| A10 | User accounts | accountsservice | User list, avatars, account type |
| A11 | Seat/session | systemd/logind | Session lifetime, VT management, `LockSession`/`UnlockSession`, sleep/shutdown hooks (consumed by T-24/T-26) |
| A12 | Privileged ops | polkit + host policy infra | Authentication-agent integration only — never roll our own privilege escalation |

### Principles (verbatim from
[07-system-integration.md](../../design/07-system-integration.md))

1. **Replace the presentation, not the stack.**
2. **Adapters are thin and testable** — stable internal API, mockable,
   daemon-agnostic above.
3. **The compositor owns what the compositor owns** — displays, brightness,
   keyboard settings go through our compositor API, **not** system daemons.
4. **Degrade gracefully when a service is absent** — explicit "unavailable"
   state; panes hide/disable; nothing blocks session startup on one
   daemon.

### D-Bus conventions

- Our services own names under `org.dragonfruit.*` on the **user session
  bus**; nothing of ours needs a system-bus service of its own.
- Privileged operations delegated to existing system services behind
  polkit, whose prompt **our shell renders** (T-29).

## Requirements

- FR-1: Every adapter implements: available / unavailable / error states;
  event subscription (no polling above the adapter); a mock implementation
  for tests.
- FR-2: Nothing above an adapter imports daemon-specific types (compile
  gate: UI modules depend only on adapter-internal API).
- FR-3: Phase-4 exit — **every adapter degrades gracefully when its daemon
  is missing, verified by masking the systemd unit in a VM**; session
  startup never blocks.
- FR-4: Brightness/display/keyboard paths do **not** route through adapters
  — they use the compositor API (guardrail against "convenient" misuse of
  e.g. `org.freedesktop.login1` brightness).
- FR-5: Secrets: no adapter or app ever stores credentials itself.
- FR-6: Restart-safe: adapters re-subscribe and re-sync on service restart
  (crashed and restarted consumers recover without user-visible error).

## Acceptance criteria

- [ ] All 12 adapters shipped at least at the depth their consumers
      (T-09/T-16/T-21) require — tracked per-adapter checkboxes in the
      PR.
- [ ] VM masking matrix passes (Phase-4 exit criterion).
- [ ] Mock-driven integration tests for Control Center + Settings panes
      run in CI without any host daemon.

## Test plan

- Per-adapter contract tests (mock + real-in-VM).
- Masking matrix scripted: `systemctl --user mask` / stop each daemon, boot
  the session, assert UI states.

## Risks / open questions

- WirePlumber API surface moves fast — pin and isolate in A3.
- CUPS/SANE breadth can eat time; ship minimalIPP print + queue status
  first (per the roadmap guardrail).
