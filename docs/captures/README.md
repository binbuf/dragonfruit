# Slice captures

Every vertical slice produces a **human-verifiable capture** here — a short
recording plus stills — and reviews it against the design docs and the
design-system gallery goldens. A slice whose capture has not been watched is
not done (see the definition of done in
[`../ROADMAP.md`](../ROADMAP.md)).

Naming convention:

```text
t01-loop-v0.mp4            # the slice's demo recording
t01-loop-v0-wayland.png    # stills for the notable cases
t01-loop-v0-x11.png
t01-loop-v0-csd.png
t02-lifecycle-motion.mp4
t02-lifecycle-motion-reduced.mp4
...
```

T-01's stills are produced by `scripts/capture-demo.sh`, which drives the live
nested session over the synthetic-input harness: `t01-loop-v0.png` (the whole
loop), `-wayland`, `-x11`, `-titlebar`, `-dock`, `-dock-minimized`, `-menu`,
`-zoomed`, `-minimized`, `-restored`, `-closed`, plus `t01-loop-v0.mp4`.

T-02's lifecycle-motion stills are produced by `scripts/capture-motion.sh`,
which runs the same nested walkthrough twice (once normally, once with
`accessibility.reduceMotion`) and writes `t02-lifecycle-motion*.png` plus
`t02-lifecycle-motion.mp4` and `t02-lifecycle-motion-reduced.mp4`.

T-03.1a's capture is the raw 60 s idle/animation frame trace,
`t03-idle-trace.txt`, produced by `scripts/idle-trace.sh` (`make idle-trace`).
It is a text artifact, not a recording: the numbers on the `SIGUSR1`
render-stats line are the budget evidence (T-03.2/T-03.4 add the DRM run and
the latency capture).

T-04.4b's material sign-off is produced by `scripts/capture-materials.sh`: it
runs the nested walkthrough in dark, light, and dark+reduced-motion, plus a
pre-material baseline (degrade tier `minimal`, blur off), writes
`t04-materials*.png`/`.mp4`, and composes two review sheets —
`t04-materials-gallery-side-by-side.png` (the compositor SSD titlebar beside
the design-system `ssd_light`/`ssd_dark` goldens) and
`t04-materials-before-after*.png`. The design-system gallery goldens themselves
are the art-direction reference and are gated by
`scripts/check-gallery-snapshots.py --strict`.

T-03.1b's capture is `t03-latency-nested.txt`, produced by
`scripts/latency-trace.sh` (`make latency-trace`): it runs the nested demo,
injects real input over the synthetic-input harness and records the
input-to-photon latency samples plus an honest pass/fail against one 60 Hz
frame. Also a text artifact — the direct-scanout counter is the `scanout stats`
line in the same trace, and T-03.4 fills the DRM half on hardware.

T-03.2's DRM first bring-up is produced by `scripts/drm-bringup.sh`
(`make drm-bringup`). On this host there is no free logind seat (the KDE
session owns DRM master), so the unit is **marked open, not skipped**:
`t03-drm-loop.open.txt` records the `libdrm` master probe
(`/dev/dri/card0: busy (Permission denied)`), the backend's own no-op-seat
self-report (`DRM bring-up: OPEN (no connected output: master busy or all
connectors disconnected)`), and the reproduction command. On a machine with a
free seat/VM the same script starts the DRM backend, records the `SIGUSR1`
frame trace to `t03-drm-loop-trace.txt`, and the T-03.4 runbook completes the
T-01 loop capture. The classification itself is pinned by
`compositor/tests/drm_bringup.rs` in `make e2e`
([ADR 0170](../design/adr/0170-drm-first-bringup-open-marker-and-no-output-guard.md)).

T-03.3's hardware input validation matrix is produced by
`scripts/input-validation.sh` (`make input-validation`). Real-device input
needs the compositor to own a libinput seat, so on this host the unit is
**marked open, not skipped**: `t03-input-matrix.open.txt` records the host
device inventory, the per-class coverage (`device`/`headless`/`gap`), and the
headless suite that already drives the mouse/keyboard/gesture/hot-corner
routing. On a free seat the same script writes `t03-input-matrix.txt` after a
real-device run. The classification is pinned by
`compositor/tests/input_validation.rs` in `make e2e`
([ADR 0171](../design/adr/0171-hardware-input-validation-matrix.md)).

T-03.4's soak and multi-GPU artifacts are produced by `scripts/drm-soak.sh`
(`make drm-soak`) and `scripts/multi-gpu-validation.sh`
(`make multi-gpu-validation`). On this host `t03-drm-soak.open.txt` records the
automated 100-cycle teardown soak (PASS — zero strays, zero leaked
sockets/tokens), the busy-card probe, and the no-op-seat self-report;
`t03-multigpu.open.txt` records the single-card classification
(`Multi-GPU: SINGLE device=card0`). On a free seat / second GPU / VM the same
scripts write `t03-drm-soak.txt` (one real DRM cycle with DRM master released)
and `t03-multigpu.txt`. The pure decisions are pinned by
`compositor/tests/multi_gpu.rs` and the dev-tool soak tests in `make e2e`
([ADR 0172](../design/adr/0172-drm-soak-teardown-and-runbook.md)); the exact
hardware commands are in
[runbook-drm-session.md](../runbook-drm-session.md).

T-05.6's Mission Control capture is produced by `scripts/capture-overview.sh`:
it runs the nested demo twice (normally and with `accessibility.reduceMotion`),
drives the overview open, Desktop Reveal, and a per-Space wallpaper slide over
the synthetic-input harness, and writes `t05-mission-control-live*.png` plus a
short `.mp4` per mode. `t05-gesture-budget-nested.txt` is the accompanying
text artifact: one `query gesture` sample per gesture (the T-05.6
gesture-scoped frame budget, judged against one 60 Hz frame) plus each
session's exit `gesture budget`/`degrade stats` lines. On the development iGPU
the nested gesture records an honest shortfall (`held=0`) and the T-04 material
ladder downgrades to `reduced`; the number is recorded, not hidden.

T-08.3's settingsd restart/resync capture is produced by
`scripts/capture-settingsd.sh` (`make settingsd-capture`): `t08-settingsd.txt`
is the D-Bus flip transcript plus the persisted file; the stills
`t08-settingsd-before-dark.png`, `-after-light.png`, `-down.png`,
`-restart.png` (and the representative `t08-settingsd.png`) record the live
`appearance.colorScheme`/`dock.size` flip, the `kill -9` (nothing visually
lost), and the shell re-syncing to a value written to the durable file while
settingsd was down; `t08-settingsd.mp4` is the short clip.

