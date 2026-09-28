# System Integration

## Summary

Wi-Fi, Bluetooth, sound, and power UI are much easier than they initially
appear because we are replacing the **presentation**, not the
hardware-management stack. Every subsystem below already exposes a D-Bus (or
equivalent) API intended exactly for custom desktop frontends.

## The adapters

| Subsystem | Service | What we consume |
|---|---|---|
| Networking | NetworkManager | Device/access-point enumeration; activate, deactivate, create, edit, delete connections. `libnm` provides a higher-level client on top of the D-Bus service. |
| Bluetooth | BlueZ | `org.bluez.Adapter1` and device objects; `bluetoothd` keeps handling controller/protocol details. |
| Audio | PipeWire + WirePlumber | PipeWire supplies the audio/video graph; WirePlumber is the policy/session manager and provides an API intended for management/status applications. |
| Power | UPower | Batteries and power devices over the system bus. |
| Power profiles | power-profiles-daemon | performance / balanced / power-saver selection where the service exists (Battery pane, Control Center). |
| Storage | UDisks2 + GIO/GVfs | UDisks provides block devices, monitoring, and mount operations; GIO has abstractions for user-interesting volumes and mounts. |
| Printing & scanning | CUPS + SANE | IPP printing via the standard `org.cups.*` / freedesktop print APIs; drivers and queues stay with CUPS. |
| Secrets | Secret Service API | Reuse the host keyring (e.g. gnome-keyring); we never build a credential store, and first-party apps never cache secrets themselves. |
| Date & time | systemd-timedated | Timezone and NTP settings for the Settings panes. |
| User accounts | accountsservice | User list, avatars, account type — for Settings' Users & Groups pane. |
| Input devices | libinput | Keyboard/mouse/trackpad enumeration and per-device capabilities. Unlike the daemons above, the **user's** input settings are compositor-applied and settingsd-owned, per principle 3; the adapter is the inventory only. |
| Mission Control / hot corners | compositor (`df_toplevel_manager`) | Trigger configuration and overview runtime state. Mission Control and hot corners are compositor-native: the compositor detects corners and owns the one overview machine, and the shell learns both over the private bridge. The adapter reuses that path; it is the projection, never a second detector. |
| Notifications / Focus | notification service | The Focus/DND policy (mode, allow list, suppressed batch) and the active-banner / history state. The service already owns the queue, the history, and the admission rule (ADR [0058](adr/0058-focus-dnd-policy-semantics.md)); the adapter is the projection over its shell-facing JSON views and never reimplements them. |
| Lock Screen policy | session idle engine + compositor lock | The `idle.*` stage delays the session's idle/lock engine applies ([ADR 0070](adr/0070-idle-timer-engine-and-policy.md)) and the compositor's fail-secure lock state ([ADR 0067](adr/0067-session-lock-protocol-and-ui.md)), mirrored by the shell over `df_toplevel_manager`. The adapter is the projection over the confirmed lock state and the applied policy; it never re-times a stage or re-implements a lock transition, and the display preferences are settingsd-owned. |
| Menu Bar configuration | shell menu bar + menu-broker | The chrome, status-item model, and clock the shell's `MenuBar` owns, and the global application-menu resolution the menu-broker owns. The adapter is the projection over the effective auto-hide/background/global-menu flags, the clock options, and the live availability of each control; it never re-renders the bar or re-resolves a menu, and the durable preferences are settingsd-owned. |
| Seat/session | systemd / logind | Session lifetime, `graphical-session.target`, VT management. |
| Privileged operations | host policy infrastructure | polkit / authentication agent integration; we never roll our own privilege escalation. |

## Principles

1. **Replace the presentation, not the stack.** The Control Center (see
   [04-shell.md](04-shell.md)) and Settings (see [08-settings.md](08-settings.md))
   are completely custom UIs over these daemons.
2. **Adapters are thin and testable.** Each adapter is a small, mockable
   module exposing a stable internal API; nothing above the adapter knows
   which daemon implements it.
3. **The compositor owns what the compositor owns.** Displays, brightness,
   and keyboard settings go through our compositor API, not through
   system daemons — the compositor owns physical outputs and input.
4. **Degrade gracefully when a service is absent.** Minimal VMs, servers,
   and CI containers lack Bluetooth, batteries, and printers. Every adapter
   exposes an explicit "unavailable" state; panes hide or disable accordingly,
   and nothing blocks session startup on one daemon.

## The adapter contract (T-07.1a)

The one contract every adapter implements lives in the dependency-free
`services/system-adapters` crate (`dragonfruit-system-adapters`); concrete
adapters depend on it and keep their daemon stack behind it
([adr/0024](adr/0024-system-adapter-contract.md)). An adapter exposes the last
state pushed by its daemon as one of three `AdapterState`s:

- `Available(snapshot)` — the daemon answered; the typed snapshot is the live
  data.
- `Unavailable` — the daemon is absent. A normal state, never an error, and
  never a startup blocker (principle 4).
- `Error(message)` — the daemon is present but could not be read.

Consumers render the **slot** the state projects: `Unavailable` hides the
status item, `Error` shows it visible but inert with the message, `Available`
shows it live. `MockAdapter` drives all three with no daemon on the bus.
Consumers read the already-pushed state — there is no poll loop above an
adapter.

## Event subscription and restart re-subscribe (T-07.1b)

The daemon pushes; the adapter never polls it and nothing above the adapter
polls. Every adapter records its daemon subscription in one `Subscription`
(`services/system-adapters/src/subscription.rs`) and exposes the transitions as
`AdapterEvent`s through `Adapter::drain_events`:

- `Subscribed { resubscribe: false }` — the first subscription.
- `Disconnected` — the daemon went away; the adapter state becomes
  `Unavailable` and the slot hides.
- `Subscribed { resubscribe: true }` — the adapter re-subscribed after the
  daemon had gone away (a restart); the adapter re-syncs.
- `Changed` — the daemon pushed data; re-read `Adapter::state`.

A consumer reacts to an event by re-reading the state; it never queries the
daemon. Events carry no snapshot so the stream and the adapter state cannot
disagree.

The absence/error split is part of the contract: `SubscribeError::Absent` maps
to `Unavailable` (hidden, not an error) while `SubscribeError::Failed` maps to
`Error` (visible, inert). Neither aborts session startup, so a missing daemon
is never a startup blocker. `MockAdapter` simulates the lifecycle with
`kill()`/`restart()`, which is how the headless suite asserts re-subscribe and
absence with no bus.

## The NetworkManager read path (T-07.2a)

The first concrete adapter is `dragonfruit-networkmanager`
(`services/networkmanager`): Wi-Fi state, the access-point list, and signal
strength read over NetworkManager's **D-Bus API** on the system bus — never
through a `libnm` link. The concrete adapters each live in their own crate
behind the dependency-free contract
([adr/0026](adr/0026-concrete-adapters-in-their-own-crates.md)), so the
contract stays free of any D-Bus stack.

One `NetworkManagerSource::read` is the whole transport seam. It returns
either the raw device/AP enumeration, `None` when NetworkManager is absent, or
an error when it is present but unreadable — the three answers map straight to
`Available`/`Unavailable`/`Error`. `DbusNetworkManager` is the live source;
`MockNetworkManager` serves a fixture in tests, so CI needs no bus and no
daemon. `NetworkManagerAdapter::refresh` is the one place the adapter touches
the daemon and it is called when NetworkManager signals a change, so nothing
above the adapter polls.

The typed `WifiSnapshot` decodes the raw read: it collapses the device list to
one aggregate Wi-Fi state, dedupes the BSSIDs to one entry per SSID (strongest
first), marks the active network, and derives the glyph/label the menu bar
draws. An absent daemon hides the Wi-Fi item, and a present-but-unreadable one
shows it visible and inert, exactly like every other adapter.

## The NetworkManager join path and polkit degradation (T-07.2b)

The one write the adapter makes is `NetworkManagerAdapter::join`, over the same
transport seam (`NetworkManagerSource::activate`). It is an explicit user
action, never a poll: one `AddAndActivateConnection` call per join request, with
the secret held only for the duration of that call. A successful join does not
invent a snapshot — NetworkManager reports the resulting state through its
subscription and the host re-reads the adapter, so the snapshot stays the single
source of truth.

Joining is authorized by polkit. When polkit refuses, `activate` returns the
denial separately from a general failure (`ActivateOutcome::Denied` versus
`Failed`), and the adapter **degrades to read-only**:

- `WifiAccess::ReadWrite` (the default) allows reads and joins;
- `WifiAccess::ReadOnly { note }` disables joins and records the daemon's
  message. The network list stays live and the status slot stays visible and
  enabled — read-only means reads keep working, not that the item goes inert.
  The adapter then refuses further joins locally, so a denied user cannot make
  the daemon re-evaluate the same request in a loop.

The degradation survives a refresh: a successful read does not re-grant a write
permission. It is adapter-level, not part of `WifiSnapshot`, because it is a
property of the session's authorization rather than of the daemon's state (see
[adr/0027](adr/0027-networkmanager-join-read-only-degradation.md)). Absence
still has its usual meaning: a join while NetworkManager is absent reports
`JoinResult::Absent` and hides the item, never an error.

## The audio path (T-07.3)

