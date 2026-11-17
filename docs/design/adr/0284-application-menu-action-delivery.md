# ADR 0284 — Application menu actions return to the publisher that owns the current generation

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-16 |
| **Task** | T-14.2b follow-up |
| **Companions** | [0041](0041-native-menu-model-publication-shape.md), [0095](0095-menu-broker-resolution-and-fixed-menu.md), [0096](0096-focus-scoped-accelerators-and-global-menu-toggle.md), [06-global-menu.md](../06-global-menu.md) |

## Context

The native tier publishes a design-system menu model, and the shell renders
it. Clicks and focus-scoped accelerators reached the shell's single
`dispatchAppAction` seam, but only shell-owned lifecycle actions had a
destination. Application-owned actions such as `edit.copy`, `back`,
`fullscreen`, and `pane.*` were logged and dropped. The application already
had a local `activate()` dispatcher, but nothing carried a command back to it.

A delivery channel must not guess. A publisher can replace its model, a
different app can take focus, a publisher can exit, and the broker can
restart. A late command for an old menu must not reach the new one.

## Decision

- **The broker records the publisher, not a well-known name.** `Publish`
  captures the caller's unique bus name and assigns a publication generation.
  A republish that changes the model, tier, or owner advances the generation.
  `Withdraw` and a disconnected owner clear both. The DBusMenu bridge has no
  activation endpoint.
- **`Resolve` carries the generation.** The shell renders one generation and
  echoes it on activation, so a command can never be applied to a menu the
  user did not see.
- **The broker is the validator and router.** `Activate(appId, action,
  itemJson, generation, windowId)` returns a flat
  `{"ok":true}`/`{"ok":false,"reason":…}` result. Before forwarding, it checks
  that the app still has a native publisher, the generation still matches, the
  action is present in the current publication, and the row is enabled. A
  disabled row is rejected even if a stale input event carried it.
- **The publisher serves the endpoint on its own unique name.** The broker
  calls `org.dragonfruit.MenuPublisher1.Activate` at
  `/org/dragonfruit/MenuPublisher1` on the recorded owner, with a bounded
  method timeout. No second app can receive another app's action.
- **One application handler.** The publisher emits `activationRequested` to
  its QML menu model, which calls the same `dispatchMenuAction` path a local
  row uses and returns its boolean through `completeActivation`. A command the
  app does not handle is rejected, not reported as accepted.
- **Shell-owned lifecycle commands stay in the shell.** `hide`,
  `hide-others`, `show-all`, `close`, `quit`, `minimize`, `zoom`,
  `bring-all-to-front`, session actions, and DBusMenu rows continue through
  their existing compositor or app-index paths. The shell sends only actions
  it did not handle itself.
- **Failure is explicit.** A focus change invalidates the rendered generation
  immediately; a publish or withdraw re-resolves the focused menu; a broker
  owner change clears the generation and re-resolves when the new owner
  appears; a publisher reconnect re-publishes its cached model; and a
  rejected, timed-out, or owner-gone activation is logged with its reason.

## Consequences

- Adding an application-owned action needs only a published row with an
  `action` and a handler in the app. The shell's dispatch seam does not grow a
  per-action branch for each app.
- The broker remains the single owner of the model and the generation, while
  the application remains the authority on whether a command is valid in its
  current state.
- A publisher that never exports the endpoint is inert: the broker rejects its
  actions instead of claiming delivery.
- The endpoint is additive on the existing session bus; no new transport,
  well-known name, or compositor protocol is introduced.