T-09.6b's Settings Wave-1 sign-off capture is produced by
`scripts/capture-settings-wave-1.sh` (`make settings-wave-1-capture`): a scratch
settingsd on the session bus and the nested demo with the Settings window
zoomed, once per shipped pane. It writes `t09-settings-wave-1-light.png` and
`-dark.png` (the Appearance pane's scheme flipped live), `-reduced.png`
(`accessibility.reduceMotion` on), one `t09-settings-wave-1-<pane>.png` per
shipped pane (appearance, wallpaper, desktop-dock, displays), the
representative `t09-settings-wave-1.png`, and `t09-settings-wave-1.mp4`. The
absent-provider stills are intentionally not in this set: absence is asserted
headlessly by `apps/settings/tests/tst_settings_absence.qml` and documented in
[../design/08-settings.md](../design/08-settings.md).

T-18.2's Wallpaper-pane capture is produced by
`scripts/capture-t18-wallpaper.sh` (`make t18-wallpaper-capture`): a scratch
settingsd and a scratch-cache `dragonfruit-wallpaperd` on the session bus, with
the nested demo opened directly at the Wallpaper pane. It writes
`t18-wallpaper-offline.png` (cold cache, dead network: the shipped
`Default.jpg` desktop and the Featured row's "available soon" note),
`t18-wallpaper-fetching.png` (the provider held in `fetching` by a hanging
CONNECT proxy, so the Featured row shows the grey shimmer skeletons) and
`t18-wallpaper-filled.png` (the real Wikimedia fetch completed, the Featured
row filled and the attribution visible). The three source rows (Featured /
Built-in / Custom) and the shipped-default Built-in tile are visible in all;
the state matrix itself is asserted headlessly by
`apps/settings/tests/tst_settings_wallpaper.qml`.

T-18.3's absence/state matrix is produced by `scripts/t18-absence-matrix.sh`
(`make t18-absence-matrix`): the headless reproduction transcript at
`t18-absence-matrix.txt`, and the reviewed matrix with the live halves at
`t18-absence-matrix.md`.

T-15.16's absent-daemon masking matrix is produced by
`scripts/t15-absence-matrix.sh` (`make t15-absence-matrix`): the headless
reproduction transcript for every shipped Wave-2/3 pane at
`t15-absence-matrix.txt`, and the reviewed 22-pane state matrix with the
live/VM halves at `t15-absence-matrix.md`. The whole T-15 breadth capture set
is indexed as a linked set in `t15-breadth.md`; its representative
whole-desktop still is `t15-breadth.png`, produced by
`scripts/capture-t15-breadth.sh` (`make t15-breadth-capture`) with every host
provider absent (no `DF_STATUS_FIXTURE` / `DF_SETTINGS_FIXTURE`).

T-17.5a's premium-gate absent-daemon and crash matrix is produced by
`scripts/t17-robustness-matrix.sh` (`make t17-robustness-matrix`): the headless
verification transcript at `t17-robustness-matrix.txt`, and the reviewed matrix
with the live nested check at `t17-robustness-matrix.md` plus its still
`t17-robustness-matrix.png`. It re-runs the T-15.16 absence rows and the
T-16.8a/T-16.8b kill and restart-policy matrices, and adds the session-start
case: every optional service absent, and the session still reaches Running
(`services/session/tests/absent_services.rs`; see
[ADR 0177](../design/adr/0177-absent-services-never-block-session-start.md)).

T-17.5b's premium-gate leak and lock enforcement is produced by
`scripts/t17-leak-lock-soak.sh` (`make t17-leak-lock-soak`): the headless
verification transcript at `t17-leak-lock-soak.txt` (a 100-cycle soak and the
lock conformance suite, each under an isolated `XDG_RUNTIME_DIR` asserted empty
afterwards; the lock suite runs `--release`), and the reviewed note with the
live nested loop check at `t17-leak-lock-soak.md` plus its still
`t17-leak-lock-soak.png`. It closes the stale-test-runtime-file leak by adding
the `DISPLAY` file to the soak tripwire and cleaning every compositor test
harness in `Drop`; see
[ADR 0178](../design/adr/0178-t17-leak-lock-enforcement.md).

T-17.6's premium-gate sign-off is produced by
`scripts/capture-t17-premium-gate.sh` (`make t17-premium-gate-capture`): it
captures the assembled nested desktop to `t17-premium-gate.png` (via
`scripts/t17-premium-gate-driver.py`), re-runs the fast headless premium-gate
rows into `t17-premium-gate.txt`, and is indexed by the sign-off report
`t17-premium-gate.md`. The report reproduces the track checklist with a verdict
and evidence link per item; the evidence boundary is
[ADR 0179](../design/adr/0179-t17-premium-gate-sign-off.md).

T-10.7's Files slice capture is produced by `scripts/capture-files.sh`
(`make files-capture`): the nested demo runs with `DF_DEMO_QT_APP` pointing at
Files over a scratch fixture tree and a scratch trash store, and it writes
`t10-files-list.png` (the representative `t10-files.png`), `t10-files-icon.png`,
`t10-files-context-menu.png`, `t10-files-multiselect.png`, `t10-files-rename.png`,
`t10-files-trash-empty.png`, `t10-files-dock-trash.png` (the Dock badge),
`t10-files-reveal.png`, and the 100k synthetic listing top and scrolled
(`t10-files-large-directory*.png`), plus `t10-files.mp4`. The app's capture
seams (`DF_FILES_START_MENU` / `_SELECT` / `_RENAME` / `_EMPTY_TRASH`) stand in
for pointer states the nested synthetic harness cannot hold. The
large-directory scroll trace is `t10-files-scroll-trace.txt` (raw files-core
streaming benchmark and the QML windowed-rendering sweep); the T-10.5 budget
record remains `t10-files-perf.txt`.

T-11.4b's OSD/DND sign-off is produced by `scripts/capture-osd-dnd.sh`
(`make osd-dnd-capture`): the nested demo runs with `DF_STATUS_FIXTURE=1
DF_NOTIFY_FIXTURE=1 DF_FOCUS_FIXTURE=dnd`, the driver crops the accent-tinted
Focus crescent left of the clock to `t11-dnd.png`, then opens the Control
Center through the real Control-Option-C shortcut and drags the Sound volume
slider (a real gesture) to present the OSD, writing `t11-osd.png` and the whole
nested window as `t11-osd-context.png`. The T-11.3a/b Control Center stills
(`t11-control-center*.png`) remain part of the set. The capture driver's
synthetic-input seam is the same harness as T-01/T-10.

T-12.5b's session sign-off is produced by `scripts/capture-session.sh`
(`make session-capture`): the nested demo runs with the synthetic-input harness
bound and the driver locks the session with the real Cmd+Ctrl+Q chord, writing
`t12-session.png` (the desktop), `t12-session-lock.png` (the first-party lock
UI), and `t12-session.mp4`. Two transcripts accompany them:
`t12-session.txt` (the compositor's `query lock`/`query session` before and
while locked — `locked=1 surfaces=1 lock-focus=1`) and
`t12-session-kill-matrix.txt` (the `dragonfruit-session` kill matrix). The real
greeter/login and DRM logout ends of the T-12 demo are T-12.6, so this nested
capture proves the session + lock surface, not the greeter round trip.

T-12.6a's live check is `t169-session-selector-nested-check.png`, a one-shot
nested `make demo` still (spectacle) confirming the desktop still renders after
the display-manager session-selection seam landed. The seam itself has no
surface; its exact-byte fixture writes are asserted by
`cargo test -p dragonfruit-dev selected_bytes_are_exact -- --nocapture` (see
[ADR 0173](../design/adr/0173-display-manager-session-selection-seam.md)).

T-12.6b's live check is `t170-real-session-return-nested-check.png` (the nested
system menu with **"Quit to existing"** visible when `DRAGONFRUIT_DEV_RETURN`
is set) and `t170-real-session-return-absent-check.png` (the same menu without
the variable, no "Quit to" row). Both are one-shot nested `make demo` stills
(spectacle) captured for this unit; the arm/ready/restore state machine, the
byte-identical snapshots, and the `DRAGONFRUIT_DEV_RETURN` export are asserted
by `cargo test -p dragonfruit-dev` and `cargo test -p dragonfruit-session` (see
[ADR 0174](../design/adr/0174-real-session-round-trip-state.md)).

T-13.7's portal sign-off is produced by `scripts/capture-portals.sh`
(`make portals-capture`): a private session bus runs the real
`xdg-desktop-portal` frontend with the Dragonfruit backend and a driver
executed *inside* `flatpak run org.mozilla.firefox`. `t13-portals.txt` is the
automated round-trip transcript (FileChooser open, Screenshot, and the
ScreenCast CreateSession/SelectSources/Start flow, plus the frontend-reported
interface versions), and `t13-portals.png` is the nested desktop with the
shell's FileChooser picker raised by that same Flatpak browser. Clipboard has
no portal (ADR 0082); its compositor-level round-trips are the T-13.5a
conformance. Needs a host seat, Flatpak, spectacle, and Pillow; not in
`make e2e`.

T-14.6a's strange-app zoo sign-off is produced by
`scripts/zoo/zoo-run.sh` (`make zoo-run`): a nested session launches Firefox
(X11), xterm, a Steam-`WM_CLASS` stand-in, GNOME Calculator (GTK4 via flatpak),
an SDL2 sample on the nested Wayland socket (T-14.6b), and an Electron client,
then records each app's raw identity, `app-index` desktop-id resolution, SSD/CSD
decoration tier, and `menu-broker` tier. `t14-zoo-matrix.md`/`.json` is the machine-readable matrix
(one pass/fail per app per behaviour) and `t14-zoo.png` is the nested desktop
with the zoo mapped. Needs a host Wayland session, spectacle, Pillow, gcc with
SDL2/X11 headers, an Electron install (`ZOO_ELECTRON_DIR`), and the Calculator
flatpak; not in `make e2e`.

T-14.7a's Dock plate geometry sign-off is produced by
`scripts/capture-dock-spacing.sh` (`make dock-spacing-capture`): the nested
demo runs with a scratch settingsd, and the driver flips `dock.position`
(bottom/left/right) and `appearance.colorScheme` (light/dark) live, writing one
crop per position/scheme to `t14-dock-spacing-{bottom,left,right}-{light,dark}.png`.
Each crop is trimmed to the nested 1920x1200 output and then to a 190 px edge
strip, so the floating plate, its `padding`/`paddingAlong` insets, and the
`edgeMargin` gap to the screen edge are all visible (T-14.7u retune, measured:
plate 78 px, cross-axis padding 15 px, along-axis padding 16 px, gap 14 px, dot
4 px sitting 8 px below the artwork, plate radius 28 px; `edgeMargin` 8 px on
every edge). Needs a host Wayland session, spectacle, and Pillow; not in
`make e2e`.

T-14.7b's Dock magnification sign-off is produced by
`scripts/capture-dock-magnify.sh` (`make dock-magnify-capture`): the nested demo
runs with the synthetic-input harness, the driver parks the pointer over the
left end, centre, and right end of the Dock under the dark and light schemes,
and writes `t14-dock-magnify-{left,center,right}-{dark,light}.png`. Because the
plate top rises by the same peak everywhere, the three stills differ in *where*
the tall icon sits, not in plate height; a side-by-side of the left third shows
the leftmost icon largest when the pointer is at the left (verified during the
T-14.7b sign-off). The same script brackets one timed sweep with compositor
`SIGUSR1` dumps and writes `t14-dock-magnify-frame-budget.txt` (raw
`frames_rendered`/`frames_skipped_no_damage` deltas). Needs a host Wayland
session, spectacle, and Pillow; not in `make e2e`.

T-14.7y's magnification-tracking sign-off is produced by
`scripts/capture-dock-magnify-sweep.sh` (`make dock-magnify-sweep-capture`): the
driver sweeps the pointer slowly across the Dock and takes six crops at evenly
spaced positions, stacked vertically into one filmstrip per scheme,
`t14-dock-magnify-sweep-{light,dark}.png`. The committed strips carry the
pre-fix row (`sweep-row-*-before.png`, captured from the T110y parent tree)
above the fixed row (`sweep-row-*-after.png`); the settled crops read
equivalently because the bug was *temporal* (ringing during motion), so the
oscillation itself is pinned by `tst_dock.qml` and measured in the T110y
PROGRESS note (pre-fix smoothed-pointer overshoot 635.5 vs target 624.5 = 11.0
px; fixed 0.0 px). The same 40-step run is recorded in
`t14-dock-magnify-sweep-trace.txt` (pre-fix: 19 plate-top reversals, max 1.11
px; fixed: 0 reversals). Needs a host Wayland session, spectacle, and Pillow;
not in `make e2e`.

T-14.7z's Dock plate corner/frost sign-off is produced by
`scripts/capture-dock-plate-corners.sh` (`make dock-plate-corners-capture`): the
nested demo runs with the synthetic-input harness, the driver detects the
resting plate's bright top rim in a full screenshot, and writes a 6x
nearest-neighbour close-up of the top-left and top-right corners side by side to
`t14-dock-plate-corners-{light,dark}.png`. It then parks the pointer over the
plate centre and writes a 2x strip to
`t14-dock-plate-magnified-{light,dark}.png`. The rim follows the corner arcs
(no straight hairline pokes past them), and the QML fill and the compositor
frost are the same integer rounded rect — pinned deterministically by the
`tst_dock.qml` rim-differential and panel-parity cases (T-14.7z, ADR 0112).
Needs a host Wayland session, spectacle, and Pillow; not in `make e2e`.

T-14.7w's Dock icon squircle-mask sign-off is produced by
`scripts/capture-dock-icon-mask.sh` (`make dock-icon-mask-capture`): the nested
demo runs with a scratch app-index corpus of three apps whose themed icon files
are a full-bleed square (red), a padded square (green), and a circle (blue), and
a scratch settingsd that pins them. For each colour scheme the driver locates
the three tiles by colour and writes a 4x nearest-neighbour crop to
`t14-dock-icon-mask-{light,dark}.png`: the red square's outer corners are the
plate background (the tile is clipped to the token squircle), the circle is
inscribed and reaches the tile edge (no double inset), and the padded square
keeps only its own padding. The mask is a Canvas clip so the software scene
graph in `tst_dock.qml` observes it (T-14.7w, ADR 0113). Needs a host Wayland
session, spectacle, gdbus, Pillow, and a session bus with no settingsd owner;
not in `make e2e`.

T-14.7c's Dock motion/frame-discipline sign-off is produced by
`scripts/capture-dock-motion.sh` (`make dock-motion-capture`): the nested demo
runs with a scratch settingsd and the synthetic-input harness, and the driver
brackets a magnification sweep, a context menu open/close, and a `dock.size`
change (via `gdbus`) with compositor `SIGUSR1` dumps, writing the per-phase raw
numbers to `t14-dock-motion-trace.txt` plus the menu and resized stills. The
trace shows zero over-budget frames for the sweep, size change, and the
trailing idle window (the settled Dock renders nothing), and a handful during
the nested popover open/close with no degrade-tier downgrade. Needs a host
Wayland session, spectacle, gdbus, Pillow, and a session bus with no settingsd
owner; not in `make e2e`.

T-14.7d's Trash artwork sign-off is produced by
`scripts/capture-dock-trash.sh` (`make dock-trash-capture`): the nested demo
runs once per Trash state with a scratch settingsd and a scratch
`XDG_DATA_HOME` whose home trash is empty, full (two `.trashinfo` records plus
payloads), or blocked so the store reads as unavailable; the driver flips
`appearance.colorScheme` (dark/light) and writes one 2x crop per state/scheme to
`t14-dock-trash-{empty,full,unavailable}-{light,dark}.png`, each showing the
whole Dock so the Trash reads next to the themed tiles. Empty is a tidy neutral
bin; full shows crumpled paper overflowing behind the rim plus an accent rim;
unavailable is dimmed with the status badge. Needs a host Wayland session,
spectacle, gdbus, and Pillow; not in `make e2e`.

T-14.7e's Add Application picker sign-off is produced by
`scripts/capture-dock-app-picker.sh` (`make dock-app-picker-capture`): a
scratch settingsd pins one installed app through `dock.pinned` and the nested
demo opens the picker filtered to that app via the shell's
`DF_APP_PICKER_FIXTURE` seam (the production picker path minus a synthetic
divider click). It writes one 2x Dock crop to `t14-dock-app-picker.png`:
the "Add Application" popover, its search field showing the query, the
matching rows with themed icons, and the pinned row's "In Dock" state.
Needs a host Wayland session, spectacle, gdbus, and Pillow; not in `make e2e`.

T-14.7f's Dock drop identity and feedback sign-off is produced by
`scripts/capture-dock-drops.sh` (`make dock-drops-capture`): the nested demo
runs once per hover state through the shell's `DF_DOCK_DROP_FIXTURE` seam (the
production Dock presentation minus a real drag source), and the three 2x crops
are stacked into `t14-dock-drops.png`. Top: an app-alias ghost showing
"Dragonfruit Files" with its real themed folder tile and the "Add to Dock"
capsule; middle: a file dragged over an app ("Open with Dragonfruit Files");
bottom: a file dragged over the Trash ("Move to Trash"). Needs a host Wayland
session, spectacle, gdbus, and Pillow; not in `make e2e`.

