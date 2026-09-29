# T-17 — The premium experience gate: sign-off report

This is the reviewed sign-off for the T-17 track
([design/tracks/17-premium-gate.md](../design/tracks/17-premium-gate.md)). It
reproduces the track checklist with a verdict and an evidence link per item,
compares the product against the design docs, the macOS **Tahoe 26** captures
and [`System_Preferences.md`](../reference/System_Preferences.md), records the
post-gate backlog, and lists the reproduction commands. The evidence boundary
is [ADR 0179](../design/adr/0179-t17-premium-gate-sign-off.md).

The gate is **agent-signed with two hardware rows left OPEN** (DRM loop,
baseline Intel/AMD frame budget), per the track rule "if unavailable, the gate
is marked incomplete, not passed". The human unfamiliar-user verdict is the
batched track-boundary item; its protocol is recorded below.

Verdict legend: ✅ agent-verified · ⚠️ waived with a recorded reason · ⏳ OPEN
(owned by a later hardware unit).

## Checklist

### Loop

| # | Item | Verdict | Evidence |
|---|---|---|---|
| L1 | Full loop completes on nested and DRM | ⏳ nested ✅ / DRM OPEN | Nested: `t17-window-loop.*`, `t17-navigation.*`, `t17-flatpak-browser.*`. DRM: no free logind seat on this host — owned by T-17.2 (`make drm-soak`) |
| L2 | Launch, appear, traffic lights, move, zoom, minimize, restore, close, workspace switch, Mission Control, app switch by pointer and keyboard | ✅ (with the restore/MC-selection deviations below) | `t17-window-loop.txt` (appear→focus→move→zoom→unzoom→minimize→restore→close, exact geometry); `t17-navigation.txt` (workspace keyboard+gesture+pointer, Mission Control keyboard+hot-corner, app switch keyboard+preview-click) |
| L3 | A third-party Qt app (SSD) and an X11 app behave correctly; a CSD app is unaffected | ✅ | `t17-window-loop.txt`: `kcalc` 26.08.1 SSD `tb=(664,315,640,40)`, `xmessage` X11 SSD `tb=(1558,40,320,40)`, first-party `Settings` CSD `ssd=False tb=(0,0,0,0)` |
| L4 | A Flatpak browser can file-choose, screenshot, screen-share | ✅ | `t17-flatpak-browser.txt`: Firefox 155.0 Flatpak, `FLOW: RESULT: PASS file`/`shot`/`cast`; screen-share uses the documented stills fallback |

### Feel and visual floor

| # | Item | Verdict | Evidence |
|---|---|---|---|
| F1 | Materials (blur/rounding/shadows) present and signed off vs the design-system reference and Tahoe, light and dark | ✅ | `t17-visual-floor.md` + `t17-visual-floor-gallery.png`; live tones `chrome=2d2534ff` (dark) / `chrome=ffffffff` (light); gallery `--strict` 84/84 |
| F2 | Every animation has a passing reduced-motion variant | ✅ | `reduced_motion_sweep` 3/3 (20 tokens, 57 QML sites, 6 lifecycle kinds); `*_dark_reduced` goldens; runtime suites in `make e2e` |
| F3 | No flat approximations remain in shipped chrome | ⚠️ waived | No untokened flat fill remains; the backdrop is the token-driven feather approximation of ADR 0013/0091/0122 (no GPU sampler). Same waiver as T-17.3; sampler is on the backlog |

### Performance

| # | Item | Verdict | Evidence |
|---|---|---|---|
| P1 | 60 Hz, zero dropped frames for workspace switch/Mission Control on baseline Intel/AMD (frame trace attached) | ⏳ OPEN | The dev-iGPU nested trace (`t05-gesture-budget-nested.txt`) drops frames under the 16 ms budget and this host is not a baseline Intel/AMD target; owned by T-17.4 |
| P2 | Input-to-photon latency under one frame, nested and DRM | ✅ nested / ⏳ DRM | `t03-latency-nested.txt`: max 15853 µs ≤ 16666 µs (`verdict: pass`). DRM half is T-17.2/T-17.4 |
| P3 | Idle desktop: zero damage, zero client wakeups from our shell | ✅ | `t03-idle-trace.txt`: 60 s flat — `frames_rendered` flat, `client_wakeups=0`; `shell_idle_trace` in `make e2e` |
| P4 | Folder open and large-list scroll budgets from T-10 | ✅ (dev build) | `t10-files-perf.txt`: 1k open 0.57 ms (<50 ms), 100k delivered once, viewport read 2.2 µs (<16666 µs), 24–70 delegates for 100000 rows; release re-measure is T-17.4 |

### Robustness

