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

Guidelines:

- Capture from the **nested** session for daily review; add a **DRM** capture
  where the slice touches hardware (T-03, T-12, T-16).
- Keep recordings short (30–90 s) and start from a known state
  (`make demo`).
- For material/visual slices, include light + dark and reduced motion.
- Do not commit large files; prefer a few MB per slice and link to longer
  recordings in the PR/issue rather than this folder.