T-14.7g's Dock launch/activation sign-off is produced by
`scripts/capture-dock-activation.sh` (`make dock-activation-capture`): the
nested demo runs once per state and a **real** synthetic pointer click on the
pinned entry drives the production click tree (the T-14.7g
`DF_DOCK_ACTIVATION_FIXTURE` seam retired in T-14.7x); the driver finds the
entry by scanning for the launched window. The three full nested stills are
stacked into `t14-dock-activation.png`: before (Settings pinned, not running),
after launch (the Settings window mapped and the running indicator on), and
after activate (the same window focused). A fourth run pins an unresolved
identity so `t14-dock-activation-missing.png` shows the Dock's not-found mark
(the notice itself needs the notification service, which `make demo` does not
start). Needs a host Wayland session, spectacle, gdbus, and Pillow; not in
`make e2e`.

T-14.7l's Dock launch-origin sign-off is produced by
`scripts/capture-dock-launch-origin.sh` (`make dock-launch-origin-capture`): the
nested demo runs once with Settings pinned and a **real** synthetic click on the
entry (T-14.7x; no activation fixture), so the Dock's click tree publishes the
entry tile before the launch. The script reads the compositor's `query motion`
record and asserts the launched window's appear originates in the bottom Dock
band, then annotates the settled still:
`t14-dock-launch-origin.png` marks the recorded origin (the entry icon) and the
window target, with the raw numbers in `t14-dock-launch-origin-trace.txt`. Needs
a host Wayland session, spectacle, gdbus, and Pillow; not in `make e2e`.

