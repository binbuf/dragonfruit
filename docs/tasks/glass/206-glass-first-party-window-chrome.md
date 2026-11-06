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

_(Implementation session records tokens, alpha/solid fallback behavior and any chroma gap.)_
