# 0224 — Notification banner, stack and lower panel are three materials

- Status: accepted
- Date: 2026-11-06
- Task: T205

## Context

The banner and the Notification Center were both on the generic Popup plate.
The reference captures measure three different effective materials: the
banner (Light 0.549/0.976), the Center stack (Light 0.551/0.706, Dark
0.507/0.550), and the Center's lower panel (Light 0.407/0.852). The Dark
banner and Dark lower panel have no independent fit, and the lower-panel Light
residual (RMSE ~0.044) is materially larger than the banner's (~0.009). A
banner can also be co-visible with the open Center, so two translucent surfaces
must not compound in the same pixels.

## Decision

- **Three presets, one flat-band family.** `notificationBanner`,
  `notificationStack` and `notificationLowerPanel` are resolved through the
  shared T195/T196 policy. On the replacement rail the compositor builds them
  with the same measured flat-band path as the Dock/Control Center, using the
  resolved own-layer alpha/tint directly rather than the generic Popup adaptive
  tint. Each region is opaque, so a panel is never frosted twice.
- **The banner region is the visible card.** `notification` declares one
  `banner` region sized to the 96 pt / 132 pt card, not the transparent strip
  below a no-action banner. The card's shadow is QML-owned only on the
  solid/software paths; on the sampled path the opaque compositor material is
  the plate.
- **The Center is two disjoint regions.** `notification-center` declares
  `stack` (`notificationStack`) and `lower` (`notificationLowerPanel`), separated
  by the panel gap. The stack contains the title and scrollable history; the
  lower panel is a fixed footer with the count/summary and the keyboard/scroll
  hint. Both are commit-latched with the surface image and cleared on close.
- **Co-visibility is geometric.** When a banner is active the shell insets the
  Center's stack below the banner card. The two trusted surfaces stay
  co-visible without overlapping, so the banner cannot sample or cover the
  Center's own content.
- **Dark banner/lower panel are labelled adaptations.** The Dark stack keeps
  the measured 0.507/0.550 fit. The Dark banner (0.56/0.30) and lower panel
  (0.40/0.30) are readable, distinct, underconstrained variants; their token
  confidence is `adapted`, and capture rows carry the same label.
- **Fallbacks and accessibility stay shared.** QML is transparent on
  `sampled-glass`, paints the same resolved tint/alpha on `solid`,
  `software-fallback` and Reduce Transparency, and uses
  `Material.popupFill()` only for the legacy rollout. Increase Contrast raises
  the resolved rim; Reduce Motion does not change optics.

## Consequences

- The three materials are independently keyed, cached and damage-scoped. A
  banner or history change re-declares only the affected region set.
- The Center's empty state, populated history and overflowing scroll state are
  deliberate: the empty text lives in the stack, the lower panel always carries
  the summary, and a visible scrollbar/hint appears only when rows overflow.
- The measured Light fits and Dark stack are reproducible within the capture
  scorer; Dark banner/lower rows are explicitly not claimed as exact fits.
- A future widget/calendar lower panel can replace the footer content without
  changing the region key or the material policy.