T-14.7h's Dock folder stack sign-off is produced by
`scripts/capture-dock-folder-stack.sh` (`make dock-folder-stack-capture`): the
nested demo runs once per state with `XDG_DOWNLOAD_DIR` pointed at a scratch
folder and the shell's `DF_DOCK_STACK_FIXTURE` seam opening the popover (the
production path minus a synthetic pointer click). The four 2x crops are stacked
into `t14-dock-folder-stack.png`: resting (the clean folder silhouette, no text
in the artwork), open (the header with the folder icon, name and "Open in
Files", separator, and rows with icons/names), empty ("Empty" row), and long
(scrollable rows plus the "N more…" summary). Needs a host Wayland session,
spectacle, gdbus, and Pillow; not in `make e2e`.

T-14.7k's Dock folder pin sign-off is produced by
`scripts/capture-dock-folder-pin.sh` (`make dock-folder-pin-capture`): the nested
demo runs twice with `XDG_DOWNLOAD_DIR` pointed at a scratch folder and the
shell's `DF_DOCK_FOLDER_PIN_FIXTURE` seam writing an extra absolute folder path
through settingsd (the production drop path minus a synthetic drag). The two 2x
crops are stacked into `t14-dock-folder-pin.png`: pinned (the user's folder tile
in the stacks region before the Trash, next to the Downloads stack, with no
caption) and open (that folder's popover: the header with the folder icon, the
folder name and "Open in Files", a separator, and rows with icons/names). Needs
a host Wayland session, spectacle, gdbus, and Pillow; not in `make e2e`.

T-14.7l's Dock launch-origin sign-off is produced by
`scripts/capture-dock-launch-origin.sh` (`make dock-launch-origin-capture`): the
nested demo runs once with Settings pinned and the shell's
`DF_DOCK_ACTIVATION_FIXTURE=launch` seam, which now goes through the Dock's click
tree so the entry tile is published before the launch. The script reads the
compositor's `query motion` record and asserts the launched window's appear
originates in the bottom Dock band, then annotates the settled still:
`t14-dock-launch-origin.png` marks the recorded origin (the entry icon) and the
window target, with the raw numbers in `t14-dock-launch-origin-trace.txt`. Needs
a host Wayland session, spectacle, gdbus, and Pillow; not in `make e2e`.

T-14.7m's Dock window chooser per-window actions sign-off is produced by
`scripts/capture-dock-chooser-actions.sh` (`make dock-chooser-actions-capture`):
the nested demo runs once per color scheme with a second Settings window
(`--launch`) so the app's Dock entry groups, and the shell's
`DF_DOCK_CHOOSER_FIXTURE` seam opens the chooser with the first row's actions
revealed (the production chooser minus a synthetic pointer hover). The two 2x
crops are stacked into `t14-dock-chooser-actions.png` (light over dark) and
saved individually: the "Show All Windows" header, the window rows, and the
hovered row's stateful Minimize glyph plus the destructive red Close. Needs a
host Wayland session, spectacle, gdbus, and Pillow; not in `make e2e`.

T-14.7n's Dock window chooser row discipline sign-off is produced by
`scripts/capture-dock-chooser-scroll.sh` (`make dock-chooser-scroll-capture`):
the nested demo runs once per color scheme with twelve Settings windows
(`--launch` repeated) so the app's Dock entry groups well past
`component.dock.chooser.maxRows` (7), and the shell's
`DF_DOCK_CHOOSER_FIXTURE=scroll` seam opens the chooser, scrolls the bounded
viewport mid-list, and highlights a visible row. The two 2x crops are stacked
into `t14-dock-chooser-scroll.png` (light over dark) and saved individually:
the pinned "Show All Windows" header, the capped seven-row viewport scrolled to
the lower rows, the scrollbar, and the hovered row's actions clear of it. Needs
a host Wayland session, spectacle, gdbus, and Pillow; not in `make e2e`.

T-14.7o's Dock window-count badge sign-off is produced by
`scripts/capture-dock-window-badge.sh` (`make dock-window-badge-capture`): the
nested demo runs with a grouped Settings entry (two windows via one `--launch`)
and a single-window Files entry, so the Settings tile shows a `2` at its
top-right corner and Files shows nothing. It runs light and dark at
`dock.size` 0.5, then light at `dock.size` 0 and 1 to record the badge at
`iconSizeMin` (32) and `iconSizeMax` (64). The two default-size 2x crops are
stacked into `t14-dock-window-badge.png` (light over dark) and saved
individually, with `t14-dock-window-badge-{min,max}.png` for the size extremes.
Needs a host Wayland session, spectacle, gdbus, and Pillow; not in `make e2e`.

T-14.7p's Dock hover-open chooser sign-off is produced by
`scripts/capture-dock-hover-chooser.sh` (`make dock-hover-chooser-capture`):
the nested demo runs with `dock.chooserOnHover` on and two grouped Dock entries
(two Settings and two Files windows), then the shell's `DF_DOCK_HOVER_FIXTURE`
seam drives the production hover-open path without a real pointer dwell. The
light run captures the dwell-open chooser anchored to the first grouped entry;
the dark run captures the same popover retargeted to the second entry with the
magnification pointer parked between them. The two 2x crops are stacked into
`t14-dock-hover-chooser.png` (light over dark) and saved individually. Needs a
host Wayland session, spectacle, gdbus, and Pillow; not in `make e2e`.

T-14.7q's Dock overflow cell and "More Windows" list sign-off is produced by
`scripts/capture-dock-overflow.sh` (`make dock-overflow-capture`): a full-size
nested output fits dozens of running groups, so the script sets the shell's
`DF_DOCK_OVERFLOW_FIXTURE=1` seam, which injects synthetic running groups and
caps the effective Dock axis so the pure planner produces a genuine overflow —
the terminal cell then opens its list. The light and dark runs capture the Dock
with the overflow cell (grid glyph + hidden-group count) and its open list. The
two 2x crops are stacked into `t14-dock-overflow.png` (light over dark) and
saved individually. Needs a host Wayland session, spectacle, gdbus, and Pillow;
not in `make e2e`.

T-14.7r's Dock Empty Trash progress and result sign-off is produced by
`scripts/capture-dock-trash-empty.sh` (`make dock-trash-empty-capture`): a real
empty finishes too fast to catch the busy state reliably, so the script drives
the shell's `DF_DOCK_TRASH_EMPTY_FIXTURE` seam over a populated trash — `busy`
shows the delayed indeterminate ring with "Emptying the Trash…", `success`
shows the check and "3 items removed". Each state is captured in light and
dark; the busy and success crops are stacked into `t14-dock-trash-empty.png`
(busy over success) and saved individually as
`t14-dock-trash-empty-{busy,success}-{light,dark}.png`. Needs a host Wayland
session, spectacle, gdbus, and Pillow; not in `make e2e`.

T-14.7s's Dock minimize-to-icon reaction sign-off is produced by
`scripts/capture-dock-minimize-reaction.sh`
(`make dock-minimize-reaction-capture`): the reaction is a ~320 ms one-hop
bounce, so the shell's `DF_DOCK_MINIMIZE_REACTION_FIXTURE=0.5` seam pins the
first running entry at the peak of the hop. The script captures a bottom Dock
(light) and a right Dock (dark) so both the cross-axis and the direction away
from the screen edge read; the two 2x crops are stacked into
`t14-dock-minimize-reaction.png` and saved individually as
`t14-dock-minimize-reaction-{bottom-light,right-dark}.png`. Needs a host Wayland
session, spectacle, gdbus, and Pillow; not in `make e2e`.

T-14.7i's Dock hover name label sign-off is produced by
`scripts/capture-dock-tooltip.sh` (`make dock-tooltip-capture`): the nested demo
runs once per entry kind through the shell's `DF_DOCK_TOOLTIP_FIXTURE` seam
(the production Tooltip presentation minus a synthetic pointer hover) and
places the magnification pointer on the target so the label sits over a
magnified icon. The three 2x crops are stacked into `t14-dock-tooltip.png`:
an app ("Dragonfruit Files"), the Downloads folder ("Downloads — 2 items"), and
the Trash ("Trash — full"). The capsule is the design-system `Tooltip`: a
rounded `surfaceElevated` surface with a hairline border, above the entry on a
bottom Dock. Needs a host Wayland session, spectacle, gdbus, and Pillow; not in
`make e2e`.

T-14.7j's Dock Tahoe visual language sign-off is produced by
`scripts/capture-dock-tahoe.sh` (`make dock-tahoe-capture`): the nested demo runs
once with the compositor's synthetic-input harness and parks the pointer off the
Dock (resting), over the Trash (hover wash + label, no magnification), over an
app icon (magnification + label), and holds the left button down (pressed) under
the light and dark schemes. The four crops are stacked into
`t14-dock-tahoe-{light,dark}.png`. A third still, `t14-dock-tahoe-reduced.png`,
sets the compositor's `Minimal` material tier (blur off) and reduced motion, so
the plate reads as a clean capsule with no glass claim. Needs a host Wayland
session, spectacle, and Pillow; not in `make e2e`.

T-14.7v's Dock region-divider sign-off is produced by
`scripts/capture-dock-dividers.sh` (`make dock-dividers-capture`): the nested
demo is captured in its default state (a pinned prefix, the running demo client
as the temporary tail, and the fixed stacks/Trash tail -> two rules), after the
unpinned client is closed (empty tail -> one rule), and after `dock.pinned` is
cleared and the demo client exits (the fixed region alone -> no rule). The three
190 px-tall bottom-Dock strips are stacked into
`t14-dock-dividers-{light,dark}.png`. Measured on the dark composite: the
three-region strip shows two 1 px rules 64 px tall (0.82 x the 78 px plate) with
a 22 px gap on each side; the empty-tail strip one rule; the single-region strip
none. Needs a host Wayland session, spectacle, gdbus, and Pillow; not in
`make e2e`.

T-15.1a's Bluetooth adapter has no surface of its own (the pane and tile are
T-15.1b); `t15-1a-bluetooth-adapter.png` is the nested demo launched with the
new `dragonfruit-bluetooth` crate in the workspace, confirming the desktop
still renders (3840x2160, 468 614 unique colours; menu bar, Dock, and client
window present, clean teardown). The adapter itself is proven headlessly by
`cargo test -p dragonfruit-bluetooth`.

