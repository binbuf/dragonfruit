# T208 — native DRM acceptance and rollout gate

**Status: OPEN.** The native baseline is re-verified, but the replacement
native rows and the native rollback cycle are not available in this session.
The shipped glass rollout default therefore remains `legacy`; the flip is a
follow-up for an operator on a free DRM seat.

- Date: 2026-11-06 23:48 – 2026-11-07 00:01 PST
- Host: `Linux fedora 7.2.13-200.fc44.x86_64 x86_64`
- Session: native Dragonfruit, `XDG_SESSION_TYPE=wayland`, `seat0`, VT2,
  logind session 2
- GPU: `04:00.0 AMD/ATI Lucienne [1002:164c] rev c1`, `amdgpu`,
  `card1`/`renderD128`
- Output: `DP-1` connected, 3840×2160 @ 143.999 Hz, mode source, VRR on;
  logical 2560×1440 at `display.scale=1.5`
- Build: installed release symlinks. The running compositor is
  `target/release/dragonfruit-compositor` mtime 2026-11-05 20:59 — **pre-T195/
  T196** (no `glass rollout` log line; settingsd schema 17; no
  `appearance.transparency`, `accessibility.reduceTransparency` or
  `accessibility.increaseContrast` keys).

The replacement source (HEAD, T207) is a newer build than the running native
session. The compositor is the session anchor: replacing it ends the session
(ADR 0158/0053), and this host has no free VT/second seat or operator login.
The native replacement row therefore cannot be produced from inside this
session. The T207 nested matrix is the replacement evidence; this report is the
matched native baseline and the explicit rollout gate.

## Capture method

Native stills are the **real KMS scanout** read back with the T191b method:

1. `pidfd_open` + `pidfd_getfd` duplicates the running compositor's DRM fd.
2. `drmModeGetResources` finds the active CRTC; `drmModeGetFB2` reads the
   primary plane (AB24, AMD three-plane DCC modifier).
3. The three dma-buf planes are imported through EGL, sampled with a GLES
   shader, and read with `glReadPixels` into a PPM/PNG.

This is not a screencopy client, not a portal grant, and not a nested window.
It covers the active CRTC's primary plane only; the hardware cursor and overlay
planes are not in the stills. The dev-only tool is
`scripts/t208-kms-readback.c`; it is not built or shipped by default.

The fullscreen row used the compositor's trusted shell protocol
(`scripts/t208-shell-fullscreen.c`): the shell service was stopped, the
compositor reissued its launch token, the dev-only client authenticated as the
shell and sent `df_toplevel.fullscreen` / `unfullscreen` to a named toplevel.
The shell was restarted afterwards. This is the same private protocol the
shipped shell uses, not a synthetic pointer or a nested session.

The raw `SIGUSR1` lines, resource samples and command transcripts are in
[`t208-native-stats.txt`](t208-native-stats.txt).

## Artifacts

| Artifact | What it shows |
|---|---|
| `t208-native-light-desktop.png` | native light desktop, menu bar + Dock |
| `t208-native-dark-desktop.png` | native dark desktop |
| `t208-native-light-live-under-dock.png` | white moving X11 panel under the Dock (light) |
| `t208-native-dark-live-under-dock.png` | white moving X11 panel under the Dock (dark) |
| `t208-native-light-live-under-menubar.png` | white panel under the menu bar (light) |
| `t208-native-dark-live-under-menubar.png` | white panel under the menu bar (dark) |
| `t208-native-menu-light.png` | native window/menu surface over the desktop |
| `t208-native-control-center-light.png`, `-dark.png` | native Control Center |
| `t208-native-notification-light.png`, `-dark.png` | native notification banner |
| `t208-native-mission-control.png` | native Mission Control grid and Spaces strip |
| `t208-native-lock.png` | native lock surface, no client content |
| `t208-native-fullscreen.png` | output-sized vkcube client with chrome suppressed |

The stills are 1280×720 review copies; the menu/panel crops are native-resolution
crops. The 4K originals are not committed.

## Native visual results

### Live backdrop sampling

A white override-redirect X11 panel was moved behind the chrome while the
wallpaper and scheme stayed fixed. Mean luminance (0..255) of the full-4K
chrome crop:

| Scheme | Surface | Baseline | Under white panel | Delta |
|---|---|---:|---:|---:|
| light | menu bar | 224.9 | 238.8 | +13.9 |
| light | Dock | 80.9 | 169.7 | +88.8 |
| dark | menu bar | 48.6 | 74.9 | +26.3 |
| dark | Dock | 42.5 | 135.0 | +92.5 |

This proves the native sampled path is alive and samples a changing client,
not a static wallpaper. It is the **legacy** material (the running build
predates the replacement), so it is the matched old-hardware baseline for the
T207 nested replacement A/B, not replacement proof.

### Light/dark, foreground and accessibility

- Light and dark desktop, menu bar, Dock, Control Center, notification banner,
  Mission Control and lock were reviewed at native 1.5×. Text and icons remain
  legible; no clipped, black or washed-out chrome was observed in the stills.
  No automated contrast score is claimed for the native rail.
- `appearance.colorScheme` and `accessibility.reduceMotion` are the only
  glass-relevant keys the native build exposes. The native settingsd rejects
  `appearance.transparency`, `accessibility.reduceTransparency` and
  `accessibility.increaseContrast` with `UnknownKey`; those replacement
  overrides are covered by the T207 nested matrix, not this native build.
- Opacity/blur transitions: the live-under stills show the material responding
  to a changing client. Time-resolved Dock sweep/menu-open transitions are the
  T207 nested rows; native input injection is not available on this seat.

### Menus and panels
The macOS reference slider and the other missing Mac measurements remain
marked in the T207 report; T208 adds no new Mac measurements and does not
re-score the reference surfaces.


The native shell fixtures captured the window/menu surface, Control Center,
notification banner, Mission Control and lock. The status-menu and Dock
chooser/hover fixtures were no-ops in this pre-T202 build; they are not claimed
as native evidence. The Dock sweep itself was not driven natively (no pointer
injection); T207 owns the nested sweep.

## Native performance and resources

`make native-status` reports the release profile selected and installed and all
session units active. `make native-capture` wrote
`~/.local/state/dragonfruit/native-captures/20261106-234833-420595`;
`make native-perf` wrote `20261106-235222-422306` and its probe measured
`vkcube: 120 frames in 377 ms = 318.30 fps` (client loop, not panel pacing).

### Refresh and presentation

| Window | p50 | p95 | p99 | Missed | Notes |
|---|---:|---:|---:|---:|---|
| idle 60 s | 6.946 ms | 1.002 s | 1.006 s | +63 | 1 Hz idle VRR intervals, not dropped 144 Hz frames |
| load 35 s (vkcube) | 6.947 ms | 12.716 ms | 13.157 ms | +984 | budget 6.944 ms, 143.999 Hz |

`refresh stats`: `refresh_hz=143.999 source=mode interval_us=6944
deadline_us=6944 outputs=1 override=0 vrr=on`. The actual refresh deadline is
used; the idle tail is the compositor's 1 Hz idle cadence. Under continuous
vkcube load the p50 sits at the budget and the p95/p99 tail is about 1.8× the
budget, so the tail is not clean at 144 Hz on this release. The workstation
diagnostic targets (<2 ms Dock, <4 ms Dock+bar) are **not comparable** here:
native GPU timer queries are disabled (`gpu stats: enabled=0,
gpu_time=unavailable reason=disabled`), so only the compositor CPU frame time
is available.

### Frame work, material and degrade

| Window | frame p50 | frame p95 | frame p99 | material path | blur draws | allocations |
|---|---:|---:|---:|---|---:|---:|
| idle 60 s | 1.969 ms | 4.938 ms | 8.747 ms | `path=gpu` | 85 758 | 232 |
| load 35 s | 6.179 ms | 8.400 ms | 8.662 ms | `path=gpu` | 101 498 | 235 |

Under load `blur_panels=202 702`, `backdrop_skipped=0`,
`blur_miss_scene=8 057`, `blur_miss_source=564`, `blur_miss_spec=24`,
`blur_miss_chain=0`. The shell's role material lines under load are
`chrome/dock/popup/drawer tier=0 blur=1 fallback=0`: the persistent Dock and
chrome keep the sampled floor while the load is applied. `degrade stats` ended
at `tier=full`; the cumulative `minimal` count includes the earlier fixture
and fullscreen runs. Allocations moved 232 → 235 (+3) and did not grow
unbounded.

