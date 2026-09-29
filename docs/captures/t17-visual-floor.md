# T-17.3 Visual floor and reduced-motion sign-off

This is the reviewed companion to the scripted capture in
`docs/captures/t17-visual-floor.*` (`make t17-visual-floor-capture`). The
evidence boundary is
[ADR 0176](../design/adr/0176-t17-visual-floor-sign-off.md); the interface
language is [ADR 0122](../design/adr/0122-tahoe-interface-language-across-chrome.md),
the material pass is
[ADR 0013](../design/adr/0013-backdrop-blur-pass.md), and the Dock's own glass
is [ADR 0102](../design/adr/0102-dock-material-role-and-qml-glass-layers.md).

## What was signed off

| Checklist item | Verdict | Evidence |
|---|---|---|
| Materials (blur/rounding/shadows) present, light | ✅ | `t17-visual-floor-gallery.png` row 2, `t17-visual-floor-light{,-titlebar,-dock}.png` vs `window_light`/`ssd_light`; `query material scheme=light chrome=ffffffff` |
| Materials present, dark | ✅ | `t17-visual-floor-gallery.png` row 1, `t17-visual-floor-dark{,-titlebar,-dock}.png` vs `window_dark`/`ssd_dark`; `query material scheme=dark chrome=2d2534ff` |
| Every animation has a passing reduced-motion variant | ✅ | `reduced_motion_sweep` 3/3; the gallery `_reduced` goldens under `--strict`; the runtime conformance suites in `make e2e` |
| No flat approximations remain in shipped chrome | ⚠️ waived | See the waiver below: no untokened flat fill remains, but the backdrop is the token-driven feather approximation of ADR 0013/0091/0122, with the GPU sampler on the post-gate backlog |

## Materials, light and dark

The live capture runs the nested demo once and captures the same chrome under
dark, light, and dark+reduced motion, so the comparison cannot drift between
launches. For each variant the driver records the compositor's resolved
material tones (`docs/captures/t17-visual-floor.txt`):

```
variant name=dark scheme=dark reduced=0
  material scheme=dark chrome=2d2534ff elevated=2d2534ff border=44394dff accent=e15c98ff
variant name=light scheme=light reduced=0
  material scheme=light chrome=ffffffff elevated=ffffffff border=ded6e5ff accent=b32a66ff
```

The review sheet `t17-visual-floor-gallery.png` pairs each captured desktop with
the design-system `window_*` golden, and the captured compositor SSD titlebar
with the `ssd_*` golden. `t17-visual-floor-menubar.png` and
`t17-visual-floor-dock.png` stack the menu-bar and Dock chrome bands across the
three variants. Rounded corners, the layered translucent chrome fill, the
hairline border, and the soft window shadow read consistently with the goldens
in both schemes; the vision review found no flat/unstyled region, no
blank/black/torn area, and no stray artifact. The design-system goldens
themselves are the art-direction reference and are gated by
`scripts/check-gallery-snapshots.py --strict` (84/84).

## Reduced motion passes

Three independent layers, all green:

- **Structural sweep** — `compositor/tests/reduced_motion_sweep.rs` enumerates
  the generated Rust and QML motion catalogs (20 animations, cross-checked),
  every QML animation site in shipped chrome (57 sites across 95 files), and
  every compositor lifecycle kind (6), and fails if any lacks a reduced-motion
  variant. Run: `cargo test -p dragonfruit-compositor --test reduced_motion_sweep`.
- **Golden end-state** — the gallery's `*_dark_reduced.png` goldens are part of
  the same `--strict` gate, so a component whose reduced variant drifts fails
  it.
- **Runtime behavior** — `window_conformance.rs` / `shell_protocol_conformance.rs`
  step each lifecycle and overview transition one frame at a time under
  reduced motion (`frames == 1`); `make e2e` is green (132 `test result: ok`,
  0 failed).

The live reduced-motion still is **pixel-identical** to the dark still
(`ImageChops.difference(...).getbbox() is None`): disabling animation changes
no static material, so the chrome renders in full under reduced motion.