T-15.1b's Bluetooth pane and Control Center tile are captured by
`scripts/capture-t15-bluetooth.sh`: `t15-1b-bluetooth-pane.png` is the Settings
Bluetooth pane (toggle card with the discoverable caption, `My Devices`,
`Nearby Devices`), and `t15-1b-bluetooth-control-center.png` is the panel with
the Bluetooth tile and known-device rows. The demo runs with
`DF_SETTINGS_START_PANE=bluetooth`, `DF_STATUS_FIXTURE` and
`DF_BLUETOOTH_FIXTURE` so no host BlueZ is needed. Needs a host Wayland
session, spectacle, and Pillow; not in `make e2e`. Vision (OpenRouter) returned
HTTP 429 during the run, so the evidence is the pixel statistics and the pane's
headless tests: the pane still is a rendered Settings window (2088x1410, 430 364
unique colours, luminance sigma 93.5) and the panel crop is 360x760 with 4 942
unique colours, sigma 33.1.

T-15.2a's storage adapter has no surface of its own (the pane and tile are
T-15.2b); `t15-2a-storage-adapter.png` is the nested demo launched with the new
`dragonfruit-storage` crate in the workspace, confirming the desktop still
renders (3840x2160, 455 585 unique colours, luminance sigma 95.5) and tears
down cleanly. The adapter itself is proven headlessly by
`cargo test -p dragonfruit-storage`. Captured by
`scripts/capture-t15-storage.sh`; vision (OpenRouter) returned HTTP 429, so the
evidence is the pixel statistics (menu-bar strip sigma 82.0, dock region sigma
37.6, client window sigma 65.2) plus the headless suites.

T-15.2b's Storage pane and Control Center tile are captured by
`scripts/capture-t15-storage-pane.sh`: `t15-2b-storage-pane.png` is the
Settings Storage pane (the `Volumes` group with the `Photos` row and its
`Mount` action, and the `Removable Media` group with the `Flash Drive` row and
`Eject`), and `t15-2b-storage-control-center.png` is the panel with the Storage
tile, the volume row, and the `Storage Settings…` link. The demo runs with
`DF_SETTINGS_START_PANE=storage`, `DF_STATUS_FIXTURE` and
`DF_STORAGE_FIXTURE` so no host UDisks2 is needed. Needs a host Wayland
session, spectacle, and Pillow; not in `make e2e`. The pane still is 2088x1410
and the panel crop is 360x780; the vision check confirmed the pane's volume
and removable-media rows render with no clipping or stray artifacts, and the
Control Center bottom is not clipped.

T-15.3a's sound-and-routing adapter has no surface of its own (the pane and
tile are T-15.3b); `t15-3a-audio-routing.png` is the nested demo launched with
the extended `dragonfruit-audio` crate in the workspace, confirming the desktop
still renders (3840x2160, 454 797 unique colours, luminance sigma 95.4; menu
bar, Dock, and client window present, clean teardown). Captured by
`scripts/capture-t15-audio-routing.sh`. Vision confirmed the composited
desktop, menu bar, Dock, and client windows render with no blank areas or
clipping. The adapter itself — output/input device lists, default-device
switching, absence — is proven headlessly by `cargo test -p dragonfruit-audio`.

T-15.3b's Sound pane and Control Center tile are captured by
`scripts/capture-t15-sound-pane.sh`: `t15-3b-sound-pane.png` is the Settings
Sound pane (the `Sound Effects` group with `Alert sound`, `Play sound effects
through`, `Alert volume` and the three playback toggles, and the `Output &
Input` group with the `Output`/`Input` tab bar and the `Name`/`Type` device
table), and `t15-3b-sound-control-center.png` is the panel with the Sound
tile's `Built-in Speakers` routing subtitle, the volume slider, and the `Mute` /
`Sound Settings…` row. The demo runs with `DF_SETTINGS_START_PANE=sound`,
`DF_SETTINGS_FIXTURE`, `DF_STATUS_FIXTURE` and `DF_SOUND_FIXTURE` so no host
WirePlumber or settingsd is needed. Needs a host Wayland session, spectacle,
and Pillow; not in `make e2e`. The pane still is 2088x1410 and the panel crop
is 360x780. Vision confirmed the two groups, the device table, the sliders, and
the toggles render; the pane scrolls (its `Output volume`, `Mute`, and `Balance`
rows are below the fold in the window-height still), and the Control Center
bottom (Clipboard tile) is fully visible with no clipping.

T-15.4a's input device adapter has no surface of its own (the pane and tile are
T-15.4b); `t15-4a-input-adapter.png` is the nested demo launched with the new
`dragonfruit-input` crate in the workspace, confirming the desktop still
renders (3840x2160; menu bar, Dock, Settings, and demo windows present, clean
teardown). Captured by `scripts/capture-t15-input.sh`. Vision confirmed the
composited desktop renders with no blank areas, clipping, or stray artifacts.
The adapter itself — the libinput device inventory, classification,
device-change diff, and absence at every seam — is proven headlessly by
`cargo test -p dragonfruit-input`.

