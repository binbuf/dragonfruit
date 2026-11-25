# Stability and the daily-driver gate

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-22 |
| **Scope** | The correctness, safety, recovery, and hardware-wiring bar for a single-user daily-driver session |
| **Contract owner** | This document |
| **Executable plan** | [`docs/tasks/stability/`](../tasks/stability/README.md) (local, gitignored), tasks T304–T348 |
| **Decisions** | [ADR 0311](adr/0311-stability-daily-driver-gate.md), [ADR 0312](adr/0312-lock-loss-ends-the-session.md), [ADR 0313](adr/0313-idle-dpms-ownership.md), [ADR 0314](adr/0314-pointer-constraints-are-unprivileged.md), [ADR 0315](adr/0315-one-output-global-and-output-scoped-input.md), [ADR 0316](adr/0316-settings-consumer-conformance.md), [ADR 0317](adr/0317-lock-and-suspend-tear-down-client-grabs.md), [ADR 0318](adr/0318-focus-clearing-is-one-helper.md), [ADR 0319](adr/0319-teardown-focus-restore.md), [ADR 0320](adr/0320-launch-time-xwayland-display-handoff.md), [ADR 0321](adr/0321-asynchronous-clipboard-source-write.md), [ADR 0322](adr/0322-portal-request-liveness.md), [ADR 0323](adr/0323-cross-filesystem-move-atomicity.md), [ADR 0324](adr/0324-log-out-through-logind.md), [ADR 0325](adr/0325-shell-watchdog-and-restart-chord.md), [ADR 0326](adr/0326-per-output-dpms.md), [ADR 0327](adr/0327-shell-idle-effects.md), [ADR 0328](adr/0328-lock-before-sleep.md), [ADR 0329](adr/0329-panel-brightness-through-logind.md), [ADR 0330](adr/0330-pointer-settings-libinput-application.md), [ADR 0331](adr/0331-media-keys-input-action-steps.md), [ADR 0332](adr/0332-xwayland-disconnect-teardown.md), [ADR 0333](adr/0333-net-frame-extents.md), [ADR 0334](adr/0334-xdnd-bridge-as-target.md), [ADR 0335](adr/0335-xdnd-bridge-as-source.md), [ADR 0344](adr/0344-screencast-producer-process-boundary.md), [ADR 0345](adr/0345-screencast-bridge-and-portal-ownership.md), [ADR 0346](adr/0346-screencast-bridge-trusted-portal-client.md), [ADR 0347](adr/0347-dock-pin-initialization-marker.md), [ADR 0348](adr/0348-settings-default-browser-async.md), [ADR 0349](adr/0349-wallpaper-provider-and-luma-async.md), [ADR 0351](adr/0351-terminal-resolution-and-visible-failure.md), [ADR 0353](adr/0353-qt-wayland-platform-plugin-preflight.md) |

This document defines what **working daily driver** means for Dragonfruit. It
is deliberately narrower than the total-experience design
([22-total-experience.md](22-total-experience.md)): it does not ask for more
surfaces, more Settings rows, more material, or more animation. It asks that
the surfaces already shipped are safe, truthful, recoverable, and wired to the
hardware they claim to control.

The 2026-11-22 review found that the tree builds, the test suite is green, and
the feature breadth is real — but a small number of defects make a normal
workday unsafe or frustrating. The gap register in [STATUS.md](STATUS.md)
already names most of them; the difference here is that they are now the
release gate, not background polish.

## 1. The daily-driver contract

A daily driver is a session a person can use for a full workday without
dropping to a TTY, losing data, or leaking a password. The contract is:

| Area | The bar |
|---|---|
| **Lock** | No client receives keyboard or pointer input while locked, including through an active popup or drag grab. A dead lock UI never unlocks. A lost shell while locked ends the session safely; it never leaves a frozen desktop. |
| **Focus** | Every visible active window has keyboard focus. Every hidden, minimized, closing, or dead surface has none. Switching Spaces, minimizing, a client crash, and a shell crash all leave a recoverable focus state; clicking the window restores typing. |
| **Session** | Log Out, Sleep, Restart, and Shut Down all work. A hung shell is restarted automatically or by a documented chord. A compositor crash ends the session and returns the greeter. |
| **Idle and power** | The `idle.*` and `power.preventSleep` keys actually dim, blank, lock, and suspend. The display blanks (DPMS), the session locks before sleep (including lid close), and input wakes it. Inhibitors are honored. |
| **Hardware basics** | Brightness changes the panel. Tap-to-click, natural scrolling, pointer speed, left-handed, scroll method, secondary click, and double-click change libinput behavior. Volume/mute/brightness keys work and show the existing OSD. |
| **Launch** | X11 apps receive `DISPLAY`; Wayland apps receive `WAYLAND_DISPLAY`; `Terminal=true` entries open a terminal or show a visible failure. A missing handler is a message, not a silent no-op. |
| **Data** | A cross-filesystem move either completes or leaves the source untouched. Trash, rename, copy, and move report conflicts. No operation leaves a half-moved tree. |
| **Recovery** | A clipboard client that never reads cannot freeze the shell. A portal request cannot hang an app forever when the shell is gone or restarted. A Files desktop that loses the compositor exits and is restarted, not spin at 100% CPU. |
| **Multi-monitor** | One `wl_output` per output; unplug removes it; a failed connector init leaves no phantom output. Popups, touch, tablets, and absolute pointers are scoped to the output they are on. |
| **Settings truth** | Every key marked `stored_only: false` has a named consumer and a test that observes the effect. A key with no consumer is `stored_only: true` and the pane says so. |

These are not “polish” items. They are the difference between a demo and a
desktop.

## 2. Reference hardware and non-goals

The gate uses one reference laptop, not every host:

| Axis | Reference A (gate) | Stretch B (not a blocker) |
|---|---|---|
| GPU | Intel or AMD iGPU, single GPU | AMD/Intel dGPU, one display |
| Panel | 1920×1080 or 2560×1440, 60–144 Hz | 3840×2160, 144 Hz |
| Scale | 1× or 2×, matched across displays | mixed 1.5×/2× |
| Displays | Internal + one external (HDMI or DP) | three displays |
| Input | Touchpad, keyboard, mouse | touchscreen, tablet, controller |
| Host | Fedora (primary), one Debian/Ubuntu check | Arch |
| Session | `make native` or the packaged session | packaged Flatpak host |