### Transport, scanout and cache

- `chrome stats`: `chrome_shm_imports=3 178`, `chrome_shm_bytes=7 050 848 616`,
  `chrome_dmabuf_imports=0`, `chrome_frame_callbacks=3 060`. The native release
  is the **SHM** transport; dmabuf is the opt-in GPU rail exercised by
  `make t21-gpu-rollout` (PASS) and the T207 nested rows.
- `scanout stats`: `direct_scanouts=0`, `direct_scanout_candidates=0` in the
  normal desktop. Fullscreen advanced candidates to 11 261 but
  `direct_scanouts` stayed 0 (see below).
- Cache churn is bounded: the `blur_miss_*` counters advance by small amounts
  per window and `material_allocations` is flat after the client exits.

### Idle wakeups and fd/memory trend

Idle (60 s): `frames_rendered` +63, `animation_frames_stepped` +0,
`client_wakeups` +126, `chrome_frame_callbacks` +1, `chrome_shm_imports` +1.
The shell chrome produced **1 frame in 60 s**; the remaining frames are the
Konsole cursor blink. Compositor fd count stayed 93, RSS 115 896 → 117 880 KB
(+1 984 KB); shell fd stayed 15, RSS 232 616 KB flat.

Load (35 s): compositor fd 96 → 107 during vkcube → 96 after; RSS 117 888 →
121 216 KB (+3 328 KB); shell fd 15 → 19 → 15; RSS 232 820 → 236 164 KB
(+3 344 KB). `material_allocations` 232 → 235. This is bounded client/material
churn, not an unbounded leak; the peak `VmHWM` is 328 588 KB.

## Fullscreen and direct scanout

Fullscreen entry works: the trusted shell driver entered fullscreen on the
vkcube toplevel, the KMS still shows the client covering the output with the
menu bar and Dock suppressed, and `unfullscreen` returned to the desktop after
the shell restarted. The scanout counters, however, show the eligible client
was **composited, not scanned out**:

```text
during fullscreen: direct_scanouts=0 direct_scanout_candidates=4019
after unfullscreen: direct_scanouts=0 direct_scanout_candidates=11261
```

The default fullscreen path therefore does **not** direct-scan out when
eligible on this rail. This is the known T190d/T192 gap, not a new regression
from the replacement; it still blocks the native rollout gate. A deliberately
visible shell overlay while fullscreen was not exercised natively (the shell
was stopped for the driver); T207 covers overlay composition in the nested
matrix.

## Lock and privacy

`DF_LOCK_FIXTURE=2500` locked the session; the compositor logged
`session locked (fail-secure)` and `session lock covers 1 output(s)`. The KMS
still `t208-native-lock.png` shows only the lock surface, with no client
content. The first timed unlock did not complete before the shell restarted;
the recovery run (`DF_LOCK_FIXTURE=1000 DF_LOCK_UNLOCK_MS=8000`) logged
`session unlocked` and restored the desktop. Native lock entry/privacy is
evidenced; **native lock-exit is OPEN** because the running pre-T207 shell
lacks the current retry fix. The nested lock/lock-exit rows pass.

## Version compatibility, fallback and v0.1.1 issues

- The native session is a pre-T195/T196 release. The replacement native visual,
  native settings overrides, native transport and native rollback render
  therefore cannot be exercised without rebuilding and restarting the session.
- The current source's T207 nested matrix is the replacement evidence: 50 rows,
  46 pass, 4 approved adaptations, 16/16 A/B pairs pass; `make
  nested-acceptance-check` PASS. Dock flat-band mean 0.0053, p95 0.0039,
  SSIM 0.9495 at 1×; legacy rail mean 0.0862.
- `make t21-gpu-rollout` PASS: Dock/menu/popovers/lock on
  `render-control+dmabuf`, SHM fallback with `gpu_reason=flag-off`; the fresh run passed and the
  committed T207 transport artifacts were left unchanged.
