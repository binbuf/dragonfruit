# 0368 — Attached menu chain ownership through exit

Status: accepted (2026-11-25)

## Context

An attached menu-bar dropdown is not one card. The parent is an attached
`menuAttached` surface with square top corners; every open submenu is a
detached card in the same popup surface, separated by a transparent gap. The
shell commits the union of the parent and every open submenu as one
`menubar-popup` layer surface ([ADR 0363](0363-menubar-popup-footprints.md)),
and [T354](0365-popup-visual-transaction.md) already retains that surface as one
accepted transaction.

The QML exit was still per card: `Popup` faded and scaled the parent while
`MenuLevelPanel` faded each submenu independently, and `levels.reset()` on the
logical close removed the chain before the visual finished. A dismissal could
therefore show a half-faded parent with full-strength material, drop submenu
cards early, or leave a card's picking region alive after the logical close.
A late provider reply could also truncate the chain mid-exit.

[ADR 0003](0003-shared-animation-clock.md) owns the one animation clock,
[ADR 0367](0367-compositor-popup-group-motion.md) the compositor-owned group
motion, and [ADR 0310](0310-menu-pointer-selection-stability.md) the pointer
selection rules.

## Decision

- **One accepted chain.** `MenuLevels.freeze()` captures the visible levels on
  a whole-chain dismissal; `MenuBarMenu` keeps the parent popup and every
  submenu card open and painted for one closing commit, but disables the
  parent, every `MenuLevelPanel` and the bar's Repeater count. A provider reply
  that lands during the exit cannot re-label, truncate, resize or destroy the
  departing snapshot.
- **One visual transaction.** The shell stages the retained crop, per-card
  material regions and an empty picking footprint, then commits exactly one
  settled closing frame with `df_layer_surface.set_popup_motion(1)`. After that
  commit it stops submitting popup content and keeps the surface mapped until
  the compositor's token finishes. The parent, every submenu card, the
  transparent gaps and the shadow reach share one lifetime.
- **One top-anchored transform.** `PopupMotionOrigin::TopCenter` is the
  `menubar-popup` origin: the accepted paint extent scales about its top edge,
  so the attached parent stays under the bar while the chain leaves. The Dock
  chooser keeps `BottomCenter` (its arrow tip). The chain keeps the measured
  menu close token (183 ms); the Dock chooser keeps its popup token. Content
  and material panels use the same origin, token and group alpha.
- **Two exit policies.** A whole-chain dismissal (mouse, keyboard, focus loss,
  lock, output removal) retains the snapshot. A deliberate navigation-driven
  swap (title switch, hover branch swap, focus-owner change) passes
  `immediate` and releases the old chain before the new one opens. A reopen
  during the compositor tail clears the latch and retargets the same accepted
  surface from its visible alpha.
- **Immediate opening stays QML-owned.** Only the closing commit opts into the
  compositor motion, so the menu's measured open path is unchanged and the two
  owners never animate one frame. Reduced motion collapses the close to zero
  duration and completes without a retained surface.
- **Scope.** Status, tray, overflow and Dock popup families remain T361. The
  persistent Dock plate is a separate surface and is never part of the motion.

## Consequences

- The parent and its previous submenu chain leave on one visual lifetime, with
  no duplicate glass, no per-card fade mismatch and no live invisible targets.
- A stale provider reply is generation-safe: the shell fences the closing
  commit by generation/content serial, and the QML snapshot is frozen.
- Title and hover branch swaps stay responsive; they do not inherit an exit
  tail.
- The QML owner no longer has to run a close animation for the retained chain;
  it only keeps the settled accepted texture for the one commit.

## Verification

- `shell::popup_motion` covers the top-centre origin and the namespace anchor
  choice; `shell::popup` covers the closing transaction and retarget rules.
- `shell_protocol_conformance` covers `menubar-popup` closing with
  `origin=top` and autonomous completion without a further client commit.
- `tst_chromestability` covers a nested three-card chain, title and hover
  branch swaps, a provider refresh during the exit, a rapid reopen, reduced
  motion and the shared dismissal routes.
- `make verify-chrome-stability` is the gate.

[Protocol](../../../protocols/dragonfruit-shell.xml) ·
[Compositor motion](../../../compositor/src/shell/popup_motion.rs) ·
[Menu owner](../../../design-system/components/MenuBarMenu.qml)
