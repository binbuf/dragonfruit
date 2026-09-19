# T-01 — Repository Scaffolding, Licensing, CI, and Nested Dev Workflow

| | |
|---|---|
| **Phase** | 1 · Foundation |
| **Area** | `desktop/` (monorepo root) |
| **Depends on** | — (first ticket) |
| **Blocks** | All others |
| **Estimate** | M |
| **Design docs** | [01-architecture.md](../design/01-architecture.md) · [11-session-and-dev-workflow.md](../design/11-session-and-dev-workflow.md) · [14-risks.md](../design/14-risks.md) |

## Summary

Stand up the monorepo skeleton exactly as the architecture doc specifies, make
the build reproducible for both Rust and Qt/QML components, wire CI on the
headless backend, decide licensing (a hard blocker for first public release),
and deliver the `dragonfruit dev --nested` daily workflow.

## Background

Dragonfruit is many cooperating processes — a Rust/Smithay compositor, a
Qt Quick shell, Rust services, first-party apps, a portal backend, packaging.
The repository layout, IPC versioning rules, and the nested-first development
loop are decided once, here, so later tickets never re-litigate them
([01-architecture.md](../design/01-architecture.md)).

## Scope

### In scope

1. **Monorepo layout** (from [01-architecture.md](../design/01-architecture.md)):

   ```text
   desktop/
   ├── compositor/             # Rust + Smithay
   ├── protocols/              # private Wayland protocols / IPC schemas
   ├── shell/
   │   ├── menubar/
   │   ├── dock/
   │   ├── control-center/
   │   ├── notifications/
   │   └── screenshot/
   ├── services/
   │   ├── settingsd/
   │   ├── menu-broker/
   │   ├── app-index/
   │   └── session/
   ├── apps/
   │   ├── settings/
   │   └── files/
   ├── portal/                 # xdg-desktop-portal-dragonfruit
   ├── packaging/
   │   ├── fedora/
   │   └── debian/
   └── design-system/
   ```

2. **Build system**: Cargo workspace for Rust crates; CMake or qmake for
   Qt 6 / Qt Quick components; a top-level task runner that builds the
   lockstep set (compositor + shell + protocol XMLs together).
3. **Licensing decision** — final call is a **blocker for the first public
   release** ([14-risks.md](../design/14-risks.md)): intended direction is
   MIT/Apache-2.0 for compositor and services, **MIT for private protocol
   XMLs** (so any third party may implement them freely), Qt-compatible
   GPL/LGPL for design system and apps. Record LICENSE files, SPDX headers,
   and a contribution/licensing policy (including the Qt commercial decision
   trigger from [14-risks.md](../design/14-risks.md)).
4. **CI pipeline**: build + unit tests for every crate and QML module; the
   compositor runs its test suite on the **headless backend** in CI
   (no display required).
5. **`dragonfruit dev --nested`** development command: launches the
   compositor with the Wayland (nested) backend, creates its own Wayland
   socket, starts the shell, services, and test apps against that socket.
   Quitting it must leave the host session (e.g. GNOME) completely
   undisturbed — no stray processes, sockets, or env leaks.
6. **Testing-ladder tooling** (from
   [11-session-and-dev-workflow.md](../design/11-session-and-dev-workflow.md)):
   scripted helpers for each rung — nested, dedicated-user real session, VM,
   primary machine, GPU-matrix machines.
7. **Crash/teardown hygiene**: a dev helper that verifies clean session
   teardown (no leaked VT master, no orphaned clients) — this becomes the
   Foundation phase-exit test.
8. **IPC versioning policy** (from
   [01-architecture.md](../design/01-architecture.md)): written into the
   repo before the first protocol lands — private Wayland protocols are
   versioned and **additive-only** within a stable release; compositor,
   shell, and protocol XMLs ship as a lockstep set with cross-version
   mixing detected at handshake; D-Bus interfaces use `org.dragonfruit.*`
   names with a per-major-version suffix (e.g. `org.dragonfruit.Settings1`).

### Out of scope

- Any compositor/shell features (later tickets).
- Packaging (T-32) beyond what CI needs to build.
- The portal, services, and apps themselves.

## Requirements

- FR-1: `cargo build` and the Qt build succeed from a clean clone on Fedora 44
  with documented toolchain versions; Qt 6 and Smithay versions are pinned.
- FR-2: One command builds the full desktop; one command runs all tests.
- FR-3: CI runs on every PR: fmt/clippy (Rust), QML lint, unit tests,
  headless compositor smoke test, and a grep gate that fails on hardcoded
  desktop names (only `dragonfruit` is legal).
- FR-4: LICENSE files exist in every crate/package directory; a doc records
  what may link what, and the protocol XML license is MIT.
- FR-5: `dragonfruit dev --nested` starts the compositor in a window, prints
  the private socket path, and tears everything down on exit.
- FR-6: The repo documents the second-VT workflow with a **dedicated
  development user** to avoid user-session service collisions
  ([11-session-and-dev-workflow.md](../design/11-session-and-dev-workflow.md)).
- FR-7: The IPC versioning policy (scope item 8) is documented in-repo and
  enforced from the first commit: protocol XMLs carry versions, and the
  lockstep handshake rejects cross-version mixing.

## Technical notes

- Pin Smithay per release; upgrades are deliberate, mechanical events
  ([14-risks.md](../design/14-risks.md) — upstream churn mitigation).
- The compositor is kept a **thin policy layer over Smithay** so upstream API
  changes stay mechanical.
- `XDG_CURRENT_DESKTOP=dragonfruit` is chosen once here (public contract —
  see [11-session-and-dev-workflow.md](../design/11-session-and-dev-workflow.md));
  grep for hardcoded desktop names must come up empty.

## Acceptance criteria

- [x] Fresh clone → full build → full test pass, documented in README.
- [x] `dragonfruit dev --nested` runs and exits cleanly 100 consecutive
      times with zero stray processes or sockets (scripted).
- [x] Licensing decided, recorded, and reviewed (blocker release gate).
- [ ] CI green on headless backend, including an artifact with build logs.
      (Workflow written and all gates verified locally; awaiting the first
      observed green run on a PR.)

### Completion notes (2026-09-18)

- Monorepo layout, Cargo workspace, CMake/Qt side, `dragonfruit dev
  --nested`, `--soak`, the desktop-name grep gate, IPC lockstep crate,
  Makefile task runner, README, licensing docs/files, and CI are in.
- Rare flake observed once in ~300 nested teardown cycles: compositor
  died silently mid-cycle and its socket survived (exit status was
  non-clean). Direct compositor cycling is 100/100 clean; headless soak
  is 100/100 clean. Filed as T-02 hardening work: guarantee socket
  cleanup on every path (RAII guard) and find the nested teardown race.

## Risks / open questions

- Smithay API churn during Phase 1 — mitigate by pinning and thin policy.
- Qt 6 availability/versions on Fedora 44 vs Debian (affects T-32 later).
- Mono-repo vs split repos: design doc says one layout; follow it.
