# 0084 — polkit authentication agent: a shell-side agent over the host helper

## Status

Accepted (T-13.6).

## Context

Privileged operations requested by Settings, Files, NetworkManager, and our
services are authorized by polkit. Rendering those prompts ourselves needs a
polkit **authentication agent**, and the design is explicit that Dragonfruit
never rolls its own privilege escalation: the agent may only relay results from
the host stack (`docs/design/07-system-integration.md`). polkit offers no
in-process shortcut for that relay — an agent registers with the authority,
offers `org.freedesktop.PolicyKit1.AuthenticationAgent`, and drives the host's
`polkit-agent-helper-1` over its PAM line protocol. The helper, not the agent,
talks to the authority and reports the outcome.

The author steps are: who hosts the agent, how it registers, how the password
reaches PAM, and how the prompt is presented.

## Decision

- **The shell hosts the agent.** `PolkitAgent` (`shell/src/polkitagent.*`,
  dockcore) replaces the presentation only; `polkitd`, PAM, and the helper stay
  the host's.
- **Registration is a session subject by default.** The agent registers the
  shell's `unix-session` (from `XDG_SESSION_ID`); a process subject is used when
  no session id exists, and both are injectable (`DF_POLKIT_SUBJECT_SESSION`,
  `DF_POLKIT_SUBJECT_PID`, `setSubjectSessionId`/`setSubjectProcess`) so a dev
  or test session can register for a specific process without displacing the
  live desktop's agent. Registration is **asynchronous**, so a missing or slow
  authority never blocks shell startup; a missing authority or an existing agent
  for the subject is a degraded state, not an error.
- **The agent object is a `QDBusVirtualObject` on the authority's bus.** polkit
  is on the system bus; the interface's `a(sa{sv})` identity list has no
  built-in QtDBus C++ mapping, so the object parses the raw `QDBusArgument` and
  the registering subject is marshalled through a small registered struct.
- **The host helper owns the credential exchange.** `PolkitHelperSession`
  (`shell/src/polkitsession.*`) spawns `polkit-agent-helper-1` (or connects to
  `/run/polkit/agent-helper.socket` on polkit 127+ hosts without a setuid
  helper), writes the authority's cookie, relays PAM prompt/info/error lines,
  and forwards the response. The shell never calls
  `AuthenticationAgentResponse` and never verifies a password; the helper does
  both. The response buffer lives in `ShellController` like the lock screen's
  (the compositor delivers raw key codes, not text), and is cleared the moment
  it is written to the helper.
- **The prompt is a design-system shell overlay.** `PolkitDialog.qml` is a pure
  view on a centered `polkit` layer surface; an absent authority or a rejected
  registration simply means it never opens. `DF_POLKIT_FIXTURE` presents a
  synthetic request for the live capture.
- **Declining is fail-closed.** Cancel, Escape, a lost keyboard, or a helper
  failure resolves the request as failure; the authority never receives a
  forged success.

## Consequences

- The shell links `Qt6::Network` for `QLocalSocket`.
- A second session agent (e.g. a desktop environment's) wins the session
  subject; Dragonfruit's agent logs a registration error and the desktop keeps
  working. In a Dragonfruit session ours is the only agent.
- PipeWire-free and root-free: nothing runs as root, and the only privileged
  binary is the distribution's own helper.
- A future auth method (T-15) adds identities/transports behind
  `PolkitHelperSession`, not a second agent.