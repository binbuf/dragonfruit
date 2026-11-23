# 0330 — Pointer settings reach libinput through one applier seam

- Status: accepted
- Date: 2026-11-23
- Task: T320

## Context

The Keyboard/Mouse/Trackpad pane shipped pointer keys in schema revision 12
and per-device `input.mouse.*` / `input.trackpad.*` keys in revision 37, but
the compositor only stored them in `PointerSettings`. libinput's defaults leave
tap-to-click and natural scrolling off while the shipped defaults are on, so
"not applied" was visible immediately. The input router is backend-generic
(`process_input_event<B>`), while libinput device configuration lives on the
concrete `input::Device` the DRM backend owns, so the application path had to
cross that boundary without leaking libinput types into the compositor state.

## Decision

1. **One pure controller, one hardware seam.** `PointerController`
   (`compositor/src/pointer_policy.rs`) stores pointer devices by libinput name
   and capability kind, resolves the effective settings, and calls a
   `PointerApplier`. The DRM backend installs the libinput applier; tests
   install a recorder. A rejected config call is logged once and the rest of
   the snapshot still runs.
2. **Apply on hotplug and on policy change.** `DeviceAdded` records the device
   and applies immediately; `DeviceRemoved` forgets it and releases the handle;
   `set_pointer_policy`, `set_pointer_family_policy`, and per-device overrides
   re-apply every live device through `apply_input_settings`.
3. **Capability decides the family.** A pointer with touch or gesture support
   is a touchpad; every other pointer is a mouse. The device name is never
   parsed.
4. **The family snapshots travel over a new v15 request.** The shell sends one
   `df_toplevel_manager.set_pointer_family_policy` per family. The compositor
   applies it only to devices of that kind, so the Mouse and Trackpad panes can
   differ. Precedence is: name override, then family snapshot, then the shared
   `input.*` pointer policy (the pre-revision-37 fallback).
5. **Fields without a libinput setter stay at the compositor or ship
   stored-only.** `input.scrollSpeed` scales the one axis path;
   `input.secondaryClick` swaps the primary/secondary button codes at the one
   button path; `input.doubleClickSpeed` feeds the titlebar double-click
   detector. libinput exposes no right/left secondary-button, scroll-speed, or
   double-click setter, and this stack has no haptic control, so
   `input.haptics` is `stored_only: true` and the pane says so. The three
   keyboard-backlight keys are stored-only with the T346 deferred note.
6. **Unify the non-per-device family keys.** The mouse and trackpad secondary
   click, scroll speed, and double-click rows write the shared `input.*` keys
   because an event cannot be attributed to a device at the compositor's
   button/axis path. Their per-family keys remain stored-only.

## Consequences

- A touchpad and a mouse can carry different speed, natural scrolling,
  handedness, scroll method, and acceleration profile; both are applied on
  plug-in, not only at session start.
- The shared `input.*` pointer keys remain live as the fallback for an older
  shell and for a store upgraded from before the family keys.
- The hardware tap/natural-scroll check stays `OPEN` on a host without a free
  seat; `compositor/tests/input_settings.rs` pins the mapping and re-apply.
- A future haptic or per-device scroll implementation can extend
  `PointerConfig` and the applier without changing the controller contract.
