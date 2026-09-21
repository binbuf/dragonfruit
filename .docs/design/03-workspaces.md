# Workspaces (Spaces) and Mission Control

## Summary

Spaces and Mission Control are the strongest reasons we own the compositor.
Because the compositor owns the scene graph, these features operate on the
real, live window surfaces rather than screenshots or scrapes.

## Spaces

Workspace organization is an **internal compositor primitive**:

- The compositor owns workspace creation, removal, and ordering.
- The shell consumes a **private, versioned interface** for
  listing/reordering/creating/removing workspaces.
- Workspace switching is a compositor animation: live surfaces are repositioned
  with scale/translation/blur, with gesture-driven progress and reversible
  interaction.

The first vertical slice ships three workspaces (see
[ROADMAP.md](../ROADMAP.md)).

### Model decisions

- **Spaces are per-display and ordered.** Each output has its own ordered
  Space list; a switch gesture advances the active Space on every display in
  lockstep (macOS semantics).
- **Fullscreen windows occupy a dedicated Space** that exists only while the
  window is fullscreen and appears in the workspace strip accordingly.
- **Each Space carries its own wallpaper**, compositor-rendered as part of the
  workspace scene so the background slides with the Space during switches,
  exactly like its windows; the shell never draws the desktop background.
- **Windows belong to applications, not Spaces.** Apps remember the Space
  they were assigned to; windows move between Spaces via the window menu
  ("Move to Space") or by dragging in the overview.
- **Minimized windows are excluded from the layout** and shown as a separate
  bottom strip in Mission Control, restorable by click.
- **Display hotplug preserves the model.** A newly attached output gets its
  own fresh Space list; detaching an output migrates that output's windows to
  the current Space of the remaining primary output before its Spaces are
  destroyed.
- Workspace events (created / removed / reordered / activated / window
  assigned) are broadcast over the private protocol; the shell renders the
  strip but never keeps a second copy of workspace state.

## Mission Control

Mission Control does not screen-scrape applications. The compositor keeps
rendering the real surfaces while temporarily applying scale, translation,
clipping, blur/shadow, and workspace transformations.

A Mission Control transition:

```text
normal scene
    │
    │ gesture progress 0 → 1
    ▼
shrink visible workspace
    │
    ├── reposition live window surfaces
    ├── reveal neighboring workspaces
    ├── display workspace strip
    └── transfer hit-testing to overview controller
```

When the user selects a window:

```text
overview → activate workspace → raise/focus window → reverse animation
```

Because the compositor owns the scene graph, this feels coherent in a way a
standalone third-party overview program cannot match.

### Gestures

Trackpad Mission Control gestures are compositor/libinput territory:
three/four-finger swipes and pinches drive the same gesture-progress pipeline
as the keyboard and hot-corner triggers, so the animation is continuous and
reversible at any progress value.

### Gesture commit rules

- Progress is clamped 0→1, with rubber-banding when swiping past the first or
  last Space.
- Release commits if progress or release velocity crosses a threshold;
  otherwise the transition animates back. One rule for swipes, pinches, and
  hot corners.
- Any trigger — gesture, keyboard, hot corner, Mission Control button —
  drives the **same** progress pipeline. There is exactly one overview
  state machine; there is no second, discrete "instant" code path.

## Design rules

- **Live surfaces, always.** Never replace windows with thumbnails during an
  overview; transform the real textures.
- **Gesture progress 0→1 everywhere.** Every transition (workspace switch,
  Mission Control, Dock magnification) is a continuous, interruptible
  animation, not a discrete state change.
- **Hit-testing transfers explicitly.** When the overview is active, its
  controller owns input; when the transition reverses, ownership returns to
  the normal focus path.
- **Shell is a consumer.** The shell renders the workspace strip and overview
  chrome through the private protocol but does not duplicate compositor
  state.
