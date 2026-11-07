---
verify: make gates
---
# T206 — Apply restrained glass to Settings and Files chrome

## Goal

Give first-party Settings and Files sidebars/toolbars the observed near-opaque appearance with consistent app-local fallback and readable content.

## Context (read first)

- Depends on T195–T197 and T202. Read the [window evidence](plan/REFERENCE.md#what-to-carry-forward) and sibling `docs/REPORT.md` §§3.6, 3.8.
- `apps/settings/{SettingsWindow,SettingsShell}.qml`, `apps/files/FilesShell.qml`, `design-system/{Material,Theme}.qml`, and `compositor/src/window/decoration.rs`.
- `docs/ipc-versioning.md` and `compositor/src/shell/trust.rs` define the current trust boundary.

## Scope

- [ ] Add app-local sidebar and toolbar presets, using System Settings and Finder as the corresponding references. Bind appearance and accessibility consistently in both app processes. Keep main content and unclassified chrome opaque.
- [ ] Use the measured near-opaque transmission where the existing alpha/composition path supports it. Compare with solid fallbacks on non-composited or unsupported paths. Assess the small measured chroma bleed separately and record any unresolved capability.
- [ ] Preserve window controls, drag/resize, client-vs-server decoration, stacking and focus. Keep the effect clipped to the sidebar/toolbar through resize, window focus and fullscreen.

## Out of scope

- A general client backdrop protocol, theme changes to arbitrary app windows, and new settings or macOS app chrome assets.

## Design notes

Use about 0.986/0.857 for the Light Settings sidebar and 0.961/0.965 for its Light toolbar. Keep Files toolbar and sidebar independent with their observed Dark values. Treat focus chiefly as a controls/text state. Verify a partly transparent first-party surface blends against the compositor's normal lower scene without changing client contents or stacking order. Keep any missing chroma transport as explicit evidence for future work.

## Done when

- [ ] Light and Dark first-party captures over neutral and saturated backdrops document luminance/chroma behavior, contrast, focused/unfocused states, and unsupported/opaque fallback.
- [ ] Settings and Files QML tests and `make gates` pass. Update `docs/design/{08-settings,09-files,10-design-system}.md` where affected and fill in Hand-off.

## Hand-off

### What landed

- **Four app-local presets** in `design-system/tokens/tokens.json` (17 total;
  the four are QML-only because the compositor never draws app chrome):
  `settingsSidebar`, `settingsToolbar`, `filesSidebar`, `filesToolbar`.
  - Settings Light sidebar 0.986/0.857, toolbar 0.961/0.965 (measured).
  - Settings Dark sidebar 0.935/0.218, toolbar 0.900/0.247 (`adapted` from
    the measured Finder pair; System Settings Dark has no independent fit).
  - Files Light sidebar 0.985/0.858, toolbar 0.994/0.922 (measured).
  - Files Dark sidebar 0.935/0.218, toolbar 0.900/0.247 (measured).
  - All four are `role: chrome`, `render: solid`, no blur, no slider.
- **One QML complement.** `Material.appChromeResolution()` /
  `appChromeFill()` / `appChromeBorder()` resolve through `Glass`. The legacy
  rollout keeps the old opaque `surfaceMuted`/`chrome` fills. A
  non-composited/unsupported client (`Glass.appChromeSupported == false`,
  capture seam `DF_APP_CHROME_SOLID=1`) keeps the same opaque plate. Reduce
  Transparency uses the shared opaque accessibility plate. Increase Contrast
  raises the separator ink; focus is a controls/text state and never changes
  the material.
- **Transparent app frame, opaque content.** `AppWindow.chromeTransparent`
  removes the generic frame/content backing behind the sidebar/toolbar, keeps
  an opaque titlebar backing, and the app shells paint the detail sheet (and
  Files path bar) opaque. New `ChromeSheet.qml` draws one fill with independent
  corner radii so the sidebar (bottom-left), detail sheet (bottom-right) and
  path bar (bottom-right) round exactly one window corner without stacking
  alpha.
- **Both app processes bind appearance/accessibility.** Settings already had
  the local Theme/Glass bindings; `SettingsBridge` gained
  `appChromeSupported`. Files gained `FilesSettings` (shared settings client)
  and the same bindings for scheme, accent, reduced motion, Reduce
  Transparency, Increase Contrast, rollout and the client-alpha capability.
  Both shells now expose `windowActive`, bound from `Window.active`, so the
  focused/unfocused titlebar/control state is live.
- **Compositor shadow ring.** `shadow_elements_outside()` subtracts the
  window's rounded body from the drop shadow. Without this, a translucent
  sidebar/toolbar sampled its own shadow and the Dark fit was off by
  ~0.02–0.05. The QML `Shadow` is hidden on the transparent-chrome path because
  the compositor owns the real window shadow.
- **Evidence rail.** `scripts/t206-first-party-chrome.py` (`--check`,
  `wallpaper`, `score`) and `scripts/capture-t206-first-party-chrome.sh`
  (`make first-party-chrome-capture`). `make first-party-chrome-check` is in
  `make gates`. `t21-live-capture.sh` gained `settings-window`/`files-window`
  states, the private-bus full-service app rail, `UNFOCUS`, and `query
  decorations` in the query dump.

### Alpha / solid fallback behavior

| Path | Settings sidebar | Settings toolbar | Files sidebar | Files toolbar |
|---|---|---|---|---|
| Light replacement | 0.986 / 0.857 | 0.961 / 0.965 | 0.985 / 0.858 | 0.994 / 0.922 |
| Dark replacement | 0.935 / 0.218 (adapted) | 0.900 / 0.247 (adapted) | 0.935 / 0.218 | 0.900 / 0.247 |
| Reduce Transparency | 0.98 / 0.97 | 0.98 / 0.97 | 0.98 / 0.97 | 0.98 / 0.97 |
| Dark Reduce Transparency | 0.96 / 0.10 | 0.96 / 0.10 | 0.96 / 0.10 | 0.96 / 0.10 |
| Legacy / unsupported | opaque `surfaceMuted` | opaque `chrome` | opaque `surfaceMuted` | opaque `chrome` |

Main content (`Theme.color.surface`), the Files path bar (`Theme.color.chrome`)
and the titlebar backing stay opaque. The capture scorer samples all four
surfaces: sidebar, toolbar, detail sheet and path bar.

### Chroma gap

The preset transport is a scalar `neutralAlpha`/`neutralTint`; the reference's
small muted-sidebar chroma pickup is **not** representable. The capture report
records the rendered channel spread separately (max 0.0745 on the Dark
saturated row, where the 0.935 alpha transmits 6.5% of the red backdrop) and
marks `chroma.unresolved = true`. Reproducing the pickup needs a future
chroma/tint transport; it is not claimed as a colour match. ADR
[0225](../../design/adr/0225-first-party-app-chrome-presets.md) records this.

### Checks run (final tree)

- `make gates` — **PASS** (glass self-test, 323 preset checks, menu/veil/
  control-center/notification checks, `first-party-chrome-check`, e2e,
  84/84 ctest).
- `make qml-test` — 84/84 passed (includes `tst_design_system`,
  `tst_settings_shell`, `tst_files_shell`, both `qmllint` targets).
- `make first-party-chrome-check` — pass.
- `make first-party-chrome-capture` — 22/22 captures; scorer PASS; max fit
  error 0.0051, max chroma spread 0.0745.
- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor shadow`
  — 10 passed (includes the new shadow-ring and rect-subtraction cases).
- `cargo test -p dragonfruit-compositor --test material_presets` — 9 passed.
- `python3 scripts/material_presets.py --check` — pass;
  `python3 tests/glass/test_material_presets.py` — 323 passed.
- `make check-tokens check-design-tokens` — current/ok.

### Artifacts

- `docs/captures/t206-{settings,files}-{light,dark}-{neutral,saturated}.png`
- `docs/captures/t206-*-{unfocused,reduce,solid,contrast}.png`
- matching `t206-*.txt` query dumps
- `docs/captures/t206-first-party-chrome-capture.json`
- `docs/captures/t206-first-party-chrome.json` / `.md`

### Deviations / remaining work

- Settings Dark is an adaptation from Finder Dark, not an independent System
  Settings fit. A controlled System Settings Dark recapture can replace
  0.935/0.218 and 0.900/0.247 later without touching Files.
- The chroma pickup remains unresolved (see above).
- The shadow-ring change is compositor-wide; it is visually neutral for opaque
  windows (the ring is outside the body) and is covered by the new unit tests.
- Follow-up: a general client backdrop protocol would supersede the app-local
  alpha path; it remains explicitly out of scope for T206.