Explicit non-goals for this gate, with the deferred owner in §7:

- NVIDIA proprietary, hybrid multi-GPU, rotated-panel sampled glass.
- HDR, VRR policy/UI, night light, color management.
- IME/CJK input and AT-SPI for shell chrome.
- Mixed-DPI per-output chrome; Stage/tiling; a search service.
- Controller remapping, printer drivers/job management, wallpaper rotation.
- Files column/gallery/preview, Open With for every MIME, and a full
  preferences app.

“Not perfect everywhere” means these are allowed to stay open. It does not
mean a lock screen that leaks keys, a file manager that loses a directory, or
a laptop that cannot dim its panel.

## 3. Invariants

### 3.1 Lock is fail-secure

The compositor owns the lock state. While `LockModel::locked` is true:

- keyboard focus is either the lock surface or `None`;
- pointer/touch/gesture input is routed only to the lock surface;
- no client can raise or focus a window;
- a dead lock surface drops input but never clears `locked`.

Clearing focus is not enough on its own: an active `xdg_popup` keyboard grab
ignores `set_focus` and forwards keys through the grab, and a move/resize/drag
pointer grab keeps consuming motion. `SessionLockHandler::lock` therefore
tears down every client grab (`keyboard.unset_grab`, `pointer.unset_grab`),
dismisses the active window's popups and any chrome popup, and closes the
compositor-owned window menu before it clears focus and before it confirms the
lock (T305, [ADR 0317](adr/0317-lock-and-suspend-tear-down-client-grabs.md)).
The same teardown runs in `suspend_session`, so a drag or popup grab does not
survive a sleep/resume. The lock surface is the only focus target when the
lock is confirmed, so the first key after the teardown reaches the lock UI and
no client sees it.

A shell crash while locked cannot recover in place: the session is locked with
no lock surface, and Smithay’s `locked_outputs` prevents a restarted shell
from re-creating one. The trusted shell disconnect while locked therefore ends
the session cleanly ([ADR 0312](adr/0312-lock-loss-ends-the-session.md)):
`df_core::destroyed` logs the reason and stops the compositor loop without
unlocking. The session entry returns to the greeter, and a fresh login starts
a fresh compositor and lock. Re-lock needs a separate lock process or a
Smithay change; that is a new decision, not a silent recovery.

Sleep is a lock trigger, not an unlock. The shell subscribes to logind’s
`PrepareForSleep` on the system bus. On `true` it calls the one `lockScreen()`
path and waits (bounded at 2 s) for the compositor’s `locked` confirmation
before the handler returns, so the lock is confirmed while the compositor can
still render; a duplicate `true` is ignored. The shell holds a logind
`sleep` delay inhibitor while awake and releases it after that confirmation
(or the 2 s bound expires), so logind’s own delay window is what keeps the
machine awake long enough to lock. On `false` the shell refreshes the lock scene and restores
DPMS; it never unlocks. A lid close is indistinguishable from a menu suspend
at this layer: both are `PrepareForSleep(true)`. If the shell is gone while
locked, the T306 policy above ends the session; the gate never unlocks on its
behalf (T318, [ADR 0328](adr/0328-lock-before-sleep.md)).

### 3.2 Focus is explicit

There is one keyboard focus owner. A surface that is not visible, is
minimized, is closing, has been destroyed, or belongs to a disconnected
client must not hold focus. Clearing focus must clear `active_window`, tell
the shell (`Unfocused`), and update the menu-bar/shortcut state in one place.

That one place is `DfState::clear_keyboard_focus(reason)` (T307,
[ADR 0318](adr/0318-focus-clearing-is-one-helper.md)). It unsets the seat
focus, clears the cached `active_window`, broadcasts `Unfocused` only when a
window was active, dismisses that window's popups, clears the shortcut
engine's focused app, clears `menu_bar_keyboard_focused`, re-applies the
menu-bar reveal rule, marks the scene dirty, and refreshes the following
magnifier. Close uses it, `after_workspace_change` uses it when the active
window is no longer on the active Space, and `minimize_window` uses it when
the active window is in the minimized transient tree. Minimize does not
implicitly focus a different window: focus becomes `None` and the user's next
click chooses the new owner.

`activate_window` must compare the keyboard’s actual current focus with the
target surface, not a cached `active_window`. Otherwise clicking the same
window after a Space round-trip is a no-op.

The shell is a trusted chrome client, not a window. If it disconnects while
holding focus, the compositor must restore focus to the active window after
removing the chrome layers, not before. A layer entry records its owning
client at creation, because `resource.client()` no longer resolves while a
client is disconnecting. `df_core::destroyed` captures whether the dying
client's layer held the keyboard before `forget_client`, removes the layers,
then restores window focus and recomputes the reserved zones (T308,
[ADR 0319](adr/0319-teardown-focus-restore.md)). A live client destroying a
single layer still restores from `DfLayerSurface::destroyed`.

Window teardown follows the same rule. `remove_window` calls
`clear_keyboard_focus("window removed")` when the removed window was active or
the seat still names its surface, so a client crash and a completed close
ghost both leave the keyboard `None` rather than on a destroyed `wl_surface`.
The synthetic `query focus` report carries a `keyboard
none|window <id>|chrome|other|dead` line beside the cached `focus` line;
`keyboard dead` is the invariant violation the conformance suites reject.

Pointer constraints follow the same focus rule (T322,
[ADR 0314](adr/0314-pointer-constraints-are-unprivileged.md)). A client's
`zwp_pointer_constraints_v1` lock or confine is scoped to its own focused
surface: any client may create one, a constraint created while the surface is
unfocused activates on pointer enter, and `DfState::focus_changed` releases an
active constraint when its window loses keyboard focus. The grab is
self-healing — a destroyed constraint or surface unsets it — so a client crash
cannot leave the pointer frozen. A constraint never confines input over
another application. Pointer constraints do not pass through the
`GrabArbiter`; the arbiter still gates raw keyboard and popup grabs, its
refusal record is bounded to 64 entries, and a disconnected client is pruned
from the sanctioned set. The conformance suite is
`compositor/tests/pointer_constraints_conformance.rs`.

