# 0016 — The live light/dark color scheme is compositor state on `DfState`

## Status

accepted

## Context

T-04's materials (SSD titlebar, window menu, chrome backdrop, window shadow)
all resolve their tones from a `ColorScheme`, but until T-04.4b every call site
stamped `ColorScheme::default()` (dark). T-04.4b must render both schemes
correctly and package the track sign-off, and T-08 (`settingsd`) will become the
owner that mirrors `appearance.colorScheme` into the session. The open question
was where the live scheme lives so the render path, the settings seam, and the
conformance tests agree, without threading a scheme argument through every
render function or adding a second material source.

## Decision

- The live scheme is a field on `DfState` (`color_scheme: ColorScheme`,
  default `ColorScheme::Dark`). It is compositor state, not a render argument.
  `DfState::set_color_scheme` flips it and requests a redraw; the settings
  mirror (T-08) and the synthetic-input `set color-scheme light|dark` command
  both call it.
- `titlebar_element`, `titlebar_element_for_motion`, and `open_window_menu`
  stamp the live scheme onto the `TitlebarElement`/`WindowMenu` they build, so
  interaction hit-testing and rendering read the same resolved object. The two
  material passes (`render::chrome_backdrop_render_elements`,
  `render::window_shadow_render_elements`) read `state.color_scheme` when they
  resolve tokens. No token value is duplicated: the generated
  `design_tokens` group remains the single source (ADR 0011/0013).
- `ColorScheme::name`/`parse` define the design-system spelling
  (`light`/`dark`) used by the settings seam and the synthetic command.
- The scheme is observable headless: a `scheme stats` render-stats line and the
  `query material` synthetic reply (scheme plus resolved
  chrome/elevated/border/accent tones), so both schemes are asserted without a
  GPU. Reduced motion stays the shared `AnimationClock`/`OverviewMachine`
  policy and composes with either scheme.

## Consequences

- T-08 sets one field; T-05/T-11 material passes read one field. There is one
  scheme owner and one place a material learns its tones.
- Material colors are no longer compile-time constants in the render path, so a
  scheme flip marks the output dirty and repaints; the existing damage path
  owns the cost.
- The default stays dark until settingsd connects, which reads over arbitrary
  client pixels; a session that never sets a scheme is unchanged.
- A future per-output or per-window scheme (e.g. forced-dark chrome) would need
  a lookup keyed on that scope; the field is deliberately session-wide for now.