The audio adapter, `dragonfruit-audio` (`services/audio`), gives the menu bar
the default sink's volume and mute plus the per-sink list. PipeWire/WirePlumber
expose no stable D-Bus volume interface, so the live source talks to WirePlumber
through the tools it ships: `pw-dump` for the graph read (JSON) and `wpctl` for
the volume/mute write. The JSON/CLI churn is pinned in one place —
`AudioData::from_pw_dump`, tested against a captured fixture — and the adapter,
model, and shell see only the typed `AudioSnapshot`
([adr/0028](adr/0028-audio-adapter-over-wireplumber-cli.md)). WirePlumber stores
channel volume on a cubic curve; the parser un-cubes it to the linear 0..=1
value the status item and slider render.

The read path is the same three-state seam as NetworkManager: `AudioSource::read`
returns the raw graph (`Ok(Some)`), absence (`Ok(None)`), or a read failure
(`Err`). `pw-dump` that cannot run or cannot reach a PipeWire core is absence —
hidden, never an error. The two writes (`set_volume`/`set_mute`) are explicit
user actions over the same seam and do not invent a snapshot; the daemon pushes
the result and the host re-reads, so the snapshot stays the single source of
truth.

## The audio routing path (T-15.3a)

The same adapter also reads and routes input devices. WirePlumber's `default`
metadata carries both `default.audio.sink` and `default.audio.source`, and the
`pw-dump` graph holds both `Audio/Sink` and `Audio/Source` nodes, so one read
fills the output and input device lists with their own default. The model
resolves each default independently (falling back to the first device in that
list when WirePlumber named none), marks it, and sorts it first; the output
list still drives the menu-bar item, the input list adds the Sound pane's
`Input` tab ([adr/0121](adr/0121-sound-routing-adapter.md)).

Two writes switch the routed device: `AudioAdapter::set_default_sink(id)` and
`set_default_source(id)`, both `wpctl set-default <node-id>` behind the same
`AudioSource` seam. They are explicit user actions, not a poll, and they invent
no snapshot: WirePlumber pushes the new default and the host re-reads. Selecting
an output device thus makes it the default, after which the existing
`set_volume`/`set_mute` (which target `@DEFAULT_AUDIO_SINK@`) control it. A
write against a node id that is not in the graph is a `Failed` outcome, not a
denial or a panic. Per-application stream routing stays out of scope (no
consumer yet). This closes the "routing and device switching deferred to T-15"
note in the T-07.3 path above.

## The power path (T-07.4)

The power adapter, `dragonfruit-power` (`services/power`), gives the menu bar
the system battery's presence, charge level, and charging state. UPower already
exposes these over its D-Bus API on the **system bus** (`org.freedesktop.UPower`:
`OnBattery` plus `EnumerateDevices` and each device's
`org.freedesktop.UPower.Device` properties), so the live source is a `zbus`
client — no extra dependency beyond what NetworkManager already brings into its
own crate ([adr/0026](adr/0026-concrete-adapters-in-their-own-crates.md)).

The read path is the same three-state seam as the other adapters:
`PowerSource::read` returns the raw device enumeration (`Ok(Some)`), absence
(`Ok(None)`), or a read failure (`Err`). A bus where UPower does not own its
name is absence — hidden, never an error. `PowerSnapshot::from_data` picks the
first present battery, maps `UPowerDeviceState` to `ChargeState` and
`UPowerBatteryLevel` to `BatteryLevel`, and clamps the percentage to 0–100.
It also reads `Capacity` and `ChargeCycles` for the health row. The battery
half is **read-only**; the power-profile half (T-15.6a) adds the one write.

The item is hidden in two different ways. UPower itself being
absent is the adapter's `AdapterState::Unavailable`. A machine that runs UPower
but has no present battery (a desktop, a VM) still answers `Available`, with
`PowerSnapshot::present()` false — the consumer hides the item then. A battery
that is present but whose `IsPresent` is false is treated the same as no
battery. Nothing blocks session startup when either is missing.

### The battery and power profiles adapter (T-15.6a)

T-15.6a grows `dragonfruit-power` from the read-only battery item into the
full battery **and** power-profiles adapter. UPower and
power-profiles-daemon are independent system-bus daemons, so the source reads
both in one `read`:

- UPower, as above, now also carrying `Capacity` (health as a percentage of
  design capacity; `BatteryHealth::from_capacity` maps a documented 80%
  threshold to `Normal`/`Service`) and `ChargeCycles`;
- power-profiles-daemon — `ActiveProfile`, the `Profiles` list (`aa{sv}` of
  `{Profile, Driver, PlatformDriver}`), `PerformanceInhibited`,
  `PerformanceDegraded`, and the `ActiveProfileHolds` count. The one write is
  [`set_active_profile`], a single `Properties.Set` of `ActiveProfile`; a
  successful write invents no snapshot and the host re-reads.

The daemon moved its well-known name from `net.hadess.PowerProfiles` to
`org.freedesktop.UPower.PowerProfiles` (matching path and interface); the live
source probes both and speaks to whichever owns its name. The model keeps the
three-profile vocabulary (`power-saver`, `balanced`, `performance`) as
[`PowerProfile`] with stable ids and glyphs; an unrecognized active id is not
guessed. The adapter's [`PowerSnapshot::changes`] is the event half: a pure
diff that reports a battery level/charge-state/health/on-battery move, a change
to the available or active profile, a newly (un)degraded performance state, or
a hold-count move, with no polling.

**Absent-daemon behavior is per daemon.** UPower absent hides the battery item;
power-profiles-daemon absent is carried inside the snapshot
(`PowerSnapshot::profiles` is `None`) and only disables the profile control —
the battery half stays live. `PowerSource::read` answers `Ok(None)` (the
adapter's `Unavailable`) only when **both** daemons are unreachable, which is a
normal hidden state. A machine that runs only power-profiles-daemon answers
`Available` with `present: false` and a live profile list; a machine that runs
only UPower answers `Available` with `profiles_available()` false. Nothing
blocks session startup in any case.

The Settings pane and Control Center tile are T-15.6b
([adr/0129](adr/0129-battery-pane-and-tile.md)); T-15.6a ships the backend and
its mock-driven tests only ([adr/0128](adr/0128-battery-power-profiles-adapter.md)).

### The Battery pane and tile (T-15.6b)

T-15.6b ships the Settings pane and the Control Center tile as one functional
unit over the T-15.6a adapter ([adr/0129](adr/0129-battery-pane-and-tile.md)).
The bridge host's `Battery` interface gains the one write the pane raises:

- `org.dragonfruit.SystemStatus1.Battery` — `State()`/`Refresh()` and
  `SetActiveProfile(profile)` (the stable `power-saver`/`balanced`/
  `performance` id; an unknown id is a failure, never a guess).
- `battery_view` carries `health`/`healthLabel`/`capacity`/`chargeCycles`,
  `chargeState`, the `profiles` list (`{id, label, glyph, active}`),
  `activeProfile`/`profileLabel`, and `profilesAvailable` alongside the charge
  fields. `profilesAvailable` is independent of `present`: a desktop with no
  battery can still select a profile.
- The shell decodes the same view through `SystemStatusModel`; the Control
  Center tile (index 9) summarizes charge and the active profile
  (`"71% · Balanced"`), uses the active profile's glyph, and links to the pane.
  It hides when the host is absent or when there is neither a battery nor a
  profile.
- The Settings app talks to the interface through a dedicated `BatteryClient`
  seam (`apps/settings/BatteryClient.{h,cpp}`, `DF_BATTERY_FIXTURE` for tests)
  and the `Settings` singleton's `battery`/`batteryAvailable`/`refreshBattery`/
  `setPowerProfile`. The pane (`apps/settings/BatteryPane.qml`) writes live.

**Absence is layered and per daemon**, matching T-15.6a. A missing host is the
pane's one-line note; a present host with no battery hides the battery/history
groups but keeps the profile picker; a missing power-profiles-daemon hides the
picker and shows its own note while the battery rows stay live. Nothing errors.
UPower no longer exposes charge history, so the `Usage History` range switch and
chart frames render an honest absent state (provider T-15.x); the pane ships no
dead controls.

[`set_active_profile`]: ../../services/power/src/adapter.rs
[`PowerProfile`]: ../../services/power/src/model.rs
[`PowerSnapshot::changes`]: ../../services/power/src/model.rs

## The Bluetooth path (T-15.1a)

The Bluetooth adapter, `dragonfruit-bluetooth` (`services/bluetooth`), gives a
pane the controller's power/discovery state plus the device list (paired,
connected, discovered). BlueZ exposes the whole tree over its D-Bus API on the
**system bus** (`org.bluez`): one `org.freedesktop.DBus.ObjectManager.GetManagedObjects`
at `/` enumerates every `org.bluez.Adapter1` and `org.bluez.Device1` object, so
the live source is a `zbus` client and decodes the property maps directly — no
extra dependency beyond the other D-Bus adapters
([adr/0026](adr/0026-concrete-adapters-in-their-own-crates.md)).

The read path is the same three-state seam as the other adapters:
`BluetoothSource::read` returns the raw object lists (`Ok(Some)`), absence
(`Ok(None)`), or a read failure (`Err`). A bus where `org.bluez` does not own
its name is absence — hidden, never an error. `BluetoothSnapshot::from_data`
picks the controller, keeps only its devices, orders them connected, paired,
then name, and derives the `powered`/`discovering` flags, the
`bluetooth`/`bluetooth-disabled` glyph, the label, and each device's 0–100
signal fill (from the BlueZ RSSI, which is `None` at the 0 sentinel).

