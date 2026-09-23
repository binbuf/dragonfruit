# T-19 — Desktop Icons (Files-Owned Desktop Surface)

| | |
|---|---|
| **Phase** | 3 (later feature — schedule after Files MVP) |
| **Area** | `apps/files/` (`dragonfruit-files --desktop`) + `compositor/` (desktop layer) |
| **Depends on** | [T-17](17-files-core.md) · [T-18](18-files-app.md) · [T-07](07-private-shell-protocols.md) (trusted desktop-layer surface + launch token) · [T-14](14-hot-corners-desktop-background.md) (Desktop Reveal) |
| **Blocks** | — |
| **Estimate** | L |
| **Design docs** | [09-files.md](../../design/09-files.md) · [04-shell.md](../../design/04-shell.md) · [02-compositor.md](../../design/02-compositor.md) |

## Summary

Desktop icons done the macOS way — **the file manager owns the desktop**:
`~/Desktop` as an icon view with exactly the same core, semantics, and
context menus as a Files window, running as **its own process**
(`dragonfruit-files --desktop`) on a compositor desktop-layer surface. The
MVP ships plain wallpaper; this ticket is explicitly "later" but its
contracts (trusted-process admission, reveal interplay) are decided now.

## Background

Desktop icons belong to Files, rendered through a compositor desktop-layer
surface; the private layer protocol admits a fixed set of trusted session
processes by launch token — the shell, and this surface
([02-compositor.md](../../design/02-compositor.md), [09-files.md](../../design/09-files.md)).
Desktop Reveal must work regardless and later exposes icons
([04-shell.md](../../design/04-shell.md)).

## Scope

### In scope

1. **Compositor desktop layer**: a dedicated layer below chrome and normal
   windows, above the per-Space wallpaper; admission by launch token —
   `bind` from any other client refused (extends T-07's trust model to the
   Files desktop surface).
2. **`dragonfruit-files --desktop` process**: shares `files-core`, not the
   browser process — a Files crash never takes the desktop, and vice versa.
3. **Icon view over `~/Desktop`** with exactly the same semantics and
   context-menu affordances as a Files window (double-click activate,
   rubber-band, inline rename, drag to Dock Trash, spring-load into
   folders/windows, the full context-menu engine).
4. **Desktop Reveal integration**: reveal (T-14) exposes background and
   icons; selection/focus transfer rules between the desktop surface,
   chrome, and windows.
5. **Windowed-app interplay**: files dragged from windows to the desktop
   land via the ops engine; deletion from the desktop appears in Trash
   (GVfs, same source of truth).
6. **Preferences**: icon size/grid on the desktop (separate from folder
   prefs but same persistence patterns).

### Out of scope

- Wallpaper rendering (compositor-owned, T-05/T-14).
- Any desktop-widget system (clock/widgets on the desktop) — not planned.

## Requirements

- FR-1: The desktop surface is admitted by token; untrusted clients cannot
  bind (extend T-07's refusal matrix).
- FR-2: `--desktop` is independently killable/restartable; browser crash
  leaves the desktop alive and vice versa.
- FR-3: Desktop icon operations behave identically to a Files window
  (ops-engine-only mutations, optimistic UI, same conflicts/undo).
- FR-4: Desktop Reveal exposes icons with correct z-order and interaction.
- FR-5: Zero polling while idle (monitor-driven like all of files-core).
- FR-6: Multi-monitor: `~/Desktop` spans outputs consistently — spec:
  per-output icon layouts derived from one logical desktop (decide and
  document: shared single grid vs per-output).

## Acceptance criteria

- [ ] Trust matrix extended and passing.
- [ ] Crash isolation test (kill browser, kill desktop independently).
- [ ] Reveal + drag-to-Trash + context-menu walkthrough passes.

## Test plan

- Restart/crash matrix; token refusal tests; DnD matrix reusing T-18
  harness against the desktop surface.

## Risks / open questions

- Per-output desktop grid policy needs a decision before implementation;
  single logical grid per Space is the least surprising — verify with
  hotplug migration behavior from T-05.
- Scheduling: explicitly after Files MVP; do not let it delay Phase-3
  exit.
