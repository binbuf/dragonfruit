# Stability and the daily-driver gate

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-22 |
| **Scope** | The correctness, safety, recovery, and hardware-wiring bar for a single-user daily-driver session |
| **Contract owner** | This document |
| **Executable plan** | [`docs/tasks/stability/`](../tasks/stability/README.md) (local, gitignored), tasks T304–T348 |
| **Decisions** | [ADR 0311](adr/0311-stability-daily-driver-gate.md), [ADR 0312](adr/0312-lock-loss-ends-the-session.md), [ADR 0313](adr/0313-idle-dpms-ownership.md), [ADR 0314](adr/0314-pointer-constraints-are-unprivileged.md), [ADR 0315](adr/0315-one-output-global-and-output-scoped-input.md), [ADR 0316](adr/0316-settings-consumer-conformance.md), [ADR 0317](adr/0317-lock-and-suspend-tear-down-client-grabs.md), [ADR 0318](adr/0318-focus-clearing-is-one-helper.md), [ADR 0319](adr/0319-teardown-focus-restore.md), [ADR 0320](adr/0320-launch-time-xwayland-display-handoff.md) |

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

### 3.3 Settings are truthful

The schema and the Settings map are a contract. A key marked
`stored_only: false` is a promise that the user’s change has an observable
effect. The 2026-11-22 review found live-marked keys with no consumer:

- `idle.dim`, `idle.blank`, `idle.lock`, `idle.suspend`, `power.preventSleep`
  (no production idle service);
- `display.brightness` (stored, never written to the panel);
- `input.pointerSpeed`, `input.naturalScroll`, `input.tapToClick`,
  `input.leftHanded`, `input.scrollMethod`, `input.secondaryClick`,
  `input.scrollSpeed`, `input.doubleClickSpeed`, `input.haptics` (stored in
  `PointerSettings`, never applied to libinput);
- `input.keyboardBrightness`, `input.adjustBrightnessLowLight`,
  `input.backlightOffAfter` (no hardware bridge).

The fix is either a consumer or `stored_only: true` plus a visible note. There
is no third option. The `stability` set adds a consumer-conformance test that
fails when a live key has no entry in the verified-consumer table.

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

- a clipboard client that never drains its pipe must not block the shell;
- a portal request must time out or cancel when the shell disappears, and a
  restarted shell must query pending requests;
- a Files desktop that loses the compositor must exit and be restarted;
- a hung shell must be caught by a watchdog or a documented restart chord;
- a `df_output.set_mode` with a zero/negative refresh must not panic the
  compositor;
- an Xwayland crash must remove its windows from the model, not leave ghosts.

### 3.5 Data operations are all-or-nothing

A directory move between filesystems is copy-to-temp, rename, remove-source.
On any error the temporary destination is removed and the source is left
intact. A retry must not create `name.1` beside a half-moved `name`.

### 3.6 Multi-monitor is per-output

Each output has exactly one `wl_output` global, removed on unplug. A failed
connector init cleans up the output, its Spaces, and its global. Popups are
constrained to the parent window’s output. Absolute pointer, touch, and
tablet events are mapped to the output under the device/pointer. The lock
surface on a secondary output receives that output’s input.

### 3.7 Launch is complete

The launch environment contains the display the app should use:

- `WAYLAND_DISPLAY` from the compositor socket;
- `DISPLAY` from `$XDG_RUNTIME_DIR/<socket>.x11-display` when present, and
  removed when absent or empty;
- `QT_QPA_PLATFORM=wayland` for Qt apps when the shell forced offscreen;
- no `QT_QPA_PLATFORM=offscreen` leak.

Xwayland starts after the session environment is exported, so `DISPLAY` is
not a startup value. The compositor writes the hand-off file when Xwayland is
ready; the shell, Files, and Settings read it at each launch, never once at
startup, so a late Xwayland start still reaches the next child. The shared
`Dragonfruit::applyX11DisplayHandoff` helper trims the file, sets `DISPLAY`,
and removes an inherited value when the file is missing or empty. The decision
is frozen in [ADR 0320](adr/0320-launch-time-xwayland-display-handoff.md).

A `Terminal=true` entry resolves `$TERMINAL`, `xdg-terminal-exec`,
`x-terminal-emulator`, then a common terminal list; if none exists it reports
a visible failure. The app index honors `OnlyShowIn`, `NotShowIn`, and
`TryExec`.

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
| S-07 | Brightness never reaches the panel | T319 |
| S-08 | Log Out is a live no-op | T313 |
| S-09 | A hung shell is never recovered | T314 |
| S-10 | Clipboard-history serving can block the shell forever | T310 |
| S-11 | Portal requests hang when the shell restarts or a second request arrives | T311 |
| S-12 | Cross-filesystem directory moves can partially move and report failure | T312 |

### P1 — first-week friction

| ID | Defect | Task |
|---|---|---|
| S-13 | Media keys do nothing | T321 |
| S-14 | The titlebar window menu is unlabeled | T329 |
| S-15 | Files lacks copy/move/paste, undo, watcher, tabs, New Window, Open With | T330–T334 |
| S-16 | Screen sharing is stills-only | T335a, T335b |
| S-17 | X11 `_NET_FRAME_EXTENTS` is missing; XDnD is a model only | T327, T328a, T328b |
| S-18 | Pointer constraints are refused for games/remote desktop; refusals grow unbounded | T322 |
| S-19 | DRM outputs get two `wl_output` globals; a failed init leaves a phantom output | T323 |
| S-20 | Popups and absolute/touch/tablet input are hard-coded to the first output | T324 |
| S-21 | `df_output.set_mode` accepts a zero/negative refresh and panics the compositor | T325 |
| S-22 | Xwayland restart leaves ghost windows and can lose new X11 windows | T326 |
| S-23 | Unpinning every Dock app is overwritten with defaults on next login | T336 |
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
| Shell kill | 20 unlocked kills → chrome back < 2 s; 20 locked kills → session ends safely, never frozen |
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
