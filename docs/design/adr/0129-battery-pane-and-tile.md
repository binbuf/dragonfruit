# 0129 — The Battery pane and tile ride the status bridge host's one profile write

- **Status:** Accepted (T-15.6b)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md),
  ADR [0128](0128-battery-power-profiles-adapter.md)

## Context

T-15.6a shipped `dragonfruit-power` as the battery **and** power-profiles
adapter: one read of UPower and power-profiles-daemon, one write
(`set_active_profile`), and a hand-off that left the Settings pane and Control
Center tile to T-15.6b. Unlike Mission Control (ADR
[0127](0127-mission-control-pane-and-tile.md)), this subsystem *does* have real
external daemons, and the shell already reads the battery over the
`dragonfruit-system-status` bridge host for the menu bar. The pane and tile must
therefore extend that existing host path, not invent a shell-local projection.

The macOS capture (ADR [0122](0122-tahoe-interface-language-across-chrome.md))
shows `Low Power Mode`, `Battery Health`, `Charging`, a `Last 24 Hours`/
`Last 10 Days` range switch, `Last charged to 100%`, a battery-level bar chart,
`Screen On Usage`, and `Options…`/`?`. Several of those providers are Apple-only
or absent on Linux.

## Decision

The pane and tile are one functional unit over the T-15.6a adapter through the
existing status bridge host.

1. **The host gains the one write.** `StatusHost::set_active_profile(profile)`
   takes the stable id, maps it with `PowerProfile::from_id`, and reports the
   [`ProfileOutcome`] as JSON; an unknown id is a failure, never a guess.
   `battery_view` grows `health`/`healthLabel`/`capacity`/`chargeCycles`,
   `chargeState`, `profilesAvailable`, `activeProfile`/`profileLabel`, and the
   `profiles` list (`{id, label, glyph, active}`).
   `org.dragonfruit.SystemStatus1.Battery` adds `SetActiveProfile(profile)`;
   the interface stops being read-only.
2. **The Settings app uses a dedicated seam.** `BatteryClient`
   (`apps/settings/BatteryClient.{h,cpp}`) is the pane's one D-Bus accessor,
   with `DbusBatteryClient` over the host and `MockBatteryClient` under
   `DF_BATTERY_FIXTURE`, mirroring the Bluetooth/storage/sound/input pattern.
   `SettingsBridge` exposes `battery`, `batteryAvailable`, `refreshBattery()`,
   and `setPowerProfile(id)`; the pane never touches the bus.
3. **The Control Center tile reads the same view.** `SystemStatusModel` already
   decodes the battery view; `ShellController::applyControlCenterData` pushes it
   to the panel. The tile summarizes charge and the active profile
   (`"71% · Balanced"`), shows the active profile's glyph, and raises
   `batterySettingsRequested`. It hides when the host is absent or when there is
   neither a battery nor a profile, so a desktop with only power profiles still
   gets the tile.
4. **Absence is layered and per daemon**, exactly as T-15.6a: a missing host is
   the pane's one-line note; a present host with no battery hides the
   battery/history groups but keeps the picker; a missing
   power-profiles-daemon hides the picker and shows its own note.
5. **Apple-only or providerless rows are adapted, not faked.**
   `Low Power Mode` maps onto the daemon's three profiles (there is no
   `Never`/`Always` in power-profiles-daemon). The `Battery Health`/`Charging`
   `i` panels show UPower capacity/cycles and the charge estimate instead of
   Apple hardware health. UPower no longer exposes charge history, so the range
   switch and chart frames render an honest absent state (provider T-15.x) and
   `Options…`/`?` are omitted; no dead controls ship.

Rejected: a settingsd key for the active profile (the daemon owns it, not
settingsd); a shell-local projection (the real daemons exist; use the host); a
read-only tile with no link (the pane is where the picker lives).

## Consequences

- The Control Center panel grows from 360×980 to 360×1040 to fit the new tile;
  the fit test and the T-15.6b capture assert it, and the T-15.5b capture script
  is updated to the new height.
- The `i` info affordance and the three power-profile glyphs
  (`power-saver`/`power-balanced`/`power-performance`), plus the `battery` and
  `info` glyphs, join `design-system/components/Icon.qml`.
- `Battery Health`'s `Normal`/`Service` verdict is the project's documented 80%
  capacity threshold (T-15.6a), not an Apple health value.
- A UPower charge-history provider remains a later T-15.x task; until then the
  Usage History group is deliberately an absent state.

[`ProfileOutcome`]: ../../services/power/src/source.rs