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
`edgeMargin` gap to the screen edge are all visible (measured: plate 75 px,
gap 8 px on every edge). Needs a host Wayland session, spectacle, and Pillow;
not in `make e2e`.

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

Guidelines:

- Capture from the **nested** session for daily review; add a **DRM** capture
  where the slice touches hardware (T-03, T-12, T-16).
- Keep recordings short (30–90 s) and start from a known state
  (`make demo`).
- For material/visual slices, include light + dark and reduced motion.
- Do not commit large files; prefer a few MB per slice and link to longer
  recordings in the PR/issue rather than this folder.