- The prior GPU-overview teardown report (T192b) was checked with a shortened
  full-rail run (`CHROME_GPU_SURFACES=all IDLE_S=2 SWEEP_S=2 SKIP_TIERS=1
  SKIP_NATIVE=1`). All 14 main scenarios, including Mission Control and
  Mission Control close, completed with no `TypeError`/`SIGSEGV` in the shell
  log. The run was stopped by a 300 s timeout during the refresh matrix, so the
  full teardown path was not re-confirmed; keep T192b's finding as a T193
  watch item rather than attributing it to the glass change.
- Software/driver/cross-GPU fallback: the native seat is single-GPU,
  normal-transform; no cross-GPU or rotated variant is available. The nested
  forced software/SHM/Minimal rows pass. Native fallback remains OPEN.
- Hotplug/mixed refresh: one connected output (`DP-1`), so the native
  hotplug/mixed-refresh row is unavailable. T207 covers the output-change row
  in the nested rail.
- Cursor/overlay planes: not in the KMS readback.

## Rollout decision and rollback

The shipped default remains `legacy`:

```text
design-system/tokens/tokens.json glass.rollout.defaultMode = "legacy"
compositor Rollout::default() = Legacy
```

`cargo test -p dragonfruit-compositor --lib
material_preset::tests::default_rollout_is_the_existing_appearance` and
`cargo test -p dragonfruit-compositor --test material_presets
rollout_defaults_to_the_existing_appearance` pass.

The rollback/selection route is now persisted by the native session helper
(`scripts/native-session.sh`), so it survives `native-rebuild`:

```bash
DRAGONFRUIT_GLASS_ROLLOUT=legacy make native-rebuild       # rollback
DRAGONFRUIT_GLASS_ROLLOUT=replacement make native-rebuild  # pre-rollout A/B
```

The configuration half was exercised with `rebuild --no-build --no-install
--no-logout`: `DRAGONFRUIT_GLASS_ROLLOUT="legacy"` appeared in
`~/.config/dragonfruit/native.env` and both the compositor and shell unit
drop-ins, then the original files were restored byte-for-byte and
`systemctl --user daemon-reload` ran. The native render cycle that reads the
value is **OPEN** (it needs the session restart).

## Required native rows still OPEN

The following rows require the current build installed and a fresh native
session. Missing: a free DRM seat (or VM/second VT) with `card1`/`DP-1` and an
operator login; this session owns DRM master and cannot be restarted without
ending the agent session.

1. Native replacement Light/Dark stills over a changing client, with the
   replacement shell/compositor.
2. Native replacement per-output p50/p95/p99, missed presentations, transport
   mix, copied bytes, GPU timing, fullscreen entry/exit and direct scanout.
3. Native accessibility overrides (`Reduce Transparency`, `Increase Contrast`,
   slider) and the glass settings keys.
4. Native lock-exit, multi-GPU/rotated fallback, hotplug/mixed refresh.
5. Native default flip and rollback render cycle.

Exact rerun on a free seat / after the operator restarts the session with the
current build:

```bash
cd /home/user/repos/binbuf/dragonfruit
make native-status
make native-capture
make native-perf
make build   # generates build/protocols/* for the shell driver
# KMS stills (dev-only):
gcc -O2 -D_GNU_SOURCE -o /tmp/df-kms-readback scripts/t208-kms-readback.c \
    $(pkg-config --cflags --libs libdrm egl glesv2)
/tmp/df-kms-readback --info
/tmp/df-kms-readback --capture /tmp/df-native.ppm
# fullscreen / direct scanout (dev-only shell driver):
gcc -O2 -D_GNU_SOURCE -Ibuild/protocols -o /tmp/df-shell-fullscreen \
    scripts/t208-shell-fullscreen.c \
    build/protocols/dragonfruit-core-protocol.c \
    build/protocols/dragonfruit-toplevel-protocol.c \
    $(pkg-config --cflags --libs wayland-client)
systemctl --user stop dragonfruit-shell.service
TOKEN=$(cat /run/user/$UID/dragonfruit-wayland.launch-token)
WAYLAND_DISPLAY=dragonfruit-wayland /tmp/df-shell-fullscreen "$TOKEN" vkcube 14
systemctl --user start dragonfruit-shell.service
# rollback after the flip:
DRAGONFRUIT_GLASS_ROLLOUT=legacy make native-rebuild
```

The native gate closes only when those rows pass with no new presentation,
resource, fullscreen, scanout or lock regression. Until then the default stays
`legacy`.
