# 0007 — Close is a ghost with deferred, single removal

## Status

accepted

## Context

T-02.4a fades/scales a closing window out while keeping it input-inert, and
removes it from the model only at completion. Unlike minimize, close is
normally followed by the client destroying its surface (`xdg_toplevel.destroy`
→ `XdgShellHandler::toplevel_destroyed`, or the X11 `destroyed_window`), which
happens on the client's schedule and can land at any point during the motion.
Two ways to remove the window can therefore race: the close animation settling
and the client destroy callback. The removal must happen exactly once, and the
shell must be told exactly one lifecycle event.

## Decision

- Add **`close`** to `WindowMotionKind`, reverse of appear/restore like
  minimize (`target → origin`, alpha `1 → 0`), timed on `motion::WINDOW_CLOSE`.
  `WindowMotionKind::is_ghost()` marks close and minimize as the kinds whose
  window is unmapped and drawn from `WindowModel::active_motions`.
- `DfState::close_window` unmaps the window immediately (input-inert), starts
  the `Close` motion, and sends the client close request
  (`xdg_toplevel.close` / X11 `WM_DELETE_WINDOW`). It is a no-op while a close
  ghost is already live.
- `toplevel_destroyed`/`destroy_x11_window` **defer** when
  `WindowModel::is_closing(window)`: the surface may already be dead, but the
  model entry is left for the animation.
- The single teardown path `DfState::remove_window` removes the entry (its
  return is the idempotence guard) and broadcasts either `Unmapped` (client
  destroy with no close motion) or the new `WindowEventKind::Closed` (settled
  close ghost, emitted from `step_window_motions`). The shell treats `Closed`
  like `Unmapped`: drop the toplevel resource and `closed()`.
- A closing window is excluded from `apply_workspace_layout`, so a workspace
  change mid-ghost cannot remap it into the input path.
- Reduced motion is the same zero-duration tween: the close commits removal on
  its first clock step through the one path.

## Consequences

- Removal is exactly once by construction: whichever of the animation or the
  client destroy arrives first, the other sees the entry already gone (or the
  motion still live) and does not double-remove.
- The ghost may lose its client buffer if the client destroys quickly; the
  render layer skips dead windows and the model removal still commits.
- T-02.4a commits removal when the close *transition* settles, matching its
  acceptance (`close transition commits removal exactly once`); a client veto
  (`xdg_toplevel.close` is advisory) is not modelled yet. T-02.4b owns
  interruption/retarget and the idle trace on top of this single removal path;
  it must not register a second motion driver or a second teardown path.