T-15.4b's Keyboard/Mouse/Trackpad pane and Control Center tile:
`t15-4b-keyboard-pane.png` is the active Settings window on the Keyboard pane
(the `Devices` inventory with `AT Translated Set 2 keyboard`, `Logitech USB
Mouse`, and `Synaptics TouchPad`; the `Key repeat rate` and `Delay until repeat`
sliders; and the `Keyboard Brightness` group), and
`t15-4b-input-control-center.png` is the 360x880 panel crop with the new
Keyboard tile (`1 keyboards, 2 pointing devices`, `Keyboard Settings…`) above
the Clipboard tile. The demo runs with `DF_SETTINGS_START_PANE=keyboard`,
`DF_SETTINGS_FIXTURE`, `DF_STATUS_FIXTURE`, and `DF_INPUT_FIXTURE`, so no host
libinput or settingsd is needed. Captured by
`scripts/capture-t15-input-pane.sh`; requires a host Wayland session, spectacle,
and Pillow, and is not in `make e2e`. Vision confirmed the pane's sections and
the three device rows render with nothing cut off, and the Control Center
bottom is complete with the Keyboard tile present and unclipped.

T-15.5a's Mission Control and hot corners adapter has no surface of its own (the
pane and tile are T-15.5b); `t15-5a-mission-control-adapter.png` is the nested
demo launched with the new `dragonfruit-overview` crate in the workspace,
confirming the desktop still renders (3840x2160; menu bar, Dock, Settings, and
demo windows present, clean teardown). Captured by
`scripts/capture-t15-overview-adapter.sh`. Vision confirmed the composited
desktop renders with no blank areas, clipping, or stray artifacts. The adapter
itself — the corner/action map, gesture gating, overview runtime state,
`changes` diff, hot-corner triggers, and absence at every seam — is proven
headlessly by `cargo test -p dragonfruit-overview`.

T-15.5b's Mission Control & Hot Corners pane and Control Center tile:
`t15-5b-mission-control-pane.png` is the active Settings window on the Mission
Control pane (the `Mission Control` group with `Swipe up to open` and `Swipe
between Spaces`, and the `Hot Corners` group with `Top Left` = `Mission
Control`, `Top Right` = `Notification Center`, `Bottom Left` = `Desktop`, and
`Bottom Right` = `Lock Screen`), and
`t15-5b-mission-control-control-center.png` is the 360x1040 panel crop with the
new Mission Control tile (`Gesture, 1 corner(s)`, `Mission Control Settings…`)
above an unclipped Clipboard tile. The demo runs with
`DF_SETTINGS_START_PANE=mission-control`, `DF_SETTINGS_FIXTURE`, and
`DF_STATUS_FIXTURE`, so no host settingsd is needed. Captured by
`scripts/capture-t15-mission-control-pane.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`. Vision confirmed the pane's
two groups and all four corner popups render with the schema-default values,
and the Control Center shows the Mission Control tile and link with nothing
clipped.

`t15-6a-power-adapter.png` is the nested demo desktop captured to prove the
battery/power-profiles adapter (T-15.6a) leaves the tree healthy. The adapter
has no surface of its own (the pane and tile are T-15.6b), so the capture only
confirms the desktop renders: menu bar, Dock, and windows composited with no
blank areas or stray artifacts. Captured by
`scripts/capture-t15-power-adapter.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

T-15.6b's Battery and power profiles pane and Control Center tile:
`t15-6b-battery-pane.png` is the active Settings window on the Battery pane
(the `Power Mode` group with the `Low Power Mode` profile picker, the `Battery`
group with `Battery Health`/`Charging` and their `i` details, and the
`Usage History` group with the range switch and the honest absent-state chart
frames), and `t15-6b-battery-control-center.png` is the 360x1040 panel crop with
the new Battery tile (`71% · Balanced`, `Battery Settings…`) above an unclipped
Clipboard tile. The demo runs with `DF_SETTINGS_START_PANE=battery`,
`DF_SETTINGS_FIXTURE`, `DF_BATTERY_FIXTURE`, and `DF_STATUS_FIXTURE`, so no host
settingsd or system-bus daemon is needed. Captured by
`scripts/capture-t15-battery-pane.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`. Vision confirmed the pane's
three groups and all three profile popup values render with the fixture state,
and the Control Center shows the Battery tile and link with nothing clipped.

`t15-7a-notify-adapter.png` is the nested demo desktop captured to prove the
Notifications and Focus adapter (T-15.7a) leaves the tree healthy. The adapter
has no surface of its own (the pane and tile are T-15.7b), so the capture only
confirms the desktop renders: menu bar, Dock, and windows composited with no
blank areas or stray artifacts. Captured by
`scripts/capture-t15-notify-adapter.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

T-15.7b's Notifications and Focus panes and tile: `t15-7b-notifications-pane.png`
is the active Settings window on the Notifications pane (the shell header card,
the `Notification Center` group with the `Show previews` popup and the three
`Show Notifications:` toggles, and the `Application Notifications` inventory),
`t15-7b-focus-pane.png` is the Focus pane (the `Off`/`Focus`/`Do Not Disturb`
selector, the summary, and the `Allowed Apps` toggles), and
`t15-7b-control-center.png` is the 360x1040 panel crop with the Focus tile. The
demo runs with `DF_SETTINGS_START_PANE=notifications|focus`, `DF_SETTINGS_FIXTURE`,
`DF_NOTIFICATIONS_FIXTURE`, and `DF_STATUS_FIXTURE`, so no host settingsd or
notification service is needed. Captured by
`scripts/capture-t15-notifications-pane.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

`t15-8a-lock-adapter.png` is the nested demo desktop captured to prove the
Lock Screen policy adapter (T-15.8a) leaves the tree healthy. The adapter has
no surface of its own (the pane and tile are T-15.8b), so the capture only
confirms the desktop renders: menu bar, Dock, wallpaper, and windows
composited with no blank areas or stray artifacts. Captured by
`scripts/capture-t15-lock-adapter.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

`t15-9a-menubar-adapter.png` is the nested demo desktop captured to prove the
Menu Bar configuration adapter (T-15.9a) leaves the tree healthy. The adapter
has no surface of its own (the pane and tile are T-15.9b), so the capture only
confirms the desktop renders: menu bar, Dock, wallpaper, and windows
composited with no blank areas or stray artifacts. Captured by
`scripts/capture-t15-menubar-adapter.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

`t15-10a-update-adapter.png` is the nested demo desktop captured to prove the
General, About, and Updates adapter (T-15.10a) leaves the tree healthy. The
adapter has no surface of its own (the pane and tile are T-15.10b), so the
capture only confirms the desktop renders: menu bar, Dock, wallpaper, and
windows composited with no blank areas or stray artifacts. Captured by
`scripts/capture-t15-update-adapter.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

`t15-8b-lock-screen-pane.png` and `t15-8b-lock-screen-control-center.png` are
the Lock Screen pane and Control Center tile for T-15.8b. The pane capture
shows the display/password timing rows (the reused `idle.blank`/`idle.lock`
keys), the energy warning, and the four `lock.*` display toggles plus the
`Set...` message button; the Control Center capture shows the Lock Screen tile
below the Battery tile. Captured by `scripts/capture-t15-lock-pane.sh`;
requires a host Wayland session, spectacle, and Pillow, and is not in
`make e2e`.

`t15-9b-menu-bar-pane.png` and `t15-9b-menu-bar-control-center.png` are the
Menu Bar pane and Control Center tile for T-15.9b. The pane capture shows the
behavior rows (auto-hide, background, recent items), the `Clock Options...`
sheet, and the per-control visibility toggles; the Control Center capture
shows the Menu Bar tile below the Lock Screen tile. Captured by
`scripts/capture-t15-menubar-pane.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

`t15-10b-general-pane.png` and `t15-10b-general-control-center.png` are the
General pane and Software Update Control Center tile for T-15.10b. The pane
capture shows the header card, the `About` and `Software Update` disclosure
rows (with the live update summary), and the `About This System` / update
dialogs are asserted headlessly; the Control Center capture shows the Software
Update tile at the bottom of the panel with its state-dependent action link.
Captured by `scripts/capture-t15-general-pane.sh`; requires a host Wayland
session, spectacle, and Pillow, and is not in `make e2e`.

`t15-11a-account-adapter.png` is the nested demo desktop captured to prove the
Users and Groups adapter (T-15.11a) leaves the tree healthy. The adapter has no
surface of its own (the pane and tile are T-15.11b), so the capture only
confirms the desktop renders: menu bar, Dock, wallpaper, and windows composited
with no blank areas or stray artifacts. Captured by
`scripts/capture-t15-account-adapter.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