Unlike the read-only battery view, Bluetooth has explicit writes, all over the
same seam and one call each, never a loop: `set_powered` (a `Properties.Set`
of `org.bluez.Adapter1.Powered`), `set_discovering`
(`StartDiscovery`/`StopDiscovery`), `pair`, and `set_connected`
(`Connect`/`Disconnect`). A successful write invents no snapshot: BlueZ pushes
the resulting `PropertiesChanged`/`InterfacesAdded`, the host re-reads, and the
snapshot stays the single source of truth. BlueZ authorizes powering and
pairing through polkit; a refusal comes back as `BluetoothOutcome::Denied` with
the daemon's message and leaves the read state live — the adapter does **not**
blanket-degrade to read-only the way the Wi-Fi join path does, because the
denied operation is per-request rather than a session-wide loss of authority
([adr/0117](adr/0117-bluetooth-adapter-absence-and-write-outcomes.md)).

The item hides in two different ways, as the battery item does. `bluetoothd`
itself being absent is the adapter's `AdapterState::Unavailable`. A machine
that runs BlueZ but has **no controller** answers `Available`, with
`BluetoothSnapshot::present()` false — the consumer hides the item then. A
controller that is present but `Powered = false` is still present: the pane
shows it with an Off switch. Neither missing daemon nor missing hardware blocks
session startup.

### The Bluetooth pane and tile (T-15.1b)

