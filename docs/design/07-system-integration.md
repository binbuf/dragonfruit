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
truth. Routing and device switching are deferred to T-15.

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
There is **no write**: the battery item is read-only (power profiles via
`power-profiles-daemon` are deferred to T-15).

Being read-only, the item is hidden in two different ways. UPower itself being
absent is the adapter's `AdapterState::Unavailable`. A machine that runs UPower
but has no present battery (a desktop, a VM) still answers `Available`, with
`PowerSnapshot::present()` false — the consumer hides the item then. A battery
that is present but whose `IsPresent` is false is treated the same as no
battery. Nothing blocks session startup when either is missing.

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
