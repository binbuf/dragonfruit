# 0001 — Titlebar double-click setting lives in the compositor until T-08

## Status

accepted

## Context

`dock.titlebarDoubleClick` (`zoom` | `minimize` | `none`, default `zoom`) is
declared by the shell's `DockSettings` and consumed by the SSD titlebar's
double-click handler, which lives in the compositor (T-01.3). There is no
settings channel from the shell to the compositor yet: T-08 introduces
`settingsd` as the one owner of desktop settings, and the compositor has no
private-protocol setting for this key today. The compositor needs a value to
dispatch the gesture now without inventing a throwaway protocol.

## Decision

The compositor owns a `TitlebarDoubleClick` field on `DfState` (default
`Zoom`) and exposes `set_titlebar_double_click`. T-08 will make `settingsd`
the authority and push the value into that field; the field is the single
compositor-side consumer, not a second owner of truth. Until then the
synthetic-input harness accepts `set titlebar-double-click <mode>` so
conformance tests can exercise the configured behavior. We rejected reading
the shell's QSettings file from the compositor (wrong owner, cross-process
file coupling) and rejected waiting for T-08 (leaves the gesture
untestable).

## Consequences

- T-08 wires `dock.titlebarDoubleClick` from `settingsd` into
  `DfState::set_titlebar_double_click`; it must not add a second consumer.
- The synthetic `set titlebar-double-click` command is test plumbing, not a
  session feature; it is only available when the synthetic-input socket is
  bound.
- The default remains `Zoom`, matching `DockSettings`, so behavior is correct
  before T-08 lands.
