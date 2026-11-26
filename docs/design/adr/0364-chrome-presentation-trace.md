# 0364 — Chrome presentation is joined by accepted generation, not callback

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-25 |
| **Task** | T353 |

## Context

The T238 correlation trace joined a shell popover opening to the compositor by
epoch-millisecond proximity. Its endpoint was the shell's `wl_surface.frame`
callback. That callback is sent after `queue_frame` on DRM and after the host
swap on nested, so it is a **pacing** event: it tells the shell it may render
the next QML frame. It does not prove that the frame which carried the accepted
chrome content reached the output. A callback for an older frame, a no-damage
wake, or a headless stand-in could satisfy the old metric.

## Decision

The compositor assigns every chrome surface a monotonic **commit generation**
on each `wl_surface.commit` and emits:

- `chrome-commit surface=<ns> gen=<n> buffer=<shm|dmabuf|none>`;
- `frame-chrome frame=<id> chrome=<ns>:<gen>,...` — the content snapshot taken
  when the frame was built;
- `frame-callback surface=<ns> frame=<id> presented=<0|1>` — a pacing event;
- `present frame=<id> output=<name> backend=<drm|nested|headless>
  clock=<hardware|estimate|host> chrome=<ns>:<gen>,...` — the final
  presentation.

The snapshot is fixed at frame build, so a commit that arrives while a frame
is in flight is not credited to that frame. A present is a native content
presentation only when it names the accepted `(surface, generation)` on the
intended output. Headless presents name `chrome=none`; no-damage passes emit
`frame-no-damage` and callbacks with `presented=0`. `frame-superseded` labels a
frame whose newer scene won.

The shell keeps the interaction id, target, generation, intended output and
accepted `commit-done` rows. It ends an interaction at the pacing callback with
detail `callback`; it never claims `presented` itself. The T238 parser performs
the join and classifies `presented`, `late_presented`,
`callback_without_content`, `discarded`, `superseded` and `missed`.
`callback_without_content` is excluded from every request-to-present
percentile. Repeated dmabuf serials across views resolve to one presented
frame; a dropped commit is `discarded`; a present on another output is not a
match; a present after the next request for the same target is late.

The trace format appends `mono_us=<process-monotonic>` after the existing
epoch-millisecond `t=`. Epoch time still aligns the shell and compositor;
`mono_us` orders one process's events deterministically.

## Consequences

- The native request-to-present metric is a content-presentation claim, not a
  frame-callback claim. Parser and compositor tests pin that boundary.
- T354 can promote the accepted generation into the visual transaction without
  changing the trace contract.
- Native DRM scanout remains the T363 evidence row; the nested run's endpoint
  is the host swap and is labelled `backend=nested clock=host`.
- Instrumentation stays opt-in (`DRAGONFRUIT_FRAME_TRACE`,
  `DRAGONFRUIT_CHROME_TRACE`) and bounded: the compositor keeps a small
  frame→chrome snapshot ring, and the shell keeps per-interaction rings.