The Settings Bluetooth pane and the Control Center tile are one functional unit
over the same adapter. Both read the `dragonfruit-system-status` bridge host's
`org.dragonfruit.SystemStatus1.Bluetooth` interface
([adr/0118](adr/0118-bluetooth-pane-and-tile.md)): its `State()` returns the
flat JSON view `BluetoothHost` projects from `BluetoothSnapshot` (adapter
power/discovery, `present`, the known-device and nearby-device lists with each
device's connection state and signal), and `SetPowered`, `SetDiscovering`,
`Pair`, and `SetConnected` are the four explicit writes. The host re-reads BlueZ
after a write; the pane and tile never invent state.

The pane is the macOS information architecture: a toggle card with the
discoverable caption (`This <device> is discoverable as "<device>" while
Bluetooth Settings is open.`), a `My Devices` list, and a `Nearby Devices`
section that shows `Searching…` while an inquiry runs. Opening the pane starts
discovery and closing it stops discovery. The Control Center tile carries the
same toggle plus the known-device rows and a `Bluetooth Settings…` entry point
(the launch itself is T-16). The absence behavior mirrors the adapter's two hide
rules: `bluetoothd` gone or no controller hides the tile and disables the pane's
controls with a one-line note; a `BluetoothOutcome::Denied` write is surfaced
per action without degrading the read state.

## The storage path (T-15.2a)

The storage adapter, `dragonfruit-storage` (`services/storage`), gives a pane
the drives, the mountable volumes, and the mounted/removable state the track
demo needs to "mount/eject a USB drive and see it in Files and the sidebar".
UDisks2 exposes the whole tree over its D-Bus API on the **system bus**
(`org.freedesktop.UDisks2`): one
`org.freedesktop.DBus.ObjectManager.GetManagedObjects` at
`/org/freedesktop/UDisks2` enumerates every `org.freedesktop.UDisks2.Drive`
and `org.freedesktop.UDisks2.Block` object (with an optional
`org.freedesktop.UDisks2.Filesystem`), so the live source is a `zbus` client
that decodes the property maps directly
([adr/0119](adr/0119-storage-adapter-absence-and-mount-outcomes.md)).

The adapter consumes **UDisks2 alone**. UDisks2 already carries the block
enumeration, the `HintAuto`/`HintSystem`/`HintIgnore` hints, and the mount
operations, so loading GIO/GVfs as well would reimplement the volume monitor
for no new state; GIO stays where the project already uses it — the Files
sidebar's volume view (`files-core`).

The read path is the same three-state seam as the other adapters:
`StorageSource::read` returns the raw object lists (`Ok(Some)`), absence
(`Ok(None)`), or a read failure (`Err`). A bus where
`org.freedesktop.UDisks2` does not own its name is absence — hidden, never an
error. `StorageSnapshot::from_data` joins each block object to its drive,
keeps only the mountable blocks that are not `HintIgnore`, orders the volumes
mounted first, removable next, then by name, and derives the
`drive-harddisk`/`drive-removable-media` glyph and each volume's display name
(label, then device node, then UUID).

The writes are explicit user actions, all over the same seam and one call
each, never a loop: `mount` and `unmount` (`Mount`/`Unmount` on the block's
`Filesystem`) and `eject` (`Eject` on the owning `Drive`). They are addressed
by the UDisks2 **object path** the snapshot carries, not by device node. A
successful write invents no snapshot: UDisks2 pushes the resulting property
changes, the host re-reads, and the snapshot stays the single source of truth.
UDisks2 authorizes mounting, unmounting, and ejecting through polkit; a
refusal comes back as `StorageOutcome::Denied` with the daemon's message and
leaves the read state live. A busy device that refuses to unmount is
`StorageOutcome::Failed`, not a denial.

The item hides in two different ways. UDisks2 itself being absent is the
adapter's `AdapterState::Unavailable`. A machine that runs UDisks2 but has
**no mountable volume** answers `Available`, with `StorageSnapshot::present()`
false — the consumer hides the item then. Neither missing daemon nor missing
hardware blocks session startup. Locked encrypted volumes have no `Filesystem`
until unlocked and are not listed as mountable; unlock and format are out of
scope.

### The Storage pane and tile (T-15.2b)

The Settings Storage pane and the Control Center tile are one functional unit
over the same adapter. Both read the `dragonfruit-system-status` bridge host's
`org.dragonfruit.SystemStatus1.Storage` interface
([adr/0120](adr/0120-storage-pane-and-tile.md)): its `State()` returns the flat
JSON view `StorageHost` projects from `StorageSnapshot` (the `present` flag, the
`mountedCount`/`volumeCount`/`removableCount` summary, and the drive and volume
lists with each volume's mounted state, mount point, filesystem, and
removable/ejectable/system flags), and `Mount`, `Unmount`, and `Eject` are the
three explicit writes. The host re-reads UDisks2 after a write; the pane and
tile never invent state.

The pane lists the mountable volumes with a Mount/Unmount action per row and a
`Removable Media` group with an Eject action per removable drive. The Control
Center tile mirrors that: the mounted-count subtitle, a Mount/Unmount row per
volume, an Eject action on removable volumes, and a `Storage Settings…` entry
point (the launch itself is T-16). Opening the pane asks the host for a re-read.

The pane follows the reference's IA only in spirit: the macOS reference puts
Storage under `General > Storage`, but this project's `General` pane has not
shipped (T-15.10). Shipping Storage as a top-level pane keeps the no-half-panes
rule without gating it behind a later task. The absence behavior mirrors the
adapter's two hide rules (ADR 0119): UDisks2 gone or no mountable volume hides
the tile and shows the pane's one-line note with the controls inert; a
`StorageOutcome::Denied` write is surfaced per action without degrading the read
state.

### The Sound pane and tile (T-15.3b)

The Settings Sound pane and the Control Center Sound tile are one functional
unit over the audio adapter's routing path. Both read the
`dragonfruit-system-status` bridge host's
`org.dragonfruit.SystemStatus1.Audio` interface
([adr/0123](adr/0123-sound-pane-and-tile.md)): its `State()` now carries the
input list (`sources`, `sourceCount`) and the `defaultSource` name alongside the
output list, and `SetDefaultSink(id)` / `SetDefaultSource(id)` are the two
routing writes the pane raises. The host re-reads WirePlumber after a write; the
pane and tile never invent state.

The pane mirrors the reference's two groups. `Output & Input` is the routing
unit: an `Output`/`Input` segmented control, a `Name`/`Type` device table whose
row selection makes that device the default (`wpctl set-default`), an `Output
volume` slider, a `Mute` toggle, and a `Balance` slider. `Sound Effects` holds
`Alert sound`, `Play sound effects through`, `Alert volume`, and the three
playback toggles. The Control Center tile stays compact: it reflects the default
output device's name and the volume/mute state it already wrote, plus a `Sound
Settings…` entry point (the launch itself is T-16).

The `Sound Effects` and `Balance` rows have no daemon of their own: they are
settingsd keys in the new `sound` group (`sound.alertSound`,
`sound.playEffectsThrough`, `sound.alertVolume`, `sound.playOnStartup`,
`sound.uiEffects`, `sound.volumeFeedback`, `sound.balance`), additive in schema
revision 11. That is the T-08 host-services provider for the rows the reference
draws, so every control in the shipped pane writes somewhere durable and
applies live — no dead toggles. An actual alert/UI-sound playback engine is the
one documented follow-up; the pane owns and persists the user's preference
today. Device selection, `Output volume`, and `Mute` are deliberately *not*
settings keys: they stay on the adapter and round-trip through the bridge host.

Absence follows the adapter's two hide rules (ADR 0121). WirePlumber gone or a
running daemon with no device hides the `Output & Input` group and shows a
one-line note; the `Sound Effects`/`Balance` controls stay live on the schema
defaults, so an absent daemon never turns the pane into a dead surface.

## The input device path (T-15.4a)

The input adapter, `dragonfruit-input` (`services/input`), gives the Keyboard /
Mouse / Trackpad pane the device inventory its rows attach to: which keyboards,
mice, and trackpads the session has, their capabilities, and the libinput
built-in defaults of their configurable features. It is deliberately
**read-only**, because input is the one subsystem principle 3 reserves for the
compositor: libinput keeps no persisted configuration and ships no setter, the
compositor owns the device handles, and the user's own keyboard/pointer choices
belong to `settingsd`, applied live over the `df_toplevel_manager` bridge
([adr/0034](adr/0034-compositor-policy-via-shell-bridge.md)). The adapter is the
inventory half, not a second settings owner.

The live source reuses libinput's shipped control tool. `CommandLibinput` runs
`libinput list-devices` once per `InputAdapter::refresh` and parses the
record-per-device output in one place (`services/input/src/libinput.rs`),
exactly as the audio adapter reads WirePlumber through `pw-dump`
([adr/0028](adr/0028-audio-adapter-over-wireplumber-cli.md)); the project links
no libinput and the compositor stays the only holder of device handles. The
parser is total — unknown keys are ignored and blank records skipped — so a tool
that grows or renames a field keeps working, and the format is pinned by a
fixture.

The read path is the same three-state seam as the other adapters:
`InputSource::read` returns the raw device list (`Ok(Some)`), absence
(`Ok(None)`), or a read failure (`Err`). A host where libinput cannot run or
cannot reach a seat is absence — hidden, never an error.
`InputSnapshot::from_data` classifies each device from its capability tokens (a
pointer with gesture support or tapping is a trackpad, any other pointer a
mouse), orders keyboards then pointers, and derives the glyph and label. A pure
`device_changes(previous)` diff reports added and removed devices, so a host
that re-reads on an add/remove has the event without per-device polling
([adr/0124](adr/0124-input-device-adapter.md)).

The item hides in two different ways. libinput itself being absent is the
adapter's `AdapterState::Unavailable`. A session that runs libinput but has
**no recognized device** answers `Available`, with `InputSnapshot::present()`
false — the consumer hides the item then, exactly as the battery item hides on
a machine with no battery. Neither missing tool nor missing hardware blocks
session startup. T-15.4b adds the pane and tile; it reads the settings values
through `settingsd` and adds the pointer keys (`Tracking speed`, `Tap to
click`, scrolling) as settingsd-owned, compositor-consumed settings.

### The Keyboard/Mouse/Trackpad pane and tile (T-15.4b)

The Settings Keyboard, Mouse, and Trackpad panes and the Control Center
Keyboard tile are one functional unit over the input inventory. The three
panes share one body (`apps/settings/InputPane.qml`) parameterised by
`section`, so the pointer controls cannot drift between them. The inventory
comes from the bridge host's `org.dragonfruit.SystemStatus1.Input` interface
(ADR [0125](adr/0125-keyboard-mouse-trackpad-pane-and-tile.md)): `State()` and
`Refresh()` over the read-only libinput adapter, with no write method. The pane
renders the device list (name plus kind) and asks the host for a re-read on
open.

Every preference row is a settingsd key, additive in schema revision 12:
`input.pointerSpeed`, `input.naturalScroll`, `input.tapToClick`,
`input.leftHanded`, `input.scrollMethod`, `input.keyboardBrightness`,
`input.adjustBrightnessLowLight`, `input.backlightOffAfter`,
`input.keyboardNavigation`, and `input.emojiKeyAction`. `Key repeat rate` and
`Delay until repeat` reuse the revision-1 `input.repeatRate`/`input.repeatDelay`.
The pointer keys map to `PointerSettings` in
`compositor/src/input/settings.rs` and are the compositor's to apply; the
keyboard-backlight and emoji-panel bridges are follow-ups, so those rows
persist their preference today (the same split T-15.3b used for the Sound
Effects keys). The Control Center tile is a compact read-only summary of the
inventory plus a `Keyboard Settings…` entry point (the launch itself is T-16).

Absence follows the adapter's two hide rules (ADR 0124). libinput gone, or a
running stack with no recognized device, hides the device list and shows a
one-line note; the settingsd-backed preference rows stay live on the schema
defaults, so an absent inventory never turns the panes into dead surfaces.

## The Mission Control and hot corners path (T-15.5a)

Mission Control and hot corners are the one subsystem with **no external
daemon**: the compositor detects a corner in its input path
(`compositor/src/input/hot_corners.rs`) and drives the single overview state
machine (`compositor/src/overview/mod.rs`), and the shell learns both over the
private `df_toplevel_manager` bridge (`hot_corner`, `overview_changed`). The
adapter, `dragonfruit-overview` (`services/overview`), therefore does not
re-implement detection or the transition — it is the **projection** of what the
compositor already publishes, behind a `MissionControlSource` seam.

The snapshot names the four corners and the five assignable actions (`none`,
`Mission Control`, `Notification Center`, `Desktop Reveal`, `Lock Screen`), the
dwell and inset, and the gesture-gating trio (`gestures.enabled`,
`gestures.spaceSwitch`, `gestures.missionControl`) the compositor applies live
over `set_input_policy`. It carries the runtime overview state too — open or
closed, the selection, and the Space/window counts. The stable action ids
(`mission-control`, `notification-center`, …) are the same spelling the
settings keys and the wire use, so the pane's popup maps straight onto a key
value.

The event half is two streams. A pure `MissionControlSnapshot::changes(previous)`
diff reports a corner reassignment, a gesture change, the overview opening or
closing, a selection move, and count changes without any polling. A hot-corner
trigger is **instantaneous** — it is never part of a snapshot — so the bridge
queues it and the adapter drains it as a `HotCornerTrigger { corner, action }`.

The durable trigger choices stay where principle 3 puts them: the compositor
owns the runtime, `settingsd` owns the preferences and the compositor applies
them live, and the adapter only reports. A missing bridge is the adapter's
`AdapterState::Unavailable` — a normal hidden state; a present bridge that
cannot be read is `Error`, visible and inert with the message. The
`MockMissionControl` source drives the states in CI, with `kill`/`restart` for
absence and re-subscribe and `trigger` for the event stream
([adr/0126](adr/0126-mission-control-hot-corners-adapter.md)).

### The Mission Control pane and tile (T-15.5b)

T-15.5b adds the Settings pane and the Control Center tile as one functional
unit ([adr/0127](adr/0127-mission-control-pane-and-tile.md)). It is the one T-15
surface where **no services-layer host is added**: the only reader of the
compositor's overview state is the shell itself (the `df_toplevel_manager`
client), and the settings values live in the same process. Adding a
`dragonfruit-system-status` source would mean inventing a bridge from a
service to a Wayland client that does not exist, so the shell projects the tile
locally instead.

- The **Control Center tile** carries a trigger summary the shell computes from
  the settingsd values (`shell/src/controlcenterpolicy.cpp::missionControlView`
  mirrors the Rust `MissionControlSnapshot::label()`): `Gesture`,
  `Gesture, n corner(s)`, `n corner(s)`, or `No trigger`, with the `overview`
  glyph, plus a `Mission Control Settings…` link. It is shell-native because
  Mission Control is compositor-native.
- The **Settings pane** (`apps/settings/MissionControlPane.qml`) writes the
  revision-13 `overview.hotCorner*` assignment keys and the revision-1
  `gestures.*` trio. Every row applies live; the compositor already applies the
  gesture trio over `set_input_policy`, and applying a corner assignment needs
  the append-only request ADR [0126](adr/0126-mission-control-hot-corners-adapter.md)
  names (deferred).
- **Absence** is a missing settings daemon: the pane's rows stay live on the
  schema defaults and it shows a one-line note. There is no adapter `present`
  hide rule, because there is no adapter view in the shell path.

## The Notifications and Focus path (T-15.7a)

Notifications and Focus do not wrap a host daemon the way Bluetooth or UPower
do: the **notification service is ourselves** (`services/notifications`,
`dragonfruit-notifications`, T-11.1a/T-11.2a). It owns the queue, the bounded
history, and the Focus/DND policy, and serves the standard
`org.freedesktop.Notifications` and the shell-facing
`org.dragonfruit.Notifications1` at `/org/freedesktop/Notifications`. The
adapter (`services/notify-adapter`, `dragonfruit-notify-adapter`) is therefore
a **projection over our own service**, never a second queue or a second policy:
"reuse, never reimplement" is load-bearing here. See
[adr/0130](adr/0130-notifications-focus-adapter.md).

- **The read** is the service's three shell-facing JSON views — `FocusPolicy()`
  (`mode`, `allowList`, `batchedCount`), `Banners()`, and `History()` — decoded
  into `NotificationsData` and folded into `NotificationsSnapshot`: the
  `FocusSnapshot` (mode, allow list, suppressed batch), the active banners, the
  recorded history, and the derived per-app list (`AppNotifications`, the rows
  the Notifications pane will draw).
- **The writes** are the two Focus setters the service already exposes:
  `set_focus_mode` (`off`/`focus`/`dnd`, reusing the service's own `FocusMode`
  vocabulary) and `set_focus_allow_list`. A successful write invents no
  snapshot; the service pushes `Changed` and the host re-reads.
- **Events** are the shared subscription lifecycle (`AdapterEvent`) plus a pure
  `NotificationsSnapshot::changes` diff (mode, allow list, batch, banner and
  history counts) that the adapter queues in `drain_changes`.
- **Absence is layered and normal.** No session bus, no daemon owning
  `org.freedesktop.Notifications`, or a daemon that does not serve our shell
  interface are all `Unavailable` and hide the item. A service that owns its
  name and serves the interface but returns an error or malformed JSON is
  `Error`: visible, inert, with the message.
- **What is not here.** The global notification preferences the macOS pane
  shows (`Show previews`, `when display is sleeping`, `when screen is locked`,
  `when mirroring`) have no owner in the service; T-15.7b adds them as
  settingsd keys (revision 14), not adapter state. The pane and Control Center
  tile are T-15.7b.

Like every adapter, the live D-Bus source is a thin mechanical layer behind the
`NotificationsSource` seam; the mock (`MockNotifications`) drives all three
states, the change stream, and absence in CI, and the live source is proven
over a private `dbus-daemon` in `services/notify-adapter/tests/session_bus.rs`.

### The Notifications and Focus panes and tile (T-15.7b)

The pane and tile ship as one functional unit over the T-15.7a adapter. The
bridge host's `Notifications` interface mirrors Bluetooth/storage/battery:

- `org.dragonfruit.SystemStatus1.Notifications` — `State()`/`Refresh()` and the
  two Focus writes `SetFocusMode(mode)` (`off`/`focus`/`dnd`) and
  `SetFocusAllowList(apps)`.
- `notifications_view` carries the Focus mode/label, the suppressed batch, the
  allow list, the banner/history counts, and the observed per-app list; the
  settingsd preferences are not on this view.
- The Settings app talks to the interface through a dedicated
  `NotificationsClient` seam (`apps/settings/NotificationsClient.{h,cpp}`,
  `DF_NOTIFICATIONS_FIXTURE` for tests) and the `Settings` singleton's
  `notifications`/`notificationsAvailable`/`refreshNotifications`/
  `setFocusMode`/`setFocusApp`.
- The **Notifications pane** (`apps/settings/NotificationsPane.qml`) owns the
  four global preferences (`notifications.showPreviews`,
  `notifications.showWhenSleeping`, `notifications.showWhenLocked`,
  `notifications.showWhenMirroring`, settingsd revision 14) and the read-only
  per-app inventory. The **Focus pane** (`apps/settings/FocusPane.qml`) owns the
  mode selector and the per-app allow list, sharing the same adapter view. The
  Control Center **Focus tile** (T-11.3b) reflects the same service policy and
  remains the tile for this unit.
- **Absence is layered.** The bridge host absent, or a foreign notification
  daemon that does not serve the shell interface, hides the per-app inventory
  and shows a one-line note; the settingsd preference rows stay live on the
  schema defaults. No write while absent is anything but a no-op.

The four presentation preferences are **stored policy**: they apply to
settingsd live (and the pane reflects them), but the notification service does
not yet read them to gate a banner. Wiring that enforcement is a follow-up; the
rows are the capture's controls and are not dead (see ADR
[0131](adr/0131-notifications-pane-and-tile.md)).

## The Lock Screen policy path (T-15.8a)

Lock Screen policy does not wrap an external daemon either: the session's
idle/lock engine (`services/session/src/idle.rs`) owns the `idle.*` stage
delays ([ADR 0070](adr/0070-idle-timer-engine-and-policy.md)) and the
compositor owns the one fail-secure lock state (`compositor/src/lock.rs`,
[ADR 0067](adr/0067-session-lock-protocol-and-ui.md)); the shell mirrors both
over the private `df_toplevel_manager` bridge. The adapter
(`services/lock-adapter`, `dragonfruit-lock-adapter`) is therefore a
**projection** over that host stack, exactly as `dragonfruit-overview` projects
the compositor's hot corners and overview machine. It reuses the session's own
`IdlePolicy`/`IdleStage` vocabulary rather than re-modelling the chain. See
[adr/0132](adr/0132-lock-screen-policy-adapter.md).

- **The read** is `LockPolicyData`: the runtime `locked` flag, the effective
  `IdlePolicy` (dim/blank/lock/suspend), and the lock-screen display options
  (show user name and photo, show password hints, show message when locked +
  its text, show the sleep/restart/shut-down buttons). `LockPolicySnapshot`
  types the lock state and names the four display options; the idle policy is
  carried unchanged.
- **Events** are the shared subscription lifecycle (`AdapterEvent`) plus a pure
  `LockPolicySnapshot::changes` diff — the lock state, each idle stage delay,
  each display option, and the message — drained through `drain_changes`. A
  lock/unlock is a `StateChanged` event, not a poll.
- **Read-only.** The compositor owns the lock and the session owns the timing;
  the durable display preferences are `settingsd`'s (the `lock.*` keys land in
  T-15.8b). The adapter has no write method, exactly as `dragonfruit-overview`
  and `dragonfruit-input` have none.
- **Absence is a normal state.** A missing compositor bridge (no
  `df_toplevel_manager` global, or an unreachable compositor) is
  `AdapterState::Unavailable` and hides the item; a present bridge that cannot
  be read is `Error`, visible and inert with the message. Neither blocks
  session startup. `MockLockPolicy` drives the states in CI, with
  `kill`/`restart` for the re-subscribe lifecycle and `push` for the state and
  policy stream.
- **The Linux adaptation.** The session's idle engine blanks and locks at one
  delay each, so the macOS battery/AC display-off pair becomes the single
  `blank` stage; `display_off_after()` and `lock_after()` are the projection the
  pane will draw. The pane and Control Center tile are T-15.8b.

### The Lock Screen pane and tile (T-15.8b)

The pane and tile ship as one functional unit, and — because there is no
external daemon and the shell is the only process that speaks to the
compositor — both are shell-native where the runtime is concerned, exactly like
Mission Control (T-15.5b; ADR [0127](adr/0127-mission-control-pane-and-tile.md)):

- **Settingsd keys, revision 15.** The Lock Screen pane
  (`apps/settings/LockScreenPane.qml`) writes the four display options as the
  new `lock.*` keys (`lock.showUserNameAndPhoto`, `lock.showPasswordHints`,
  `lock.showMessageWhenLocked`, `lock.showPowerButtons`) plus the custom
  `lock.message`; their suffixes are `LockDisplayOption::id()` from the T-15.8a
  adapter. The two timing rows reuse the existing revision-5 `idle.blank` /
  `idle.lock` keys the session idle engine already reads (`IdlePolicy::from_keys`).
  Every row applies live and persists; there is no Apply button, and the pane
  never touches the compositor's lock hot path.
- **The Control Center tile is projected by the shell.** No services-layer host
  is added (the lock state and idle policy are not on the bus). The pure
  `lockPolicyView(values)` in `shell/src/controlcenterpolicy.cpp` turns the
  `idle.lock` delay into the tile subtitle (`Password after 10 min`,
  `No password required`), mirroring the Mission Control projection; the QML
  tile renders it and raises `lockScreenSettingsRequested` (the Settings launch
  is T-16's entry point).
- **Absence is a missing settings daemon.** With no daemon the pane's rows stay
  live on the schema defaults and it shows a one-line note; the tile is always
  present while the shell runs.
- The four `lock.*` display keys are **stored policy**: settingsd holds them
  live and the pane reflects them, but the lock-screen renderer does not yet
  read them to hide the name/photo, hints, message, or power buttons. Wiring
  that is a follow-up (see
  [adr/0133](adr/0133-lock-screen-pane-and-tile.md)); the rows are the capture's
  controls and are not dead.

## The Menu Bar configuration path (T-15.9a)

Menu Bar configuration does not wrap an external daemon either: the shell's
`MenuBar` (`shell/menubar/MenuBar.qml`) owns the chrome, the status-item model,
and the clock, and the menu-broker (`services/menu-broker`) resolves the
focused app's global menu. The adapter (`services/menubar-adapter`,
`dragonfruit-menubar-adapter`) is therefore a **projection** over that host
stack, exactly as `dragonfruit-overview` projects the compositor and
`dragonfruit-lock-adapter` projects the session/compositor pair. See
[adr/0134](adr/0134-menu-bar-configuration-adapter.md).

- **The read** is `MenuBarData`: whether the bar is hidden by auto-hide, the
  effective `MenuBarAutoHide` mode (`never`/`always`/`full-screen`), the
  background and global application-menu flags, the `ClockOptions`
  (`showDate`/`showSeconds`), and the availability of each `MenuBarControl`
  (`clock`, `wifi`, `bluetooth`, `battery`, `volume`, `focus`,
  `accessibility`). `MenuBarSnapshot` types all of it; the control ids are the
  shell status-item ids, so a projection maps straight onto the status row.
- **Events** are the shared subscription lifecycle (`AdapterEvent`) plus a pure
  `MenuBarSnapshot::changes` diff — the runtime hidden flag, the auto-hide
  mode, the background and global-menu flags, each clock option, each control
  slot — drained through `drain_changes`.
- **Per-control absence is not adapter absence.** One control's daemon going
  away is an absent `MenuBarControlState` slot inside an `Available` snapshot
  (mirroring `StatusItem`'s `available`/`enabled`), so only that slot hides.
- **Read-only.** The durable preferences are `settingsd`'s and the shell
  renders the bar; the adapter has no write method, exactly as
  `dragonfruit-overview` and `dragonfruit-lock-adapter` have none.
- **Absence is a normal state.** A missing menu-bar bridge is
  `AdapterState::Unavailable` and hides the item; a present bridge that cannot
  be read is `Error`, visible and inert with the message. Neither blocks
  session startup. `MockMenuBar` drives the states in CI, with
  `kill`/`restart` for the re-subscribe lifecycle and `push` for the state and
  configuration stream.
- **The Linux adaptation.** The Apple-only controls (AirDrop, Screen
  Mirroring) and the deferred Search/Spotlight entry are omitted; the richer
  macOS `Clock Options...` rows the shell clock does not yet render are
  recorded as follow-ups rather than shipped as dead controls. The pane and
  Control Center tile are T-15.9b.

## The status bridge host (T-07.5a)

The adapters are Rust crates; the menu bar is C++/QML. T-07.5a bridges them in
a dedicated session-bus host, `dragonfruit-system-status`
(`services/system-status`), rather than linking an adapter into the shell
([adr/0029](adr/0029-system-status-bridge-host.md)). The host owns the
NetworkManager, audio, and power adapters and serves
`org.dragonfruit.SystemStatus1` at `/org/dragonfruit/SystemStatus1`:

- `org.dragonfruit.SystemStatus1.Wifi` — `State()`/`Refresh()` (the decoded
  Wi-Fi view as JSON, with the network list and the polkit read-only
  degradation) and `Join(ssid, secret)`.
- `org.dragonfruit.SystemStatus1.Audio` — `State()`/`Refresh()` and
  `SetVolume(volume)`, `SetMute(muted)`, `ToggleMute()`, plus the T-15.3b
  routing writes `SetDefaultSink(id)` and `SetDefaultSource(id)`.
- `org.dragonfruit.SystemStatus1.Battery` — `State()`/`Refresh()` and
  `SetActiveProfile(profile)` (the T-15.6b power-profile write).
- `org.dragonfruit.SystemStatus1.Storage` (T-15.2b) — `State()`/`Refresh()`
  plus `Mount(volumePath)`, `Unmount(volumePath)`, `Eject(drivePath)`.
- `org.dragonfruit.SystemStatus1.Input` (T-15.4b) — `State()`/`Refresh()` only
  (the read-only keyboard/mouse/trackpad inventory; libinput has no setter).
- `org.dragonfruit.SystemStatus1.Notifications` (T-15.7b) —
  `State()`/`Refresh()` plus `SetFocusMode(mode)` and
  `SetFocusAllowList(apps)` (the Notifications/Focus panes' writes).

The host core (`StatusHost`) is adapter-only and CI-tested with the mocks; the
D-Bus layer is a thin mechanical wrapper. The shell decodes the JSON in one
model (`shell/src/systemstatusmodel.*`) and draws the design-system popovers
(`WifiMenu.qml`, `VolumeMenu.qml`, `BatteryMenu.qml`). `--placeholders` is
gone; `DF_STATUS_FIXTURE=1` selects the fixture client for headless use and
the capture script, and every live item otherwise degrades to hidden when its
daemon is absent. The host itself is started by the session (a systemd user
unit at packaging time); the dev tool does not start it, so a live dev session
shows only the items whose daemons the host can reach once it is running.

The bridge keeps the adapter contract's no-poll rule: the host reads an
adapter on startup, on the shell's explicit `Refresh()` (menu open), and after
an action reply. Wiring the daemons' own change signals (NetworkManager,
`pw-mon`, UPower `PropertiesChanged`) into the host so a view updates within
one event is T-07.6, and the item degradation stays where it already lives —
`Unavailable` hides the item, `Error` shows it visible and inert.

## The absent-daemon masking matrix (T-07.6a)

Masking (or `systemctl stop`ping) a daemon is a supported session state, not an
incident. The matrix below is the contract the status bar must satisfy; the
same three-state projection is implemented once in each adapter and asserted at
every seam. "Hides" means the slot is not drawn and has no popover; "inert"
means drawn but not actionable.

| Masked | Wi-Fi item | Volume item | Battery item | Session |
|---|---|---|---|---|
| none | live | live | live | normal |
| NetworkManager | **hides** | live | live | unaffected |
| WirePlumber/PipeWire | live | **hides** | live | unaffected |
| UPower | live | live | **hides** | unaffected |
| all three | **hides** | **hides** | **hides** | unaffected; actions report `absent` |
| (no bridge host) | **hides** | **hides** | **hides** | unaffected |

Two motion-adjacent cases stay distinct from absence and are part of the
matrix's negative space: a daemon that is *present but unreadable* projects
`Error`, so its item is visible but inert and never leaks an `error` into a
neighbour's view; a machine that runs UPower but has **no present battery**
reports `available` with `present: false` and the consumer hides the battery
item by that second rule (see [The power path](#the-power-path-t-074)).

The headless half of the matrix is the CI gate. Each adapter's mock source
(`MockNetworkManager`/`MockAudio`/`MockPower`) has `kill()`/`restart()` to
simulate the daemon's `Disconnected`/re-subscribe lifecycle, and the three
seams are asserted:

- **Bridge host** (`services/system-status/tests/host.rs`):
  `the_absent_daemon_masking_matrix_hides_only_the_masked_item` masks each
  daemon in turn and then all three, and asserts the neighbour items stay
  `available`, no slot reports `error`, and `join`/`set_volume`/`set_mute`
  answer `absent`. `masking_one_daemon_never_turns_a_neighbour_into_an_error`
  covers the present-but-unreadable case.
- **Shell decode model** (`shell/tests/tst_statusmodel.cpp`):
  `theAbsentDaemonMaskingMatrixHidesOnlyTheMaskedItem` applies the host's JSON
  views and checks only the masked item's `visible`/`enabled` flip;
  `anUnreachedBridgeHostLeavesEveryItemHidden` covers the no-host default.
- **Menu bar** (`shell/tests/tst_menubar.qml`):
  `test_absent_daemon_matrix_hides_and_refuses_each_item` checks the masked slot
  is hidden and refuses to open while every other slot still opens its popover.

The real-daemon half — actually stopping the three daemons in a VM and watching
the bar — is a manual/VM check (the design's "real masking in a VM when
available"); no VM is available in CI, so it is recorded here rather than
automated. The host itself is a separate process, so "no bridge host" is the
same hidden-everything state on a live session.

## The menu-bar idle trace (T-07.6b)

A live menu bar must still cost nothing when the desktop is idle (FR-6). The
no-poll rule is proven at both seams, not asserted:

- **Compositor** — `compositor/tests/shell_idle_trace.rs` maps the menu-bar
  `df_layer_surface` (the live bar chrome), commits one frame, then sits idle;
  the compositor must render no further frames and wake no client
  (`client_wakeups` flat). `make menubar-idle-trace` runs it and
  `scripts/capture-live-menubar.sh` records the raw counters to
  `docs/captures/t07-shell-idle-trace.txt`.
- **Bridge host** — `services/system-status/tests/host.rs`
  (`the_live_status_items_never_poll_the_daemons`) shows the host touches a
  daemon only on the startup sync, an explicit `Refresh()`, or after an action:
  serving the views many times leaves the mock read counters unchanged, and one
  `refresh()` moves each by exactly one. A background poll would fail the
  test.

The shell side adds no timer: `SystemStatusClient` is event-driven (the shell
asks for a resync only when a menu opens or after an action, and the host's
`run` parks its main thread instead of polling). The T-07 slice captures are
`docs/captures/t07-live-menubar.png` (Wi-Fi, volume, and battery live over the
bridge fixture) and `docs/captures/t07-live-menubar-absent.png` (no fixture and
no host: every live item hidden); `scripts/capture-live-menubar.sh` regenerates
both.

## D-Bus conventions

Our services own names under `org.dragonfruit.*` on the **user session bus**
(settings, menus, app index, notifications, portal). Nothing of ours needs a
system-bus service of its own; privileged operations are delegated to
existing system services behind polkit, whose authentication prompt is
rendered by our shell. Absence of a daemon is a normal state, not an error
(see principle 4 above).

## Portals

Once the desktop is a real Wayland session, `xdg-desktop-portal` support
arrives **surprisingly early** in the sequence (see
[ROADMAP.md](../ROADMAP.md)). The portal project is specifically designed
around a common frontend working with desktop-environment-specific backends.

We ship:

```text
xdg-desktop-portal-dragonfruit
```

for our native file chooser, screenshots, and screen sharing. The
FileChooser interface is delegated to **Files in chooser mode**, backed by the
same `files-core` library — one browsing implementation, one set of semantics
(see [09-files.md](09-files.md)). This becomes especially important for
browsers, Electron programs, Flatpak applications, conferencing programs,
and screen capture.

### The backend service and its registration (T-13.1a)

The backend is a real session-bus service (`portal/`): it owns the standard
name `org.freedesktop.impl.portal.desktop.dragonfruit` and serves
`/org/freedesktop/portal/desktop`, the names the portal spec's `.portal`
descriptor uses. `xdg-desktop-portal` finds it through three installed files,
embedded in the crate and written by `portal::data::install_into` at
packaging time:

- `dragonfruit.portal` (`{DATADIR}/xdg-desktop-portal/portals/`) — the
  backend's D-Bus name and its `Interfaces=` list, kept in lockstep with
  `model::BACKEND_INTERFACES`. T-13.1a ships it empty; Settings and
  GlobalShortcuts arrive in T-13.1b and the capture interfaces follow.
- `dragonfruit-portals.conf` (`{DATADIR}/xdg-desktop-portal/`) — the
  `[preferred]` selection `default=dragonfruit;gtk`, read when
  `XDG_CURRENT_DESKTOP` contains `dragonfruit`, so the generic backend covers
  the interfaces we have not implemented yet.
- `org.freedesktop.impl.portal.desktop.dragonfruit.service`
  (`{DATADIR}/dbus-1/services/`) — D-Bus activation, so the frontend can
  start the backend on demand.

**The frontend's absence is a normal state.** The backend registers and
serves whether or not `xdg-desktop-portal` is running; it only observes the
frontend's well-known name — one probe at startup, then a live watch of
`org.freedesktop.DBus.NameOwnerChanged` — and exposes that on the
`org.dragonfruit.Portal1` diagnostic interface at the standard path
(`Frontend()`, `FrontendPresent()`, `FrontendOwner()`, the `FrontendChanged`
signal). An absent or unknown frontend never blocks session startup. The
contract is frozen in
[adr/0074](adr/0074-portal-backend-registration-and-frontend-degradation.md);
`portal/tests/session_bus.rs` proves the registration and both paths on a
private bus.

### Settings and GlobalShortcuts (T-13.1b)

The first concrete interfaces are read-through projections of services that
already own their state. Both are served at the standard path and listed in
`dragonfruit.portal`; the contract is frozen in
[adr/0075](adr/0075-settings-and-globalshortcuts-portals.md).

- **`org.freedesktop.impl.portal.Settings` (version 2)** projects `settingsd`
  onto two namespaces and never owns a value: `org.freedesktop.appearance`
  (`color-scheme` derived from `appearance.colorScheme`, `contrast`, and
  `accent-color` when `appearance.accent` is a concrete `#rrggbb`) and
  `org.dragonfruit.desktop` (every settingsd key, read-only). One `GetAll`
  resync at startup plus a resync on each `Changed` and on a settingsd name
  (re)appearance keeps it live; a session with no settingsd still answers with
  the honest appearance defaults.
- **`org.freedesktop.impl.portal.GlobalShortcuts` (version 1)** owns the
  session bookkeeping — create/bind/list/close one session object per caller
  path, and the `Activated`/`Deactivated`/`ShortcutsChanged` signals. It never
  installs a grab: the compositor's `ShortcutEngine` remains the one arbiter,
  and the diagnostic `ActivateShortcut` on `org.dragonfruit.Portal1` is the
  bridge a later task drives from that engine.
- `portal/tests/portals.rs` drives both interfaces over a private bus with a
  real settingsd object beside them; the value/namespace/session logic lives
  in the pure `portal::settings` and `portal::shortcuts` modules and is
  unit-tested without a bus.

### FileChooser (T-13.2a)

`org.freedesktop.impl.portal.FileChooser` is the third concrete interface and
the first *interactive* one: the standard method does not return until a
presenter has chosen, so the backend registers the request and awaits a
one-shot completion (ADR [0076](adr/0076-filechooser-portal-and-presenter-seam.md)).

- **Browsing is files-core's, unchanged.** `portal::chooser` normalizes every
  presenter selection through `dragonfruit-files-core::Location` into a
  canonical `file://` URI and discards anything that cannot be normalized, as
  the portal spec requires. The diagnostic `ListDirectory` lists through the
  same `DirectoryModel`/`StdFsSource` and collation Files uses — the portal
  links the core as a library, never a process.
- **The presenter seam is the registry.** `portal::chooser::ChooserRegistry`
  owns the live requests; `OpenFile`/`SaveFile`/`SaveFiles` (version 3)
  register one and await. The diagnostic `org.dragonfruit.Portal1` carries a
  presenter's half: `FileChooserOpened` when a request waits, and
  `CompleteFileChooser`/`CancelFileChooser`/`PendingFileChoosers`. T-13.2a
  ships no dialog; the test client is the presenter, and the T-13.2b picker
  takes the same calls.
- **Absence is not a state here.** There is no external owner to be absent:
  the chooser either has a presenter (a request completes) or the completion
  never arrives. The settings app's "No file chooser is available." row keys
  off the *frontend* (`org.freedesktop.portal.Desktop`), not this backend, and
  is unchanged.
- `portal/tests/filechooser.rs` drives a real D-Bus client acting as the
  presenter: OpenFile/SaveFile return the normalized path, a cancelled request
  answers code 1, and `ListDirectory` proves the files-core link. The
  options/result and registry logic live in `portal::chooser` and are
  unit-tested without a bus.

### The picker UI (T-13.2b)

The presenter T-13.2a deferred is a shell overlay surface (ADR
[0077](adr/0077-filechooser-picker-seat.md)):

- **The view is `Dragonfruit.Screenshot`/`FileChooser.qml`** — a pure
  design-system dialog (title from the request, an Up control and folder
  breadcrumb, the entry list with folders first, a SaveFile name field, and
  Cancel plus the caller's `accept_label`). It owns no D-Bus.
- **`ChooserBridge` (`shell/src/chooserbridge.{h,cpp}`, dockcore)** is the
  presenter: it watches `FileChooserOpened`, publishes the request and rows to
  the view, and answers with `CompleteFileChooser`/`CancelFileChooser`. A
  missing portal is a supported state — the picker simply never opens.
- **Browsing is still files-core's.** The bridge lists through the C ABI
  (`df_files_begin`/`df_files_poll`, the `files_core_list.h` slice) in-process,
  the one implementation Files and `ListDirectory` use. The portal normalizes
  the final selection as before.
- **`ShellController`/`ShellProtocol`** add the centred `file-chooser` overlay
  surface (on-demand keyboard, card-sized input region) and render into it
  while a request waits. `DF_CHOOSER_FIXTURE=<folder>` presents it for the
  live visual check with no portal.
- `tst_chooser` (bridge: listing, selection, save, go-up, cancel, and a
  round-trip against a fake `org.dragonfruit.Portal1`) and `tst_chooserui`
  (the view) are the headless proof; `portal/tests/filechooser.rs` still proves
  the backend half. T-13.7 adds the real frontend routing check.

### Screenshot portal and selection UI (T-13.3a)

`org.freedesktop.impl.portal.Screenshot` (version 2) is the fourth concrete
interface and the second interactive one. Contract frozen in ADR
[0078](adr/0078-screenshot-portal-and-selection-overlay.md).

- **The registry is the seam.** `portal::screenshot` owns the live requests and
  the one-shot completion; the synchronous `Screenshot` method registers and
  awaits, the diagnostic `org.dragonfruit.Portal1` carries the presenter half
  (`ScreenshotOpened`, `CompleteScreenshot`/`CancelScreenshot`/
  `PendingScreenshots`). The captured URI is normalized through files-core to a
  canonical `file://` URI, as the chooser's selection is.
- **The mode is a Dragonfruit option extension.** A `mode` string option
  (`fullscreen`/`region`/`window`) lets the desktop shortcut and the headless
  tests request a selection; a standard frontend that omits it is `fullscreen`
  (or `region` when `interactive`). The desktop's Cmd+Shift+3/4 shortcut drives
  the same overlay without a portal request (`beginLocal`).
- **The selection UI is a shell overlay.** `Dragonfruit.Screenshot`/
  `SelectionOverlay.qml` is a pure view — full-output scrim, a mode badge, a
  region drag with live pixel dimensions, a window crosshair, Escape cancel and
  Return accept. `ScreenshotBridge` (`shell/src/screenshotbridge.{h,cpp}`,
  dockcore) is the presenter: it opens the overlay in the request's mode,
  emits the chosen rectangle on the capture seam, and answers the portal.
  `ShellController`/`ShellProtocol` add the full-output `screenshot` overlay
  surface. `DF_SCREENSHOT_FIXTURE=<mode>` presents it for the live visual check
  with no portal or hardware.
- **Capture delivery and save/copy (T-13.3b).** The shell never reads the
  framebuffer itself: it asks the compositor over the private protocol
  (`df_toplevel_manager.capture_screenshot`, v6) and the compositor renders a
  second, offscreen pass of the frame's elements and writes a PNG to a path the
  shell supplies. `screenshot_saved`/`screenshot_failed` answer the request.
  The request is only reachable by the authenticated shell (the portal
  presenter), so no client-facing grab protocol exists and the
  `check-no-capture-grab` gate stays green. `ScreenshotWriter` (dockcore) then
  saves the image into `Pictures/Screenshots` and copies it (and the saved
  `file://` URI) to the clipboard; a waiting portal request is completed with
  the saved URI. The nested backend implements the render; DRM and headless
  answer `screenshot_failed`. Contract in ADR
  [0079](adr/0079-screenshot-capture-delivery-and-save-copy.md).
- `portal/tests/screenshot.rs` drives the blocking round trip for each mode
  with a test client as the presenter; `tst_screenshot` (bridge: mode,
  selection hand-off, cancel, and a round-trip against a fake
  `org.dragonfruit.Portal1`), `tst_screenshotui` (the view), and
  `tst_screenshotwriter` (save + clipboard) are the shell proof;
  `shell_protocol_conformance` proves the capture request is answered on the
  private protocol. T-13.7 adds the real frontend routing check.

### ScreenCast portal and source picker (T-13.4a)

`org.freedesktop.impl.portal.ScreenCast` (version 3) is the fifth concrete
interface and the third interactive one. Contract frozen in ADR
[0080](adr/0080-screencast-portal-and-source-picker.md). The live PipeWire
stream is deferred to T-13.4b; this slice returns the chosen *source handle*.

- **The session lifecycle is the pure model.** `portal::screencast` owns a
  `ScreenCastRegistry` of sessions and picker requests: `CreateSession`
  registers a session served at the caller's path by a standard
  `org.freedesktop.impl.portal.Session` object, `SelectSources` registers a
  picker request and awaits its one-shot completion, and `Start` returns the
  chosen sources as `streams` (`a(ua{sv})`). Version 3 is advertised so the
  `source_type` stream property is present; persistence (`persist_mode` /
  `restore_data`, v4) is deliberately absent, and virtual monitors (`4`) are
  not advertised.
- **The presenter seam is the diagnostic interface.** `ScreenCastOpened`
  carries the picker model (handle, session, app id, `types` bitmask,
  `multiple`, and the raw options, including `cursor_mode`), and
  `CompleteScreenCast(handle, a(su))` / `CancelScreenCast(handle)` /
  `PendingScreenCasts` are the presenter's half. The node id in each stream is
  a placeholder (`0`) with a Dragonfruit `id` property carrying the source
  handle until T-13.4b attaches the real PipeWire node.
- **The source picker is a shell overlay.** `Dragonfruit.Screenshot`/
  `ScreenCastPicker.qml` is a pure view — a centred card with Screens/Windows
  sections, per-row selection (checkbox when `multiple`, radio otherwise), a
  hint, and Cancel/Share. `ScreenCastBridge` (`shell/src/screencastbridge.{h,cpp}`,
  dockcore) watches `ScreenCastOpened`, publishes the request and the source
  list, and answers the portal; a missing portal is a supported state. The
  available sources come from the compositor's monitor/window projection
  (`ShellProtocol::screencastSources`), filtered by the request's `types` — the
  shell does not invent sources. `ShellController`/`ShellProtocol` add the
  centred `screencast` overlay surface; `DF_SCREENCAST_FIXTURE=<1|monitors|windows>`
  presents it for the live visual check with no portal.
- `portal/tests/screencast.rs` drives the whole session over a private bus with
  a test client as the presenter; `tst_screencast` (bridge: options,
  single/multi selection, the chosen-source hand-off, cancel, and a round-trip
  against a fake `org.dragonfruit.Portal1`) and `tst_screencastui` (the view)
  are the shell proof. T-13.7 adds the real frontend routing check.

## Clipboard (T-13.5a/T-13.5b)

The clipboard is not a portal: it is the compositor's standard `wl_data_device`
selection, delegated to Smithay, with `wlr-data-control` as the manager
surface. Contract in
[ADR 0082](adr/0082-clipboard-data-device-bridge-ownership.md) and
[ADR 0083](adr/0083-clipboard-history-store-and-observer.md).

- **One owner.** Ordinary clients set and read the clipboard through
  `wl_data_device`; the selection source is offered to the focused client's
  data device, and `set_selection` from a client without keyboard focus is
  denied. The Xwayland bridge (T-06) maps X11 selections onto the same
  selection, so copy/paste works across the boundary.
- **`wlr-data-control` is the manager half.** The global is advertised with an
  open filter; a manager sees and sets the selection without focus. The shell's
  clipboard history (T-13.5b) is an observer over this surface and forwards the
  source it observes rather than becoming a second owner.
- **MIME is opaque.** Text (`text/plain;charset=utf-8`), images (`image/png`),
  and file lists (`text/uri-list`) cross the offer pipe byte-for-byte; the
  bridge does not decode or re-encode. The shell's screenshot copy (T-13.3b)
  already offers image plus `file://` URI through Qt's clipboard, which is the
  same data device.
- `clipboard_round_trips_text_image_and_uri_list` in
  `compositor/tests/shell_protocol_conformance.rs` drives two independent
  clients through the whole matrix on the headless backend.
- **History (T-13.5b).** The shell binds the vendored
  `wlr-data-control-unstable-v1` client protocol, reads the offered
  `text/uri-list` / `image/*` / `text/plain*` payloads, and feeds the pure
  `ClipboardHistory` store (dockcore). The store classifies, deduplicates,
  caps (50 entries / 1 MiB each / 8 MiB total), honours pinned entries, refuses
  the common password-manager secret hints, clears the unpinned entries on lock
  (pinned survive), and serializes to a best-effort JSON file under the app's
  data directory. The Control Center's clipboard section renders the store and
  raises copy / pin / clear requests; copying an entry re-serves it through the
  data-control device — the one time the shell takes selection ownership.
  `tst_clipboardhistory` and the Control Center tests are the headless proof.

## polkit authentication agent (T-13.6)

Privileged operations are authorized by polkit; the shell hosts the
authentication agent so their prompts use one design-system dialog instead of a
toolkit default. The presentation is ours; the stack is the host's
([adr/0084](adr/0084-polkit-authentication-agent.md)). Nothing of ours runs as
root and no credential is verified by us.

- **The agent is a real polkit agent.** `PolkitAgent`
  (`shell/src/polkitagent.*`, dockcore) exports
  `org.freedesktop.PolicyKit1.AuthenticationAgent` at a Dragonfruit path on the
  **system bus** and registers it with `org.freedesktop.PolicyKit1` as the
  shell's session agent (a process subject when there is no session id, and an
  explicit seam for a dev/test session). Registration is asynchronous: an
  absent authority, or another agent already serving the subject, is a normal
  degraded state — the desktop keeps working and the dialog simply never opens.
- **The host helper owns PAM.** `PolkitHelperSession`
  (`shell/src/polkitsession.*`) spawns the distribution's
  `polkit-agent-helper-1` (or connects to `/run/polkit/agent-helper.socket` on
  setuid-free polkit hosts), writes the authority's cookie, relays the PAM
  `PAM_PROMPT_*`/`PAM_TEXT_INFO`/`PAM_ERROR_MSG` lines, and forwards the typed
  answer. The helper reports `SUCCESS`/`FAILURE` to the authority itself, so the
  shell never calls `AuthenticationAgentResponse` and never sees a credential
  decision. The response buffer is keyed by `ShellController` (the compositor
  delivers raw key codes, not text) and cleared as it is written.
- **The prompt is a shell overlay.** `Dragonfruit.Screenshot`/
  `PolkitDialog.qml` is a pure view — title, action message, identity,
  masked password field, an expandable details list, and Cancel/Authenticate —
  rendered into a centered `polkit` layer surface. `DF_POLKIT_FIXTURE` presents
  a synthetic request for the live capture. Cancel, Escape, lost keyboard, or a
  helper failure all resolve the request as failure: the agent is fail-closed.
- `tst_polkitagent` proves the helper conversation against fake helper scripts,
  the D-Bus agent against a fake authority on a private bus, a refused
  registration, the fixture, and — when the host runs polkit — a **real
  `pkcheck` request** that raises the agent. `tst_polkitui` covers the view.
  T-13.7 exercises it through a real Flatpak/browser walkthrough.

## Flatpak validation (T-13.7)

The portal set is proven against a real Flatpak caller, not only the
private-bus test presenter. `scripts/capture-portals.sh` starts a private
session bus, installs the three discoverability files, runs the backend,
`settingsd`, and the **real** `xdg-desktop-portal` frontend, then executes a
driver *inside* `flatpak run org.mozilla.firefox`. Every call therefore
crosses the sandbox D-Bus proxy and the frontend before it reaches the
backend. The artifacts are `docs/captures/t13-portals.txt` (the round-trips)
and `docs/captures/t13-portals.png` (the shell's FileChooser picker raised by
the Flatpak request). The live still runs with the private bus exported into
`make demo`, so the shell is the presenter just as in a real session; the
shell needs `settingsd` on that bus to finish startup.

Running against the real frontend found two wire-contract bugs the
private-bus tests could not (they call the backend directly and parse its own
files):

- **Comments in the data files are `#`, not `;`.** `.portal`/`*-portals.conf`
  are GLib key files.
- **The implementation `version` property is lower-case.** The standard
  spells `AvailableSourceTypes`/`AvailableCursorModes` in Pascal case but
  `version` in lower case. Exporting `Version` made the frontend read 0 and
  silently disable cursor-mode binding (which then rejected `SelectSources`
  with `cursor_mode`), the Screenshot `uri` result, and persistence.

Both are frozen in [adr/0085](adr/0085-real-frontend-portal-compatibility.md)
and pinned by
`the_standard_interfaces_expose_the_lowercase_version_property`
(`portal/tests/portals.rs`). With them fixed the frontend reports ScreenCast
version 3, `AvailableCursorModes` 3, and Screenshot version 2.

Clipboard has no portal (ADR 0082): a Flatpak app's clipboard is the Wayland
data device through the sandbox proxy, covered by the T-13.5a compositor
conformance rather than this walkthrough. The ScreenCast stream still reports
the named stills fallback (`df_stream_mode=stills`); a live PipeWire producer
extends `portal::stream::StreamTransport` without changing this path.
