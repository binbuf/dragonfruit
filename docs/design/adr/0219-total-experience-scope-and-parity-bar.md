# 0219 — Total desktop experience scope and parity bar

## Status

proposed — defines Track 22 / v0.2.0.

## Context

The T-20/T-21 and liquid-glass work defined the material and presentation path.
The remaining product gap is behavioural: the Dock click tree, menu bar and
extras, Overview/workspaces, app switcher, Search, Applications drawer, System
Settings, animations, HUDs, and the Linux ownership of every setting. Without a
scope decision, every macOS surface becomes an open-ended task and the desktop
widens without becoming coherent.

## Decision

- The v0.2.0 scope is the **core desktop experience**: every always-visible
  shell surface, navigation/window management, Search/launching, Settings,
  transients/feedback, and the cross-distro integration those surfaces need.
- Parity is **behavioural and perceptual**, not a pixel copy. The bar is the
  measured interaction model, information architecture, motion language, and
  quality floor — with original names/assets.
- The source of truth is the capture inventory and the measured artifacts; a
  task may not invent a geometry, timing, label, or state.
- Every surface has a keyboard path, an AT-SPI path, a reduced-motion path, and
  an absence path.
- Apple account services (iCloud, Siri, Apple Intelligence, Screen Time,
  Wallet, Game Center, Find My, App Store) are out of scope. AirDrop/Continuity
  is omitted by default; optional local-sharing integrations are evaluated
  separately.
- A checked task is agent-done, not human-signed-off. Human sign-off is batched
  at phase boundaries.

## Consequences

- `docs/design/22-total-experience.md` is the target; `STATUS.md` records the
  delta.
- `docs/tasks/0.2.0/` is the executable sequence; capture tasks run on the
  macOS host, implementation tasks on Linux.
- New Settings rows require an owner and an absence path; no fake toggles.
- New animations require a measured token and a reduced-motion variant.
- The existing T-21 presentation path and material rollback flags remain in
  force.
- Adding a surface outside the inventory requires a scope update, not a
  drive-by task.
