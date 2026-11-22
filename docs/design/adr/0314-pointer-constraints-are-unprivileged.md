# 0314 — Pointer constraints are not privileged

## Status

Accepted (2026-11-22). Supersedes the pointer-constraint clause in the
`GrabArbiter` model in [`02-compositor.md`](../02-compositor.md) and
`compositor/src/input/constraint.rs`.

## Context

`zwp_pointer_constraints_v1` lets a client lock or confine the pointer while
its surface has focus. It is the protocol games, emulators, remote-desktop
viewers, and VM displays use for mouse-look and absolute input. The compositor
implements the grab correctly (`PointerConstraintGrab` is self-healing and
surface-scoped), but `new_constraint` sends every request through the
`GrabArbiter`, which only sanctions the shell and the Files desktop:

```rust
if self.grab_arbiter.request(client.id(), GrabKind::Pointer).is_err() {
    return;
}
```

The result:

- a game or VM viewer cannot lock the pointer; mouse-look is broken;
- every refused request appends a `GrabRefusal` and logs a line, with no bound,
  so any client can grow memory/stderr by looping create/destroy;
- the sanctioned set is never pruned on disconnect, so it grows across shell
  restarts.

The arbiter’s purpose is to stop a client from grabbing the *session’s* raw
keyboard or installing a popup grab that outlives its surface. A pointer
constraint is different: the protocol already scopes it to the client’s own
focused surface, and the grab releases when the surface loses focus or the
resource is destroyed. Treating it as a privileged session grab is a category
error.

## Decision

1. **Any client may create a pointer constraint.** Remove the
   `GrabArbiter::request(..., GrabKind::Pointer)` gate from `new_constraint`.
   The existing focus check and `PointerConstraintGrab` remain.
2. **Activate on pointer enter.** A constraint created while the surface is not
   focused stays registered and activates when the pointer enters, as the
   protocol expects. The current “activation on a future pointer-enter is a
   follow-up” comment is removed.
3. **The arbiter keeps the privileged cases.** `GrabKind::RawKeyboard` and
   `GrabKind::Popup` still require a sanctioned client. The arbiter remains the
   one gate for those.
4. **Bound the refusal log and prune sanctioned clients.** Keep at most 64
   refusals (a `VecDeque`), and remove a client from `sanctioned` when it
   disconnects. The refusal record is diagnostic, not a queue.
5. **No global pointer capture.** A constraint only affects the client’s own
   focused surface; it cannot confine the pointer over another app. The
   `PointerConstraintGrab` continues to unset itself when the constraint is
   gone.

Rejected alternatives:

- **Sanction every focused client.** That is the same as allowing the
  constraint; the extra state adds nothing.
- **Keep the gate and add a setting.** Games/remote desktop should work without
  a preference; the protocol is safe by construction.
- **Drop the arbiter entirely.** Raw keyboard grabs and popup grabs still need
  a session-level policy; keep the arbiter for those.

## Consequences

- Games, emulators, remote desktop, and VM viewers work.
- A malicious client can only constrain its own focused surface; it cannot
  capture input from another app.
- The refusal vector is bounded and the sanctioned set is pruned; a spam loop
  is a log line at most.
- The conformance test covers lock, confine, destroy, focus loss, and a spam
  loop.