### 3.3 Settings are truthful

The schema and the Settings map are a contract. A key marked
`stored_only: false` is a promise that the user’s change has an observable
effect. The 2026-11-22 review found live-marked keys with no consumer; the
stability set has since closed them:

- `idle.dim`, `idle.blank`, `idle.lock`, `idle.suspend`, `power.preventSleep`
  — the compositor-owned idle chain and the shell's logind path (T315/T317).
- `display.brightness` — the logind panel write (T319).
- `input.pointerSpeed`, `input.naturalScroll`, `input.tapToClick`,
  `input.leftHanded`, `input.scrollMethod`, `input.secondaryClick`,
  `input.scrollSpeed`, `input.doubleClickSpeed` — applied live by T320
  ([adr/0330](adr/0330-pointer-settings-libinput-application.md)). The
  libinput half runs on hotplug and on every policy change; scroll speed,
  secondary click, and double-click are compositor-side because libinput has
  no setter for them.
- `input.haptics` — `stored_only: true`; this device stack exposes no haptic
  control, and the pane says so.
- `input.keyboardBrightness`, `input.adjustBrightnessLowLight`,
  `input.backlightOffAfter` — `stored_only: true` with the deferred-hardware
  note; the keyboard-backlight bridge is T346's deferred register.

The fix is either a consumer or `stored_only: true` plus a visible note. There
is no third option. The `stability` set adds a consumer-conformance test that
fails when a live key has no entry in the verified-consumer table.

The Dock's pinned set has the same truthfulness problem from the other
direction: an empty list used to mean both "never seeded" and "the user removed
every pin", so clearing the Dock was undone at the next login. Revision 39 adds
`dock.pinsInitialized`, the only first-run signal. The shell seeds the installed
defaults once while it is false and writes it on the first seed or pin edit; the
settings migration marks it true whenever the persisted file already names
`dock.pinned` (including an empty list), so a user who emptied the Dock is never
re-pinned. `tst_dockcore` covers the decision and the settingsd tests cover the
migration.

The Mouse and Trackpad panes carry separate `input.mouse.*` /
`input.trackpad.*` snapshots over the v15
`df_toplevel_manager.set_pointer_family_policy` request. The compositor
classifies a pointer by capability (touch/gesture support means touchpad) and
applies each snapshot only to that kind, with a per-device name override first
and the shared `input.*` policy as the fallback. The non-per-device rows
(secondary click, scroll speed, double-click) write the shared keys because
the button/axis path cannot attribute an event to a device.

The behavioral half of the map gate is `scripts/settings-consumers.json`,
checked by `make check-settings-consumers` (part of `make verify-stability`).
The script parses every `stored_only: false` key from
`services/settingsd/src/schema.rs` and requires a table entry with a
`consumer`, a `test`, a `state` (`verified` or `known-gap`), and a `task`.
`stored_only: true` keys are not in the table. A `known-gap` entry names the
task that will remove it and fails once that task is marked landed in the
task-set roadmap; a `verified` entry names a test that observes the effect.
The 2026-11-22 baseline seeds the known gaps listed above. This table is the
behavioral complement to the label-only schema-mirror and Settings-map tests:
those prove the row and the key agree, while this gate proves a live key has a
real consumer behind it.

### 3.4 Recovery is bounded

No single component failure may require a TTY reboot:

- a clipboard client that never drains its pipe must not block the shell.
  Every stored entry is served through `ClipboardSourceWriter`
  (`shell/src/clipboardwriter.*`), which sets the fd non-blocking, writes one
  chunk per `QSocketNotifier` writable callback, resumes at the last offset on
  `EAGAIN`, and closes the fd on `EPIPE`, cancellation, or a 2 s deadline. The
  data-control `send` dispatch never loops on `write()`; a second request for
  the same MIME, a new selection, source cancellation, and protocol teardown
  all cancel the in-flight writer;
- a wallpaper change must not block the shell on `wallpaperd` or a
  full-resolution decode. `WallpaperProviderClient` reads `DefaultSource` /
  `BuiltinDefaultSource` with one coalesced `Properties.GetAll` bounded to 2 s;
  a missing or slow provider keeps the last resolved values. The menu-bar luma
  decodes a scaled thumbnail on a worker thread and caches it by source +
  mtime + size + fallback colour, so a 4K change never freezes the GUI thread
  and a Space switch is a cache hit
  ([ADR 0349](adr/0349-wallpaper-provider-and-luma-async.md), T338);
- a Files desktop that loses its compositor connection must exit instead of
  spinning. The dispatch fd stays readable after the compositor goes away, so
  `DesktopLifecycle` disables the `QSocketNotifier` and exits non-zero;
  `dragonfruit-files-desktop.service`'s `Restart=always` then starts a fresh
  process against the reissued launch token. The same object makes a second
  `background` layer request a no-op, so the process owns exactly one
  ([ADR 0350](adr/0350-files-desktop-exit-and-single-layer.md), T339);
- a Settings pane must not block its GUI thread on app-index or a default-apps
  subprocess. The Desktop & Dock default-browser row reads app-index
  `Enumerate` through `QDBusPendingCallWatcher` (5 s bound) and runs
  `xdg-settings` through `QProcess` signals with a 1.5 s read / 2 s write
  timeout; `waitForFinished` never runs on the GUI thread. The row exposes
  `defaultBrowserLoading`, keeps its last known choices/value while a read is
  in flight, and reports a rejected write through `defaultBrowserError`. A
  missing app-index empties the choices; a failed or timed-out delegate keeps
  the last known host default. Neither is a spinner: the row is disabled with
  a reason and never waits forever
  ([ADR 0348](adr/0348-settings-default-browser-async.md), T337);
