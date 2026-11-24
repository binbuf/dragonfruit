# 0341 — A closed modal is input-disabled; opacity is render-only

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-23 |
| **Companions** | [0266](0266-settings-component-contracts.md), [24-system-settings.md](../24-system-settings.md) |

## Context

`Dialog` and `Sheet` keep their card and scrim in the scene graph for the
close fade. Their root used `visible: opacity > 0`, but the root's opacity was
never bound, so it stayed at 1. Only the card and scrim animated to opacity 0.
An invisible card still hit-tests: its `ScrollView`, buttons, and controls
were above the pane and consumed presses in the card rectangle.

System Settings instantiates many dialogs closed and adopts them into the
window overlay. The centered closed cards therefore shadowed a fixed
rectangle in every pane. A click on a row, Select, toggle, radio, or list
inside that rectangle was consumed; a click outside it worked. Because the
detail view scrolls under a resting pointer, controls moved into and out of
the dead rectangle, and the failure read as intermittent: "clicks work
sometimes when scrolling, but most dropdowns and controls do nothing."

The component suite missed the leak because it clicked standalone controls or
the shell's top rows. The stationary-click regression test also failed once a
closed modal was placed in its host, proving the overlap.

## Decision

1. **A closed modal is input-disabled.** `Dialog` and `Sheet` set
   `enabled: root.open`. The card/scrim fade stays render-only; a closed
   subtree never owns the pointer, wheel, or keyboard.
2. **An open modal owns the window.** The scrim carries a `MouseArea` that
   consumes presses and wheel. The card is declared above it, so its own
   controls keep their targets.
3. **The regression is executable.** `tst_settings_components` places a
   `Select` under a closed `Dialog` and `Sheet`, activates it on a stationary
   click, then opens the Dialog and asserts the modal owns the pointer.

## Consequences

- Pane controls are never shadowed by a closed dialog or sheet, regardless of
  the card's position or the pane's scroll offset.
- A closed modal is absent from input delivery immediately on close; the fade
  is visual only. This also removes its invisible subtree from wheel and
  keyboard routing.
- Clicking outside an open modal is consumed by the scrim instead of leaking
  to the pane behind it.
- A future modal-like component must follow the same contract: `enabled` is
  the input gate, not `opacity` or `visible`.
