# ADR 0283 — Settings keys carry a stored-only flag and a semantic rule

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-16 |
| **Task** | T282 |
| **Companions** | [24-system-settings.md](../24-system-settings.md) §3.7, [08-settings.md](../08-settings.md), `docs/design/settings-keys.md` |

## Context

The schema declared a key's D-Bus type, default, range, and enum, plus free
text owner/consumer strings. That was not enough to keep three promises:

1. A text key could hold `7am`, `blue`, or `relative.jpg` even though every
   consumer expected `HH:MM`, `#rrggbb`, or an absolute path.
2. Many shipped controls wrote keys no process read; the consumer text often
   said “stored-only” but nothing enforced it, and a new key could name a
   module that does not exist.
3. The Settings app's client mirror was hand-maintained, so a wrong type or a
   missing enum silently became a rejected live write (two real drifts were
   found: `input.mouse.doubleClickSpeed` and `menu.overflow`).

## Decision

- `KeySpec` gains `semantic: KeySemantic` and `stored_only: bool`.
  `KeySemantic` covers `None`, `TimeOfDay`, `Color`, `AbsolutePath`,
  `ModuleList`, `DayList`, `LayoutList`, `ReplacementList`, `BoundedText(n)`,
  and `Shortcut(n)`. `validate()` runs the semantic after type/enum/range.
- Non-finite numbers are a typed `SettingsError::NonFinite`, checked before any
  comparison. The JSON writer returns `None` instead of panicking.
- A `stored_only` key is an honest preference with no shipped reader. A schema
  test requires every non-flagged key's consumer to start with one of the
  process/module roots the tree builds (`shell/`, `apps/settings`,
  `apps/files`, `compositor/`, `session/`, `services/wallpaperd`,
  `dragonfruit-system-status`, `settingsd`). Consumer-less keys (search
  service, input method, dictation delegate, backup backend, deferred shortcuts,
  login window style, and the T281 additions) are flagged.
- `dragonfruit-settingsd --print-schema-json` is the machine-readable schema
  (key, type, default, enum, min/max, stored-only, since, owner, consumer).
  `tst_settings_schema_mirror` compares the client's
  `settingsSchemaDefaults()`/`settingsSchemaInfo()` tables against it; the two
  drifts above now fail CI.
- The Settings map keeps the same `storedOnly` flag per row, so a live row and
  a stored-only key cannot disagree silently.

## Consequences

- A key that becomes live must change its `stored_only` flag, its consumer
  string, the map row, and add a consumer test in one change.
- A new semantic list must extend `KeySemantic`, the daemon test, and the
  client mirror test's expected dump. The client does not duplicate the
  semantic check; a bad value is still rejected by the daemon and reverted
  through the one write path.
- The dump is a build artifact of the daemon binary. A standalone Qt-only tree
  skips the mirror test with a message; the Rust workspace tests always cover
  the daemon half.
- Stored-only is not a verdict on the preference's usefulness; it is a promise
  that the pane says what applies today.