`t15-11b-users-pane.png` and `t15-11b-users-control-center.png` are the Users &
Groups pane and Control Center Users tile for T-15.11b. The pane capture shows
the user list (avatar, display name, role), the `Add User…` / `Add Group…`
buttons, the `Automatically log in as` popup, and the group list; the tile
capture shows the read-only user summary (`2 Users`) and its settings link.
Captured by `scripts/capture-t15-users-pane.sh`; requires a host Wayland
session, spectacle, and Pillow, and is not in `make e2e`.

`t15-12a-printer-adapter.png` is the nested demo desktop captured to prove the
Printers and Scanners adapter (T-15.12a) leaves the tree healthy. The adapter
has no surface of its own (the pane and tile are T-15.12b), so the capture only
confirms the desktop renders: menu bar, Dock, wallpaper, and windows composited
with no blank areas or stray artifacts. Captured by
`scripts/capture-t15-printer-adapter.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

`t15-12b-printers-pane.png` and `t15-12b-printers-control-center.png` are the
Printers & Scanners pane and Control Center Printers tile for T-15.12b. The pane
capture shows the `Default printer` / `Default paper size` popups, the printer
rows with their state dots and per-printer disclosure, and the scanner list; the
tile capture shows the read-only summary (`2 Printers, 1 Scanner`) and its
settings link. Captured by `scripts/capture-t15-printers-pane.sh`; requires a
host Wayland session, spectacle, and Pillow, and is not in `make e2e`.

`t15-13a-privacy-adapter.png` is the nested demo desktop captured to prove the
Privacy and Security adapter (T-15.13a) leaves the tree healthy. The adapter
has no surface of its own (the pane and tile are T-15.13b), so the capture only
confirms the desktop renders: menu bar, Dock, wallpaper, and windows composited
with no blank areas or stray artifacts. Captured by
`scripts/capture-t15-privacy-adapter.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

`t15-13b-privacy-pane.png` and `t15-13b-privacy-control-center.png` are the
Privacy & Security pane and Control Center Privacy tile for T-15.13b. The pane
capture shows the flat list of portal permission categories (Camera with its
`2 apps` secondary, Location Services, the empty `None` rows) and the
per-category permission dialog; the tile capture shows the read-only summary
(`3 Apps`) and its settings link. Captured by
`scripts/capture-t15-privacy-pane.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

`t15-14a-accessibility-adapter.png` is the nested demo desktop captured to prove
the Accessibility adapter (T-15.14a) leaves the tree healthy. The adapter has no
surface of its own (the pane and tile are T-15.14b), so the capture only
confirms the desktop renders: menu bar, Dock, wallpaper, and windows composited
with no blank areas or stray artifacts. Captured by
`scripts/capture-t15-accessibility-adapter.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