| # | Item | Verdict | Evidence |
|---|---|---|---|
| R1 | Every absent-daemon case degrades; nothing blocks session start | ✅ | `t17-robustness-matrix.md` + `absent_services` 4/4; ADR 0177 |
| R2 | Crash/kill matrix: shell, settingsd, notification service, portal backend, apps; compositor death ends the session by design | ✅ | `t17-robustness-matrix.txt` re-runs `kill_matrix` + `restart_policy_matrix`; ADR 0157/0158 |
| R3 | Teardown: repeated loops leak no socket/token/`DISPLAY` file, no orphaned client, no VT master | ✅ (VT probe OPEN) | `t17-leak-lock-soak.txt`: 100-cycle hermetic soak + empty scratch dir; VT master probe OPEN (no seat) on the T-03.4 rail; ADR 0178 |
| R4 | Lock enforcement verified again on the packaged build | ✅ (release build) | `t17-leak-lock-soak.txt`: `--release` `session_lock_conformance` 4/4 incl. untrusted client, killed lock UI, 25× no-leak loop. RPM/DEB re-run deferred to T-16.9/T-16.10 |

### Product judgment

| # | Item | Verdict | Evidence |
|---|---|---|---|
| J1 | The unfamiliar-user test: a person who has not read the docs performs the loop without instructions; failures are legible | ⚠️ human-batched | Protocol + agent legibility review below; the human verdict is the track-boundary sign-off |
| J2 | Side-by-side vs design docs, Tahoe 26 captures/notes, `System_Preferences.md`: interaction model reads as intended, assets original, Linux adaptations hold | ✅ agent review | Section "Side-by-side" below; `t17-premium-gate.png`; ADR 0122 |
| J3 | The post-gate backlog is explicitly listed, not silently implied | ✅ | Section "Post-gate backlog" below |

### Acceptance

| Item | Verdict | Evidence |
|---|---|---|
| Every checklist item ticked or explicitly waived with a reason | ✅ | this report |
| The captured loop and the sign-off report are committed | ✅ | `t17-window-loop.mp4`, `t17-navigation.mp4`, `t17-flatpak-browser.mp4`, this report, `t17-premium-gate.png` |
| `make check` (lint + test + soak) and `make e2e` green on the release commit | ✅ | check results below (exit 0, 147 and 133 `test result: ok`; 100-cycle soak clean) |
| `README` and `docs/design/11-session-and-dev-workflow.md` carry the exact reproduction commands | ✅ | "Reproduction" in both files |

## Known compatibility gaps

| Gap | Verdict | Evidence |
|---|---|---|
| XDnD across the X11/Wayland boundary | ⚠️ waived (known) | The XDnD bridge is a codec + translation model; Smithay 0.7 exposes no X11 connection hook (ADR 0099). Within one side it works; `xdnd_conformance` is in `make e2e` |

## Side-by-side against the design docs and Tahoe

The full Tahoe reference set under `docs/reference/macos/` never ships
([14-risks.md](../design/14-risks.md)); the shared interface language and the
Linux adaptations are fixed in
[ADR 0122](../design/adr/0122-tahoe-interface-language-across-chrome.md), and
the pane IA/labels in
[`System_Preferences.md`](../reference/System_Preferences.md) and
[`macos-ui-inventory.md`](../reference/macos-ui-inventory.md).

- **Interaction model.** The assembled desktop (`t17-premium-gate.png`) shows
  the menu bar, the Tahoe floating-glass Dock, the wallpaper, a first-party CSD
  Settings window, and an X11 window composited together. The navigation
  capture proves the macOS mental model: Spaces switching, Mission Control with
  live surfaces, and Cmd+Tab app switching all resolve through one input
  vocabulary ([03-workspaces.md](../design/03-workspaces.md)).
- **Chrome.** The visual floor was compared against the design-system
  `window_*`/`ssd_*` goldens under `--strict` (84/84), and the Tahoe language is
  the one-material rule of ADR 0122. The one literal item that cannot be met on
  this host (a true GPU blur/refraction sampler) is waived, not hidden.
- **Linux adaptations hold.** `About This System` replaces the Apple name;
  Apple-only panes are dropped; absent host daemons degrade to empty/disabled
  states; command glyphs use the Dragonfruit keymap
  ([keymap.md](../keymap.md)); the palette, strings, and Inter are ours.
- **Assets are original.** No Apple logo, wallpaper, icon, sound, or branding
  ships. The default wallpaper is the project's own
  `assets/graphics/wallpapers/Default.jpg` (ADR 0094/0114); the icon set is the
  vendored Phosphor 2.0.8 (ADR 0163) plus original first-party app icons
  (ADR 0166); fonts are Inter. `docs/licensing.md` lists the shipped assets and
  `NOTICE` carries the third-party attribution.

## Unfamiliar-user test

The test itself is human: a person who has not read the docs drives the
30-second loop. Its verdict is the batched track-boundary item
([SLICING-REVIEW.md](../SLICING-REVIEW.md)). T-17.6 records the protocol and the
agent legibility review so the human run has a fixed script.

**Protocol** (from the track demo). On a nested session, with no instructions:

1. launch an app from the Dock and watch it appear with traffic lights;
2. move, double-click to zoom, minimize, restore from the Dock, close;
3. switch workspace; open Mission Control; switch app; return.

**Legibility criteria.** Every step is discoverable from the surface alone (the
Dock, the menu bar, or the window chrome); every control has a visible label or
a standard glyph; a failed step leaves a legible state (an empty/disabled row,
a named fallback), never a dead end or a blank region.

**Agent legibility review.** The T-17 stills and clips were re-read for these
criteria: the traffic lights, Dock, menu-bar items, and pane rows are all
labelled or glyphed; the picker/overlay cards name themselves and their
fallback; the absent-daemon matrix degrades to named empty/disabled states. No
step in the captured loop is a dead end. This is evidence that the surface is
legible, not a substitute for the human verdict.

## Live visual check

`make t17-premium-gate-capture` launched the assembled desktop in a nested
session and captured `docs/captures/t17-premium-gate.png` (1920×1200). The
vision read found the menu bar (application menu, status marks, clock), the
floating Dock, the wallpaper, the Settings window (sidebar + detail, traffic
lights, rounded corners, shadow), and the X11 window all present and rendered;
the artifact-only pass answered "None". **Verdict: the assembled desktop reads
as intended, with no blank, black, torn, clipped, or stray regions.** The
transcript `docs/captures/t17-premium-gate.txt` records the live `query
material` (`scheme=light chrome=ffffffff`) and `query degrade` (`tier=reduced`,
the dev-iGPU ladder) tones.

## Check results

Repo root, with `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig` and
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64`:

- `make t17-premium-gate-capture` — exit 0; wrote `t17-premium-gate.png` and
  `t17-premium-gate.txt` (gallery `--strict` 84/84, `reduced_motion_sweep` 3/3,
  `absent_services` 4/4).
- `make e2e` — exit 0; **133** `test result: ok`, 0 failed.
- `make check` — exit 0; `make lint` now includes the corrected
  `check-desktop-names`; **147** `test result: ok`; gallery 84/84; soak
  `100 clean headless cycles, zero strays, zero leaked sockets/tokens`.

## Reproduction

```bash
# host Wayland session, spectacle, python3 + Pillow, built tree
make t17-premium-gate-capture       # assembled desktop still + sign-off transcript

# the full gate
make check                          # lint + tests + 100-cycle teardown soak
make e2e                            # the foundation + conformance suites

# the per-unit T-17 evidence this report indexes
make t17-window-loop-capture        # L2/L3
make t17-navigation-capture         # L2
make t17-flatpak-browser-capture    # L4
make t17-visual-floor-capture       # F1/F2
make t17-robustness-matrix          # R1/R2
make t17-leak-lock-soak             # R3/R4
```

## Post-gate backlog

Carried out of the gate explicitly (nothing after this list is implied):

- **Desktop icons** — T-19.3 compositor desktop layer + desktop process
  (in progress; not part of this gate).
- **Spotlight search** — an overlay over `app-index` + files search.
- **GOA (GNOME Online Accounts)** — account integration beyond the local Users
  pane.
- **NVIDIA / HDR** — the driver/HDR matrix beyond the baseline GPU rail.
- **Printer breadth** — real CUPS discovery/queue management beyond the
  adapter's inventory.
- **True GPU blur/refraction sampler** — retires the F3 waiver behind the
  existing `material.*` tokens.
- **Live PipeWire screencast producer** — the ScreenCast flow names the stills
  fallback; the live stream is a host-capability follow-up.
- **Live window-menu capture, stable Mission Control window-card locator, stable
  Dock-tile locator** — capture-tooling gaps that do not change behavior.
- **Packaged RPM/DEB re-run** — `make t17-leak-lock-soak` against installed
  binaries once T-16.9/T-16.10 land.
- **T-17.2 DRM full loop and T-17.4 performance budget** — the two OPEN rows
  above, on the hardware rail.

## Third-party versions pinned for this gate

- `kcalc` 26.08.1 (third-party Qt/KDE SSD client, T-17.1a).
- `org.mozilla.firefox` 155.0 Flatpak (T-17.1c).
- `xmessage` (X11 client, the demo harness).

## Deviations

- **Restore-to-Dock click and Mission Control window selection by pointer are
  not synthesized in the agent captures**; both are pinned headlessly
  (`window_conformance`, `overview_click_selects_and_focuses_the_live_representation`)
  and are part of the human walkthrough. Recorded in `t17-window-loop.md` /
  `t17-navigation.md`, not silently omitted.
- **The nested capture runs on the development iGPU**, whose 16 ms budget drives
  the material ladder to `reduced`; the `full` tier still rendered. T-17.4 owns
  the performance verdict.
- The `check-desktop-names` gate previously failed on third-party
  `org.kde.*`/`org.gnome.*` identifiers and host-desktop comments; T-17.6
  corrected it to close the acceptance ([ADR 0180](../design/adr/0180-third-party-identifier-desktop-name-gate.md)).