- a portal request must time out or cancel when the shell disappears, and a
  restarted shell must query pending requests. Every interactive completion
  (`FileChooser`, `Screenshot`, `ScreenCast.SelectSources`) is bounded by a
  30 s deadline; expiry removes the pending entry and answers the standard
  `cancelled` code. The shell owns `org.dragonfruit.PortalPresenter1` for its
  lifetime; the backend watches that name and cancels every pending request
  when its owner disappears or is replaced. The bridges answer `Cancel*` for
  the old handle before a second request replaces it, and each bridge queries
  its `Pending*` list at startup and cancels anything a previous shell left
  behind (a stale row carries no options, so re-presenting it would answer
  wrongly). A second screenshot/screencast request cancels the first; chooser
  requests still queue. ADR [0322](adr/0322-portal-request-liveness.md)
  records the decision;
- a Files desktop that loses the compositor must exit and be restarted;
- a hung shell is caught by a GUI-thread watchdog and can be restarted by a
  documented chord. `dragonfruit-shell.service` is `Type=notify` with
  `WatchdogSec=30`; `ShellWatchdog` sends `READY=1` and a `WATCHDOG=1`
  heartbeat every 10 s from a `QTimer` on the GUI thread, so a GUI-thread
  deadlock stops the beat and systemd restarts the process. `Ctrl+Alt+Esc`
  resolves to `InputAction::RestartShell`; the compositor closes the trusted
  shell connection itself (so the chord works even when the GUI thread is
  hung), the disconnect path reissues the one-time launch token, and
  `Restart=always` starts a fresh shell. The exact chord is the only key
  taken; a shell loss while locked still ends the session safely
  ([ADR 0312](adr/0312-lock-loss-ends-the-session.md), T314,
  [ADR 0325](adr/0325-shell-watchdog-and-restart-chord.md));
- Log Out must end the session through logind, never a direct signal to the
  shell or compositor. The system menu row and `Super+Shift+Q` share the
  Restart/Shut Down confirmation, then call
  `Manager.TerminateSession($XDG_SESSION_ID)`; the compositor anchor exits and
  the T-12.2 session entry stops the target and returns the greeter. A host
  without logind falls back to `systemctl --user stop
  dragonfruit-session.target`, and only a host with neither path disables the
  row with a reason. ADR [0324](adr/0324-log-out-through-logind.md) records
  the decision;
- a `df_output.set_mode` with a zero, negative, or absurd refresh must not
  panic the compositor. The compositor's `valid_mode` helper rejects a zero
  size, a non-positive refresh, and a refresh above 1000 Hz (millihertz on the
  wire); the request is still acked, and the ack reports the live mode, not
  the rejected one. The DRM and nested frame paths defensively filter a
  non-positive `current_mode().refresh` and use the 60 Hz fallback period
  instead of dividing by it, so a persisted mode or a backend bug cannot reach
  `Duration::from_secs_f64(inf)` (T325);
- an Xwayland crash must remove its windows from the model, not leave ghosts.
  The X11 WM's `disconnected` (and `XWaylandEvent::Error` when no WM is live)
  calls `remove_all_x11_windows`, which routes every tracked X11 window through
  `remove_window` (so the shell receives one `Unmapped` per window), releases
  the keyboard from a dead X11 surface, unmaps and clears the override-redirect
  map, and drops the entries from the model, scene, and workspaces. When a WM
  is live, the restart is scheduled from `disconnected`, which is delivered
  only after the old WM's event channel has drained; restarting on `Error`
  would let a queued association or configure event mutate the new server's
  windows. Because Smithay's `xwayland_shell` state never clears its serial →
  `wl_surface` map, the global is recreated on restart so a new surface cannot
  be associated with the dead one. With both the model entry and the override
  map gone, a reused X11 window id maps as a new `Window` and takes focus
  normally (T326). [ADR 0332](adr/0332-xwayland-disconnect-teardown.md) records
  the ordering decision.
- a managed X11 window must tell clients where the compositor's titlebar is.
  `_NET_FRAME_EXTENTS` publishes `(0, 0, titlebar_height, 0)` for an SSD
  window and zero for a CSD window, is written before the frame map, and is
  republished on decoration-tier, chrome-style, and window-state changes. X11
  pixels are logical points at every integer scale, so the output scale is
  not applied twice (T327,
  [ADR 0333](adr/0333-net-frame-extents.md));
- a screen share must not stall the desktop or leak a node. The live
  producer (`compositor/src/screencast_producer.rs`) hands each frame to a
  disposable host tool, `dragonfruit-screencast-producer`, over a Unix socket;
  the tool owns the `Video/Source` node and the frame pump, and a dead or hung
  tool costs one stream, never the session. Frames come from the same
  offscreen capture pass the screenshot uses (no second render, no
  screencopy), a stream is cropped to its creation geometry, offered at most
  once per `1 / fps`, and an unchanged picture is not re-sent. A missing
  PipeWire host, tool, or node is the named `pipewire-producer-unavailable`
  fallback, and a stream is released rather than left half live (T335a,
  [ADR 0344](adr/0344-screencast-producer-process-boundary.md); the live check
  is `make t335a-screencast`).
- a live screen-share stream is owned by its portal session and its compositor
  client, and by nothing else. `Start` creates one node per chosen source and
  the session holds it: a repeated `Start` releases the previous set first,
  `Session.Close` (a client disconnect or app exit) releases that session's
  nodes, and the presenter-loss watch releases every live stream when the
  trusted shell leaves — a caller never leaves a producer running. The
  compositor mirrors the rule on its side: v17 `create_screencast` answers
  `screencast_node` (the node id, output, geometry) or `screencast_failed`
  with the portal's fallback label, never a placeholder id, and a client's
  manager object or `df_core` disconnect, or compositor teardown, releases
  every stream that client created. A stream refusal is per-stream and named
  (`df_stream_mode = stills`, `df_fallback =
  pipewire-producer-unavailable`); the picker says the app will receive still
  images. The live half is the portal's own trusted client: the backend holds
  the portal role's one-time token (`<socket>.portal-launch-token`), opens its
  own `df_toplevel_manager` v17 connection, and creates the stream; one
  dedicated thread owns that connection, a reopen is verified before the picker
  is told `pipewire`, and the compositor releases the portal's streams when the
  connection ends. A presented token is bound to an existing mint, never
  adopted, so a restarted session cannot accidentally trust an old value.
  Captured pixels still come from the nested/DRM capture pass, so the client's
  frames need a session that renders; `make t335b-screencast-client` proves the
  transport and the graph node on the headless rail and records the nested
  frame run (T335b,
  [ADR 0345](adr/0345-screencast-bridge-and-portal-ownership.md),
  [ADR 0346](adr/0346-screencast-bridge-trusted-portal-client.md)).