## Waiver: the backdrop is a token-driven approximation

The T-17 checklist says "no flat approximations remain in shipped chrome". The
old flat/opaque fills are gone — every chrome surface resolves its fill, rim,
and shadow through the `Theme.material` group and the compositor backdrop pass,
and the menu bar is `Theme.color.chrome` over that pass. What remains is the
**flat-tone feather approximation** for the backdrop blur: the compositor
renderer has no GPU texture sampler yet, so the pass draws a stack of
translucent rounded feather layers instead of sampling the live scene
([ADR 0013](../design/adr/0013-backdrop-blur-pass.md),
[ADR 0091](../design/adr/0091-dock-tahoe-floating-glass-language.md)). ADR 0122
makes the token-driven approximation the shipping form and records a true
liquid-glass refraction pass as a future material-track item. This sign-off
therefore **waives the literal item** with that recorded reason: the shipped
chrome is intentional and matches the design-system reference; the GPU sampler
is the post-gate follow-up, behind the same token contract.

## Live visual check

The nested session was launched (`make demo`) and the floor sweep captured it.
`docs/captures/t17-visual-floor-dark.png` (also `-light.png`, `-reduced.png`)
shows the menu bar, Dock, wallpaper, the first-party CSD Settings window, and
the X11 `xmessage` client composited; rounding, shadows, text, and layout read
as intended, with no tearing, black regions, or stray chrome. The gallery and
menubar/dock review sheets confirm the material consistency. The X11 client
draws its own raw-X11 content (a light window in the dark capture); that is the
client's rendering, unrelated to the shell chrome.

Capture paths: `docs/captures/t17-visual-floor-gallery.png`,
`docs/captures/t17-visual-floor-menubar.png`,
`docs/captures/t17-visual-floor-dock.png`, and the per-variant stills.

## Check results

Repo root, with `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig` and
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64`:

- `make t17-visual-floor-capture` — exit 0; the driver reported
  `visual-floor capture complete` for all three variants and wrote the stills,
  the transcript, and the review sheets.
- `./scripts/check-gallery-snapshots.py --strict` — `gallery visual regression
  passed (84 snapshots)`.
- `cargo test -p dragonfruit-compositor --test reduced_motion_sweep` — 3 passed,
  0 failed (20 animations, 57 sites, 6 lifecycle kinds).
- `make e2e` — exit 0, 132 `test result: ok`, 0 failed.
- `make lint` — fails only at `check-desktop-names` on the pre-existing
  StatusNotifier/`org.kde` lines, `scripts/zoo/zoo-run.sh`, and the Makefile
  comment; no new file is flagged. The remaining gates
  (`check-no-capture-grab`, `check-i18n`, `check-phosphor`, plus the `fmt-check`,
  `clippy`, `qml-test`, `check-tokens`, `check-design-tokens` that ran before
  the failure) are green.

## Reproduction

```bash
# host Wayland session, spectacle, python3 + Pillow, ffmpeg (optional), built tree
make t17-visual-floor-capture     # or: bash scripts/capture-t17-visual-floor.sh

# the automated reduced-motion floor
cargo test -p dragonfruit-compositor --test reduced_motion_sweep
./scripts/check-gallery-snapshots.py --strict
```

## Follow-ups

- **True GPU blur/refraction sampler** (the waived item): replace the feather
  layers behind the existing tokens; track on the post-gate backlog with
  Spotlight search, desktop icons, and the other post-gate material items.
- The window menu could not be opened on the live X11/Qt SSD clients during
  this capture (a right-click on the titlebar left `query window-menu` closed);
  the compositor popup material is instead covered by the design-system
  `menu_*`/`popup_*` goldens under `--strict`. Re-check the live window menu on
  a future nav capture.
- The nested capture runs on the development iGPU, whose 16 ms budget drives
  the material ladder to `reduced` (`degrade tier=reduced`); the `full` tier
  still rendered 35 frames. T-17.4 owns the performance verdict.