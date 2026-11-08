# 0240 — Focus maps to the canonical app identity; DBusMenu clicks and hide verbs are real

## Status

accepted — completes the shell half of [06-global-menu.md](../06-global-menu.md)
and extends ADRs [0095](0095-menu-broker-resolution-and-fixed-menu.md),
[0096](0096-focus-scoped-accelerators-and-global-menu-toggle.md), and
[0098](0098-dbusmenu-bridge-in-app-index.md).

## Context

The broker owned native/DBusMenu resolution and the fixed menu's live flags,
but the shell did not consume the bridge: a standard `RegisterWindow` keyed the
model by the caller's bus name, focus carried only a raw `app_id`, and a
`dbusmenu:<id>` row only logged. Hide/Hide Others/Show All were live-looking
rows with stub actions. The bar also lacked Services, Recent Items, a dynamic
Window list, type-ahead, and a machine-readable reason for disabled rows.

## Decision

- **Focus resolves one canonical identity.** The shell asks app-index for
  `AppMenuWindows` and matches the focused app id or the canonical desktop id
  (suffix-normalized) against the registration's app id. The compositor also
  replays the current focused window when the shell binds its toplevel manager,
  so the bar starts on the real app rather than the desktop. The shell calls
  the additive `FocusWindow(windowId, canonicalAppId)`; app-index re-keys the
  registration (withdrawing the old owner-keyed model when no other window uses
  it) and re-publishes it at the `dbusmenu` tier. The shell resolves the broker
  with the canonical desktop id; focus loss pushes an empty focus and the bar
  returns to the Files fixed menu.
- **DBusMenu rows are live and routable.** A registration match reads a fresh
  `WindowMenu` (running `AboutToShow`) so dynamic rows are current, while a
  native publication keeps tier priority. A click on `dbusmenu:<id>` calls
  `WindowMenuEvent(focusedRegistrationWindow, id)`, which forwards the owning
  app's `Event(id, "clicked")`; the same dispatch seam handles an accelerator.
- **Hide verbs are tracked compositor window operations.** Hide and Hide Others
  minimize every visible window of the target app(s) through
  `df_toplevel.minimize`; the shell records exactly the ids it hid. Show All
  restores those ids (falling back to all currently minimized windows when the
  set is empty, so the enabled verb is never inert). Hide Others re-raises the
  focused app. User-minimized windows are not claimed by a hide.
- **Fixed content is complete.** The synthesized application menu carries
  Services (present but explicitly disabled until a service registry exists),
  the system menu carries Recent Items from the shell's recency list capped by
  `menu.recentItems`, and a no-export/DBusMenu app gets a synthesized Window
  menu (Minimize, checked window rows, Bring All to Front). An exported Window
  menu wins. Every disabled row carries `disabledReason`.
- **Keyboard behavior is one component.** `MenuBarMenu` handles arrows,
  Return/Space, Escape, Left/Right submenu direction and top-level switching,
  and a 700 ms type-ahead buffer that skips disabled rows. `MenuRow` exposes a
  disabled row's reason through accessibility.

## Consequences

- The three tiers are end-to-end: native publication, registered DBusMenu, and
  fixed-only. A toolkit that exports nothing still keeps the fixed menus.
- Hide/Show use the existing minimize/restore motion and state; no new
  compositor state bit or protocol request was added.
- Recent Items currently remembers applications, not documents or servers;
  `menu.recentItems` is the shared cap, and a document source is a follow-up.
- Live re-projection happens at focus and at menu read time; there is still no
  push subscription for DBusMenu property changes.