Clipboard serving is the bounded write in that list. The writer holds the
payload's implicit share for the life of the transfer, so no per-write copy is
made. The deadline is a safety net for a client that neither reads nor closes
its end; a client that closes its read end gets `EPIPE` immediately and does
not take the shell down (SIGPIPE is blocked around the write and ignored by
the shell process). ADR 0321 records the decision.

### 3.5 Data operations are all-or-nothing

A move within one filesystem is a single `rename(2)`: the destination either
appears complete or the source stays where it was. Across filesystems
`rename(2)` reports `EXDEV`, so the shell copies the entry into a hidden temp
sibling in the destination folder, renames that temp into the requested name,
and only then removes the source. The temp is on the destination filesystem
by construction, so the second rename is atomic and never crosses a mount.

The source is never touched until the destination rename succeeds. Any copy
or rename error removes the partial temp (including read-only directories
left by metadata preservation) and leaves the source complete. The
destination name comes from the existing generated-name helper, so a retry
after a failure reuses the original name and never creates `name.1` beside a
half-moved `name`; a broken symlink counts as occupied.

Permissions and modification times are copied where the filesystem supports
them. Metadata loss is best-effort and does not fail an otherwise complete
move. Symlinks are copied as links, not followed. If source removal fails
after the destination rename, the destination is already complete: the move
stays successful and the leftover source is logged rather than deleting good
data. A directory is never moved into its own subtree. The synchronous core
and the Dock's asynchronous owner share this path; ADR
[0323](adr/0323-cross-filesystem-move-atomicity.md) records the decision.
T330 ports the same rule into `files-core` (`StdFsOps::move_as`), so Files
copy/cut/paste and the Dock drop path are two implementations of one decision,
not two policies; ADR [0337](adr/0337-files-clipboard-paste-and-conflicts.md)
records the Files side.

### 3.6 Multi-monitor is per-output

Each output has exactly one `wl_output` global, removed on unplug.
`backend::add_output` creates that global and returns the `GlobalId`; the DRM
backend stores the exact handle in `SurfaceData` and removes it when the output
disconnects. There is no second `create_global` on the connector path, and
nested/headless create their one global at startup. A failed connector init is
transactional: after `add_output`, a plane-query or DRM-output-init failure
calls `backend::remove_output`, which migrates and re-homes the output's
windows, clears its Spaces from the workspace, unmaps it from the scene, and
removes the global. The client-visible globals and the workspace are then
exactly as they were before the connector appeared. The VRR probe is
deliberately non-fatal (an unsupported/refused property keeps the fixed
period), so it does not discard a working output. The synthetic-output harness
drives the same add/remove path in the headless tests; the real hotplug proof
stays on the hardware rail.

Popups are constrained to the parent window's output. `set_popup_geometry`
resolves the output containing the parent window's centre (the
protocol-correct anchor — focus can move while a popup is open), falling back
to the focused output and then the primary, and passes that geometry to
`constrained_popup_geometry`. A menu opened on the second monitor therefore
flips and slides against that monitor.

Absolute pointer, touch, and tablet events are mapped to the output under the
device/pointer. The device's normalized position is mapped across the
output-layout bounding box; the output containing that point is the authority,
the output under the current pointer is the fallback when the layout has a
gap, and the nearest output is the last resort. The position is then
interpreted within that output's geometry. This is the
`output_input::absolute_output` rule shared by the router and the headless
tests. A single live lock surface is always the locked absolute target even
when the pointer rests on another display; with several lock surfaces the
event position, then the pointer, selects one. The decision is frozen in
[ADR 0315](adr/0315-one-output-global-and-output-scoped-input.md).

A per-device `map-to-output` override is not part of this task: a device is
scoped by the position it reports. A real touchscreen whose axis range is
local to one display therefore follows the layout-mapping default (the same
default wlroots uses) until the input-settings follow-up adds an explicit
mapping.

### 3.7 Launch is complete

The launch environment contains the display the app should use:

- `WAYLAND_DISPLAY` from the compositor socket;
- `DISPLAY` from `$XDG_RUNTIME_DIR/<socket>.x11-display` when present, and
  removed when absent or empty;
- `QT_QPA_PLATFORM=wayland` for Qt apps when the shell forced offscreen;
- no `QT_QPA_PLATFORM=offscreen` leak.

The shell's own Qt-side GL context (not a launched app) needs the Qt 6 Wayland
platform plugin when the GPU chrome rail is requested. `shell/src/main.cpp`
checks for the plugin before `QGuiApplication` and falls back to the offscreen
QPA with a warning when it is absent; the fallback is a diagnostic, not a
supported long-term mode. `scripts/check-native-deps.sh` is the read-only,
distro-aware preflight (it never installs packages), and the native session
runs it before arming a login. The decision is frozen in
[ADR 0353](adr/0353-qt-wayland-platform-plugin-preflight.md).

Xwayland starts after the session environment is exported, so `DISPLAY` is
not a startup value. The compositor writes the hand-off file when Xwayland is
ready; the shell, Files, and Settings read it at each launch, never once at
startup, so a late Xwayland start still reaches the next child. The shared
`Dragonfruit::applyX11DisplayHandoff` helper trims the file, sets `DISPLAY`,
and removes an inherited value when the file is missing or empty. The decision
is frozen in [ADR 0320](adr/0320-launch-time-xwayland-display-handoff.md).

