# T-07 — Menu Bar Goes Live: Wi-Fi, Volume, Battery

> **Track, not a single slice.** This file is the design reference. It is executed as 6 one-session units: [T-07.1](units/027-t-07.1-adapter-framework-and-contract.md) · [T-07.2](units/028-t-07.2-networking-adapter-networkmanager.md) · [T-07.3](units/029-t-07.3-audio-adapter-pipewire-wireplumber.md) · [T-07.4](units/030-t-07.4-power-adapter-upower.md) · [T-07.5](units/031-t-07.5-status-item-menus-and-placeholder-removal.md) · [T-07.6](units/032-t-07.6-absent-daemon-matrix-and-capture.md). Strict order and prerequisites live in [ROADMAP.md](../ROADMAP.md).

| | |
|---|---|
| **Slice** | 7 of 17 — the chrome stops being a prototype |
| **Area** | `services/` adapters (Rust) · `shell/` status content |
| **Depends on** | T-01 |
| **Blocks** | T-08 (host-service keys), T-11 |
| **Legacy detail** | [legacy/20-system-service-adapters.md](legacy/20-system-service-adapters.md) (MVP slice A1/A3/A4) · [legacy/09-menu-bar.md](legacy/09-menu-bar.md) · [07-system-integration.md](../design/07-system-integration.md) |

## Demo

```
the menu bar shows real Wi-Fi state, volume, and battery instead of
placeholders
→ click Wi-Fi → the network list; join a network; the glyph updates
→ click volume → a slider; mute; the glyph updates
→ click battery → charge state and percentage
→ mask/stop a daemon → its item hides or says "unavailable"; the session is
  unaffected
```

Capture: `docs/captures/t07-live-menubar.*`, plus an absent-daemon still.

## Why now

The menu bar is the most-seen surface in the product and it is currently a
placeholder. The three adapters in this slice are the MVP subset the vertical
slice names explicitly, and they are what make the bar worth looking at while
the loop slices continue. They are also the first real consumers of the
"degrade gracefully when a daemon is absent" rule.

## Inherited and reused

- `StatusItem` slots with `available`/`enabled`/`level`/`label`/`tint`, and the
  original status glyphs (`legacy/09-menu-bar.md`).
- `--placeholders` demo mode (removed at the end of this slice; the real
  adapters replace it).
- The adapter principles and D-Bus conventions from
  [07-system-integration.md](../design/07-system-integration.md).
- Existing service crates layout under `services/`.

## Scope

### In

1. **A1 networking (MVP subset)**: current Wi-Fi state, access-point list,
   join/activate, signal strength. NetworkManager via its D-Bus API; no libnm
   link required.
2. **A3 audio (MVP subset)**: default sink volume, mute, per-sink list.
   PipeWire/WirePlumber via its D-Bus/pipewire API.
3. **A4 power (MVP subset)**: battery presence, level, charging state.
   UPower.
4. **Adapter contract**: available / unavailable / error states; event
   subscription (no polling above the adapter); a mock implementation for
   tests; re-subscribe on service restart.
5. **Status-item menus**: Wi-Fi list/join, volume slider/mute, battery
   read-only. Use design-system popovers; keyboard accessible.
6. **Graceful degradation**: absent daemon → hidden or explicit
   "unavailable", never an error, never a startup blocker; verified with a
   masked-unit matrix (mock in CI, real masking in a VM when available).
7. **Remove `--placeholders`** from `make dev` once the real items work; keep
   the demo app menu until T-14.

### Out / explicitly deferred

- Control Center tiles and the panel (T-11).
- Bluetooth, sound devices/routing, power profiles, VPN, the remaining
  adapters (T-15).
- Settings panes for these (T-09/T-15).

## Acceptance

- [ ] The demo runs and both captures are committed.
- [ ] Each item reflects an adapter event within one event (no polling).
- [ ] The absent-daemon matrix passes (all three daemons masked).
- [ ] The menu bar still contributes zero idle wakeups (idle trace).
- [ ] `make e2e` and `make soak` stay green; the T-01 loop still passes.

## Test plan

- Unit: adapter state machines against mocks.
- Integration: mock-driven menu-bar state; one real daemon where available.
- VM/manual: masking matrix.
- Regression: idle trace, loop demo.

## Risks

- **Adapter sprawl**: keep the MVP subset strictly read + one toggle per item;
  resist building the full panes here.
- **WirePlumber API churn**: pin and isolate behind the adapter interface.
- **NetworkManager permissions**: joining may need polkit; if unavailable,
  degrade to read-only and record it.

## Hand-off

- T-08 routes host-service keys through settingsd once the adapters exist.
- T-11 consumes these adapters for Control Center.
- T-15 completes the adapter roster.
