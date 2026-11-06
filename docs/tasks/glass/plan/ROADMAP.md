# Dragonfruit Liquid Glass roadmap

[Workstation evidence and integration map](REFERENCE.md) · [Harness setup](../README.md)

Use the existing private shell surfaces and compositor material paths. Stage the new materials behind one internal rollout switch, available to fixtures and native verification. Preserve the current presentation, resource and interaction guarantees while calibrating the new look. T208 controls default enablement.

## Phase 1 — Evidence, presets, and settings

- [x] T194 — Pin reference evidence and capture a baseline → [194-glass-reference-and-baseline.md](../194-glass-reference-and-baseline.md)
- [x] T195 — Define role presets and material resolution → [195-glass-presets-and-resolution.md](../195-glass-presets-and-resolution.md)
- [x] T196 — Connect Appearance and Accessibility to the compositor → [196-glass-live-settings.md](../196-glass-live-settings.md)
- [x] T197 — Extend compositor material geometry and composition → [197-glass-panel-composition.md](../197-glass-panel-composition.md)

## Phase 2 — Dock and menu surfaces

- [x] T198 — Calibrate Dock optics → [198-glass-dock-optics.md](../198-glass-dock-optics.md)
- [x] T199 — Match Dock magnification and tooltip behavior → [199-glass-dock-motion.md](../199-glass-dock-motion.md)
- [x] T200 — Render the no-blur menu-bar veil → [200-glass-menubar-veil.md](../200-glass-menubar-veil.md)
- [x] T201 — Connect menu-bar visibility settings → [201-glass-menubar-visibility.md](../201-glass-menubar-visibility.md)
- [x] T202 — Apply glass to dropdowns, context menus, and status menus → [202-glass-menu-families.md](../202-glass-menu-families.md)

## Phase 3 — System panels and first-party windows

- [x] T203 — Distinguish Control Center panel and module materials → [203-glass-control-center.md](../203-glass-control-center.md)
- [~] T204 — Map the Notification Center and route its actions → [204-glass-notification-center-integration.md](../204-glass-notification-center-integration.md) ⟵ running
- [ ] T205 — Calibrate notification banner, stack, and lower panel → [205-glass-notification-materials.md](../205-glass-notification-materials.md)
- [ ] T206 — Apply restrained glass to Settings and Files chrome → [206-glass-first-party-window-chrome.md](../206-glass-first-party-window-chrome.md)

## Phase 4 — Acceptance and rollout

- [ ] T207 — Verify the replacement in nested and fallback sessions → [207-glass-nested-acceptance.md](../207-glass-nested-acceptance.md)
- [ ] T208 — Verify native DRM and enable the replacement by default → [208-glass-native-rollout.md](../208-glass-native-rollout.md)

<!-- symphony:status -->
**Pipeline status** — updated 2026-11-06T23:12:57Z · 10/15 done

- Completed: T194, T195, T196, T197, T198, T199, T200, T201, T202, T203
- Blocked: none
- Failed: none
- Remaining: T204, T205, T206, T207, T208
- Last finished: T203 — done · Landed distinct Control Center outer/module/expanded materials, region metadata, accessibility fallbacks, 14 captures with separate outer/nested errors, tests, ADR and docs; make gates passes.
<!-- /symphony:status -->
