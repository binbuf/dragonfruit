# 0180 — The desktop-name gate ignores third-party identifier namespaces

## Status

accepted

## Context

`scripts/check-desktop-names.sh` guards the public
`XDG_CURRENT_DESKTOP=dragonfruit` contract
([ipc-versioning.md](../../ipc-versioning.md)): no source, build, or script file
may hardcode another desktop's name. The gate scans every `.rs`/`.cpp`/`.h`/
`.qml`/`.sh`/`.xml`/`.yml`/`.toml`/`Makefile`/`.desktop`/`.conf` file for the
words `gnome|kde|plasma|…` with `grep -wiE`.

That match is too broad. Real third-party compatibility surfaces legitimately
carry those tokens as **well-known names**:

- `org.kde.StatusNotifierItem` / `org.kde.StatusNotifierWatcher` — the
  StatusNotifier tray protocol `app-index` implements (T-14.3);
- `org.gnome.Calculator.desktop` — a real Flatpak app id in the zoo
  (T-14.6a) and an example in `shell/src/apppicker.h`.

The gate had been failing on these since the tray and zoo work, and every task
recorded it as a pre-existing red rather than weaken a contract gate. As the
gate-closing unit, T-17.6 must leave `make check` green, so the false positives
are fixed instead of waived.

## Decision

- **Strip third-party namespaces before matching.** The scan replaces
  `org.<desktop>.<member>` (e.g. `org.kde.StatusNotifierItem-1-1`,
  `org.gnome.Calculator.desktop`) with nothing, then applies the existing
  word-boundary match. The namespace list is the same desktop list the gate
  already knows.
- **A bare desktop-name literal still fails.** `"KDE"`, `XDG_CURRENT_DESKTOP=GNOME`,
  `hyprland`, and a comment naming another desktop still trip the gate; the
  `df-allow-desktop-name` marker remains the escape hatch.
- **Comments that merely name the host or a reference desktop are reworded**
  (`the KDE Wayland session` → `the host Wayland session`, `GNOME Calculator` →
  `Calculator`), so the gate stays strict about names rather than growing a
  comment exemption.
- **Fix the one real hardcode the gate found**: `scripts/zoo/zoo-run.sh` set
  `XDG_CURRENT_DESKTOP=Dragonfruit` (capital D). The contract is lowercase
  `dragonfruit`; the zoo env is corrected.

## Consequences

- `make check` is green again, so the T-17.6 acceptance can be met and CI
  stops carrying a permanently red gate.
- A future compatibility surface may add `org.<name>.*` identifiers without
  editing the gate; a new desktop name in the alternation list is still a
  deliberate change.
- The gate no longer distinguishes a quoted string from an identifier for
  third-party namespaces, which is the intended scope: the contract governs
  `XDG_CURRENT_DESKTOP` values, not D-Bus/app-id namespaces.