A `Terminal=true` entry resolves `$TERMINAL`, `xdg-terminal-exec`,
`x-terminal-emulator`, then ptyxis / gnome-terminal / konsole / alacritty /
kitty / foot / xterm. `$TERMINAL` may carry its own arguments and is used only
when its program is executable; a stale value falls through. `xdg-terminal-exec`
owns the terminal choice and receives the command after `--`; the other
candidates get their own execute separator (`-e` or `--`). If no terminal
exists the launcher returns no command — never the unwrapped app — and the Dock
raises its launch-failure notification. The default Dock pins include a
Terminal tile only when a terminal resolves, and the DEB/RPM/Arch metadata
recommends one. The decision is frozen in
[ADR 0351](adr/0351-terminal-resolution-and-visible-failure.md).

The app index parses `OnlyShowIn`, `NotShowIn`, and `TryExec`. `Enumerate` —
the catalog behind the Applications drawer, the Add Application picker, and
the Dock's default pins — omits an entry whose non-empty `OnlyShowIn` does not
name a desktop in `XDG_CURRENT_DESKTOP` (the session exports `dragonfruit`),
whose `NotShowIn` names one, or whose non-empty `TryExec` does not resolve as
an executable (directly when absolute, otherwise on `PATH`). `Resolve` and
`Lookup` keep the full identity corpus, so a running app still resolves when
its launcher entry is hidden. `NoDisplay`/`Hidden` stay in the catalog with
their flag and the shell's launcher filter still removes them. With no
`XDG_CURRENT_DESKTOP`, `OnlyShowIn` entries hide and `NotShowIn` entries stay
visible. The decision is frozen in
[ADR 0352](adr/0352-app-index-desktop-visibility-filters.md).

X11 launch is also a data path in both directions. The compositor's XDnD
bridge owns a dedicated X11 connection, publishes an `XdndAware` bridge window
as Xwayland's root `XdndProxy`, and reads the `text/uri-list` selection (direct
and `INCR`) when an X11 drag lands on a Wayland surface. The drop is delivered
as a compositor-owned `text/uri-list` data offer to the surface under the
pointer; only Copy is accepted, so a client that declines cannot lose the
source. The reverse direction reuses the same connection, atom table, and
bridge window: a Wayland drag source is driven through
`XdndEnter`/`Position`/`Drop`/`Leave`, waits for `XdndStatus` before dropping,
and serves `text/uri-list` to the X11 target from the Wayland client's data
source. An XdndAware X11 window under the pointer still receives its own drag.
The decisions are frozen in [ADR 0334](adr/0334-xdnd-bridge-as-target.md) and
[ADR 0335](adr/0335-xdnd-bridge-as-source.md).

### 3.8 Idle and power are real

The compositor owns the live inactivity timer and DPMS because it sees every
input event and owns DRM. The shell owns brightness, the lock UI, and logind
suspend because it already owns those paths. The `idle.*` policy is forwarded
to the compositor; the compositor emits `idle-dim`, `idle-blank`,
`idle-lock`, `idle-suspend`, and `idle-awake` actions. The shell applies them.
`idle-inhibit` surfaces suppress the chain. `PrepareForSleep(true)` locks
before sleep.

The pure `services/session/src/idle.rs` engine remains the unit-test oracle
for the stage rules; ADR 0313 records why the live timer is in the compositor
for this gate.

T315 lands the compositor half as `compositor/src/idle.rs`: a clock-injected
`IdleTimer` that normalizes the four delays exactly as the session oracle does,
tracks `last_activity` in `notify_activity`, and is frozen by an
`idle-inhibit` surface or the suspended state. `DfState` arms one calloop timer
at the next enabled deadline and emits the stable `idle-dim`, `idle-blank`,
`idle-lock`, `idle-suspend`, and `idle-awake` strings through the existing
`input_action` event. A late tick emits every crossed stage in order, so the
shell still dims, blanks, and locks before it suspends. A disabled stage (`0`)
is skipped and does not constrain a later stage. T315 ships the request and the
compositor timer; the compositor applies its shipped defaults until the shell
sends a policy. Older compositors ignore the additive v14 request.

T316 lands the panel-power half as a per-output connector property
([ADR 0326](adr/0326-per-output-dpms.md)). `SurfaceData` keeps the connector
handle from `connector_connected`, and `df_output.set_dpms` (v5) resolves one
output and writes that connector's `DPMS` property with `SETPROPERTY`; it never
uses `DrmDevice::activate(false)`, so blanking one monitor cannot disable every
connector on the device. The idle chain applies the stage on the compositor
side: `idle-blank` blanks every on output, while `idle-awake` and
`resume_session` wake every off output and request a repaint. A blanked output
is render-gated because an atomic commit would set the CRTC active and turn the
panel back on; `frame_finish` also reapplies DPMS off once for a frame that was
queued just before the blank. The wake path re-reads the kernel CRTC state into
the `DrmCompositor` before the repaint: the connector write bypasses the
compositor, and without that resync the first wake frame is submitted as a page
flip against an inactive CRTC and fails with `EINVAL` (the black-on-wake
regression seen in a native session). Nested and headless keep the state and
report a named fallback through `df_output.dpms`
(`none|nested|headless|failed`); a failed DRM write is logged, leaves the
previous state live, and never ends the session. The actual panel-off remains
hardware-open for the T347 DRM pass; headless conformance proves the
request/ack and the fallback.

T317 lands the shell effects. `shell/src/idlepolicy.*` reads `idle.dim`,
`idle.blank`, `idle.lock`, `idle.suspend`, and `power.preventSleep` from
settingsd and forwards one normalized `df_toplevel_manager.set_idle_policy`
snapshot on startup and on every change; `power.preventSleep` zeroes the
suspend stage so it never arms. The controller maps the compositor's actions:
`idle-dim` maps a full-output translucent `overlay` layer surface (namespace
`idle-dim`, empty input region, no keyboard) and drops the forwarded brightness
to half until T319 owns the panel backlight; `idle-blank` calls
`df_output.set_dpms(false)` and remembers the pre-blank state; `idle-lock` calls
the existing `lockScreen()`; `idle-suspend` calls the existing logind `Suspend`;
`idle-awake` restores the pre-dim brightness and the pre-blank DPMS state.
Disabled stages (`0`) do nothing, repeated stages are idempotent, and no new
HUD or OSD is introduced. `tst_idle` drives the pure controller with a mock
protocol/logind sink, and the settings-consumer table marks all five keys
verified.

