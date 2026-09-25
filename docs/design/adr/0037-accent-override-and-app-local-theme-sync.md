# 0037 — Accent override in the Theme, and app-local Theme sync

## Status

accepted

## Context

ADR [0033](0033-theme-binding-single-writer.md) gave the shell one writer of
`Theme.dark`/`Theme.reducedMotion`, but left `appearance.accent` unconsumed:
`Theme.color.accent` was a read-only scheme token, so honoring the override
needed a design-system change. T-09.2 (the Appearance pane) is that change.

The Settings app is a **separate process** from the shell, so the shell's
`ThemeBinding` cannot reach the app's `Theme` singleton. Before this task the
app followed the host style hint, so flipping Appearance inside the app did
not restyle the app itself.

## Decision

- **The generated `Theme` gains a writable `accentOverride`.** The generator
  (`scripts/gen-tokens.py`) emits the accent family (`accent`, `accentHover`,
  `accentMuted`, `accentContent`) of each scheme as bindings over a single
  top-level `accentOverride` (empty = the token accent), deriving hover,
  muted, and on-content (`accentContent` by luminance) from it. No component
  changes: every existing `Theme.color.accent*` accessor follows.
- **Two writers, one per process.** The shell's `ThemeBinding` sets
  `Theme.accentOverride` next to `dark`/`reducedMotion` (ADR 0033 stays: no
  second bus connection). The Settings app sets the same property on its own
  `Theme` through QML `Binding`s in `SettingsShell.qml` that read the shared
  `Settings` singleton — the app-local counterpart of `ThemeBinding`.
- **`accentMuted` is a derived tint**, so a custom accent never leaves a
  stale magenta selected-row/status background. Rejected: overriding only the
  base accent (leaves `accentHover`/`accentMuted`/`accentContent` stale); a
  per-component override (every consumer would have to know about it).

## Consequences

- Any first-party app that renders settings controls is expected to mirror
  `appearance.colorScheme`/`appearance.accent`/`accessibility.reduceMotion`
  onto its own `Theme` (the shell cannot do it cross-process). The
  `SettingsShell` bindings are the copyable pattern.
- `Theme.accentOverride` is a public design-system surface; the gallery and
  tests may assign it directly like `Theme.dark`.
- The compositor does not consume accent (its chrome roles use the scheme's
  `accent` token directly); only the QML `Theme` is overridable.