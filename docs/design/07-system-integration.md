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
  `SetVolume(volume)`, `SetMute(muted)`, `ToggleMute()`.
- `org.dragonfruit.SystemStatus1.Battery` — `State()`/`Refresh()` only (the
  read-only battery view; no write).

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
- **Save/copy is T-13.3b.** The shell hands the selection to
  `captureRequested`; turning it into a saved image and calling
  `ScreenshotBridge::complete(uri)` is the next task, which also keeps the
  compositor-side capture portal-only.
- `portal/tests/screenshot.rs` drives the blocking round trip for each mode
  with a test client as the presenter; `tst_screenshot` (bridge: mode,
  selection hand-off, cancel, and a round-trip against a fake
  `org.dragonfruit.Portal1`) and `tst_screenshotui` (the view) are the shell
  proof. T-13.7 adds the real frontend routing check.