The compositor deliberately duplicates the normalized stage rules rather than
adding a dependency edge to `dragonfruit-session`. `idle_policy_conformance`
drives the duplicate with a fake clock (the four stages, skipped stages,
inhibitor suppression, a live policy change, and a wake), and the session
engine's own tests stay the oracle. If the rules grow beyond this shape, the
shared crate ADR 0313 mentions is the extraction point.

T318 lands the sleep hook. `shell/src/suspendlock.*` subscribes to
`org.freedesktop.login1.Manager.PrepareForSleep` on the system bus and holds a
logind `sleep` delay inhibitor while awake. On `PrepareForSleep(true)` it calls
the existing `lockScreen()` once, pumps the GUI loop until the compositor
reports the lock (bounded at 2 s), then releases the inhibitor so logind may
sleep; a duplicate `true` is ignored and a missing lock path is logged rather
than blocking. On `PrepareForSleep(false)` it refreshes the lock scene’s clock
and every configured lock surface, wakes DPMS with `df_output.set_dpms(true)`,
and re-takes the inhibitor for the next cycle. It never unlocks: a wake cannot
clear the lock, and a shell that disappears while locked is still the T306
case. `tst_locksuspend` drives a fake logind signal on a private bus and
asserts one lock request, confirmation before the handler returns, one
inhibitor release, duplicate idempotence, and a wake that only refreshes and
wakes DPMS. `services/session/tests/suspend_resume.rs` proves the cycle accepts
the same external hook without an idle-chain request. The decision is frozen in
[ADR 0328](adr/0328-lock-before-sleep.md).

T319 lands the panel-brightness half. `libs/settings-client/backlight.*` is the
one device resolver: it picks a `raw` device over `platform` over `firmware`
under `/sys/class/backlight`, reads `max_brightness`, and maps the 0..1 setting
onto the device range. `shell/src/brightnessbackend.*` calls logind’s
`Session.SetBrightness("backlight", name, value)` asynchronously with a short
timeout, so a slow or absent logind never blocks the GUI thread. The shell
forwards the same effective level it already sends the compositor (including
the battery and idle-dim factors); the compositor keeps the value for state and
the OSD, while the backend is the physical write. When logind is absent the
backend writes the sysfs `brightness` file only if it is writable; otherwise it
logs and reports a visible absence reason instead of claiming success. With a
real backlight the idle dim stage is the panel write; the T317 translucent
overlay remains only as the nested/headless fallback, so the screen is never
dimmed twice. `tst_brightness` drives a fake logind session and a temporary
sysfs tree and asserts the preferred device, the mapped value, the sysfs
fallback, the absent no-op, and the non-blocking dispatch. The Displays pane
shows a “no backlight” note from the same resolver. The decision is frozen in
[ADR 0329](adr/0329-panel-brightness-through-logind.md).

T321 lands the media keys. The five XF86 volume/mute/brightness keysyms are
bound in the compositor's default system table with no modifier chord, so they
resolve before the focused client and travel over the existing `input_action`
event as `volume-up`, `volume-down`, `mute`, `brightness-up`, and
`brightness-down`. The pure `shell/src/mediakeys.*` policy maps one action to
one write: volume and brightness move by 5% and clamp to `[0, 1]`, mute toggles
without changing the level, and the volume keys preserve the mute state. The
existing audio client and settingsd `display.brightness` path perform the
write, and the existing `showOsd` presents the result; there is no second audio,
brightness, or OSD state. The shortcut filter already ignores auto-repeat of an
intercepted key, so holding a media key does not ramp: one press is one step.
An absent audio view or panel writer is a silent no-op, never a window over
nothing. `input::shortcuts`, `tst_mediakeys`, and `tst_osd` cover the binding,
the step/absence policy, and the presentation. The reference-keyboard check is
hardware-open in the T347 report. The decision is frozen in
[ADR 0331](adr/0331-media-keys-input-action-steps.md).

## 4. Defect register and task map

Severity is product risk, not effort. P0 items are the release blockers; P1
items are the first-week friction; P2 items are deferred in §7.

### P0 — do not dogfood without these

| ID | Defect | Task |
|---|---|---|
| S-01 | Lock did not clear keyboard/pointer grabs; a popup could receive lock-screen keys (closed by T305) | T305 |
| S-02 | A shell crash while locked leaves a frozen, inputless session | T306 |
| S-03 | Focus is lost after a Space round-trip, minimize, client crash, or shell crash | T307, T308 |
| S-04 | No idle/DPMS/auto-lock/suspend; no lock on lid close | T315–T318 |
| S-05 | X11 apps launch without `DISPLAY` | T309 |
| S-06 | Mouse/trackpad settings never reach libinput | T320 |
| S-07 | Brightness never reaches the panel (closed by T319) | T319 |
| S-08 | Log Out terminates the session through logind (closed by T313) | T313 |
| S-09 | A hung shell is never recovered (closed by T314) | T314 |
| S-10 | Clipboard-history serving can block the shell forever (closed by T310) | T310 |
| S-11 | Portal requests hang when the shell restarts or a second request arrives | T311 |
| S-12 | Cross-filesystem directory moves can partially move and report failure (closed by T312) | T312 |

### P1 — first-week friction

