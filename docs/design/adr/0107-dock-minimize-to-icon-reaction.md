# 0107 — The minimize-to-icon reaction rides the bounce phase map

## Status

accepted

## Context

T-14.7s adds an opt-in acknowledgment bounce when a window minimizes. The
existing launch/attention bounce already solved the hard part: transient
per-entry animation must not mutate the entry model, because reassigning the
Repeater model recreates delegates and drops hover/press state (ADR 0100). The
question was where to detect a minimize and how to carry it.

A minimize is not an event the compositor signals to the shell directly; it is a
state change in the next running-window projection (a window enters `minimized`,
or the owning entry's minimized count increases). The reaction is opt-in
(`dock.minimizeReaction`, default off) and must disappear under reduced motion.

## Decision

- **Detection is a pure projection comparison in the shell.** The merged Dock
  entries carry a per-window `windowList`; `dockMinimizedCounts` extracts each
  app entry's minimized count and `dockMinimizedPulses` returns the entry ids
  whose count increased, in entry order. The per-window `minimized` rows are
  skipped so the owning app is never double-counted. The first projection only
  seeds the counts, so a Dock that starts with minimized windows does not pulse.
- **A pulse is one entry id and a start time.** Rapid minimizes on the same
  entry restart the clock (coalesced into the last). The one-shot clock is a
  pure function (`dockMinimizeReactionPhase`, `kMinimizeReactionMs`).
- **The pulse rides `bouncePhases` as a separate field.** Each value may also
  carry `{ minimize: true, minimizePhase: 0..1 }`. `Dock.qml` renders it as its
  own one-hop bounce with the `component.dock.minimizeReaction.amplitudeRatio`
  token, so it never fights a launch/attention hop for the same entry; the entry
  model is untouched.
- **It is opt-in and reduced-motion-aware.** The shell only starts pulses when
  the key is on, and `Dock.qml` returns zero translation under reduced motion;
  the running indicator/state stay the legibility source.
- **No new surface and no compositor protocol.** The compositor's minimize
  ghost remains the motion owner; the reaction is a Dock-local acknowledgment.

## Consequences

- Later Dock animation units extend the same `bouncePhases` map rather than
  mutating entries; the map is now the home for launch, attention, and minimize
  phases.
- The neighbor lateral ripple (the reference's second mode) is deferred; if it
  is added it must be its own token-gated field and must not rebuild the model.
- `kMinimizeReactionMs` mirrors `component.dock.minimizeReaction.duration` in
  the token source; the QML amplitude reads the token, the shell owns the clock.

## References

- [04-shell.md](../04-shell.md) "Dock minimize-to-icon reaction".
- ADRs [0100](0100-dock-motion-phase-map-and-popover-buffer.md),
  [0103](0103-dock-window-management-and-entry-affordances.md),
  [0005](0005-minimize-restore-motion-and-ghost.md).
- `docs/tasks/110s-t-14.7s-dock-minimize-to-icon-reaction.md`.