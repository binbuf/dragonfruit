# T-29 — Clipboard Manager and polkit Authentication Agent

| | |
|---|---|
| **Phase** | 5 · Desktop infrastructure |
| **Area** | `shell/` (clipboard UI) + `services/` (agent) |
| **Depends on** | [T-02](02-compositor-core.md) (`wlr-data-control`) · [T-08](08-design-system.md) · [T-07](07-private-shell-protocols.md) · [T-20](20-system-service-adapters.md) (A12) |
| **Blocks** | Daily-driver bar (clipboard; auth UI consistency) |
| **Estimate** | M |
| **Design docs** | [07-system-integration.md](../design/07-system-integration.md) · [04-shell.md](../design/04-shell.md) · [02-compositor.md](../design/02-compositor.md) |

## Summary

Two small but load-bearing desktop utilities: (1) a clipboard manager
backed by `wlr-data-control` seeing text, images, and files — the shell's
window into the shared clipboard; (2) the shell-hosted **polkit
authentication agent**, so privileged operations from Settings, Control
Center, and our services surface one consistent design-system prompt
instead of a toolkit default.

## Background

The compositor implements `wlr-data-control` "so the shell's clipboard
manager sees text, images, and files"
([02-compositor.md](../design/02-compositor.md)). The shell hosts the
polkit authentication agent "so privileged operations requested by
Settings, Control Center, and our services surface one consistent,
design-system prompt" ([04-shell.md](../design/04-shell.md)). Privileged
operations are delegated to system services behind polkit — nothing of
ours runs as root, and we never roll our own privilege escalation
([07-system-integration.md](../design/07-system-integration.md)).

## Scope

### In scope — clipboard

1. `wlr-data-control` client: observe selection + primary selections;
   offer/serve on behalf of the manager's history.
2. History UI (small popover/Control-Center slot): recent entries with
   previews (text snippet, image thumb, file list), search, pin, clear;
   keyboard-navigable; AT-SPI roles.
3. Formats: text (incl. UTF-8 oddities), images (common formats), and
   **files** (URI lists) — the daily-driver clipboard bar
   ([13-roadmap.md](../design/13-roadmap.md)) lists "text, images, files."
4. Size/privacy policy: cap history size and entry size; clear on
   lock/unlock (configurable); no cloud anything.

### In scope — auth agent

1. polkit agent registration on the session bus; render prompts for
   Settings (e.g., admin actions), Files (`admin://` GVfs mounts, T-17),
   and our services' privileged requests.
2. Design-system Dialog/Sheet styling, identity selection where polkit
   offers choices (admin vs user), error states, and reduced-motion.
3. Never our own privilege scheme: agent only relays PAM/polkit results
   (A12 adapter rules).
4. Restartable — the system falls back to another agent if ours is absent;
  we ship ours so the default look is consistent
  ([04-shell.md](../design/04-shell.md)).

### Out of scope

- Clipboard sync between devices — not planned.
- Any privilege escalation logic itself.

## Requirements

- FR-1: Clipboard round-trip matrix: text (plain + formatted variants),
  image (PNG screenshot), files (from Files, to Firefox), including
  cross-Xwayland (with T-06) — all pass.
- FR-2: History persists across shell restart (best-effort); cleared on
  lock by default.
- FR-3: Auth agent handles the standard polkit prompt flows: auth-user,
  auth-admin, with dismiss/error paths; tested against a real polkit
  action (e.g., timedate change via Settings).
- FR-4: All prompts keyboard-navigable, AT-SPI-announced; Focus/DND never
  suppresses an auth prompt (security beats quiet).
- FR-5: Agent crash/restart: polkit re-registers; in-flight prompts fail
  closed (deny), never hang.

## Acceptance criteria

- [ ] Clipboard matrix green (including files DnD into Files via
      clipboard and paste into Xwayland apps).
- [ ] Auth agent demonstrated on: a Settings admin toggle, an `admin://`
      mount in Files, and a service request — all with one consistent
      visual language.
- [ ] Restart drills for both components pass.

## Test plan

- Scripted clipboard matrix in nested + VM (multi-format pairs).
- polkit flow tests using the reference Fedora actions; failure-path
  tests (wrong password, dismissed).

## Risks / open questions

- Sensitive-clipboard policy (password managers mark secrets as
  confidential via `x-kde-passwordManagerHint` etc.) — respect common
  hints, never store marked secrets in history.
- Data-control semantics under seat restarts — verify with T-24 drills.