| ID | Defect | Task |
|---|---|---|
| S-13 | Media keys do nothing (closed by T321) | T321 |
| S-14 | The titlebar window menu is unlabeled (closed by T329; the trusted shell renders the compositor-owned model) | T329 |
| S-15 | Files lacked copy/move/paste, undo, watcher, tabs, New Window, Open With (closed by T330–T334; Open With and the native menu landed in T334) | T330–T334 |
| S-16 | Screen sharing is stills-only | T335a, T335b |
| S-17 | X11 `_NET_FRAME_EXTENTS` is missing (closed by T327); XDnD is live in both directions (target role T328a, source role T328b) | T327, T328a, T328b |
| S-18 | Pointer constraints are refused for games/remote desktop; refusals grow unbounded (closed by T322) | T322 |
| S-19 | DRM outputs get two `wl_output` globals; a failed init leaves a phantom output (closed by T323) | T323 |
| S-20 | Popups and absolute/touch/tablet input are hard-coded to the first output | T324 |
| S-21 | `df_output.set_mode` accepts a zero/negative refresh and panics the compositor (closed by T325) | T325 |
| S-22 | Xwayland restart leaves ghost windows and can lose new X11 windows (closed by T326) | T326 |
| S-23 | Unpinning every Dock app is overwritten with defaults on next login (closed by T336) | T336 |
| S-24 | Settings Desktop & Dock and wallpaper paths block the GUI thread | T337, T338 |
| S-25 | Files desktop spins on compositor loss and creates its background layer twice | T339 |
| S-26 | No terminal fallback; `Terminal=true` entries fail on Fedora | T340 |
| S-27 | App index ignores `OnlyShowIn`/`NotShowIn`/`TryExec` | T341 |
| S-28 | Native bring-up has dependency, transaction, DM-selection, stale-target, and `XCURSOR_PATH` traps | T342–T344 |
| S-29 | Session supervisor can respawn every 50 ms and can kill a reused PGID | T345 |

## 5. Acceptance matrix

### 5.1 Headless/nested (CI)

- `make verify-stability` (T304) runs the compositor, session, portal,
  app-index, files-core, and shell/QML suites that cover the contract.
- Every P0/P1 task adds a deterministic test named in its `## Done when`.
- A headless run proves protocol/state behavior; it does not prove a panel
  turned off, a touchpad tapped, or a lid closed.

### 5.2 Native (reference A)

T347 runs the matrix on the reference laptop and writes a report with one row
per check and a PASS/FAIL/`OPEN` verdict:

| Check | Pass condition |
|---|---|
| Cold login | Usable desktop in < 5 s; no error dialog; chrome and wallpaper present |
| Workday | 8 hours of normal use; no shell/compositor restart; no stuck focus; no leaked sockets |
| Idle soak | 24 hours; dim → blank → lock → suspend at the configured stages; input wakes; no runaway CPU |
| Suspend/resume | 20 cycles; session survives; lock is shown on wake; Wi-Fi/Bluetooth/audio recover |
| Lid close | 20 cycles; locks before sleep; external display resumes |
| Lock/unlock | 50 cycles; PAM; no key leak to a client; shell restart while locked ends the session safely |
| Shell kill | 20 unlocked kills → chrome back < 2 s; 20 locked kills → session ends safely, never frozen; a blocked GUI thread is watchdog-killed and restarted |
| Hotplug | 20 plug/unplug cycles; one output each; no phantom; popups/touch on the right output |
| Launch | X11 app, Wayland app, Flatpak file dialog, screenshot, screen share fallback |
| Data | Cross-filesystem move; interrupt; source intact; retry clean |
| Battery | Drain measurement on the reference laptop; no idle wakeups above the existing budget |
| Journal | No panics, no GPU errors, no repeated service restarts, no leaked processes |

### 5.3 Human

T348 batches the human verdict. An agent task is not human-signed-off. The
verdict records the hardware profile, the PASS/`OPEN` rows, and the known
limits. If the matrix is not PASS, the README’s “not yet a daily driver” note
stays and the blocker is named.

## 6. Release gate and status transition

The order is fixed:

1. `make verify-stability` is green for every task.
2. `make gates` is green on the final tree.
3. T347 has a PASS or an explicit `OPEN` with a command for every row.
4. T348 records the human verdict and updates `STATUS.md`, `14-risks.md`,
   `README.md`, and the task set progress.

The README status line changes only at step 4. A green `make gates` alone is
not a daily-driver claim; it is necessary but not sufficient.

## 7. Deferred register

These are explicitly out of this gate. T346 records each with a reason, user
impact, owner, and promotion trigger; this table is the summary.

| Deferred | Why | Promote when |
|---|---|---|
| IME / CJK input (G-20) | No first-party IME process; a host IME integration is a design decision | A CJK user is in the reference set, or a packaged distro requires it |
| AT-SPI for shell chrome (G-19) | Offscreen QPA has no accessibility backend | A screen-reader user is in the reference set |
| Night light, HDR, VRR UI, color management (G-13) | No compositor/colord half | A reference display requires it |
| Multi-GPU/rotated sampled glass (G-08) | Falls back to feather; visual only | The reference hardware has a hybrid GPU or a rotated panel |
| Direct scanout/cursor/overlay evidence (G-03) | Native performance gate, not a daily-use blocker | The T347 latency row fails |
| Mixed-DPI per-output chrome (G-02) | One shell `m_outputScale`; a significant refactor | The reference setup includes 1× + 2× displays |
| Search service, Stage/tiling | Design-only; the drawer search covers the basic need | A user workflow requires it |
| Controller remap, printer drivers/job manager | Out of the first-party app bar | A reference user requires it |
| Wallpaper rotation, Files column/gallery/preview | Files completeness beyond copy/move/paste/undo | Files P1 tasks are done and a user asks |
| Keyboard backlight, auto-brightness, audio balance, smart zoom | Stored-only or hardware bridge pending | The hardware exposes it and the reference user needs it |
| Tiling/Stage, app-level search | Not part of the 30-second loop | After the daily-driver gate |

## 8. Keeping this current

- Every landed stability task updates its row in §4 and the matching
  `STATUS.md` gap row. A task that changes a contract sentence updates §3.
- A new P0/P1 defect gets a task id and a row before it gets a fix.
- A deferred item moves out of §7 only with a task, a test, and a hardware
  evidence row.
- The task set is local and gitignored; this document, the ADRs, and the
  design index are tracked. Do not force-add the task set.
