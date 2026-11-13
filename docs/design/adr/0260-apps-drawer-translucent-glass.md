# 0260 — Applications drawer uses a translucent sampled-glass card

- Status: accepted
- Date: 2026-11-13
- Task: T248

## Context

ADR [0259](0259-apps-drawer-scroll-material-and-pacing.md) gave the
Applications drawer its own compositor-owned `appsDrawer` glass preset, but set
it to a near-opaque **0.97** alpha because the large browsing surface had to
stay legible. The structural review in `macos-design-review` §4 describes the
same surface as a **large light glass panel**, and the T-19.2 reference palette
sampled from the drawer capture (`#B3B3B3` over black) resolves to roughly
**70–74 % opaque**. The shipped card therefore read as a flat near-opaque plate
rather than the glass used by menus, the Dock, Control Center and notifications.

## Decision

- **Lower the sampled card's coverage, keep the glass.** The `appsDrawer` preset
  now resolves to **0.72 / 0.96** in light (the reference panel reading) and
  **0.82 / 0.26** in dark (the light transmission with the measured dark
  detached-menu tint, raised slightly so 13 pt labels keep contrast over a
  bright wallpaper).
- **Keep the existing optical family.** The 14 pt light / 16 pt dark frost,
  0.22 / 0.20 rim, 1 px edge, 8 pt tail, 0.12 sharp fraction and restrained
  corner lens are unchanged. This is a transmission change, not a new material.
- **The QML card stays transparent on the sampled path.** The compositor still
  owns the blur, rim, lens and adaptive tint; QML owns only the content and the
  fallback plate.
- **The legacy fallback plate also keeps a little transparency.**
  `panelFallbackOpacity` drops from 0.97 to **0.92**, matching the popup-family
  fallback, so the legacy rollout's no-blur path does not snap back to a
  near-opaque slab. The replacement no-sampled-blur, software and Minimal paths
  keep the shared opaque accessibility plate so the card stays readable when
  the compositor cannot sample the backdrop.
- **No new surface or role.** The drawer remains the `Drawer` material role and
  the `appsDrawer` optical preset; only the calibration values and their
  provenance change.

## Consequences

- The wallpaper and windows are visible through the card on the sampled path,
  while the blurred backdrop and near-white/dark tint keep the grid and labels
  readable.
- The light reading is anchored to the reference panel gray; the dark reading
  is explicitly adapted because no dark drawer fit was measured.
- The design-system, compositor and shared Python preset tests pin the new
  alpha/tint pair and the continued sampled-glass render mode.
