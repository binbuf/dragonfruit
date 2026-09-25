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