`t15-14b-accessibility-pane.png` and
`t15-14b-accessibility-control-center.png` are the Accessibility pane and
Control Center Accessibility tile for T-15.14b. The pane capture shows the
header card, the live `Vision` card (`Screen Reader` and `Accessibility`
status rows) and the `Motion` card with the `Reduce Motion` toggle; the tile
capture shows the read-only `Screen Reader On` summary and the compact
seventeenth tile in the fixed panel. Captured by
`scripts/capture-t15-accessibility-pane.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

`t15-15a-vpn-adapter.png` is the nested demo desktop captured to prove the
Network advanced (VPN) adapter (T-15.15a) leaves the tree healthy. The adapter
has no surface of its own (the pane and tile are T-15.15b), so the capture only
confirms the desktop renders: menu bar, Dock, wallpaper, and windows composited
with no blank areas or stray artifacts. Captured by
`scripts/capture-t15-vpn-adapter.sh`; requires a host Wayland session,
spectacle, and Pillow, and is not in `make e2e`.

`t15-15b-vpn-pane.png` and `t15-15b-vpn-control-center.png` are the Network
advanced (VPN) pane and Control Center VPN tile for T-15.15b. The pane capture
shows the `VPN` group (`Work VPN` connected, `Home` idle) with the live
connect/disconnect toggles; the tile capture shows the read-only `Work VPN`
summary and the compact eighteenth tile in the fixed panel. Captured by
`scripts/capture-t15-vpn-pane.sh`; requires a host Wayland session, spectacle,
and Pillow, and is not in `make e2e`.

T-16.6a's accessibility audit is `t16-a11y-atspi.txt`, produced by
`scripts/t16-a11y-audit.sh` (`make t16-a11y-audit`): the live AT-SPI tree of
the first-party apps (Settings and Files) plus a keyboard-only walkthrough
whose focus moves and Settings pane selections are read back from AT-SPI over
the compositor's synthetic-input harness. The representative whole-desktop
still is `t16-a11y-desktop.png`; the shell's offscreen chrome is not on the
accessibility bus, and its global keyboard flows are proven headlessly (see
[ADR 0154](../design/adr/0154-atspi-and-keyboard-audit-boundary.md)). Requires
a host Wayland session, python3 with `pyatspi` and Pillow, spectacle, and the
built tree; not in `make e2e`.

T-16.6b's compositor magnifier is `t16-magnifier.png`, produced by
`scripts/capture-t16-magnifier.sh`: a stacked A/B of the nested demo with the
magnifier off (top) and on at 2x, centred (bottom). The bottom half is a
centred zoom of the top — UI enlarged and edge content (the top bar, the Dock)
cropped out of view, with no blank/torn regions; the compositor model and the
synthetic control are proven headlessly (`magnifier::tests`,
`window_conformance::magnifier_reports_its_view_transform_and_zoom`). Requires
a host Wayland session, spectacle, Pillow, and the built tree; not in
`make e2e`.

T-16.7's locale switch is `t16-i18n.png`, produced by
`scripts/capture-t16-i18n.sh` (`make t16-i18n-capture`): a stacked A/B of the
Settings window with `DRAGONFRUIT_LOCALE=en_US` (top, source strings) and
`es_ES` (bottom, `translations/dragonfruit_es.ts`). The Spanish half shows the
window title `Ajustes`, the sidebar pane labels, the search placeholder, and
the General pane header/description/absence message in Spanish; the per-half
stills are `t16-i18n-en.png` and `t16-i18n-es.png`. The catalog parser, locale
resolution, default-`QLocale` formatting, and a real QML `qsTr()` lookup are
proven headlessly (`tst_i18n`), and the string-extraction gate is
`scripts/i18n-extract.py` (`make check-i18n`). Requires a host Wayland session,
spectacle, Pillow, and the built tree; not in `make e2e`.

T-16.8a's crash/kill matrix is `t16-kill-matrix.txt`, produced by
`scripts/t16-kill-matrix.sh` (`make t16-kill-matrix`): a headless reproduction
of every restartable-component kill and its documented recovery. Rows: the
session supervision matrix (`cargo test -p dragonfruit-session --test
kill_matrix`, derived from the shipped `SessionPlan::default_session` — shell,
the five on-failure daemons, the portal backend, an app, and the compositor
anchor), the compositor app-crash test against real Wayland clients, and the
lock UI's fail-secure kill. T-16.8b adds the restart-policy matrix row
(`cargo test -p dragonfruit-session --test restart_policy_matrix`: every policy
crossed with every exit kind, plus compositor death for each exit). No host
session, VM, or real services are needed; the live/VM real-binary half is
recorded by hand in `t16-kill-matrix.md`. The contracts are
[ADR 0157](../design/adr/0157-t16-crash-kill-matrix.md) and, for the
compositor-death outcome and restart policy,
[ADR 0158](../design/adr/0158-compositor-death-ends-the-session.md).

T-17.1a's nested window-loop sign-off is produced by
`scripts/capture-t17-window-loop.sh` (`make t17-window-loop-capture`): the
nested demo runs with the synthetic-input harness and a real third-party Qt
client (`kcalc`, server-side decoration) on the private socket. The driver
performs launch/appear, focus, move (a titlebar drag of exactly (90,70)), zoom
(double-click), minimize (yellow light), restore, and close (red light) on the
Qt SSD window, and writes `t17-window-loop.png` (the representative whole
desktop: the Qt SSD client, the first-party CSD Settings, and the X11
`xmessage` client all mapped), the step stills `-focused`, `-move`, `-zoom`,
`-minimized`, `-restored`, `-closed`, the CSD-unaffected crop `-csd`, the X11
SSD titlebar crop `-x11`, the clip `t17-window-loop.mp4`, and the machine
transcript `t17-window-loop.txt` (the `query decorations`/`query identity`
report around each lifecycle primitive). The restore uses the same compositor
primitive the Dock tile invokes; the Dock-tile geometry itself is covered by the
headless T-01/T-02 suites and the human walkthrough (the agent capture's
Dock-tile pixel diff is ambiguous because an unpinned running app's tile makes
the pinned tiles re-center — see
[ADR 0159](../design/adr/0159-t17-nested-window-loop-capture.md)). The loop's
per-behaviour conformance remains
`traffic_lights_drive_zoom_minimize_and_close`,
`titlebar_drag_and_double_click_move_and_zoom`,
`window_menu_runs_zoom_minimize_close_and_move_to_space`,
`x11_traffic_lights_drive_zoom_minimize_and_close`,
`ssd_toplevel_carries_a_titlebar_and_csd_does_not`,
`decoration_tier_matrix_default_explicit_ssd_and_csd_side_by_side`, and
`milestone_e2e` (all in `make e2e`). Needs a host Wayland session, spectacle,
python3+Pillow, ffmpeg (optional clip), and the built tree; not in `make e2e`.

T-17.1b's nested navigation sign-off is produced by
`scripts/capture-t17-navigation.sh` (`make t17-navigation-capture`): the nested
demo runs with the synthetic-input harness and the driver exercises workspace
switching (keyboard `Ctrl+Left`/`Ctrl+Right`, a three-finger swipe caught
mid-slide, and a pointer click on a Mission Control strip card), Mission
Control (keyboard `Ctrl+Up` and a top-left hot-corner dwell), and app switching
(keyboard `Cmd+Tab` and a pointer click on a live preview) on the live
session, asserting each path through `query spaces`/`query grid`/`query
wallpaper`/`query switcher`. It writes `t17-navigation.png` (the representative
Space 0 desktop), the step stills `-workspace-keyboard`, `-workspace-gesture`,
`-mission-control-keyboard`, `-mission-control-pointer`, `-workspace-pointer`,
`-app-switch-keyboard`, `-app-switch-pointer`, `-app-switch-pointer-committed`,
the clip `t17-navigation.mp4`, and the machine transcript
`t17-navigation.txt`. The harness and its evidence boundary are
[ADR 0160](../design/adr/0160-t17-navigation-capture.md); pointer Mission
Control *selection* is not synthesized (the composition-level round-trip stays
headless-pinned) and is recorded as a follow-up. Needs a host Wayland session,
spectacle, python3+Pillow, ffmpeg (optional clip), and the built tree; not in
`make e2e`.

T-17.1c's live Flatpak/browser sign-off is produced by
`scripts/capture-t17-flatpak-browser.sh` (`make t17-flatpak-browser-capture`):
a private session bus runs the real `xdg-desktop-portal` frontend with the
Dragonfruit backend, the nested demo runs with the synthetic-input harness, and
a real Flatpak browser (`org.mozilla.firefox`) is executed *inside its sandbox*
to issue file-choose (`FileChooser.OpenFile`), screenshot (interactive
`Screenshot.Screenshot`), and screen-share (`ScreenCast.CreateSession` /
`SelectSources` / `Start`). The **live shell** presents and answers each
request (the picker, the selection overlay, the source picker); the driver
clicks/drags each by synthetic input and the run fails unless the client's
portal response returns the file URI, the screenshot URI, or the stream list.
It writes `t17-flatpak-browser.png` (the nested desktop), the step stills
`-file-choose`, `-file-choose-accepted`, `-screenshot`, `-screenshot-accepted`,
`-screen-share`, `-screen-share-accepted`, the clip
`t17-flatpak-browser.mp4`, and the machine transcript
`t17-flatpak-browser.txt` (the client's portal calls and responses). The
harness and its evidence boundary are
[ADR 0161](../design/adr/0161-t17-flatpak-browser-capture.md); the review notes
are in `t17-flatpak-browser.md`. Needs Flatpak + Firefox,
`/usr/libexec/xdg-desktop-portal`, a host Wayland session, spectacle,
python3 (host and sandbox) with PyGObject/Pillow, ffmpeg (optional clip), and
the built tree; not in `make e2e`.

T-17.3's visual-floor and reduced-motion sign-off is produced by
`scripts/capture-t17-visual-floor.sh` (`make t17-visual-floor-capture`): the
nested demo runs once with the synthetic-input harness and
`scripts/t17-visual-floor-driver.py` captures the same chrome under dark, light,
and dark+reduced motion (whole desktop, menu-bar band, compositor SSD titlebar,
Dock band), so the comparison cannot drift between launches. It writes
`t17-visual-floor-dark.png` / `-light.png` / `-reduced.png` and the per-variant
`-menubar`/`-titlebar`/`-dock` crops, the live `query material`/`query degrade`
transcript `t17-visual-floor.txt`, and two review sheets against the
design-system goldens: `t17-visual-floor-gallery.png` (each captured desktop
beside `window_*`, each captured titlebar beside `ssd_*`) and
`t17-visual-floor-menubar.png`/`-dock.png` (the chrome bands across variants).
The reviewed verdict and the recorded approximation waiver are in
`t17-visual-floor.md`; the evidence boundary is
[ADR 0176](../design/adr/0176-t17-visual-floor-sign-off.md). The automated
reduced-motion floor is `compositor/tests/reduced_motion_sweep.rs` and the
gallery `--strict` gate (both in `make e2e`/`make visual-test`). Needs a host
Wayland session, spectacle, python3+Pillow, and the built tree; not in
`make e2e`.

T-12.6c's second-VT dev harness is produced by
`scripts/second-vt-validation.sh` (`make second-vt-validation`). It records the
dedicated-user **refusal** (running `dev --real --plan` for the host user, who
holds the seat) and the **preflight/free-VT selection** against a mocked
`loginctl` (the host desktop on VT2, so `dfdev` gets VT3) in
`t171-second-vt.open.txt`. The one real start → switch → return → teardown cycle
needs a free logind seat and the dedicated development user; on a host with
neither it is **marked open, not skipped** and the artifact stays
`t171-second-vt.open.txt` (a free seat replaces it with `t171-second-vt.txt`).
`t171-second-vt-nested-check.png` is the required nested `make demo` visual
check. The live cycle and the two-TTY stills are on the T-159…T-161 rail; the
pure logic is pinned by `cargo test -p dragonfruit-dev` (`second_vt::tests`) and
[ADR 0175](../design/adr/0175-second-vt-dev-harness.md).

T-16.4's suspend/resume soak is produced by
`scripts/t16-suspend-resume-soak.sh` (`make t16-suspend-resume-soak`). It
writes `t16-suspend-resume-soak.txt` (`t16-suspend-resume-soak.md` is the
reviewed note): one headless compositor session driven through 100
suspend→resume cycles with a live client (scene intact each cycle, clean
teardown leaks nothing) plus the session-supervision 100-cycle soak, both in an
isolated `XDG_RUNTIME_DIR`; the real-machine logind half is recorded **OPEN**,
not skipped, on a host that cannot sleep itself and before the logind backend
lands. The live nested still is `t16-suspend-resume-soak.png`; the contract is
[ADR 0181](../design/adr/0181-t16-suspend-resume-soak.md).

Guidelines:

- Capture from the **nested** session for daily review; add a **DRM** capture
  where the slice touches hardware (T-03, T-12, T-16).
- Keep recordings short (30–90 s) and start from a known state
  (`make demo`).
- For material/visual slices, include light + dark and reduced motion.
- Do not commit large files; prefer a few MB per slice and link to longer
  recordings in the PR/issue rather than this folder.
