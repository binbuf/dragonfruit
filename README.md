# Dragonfruit

Dragonfruit is a Linux desktop environment: a Rust/Smithay compositor, a
Qt Quick (QML) shell and first-party apps, and small Rust services over
D-Bus — macOS-inspired interaction, original assets and code, built on
the Linux stack (Wayland, logind, NetworkManager, PipeWire, …).

**Status: pre-alpha.** The compositor core, input, window model, Spaces,
Xwayland, private shell protocols, design system, menu bar, and Dock are
built (the inherited foundation). The current plan is a sequence of vertical
slices that turn that machinery into a testable desktop — see the
[roadmap](docs/ROADMAP.md). The design docs in
[docs/design/](docs/design/) are the source of truth for what to build.

## Repository layout

```text
compositor/        Rust + Smithay compositor
protocols/         private Wayland protocols + the df-ipc contract crate
shell/             Qt Quick shell: menu bar, Dock, Control Center, …
services/          settingsd, menu-broker, app-index, session
apps/              Settings, Files
portal/            xdg-desktop-portal-dragonfruit
packaging/         fedora/, debian/ (T-32)
design-system/     QML module `Dragonfruit` — the visual identity
tools/             the `dragonfruit` dev command
scripts/           repo gates (desktop-name check, …)
docs/              in-repo policy docs (licensing, IPC versioning, …)
```

## Toolchains (pinned)

| Tool | Version | Where pinned |
|---|---|---|
| Rust | 1.98.1 (MSRV 1.80.1) | `rust-toolchain.toml`, `Cargo.toml` |
| Smithay | =0.7.0 | root `Cargo.toml` (exact pin; upgrades are deliberate events) |
| Qt | 6.11 | root `CMakeLists.txt` (`find_package(Qt6 6.11 ...)`) |
| CMake | ≥ 3.24 | root `CMakeLists.txt` |

Fedora 44 native packages:

```bash
sudo dnf install rustc cargo qt6-qtdeclarative-devel cmake ninja-build \
                 libxkbcommon-devel wayland-devel \
                 libdrm-devel mesa-libgbm-devel libinput-devel \
                 libseat-devel systemd-devel
```

(If `cmake`/`ninja` live in a local Qt toolchain prefix, the Makefile
finds them at `$DF_TOOLCHAIN` or `~/.local/df-toolchain/usr`. On machines
without the DRM `-devel` packages, a user-space sysroot at
`~/.local/df-devroot/lib64` is picked up automatically — see
`docs/tasks/legacy/PROGRESS.md`, T-02.)

## Build

One command builds the full desktop — the Cargo workspace (compositor,
services, tools) and the CMake/Qt side (design system, shell, apps):

```bash
make build
```

## Run it

Pick the workflow and command that matches what you are doing:

| If you want to... | Use | What happens |
|---|---|---|
| Keep your current desktop and run Dragonfruit in a window | `make dev` (thin services) or `make dev-full` (full services) | Nested compositor; builds first; no install or logout. Fastest loop for app/QML work. |
| Install Dragonfruit as the session/compositor instead of your current DE | `make native` (release daily driver) or `make native-debug` (fast iteration) | Provisions dependencies, builds, installs the session, preselects Dragonfruit, and logs out. One-time setup. Release is the default because the native session is the dogfooding rail. |
| App or shell change | `make native-restart` | Rebuilds and restarts the shell in place; no logout. New app launches use the new binaries. |
| Service change | `make native-restart NATIVE_UNITS=services` | Rebuilds and restarts the services in place; no logout. |
| Keep a debug session for fast iteration | `make native-restart NATIVE_ARGS=--debug` | Restarts the installed debug profile. A profile switch needs `native-rebuild`/`native-reboot`, not `restart`. |
| Compositor change, or you want a clean session | `make native-rebuild` | Rebuilds, refreshes the install, then logs out; log back in to Dragonfruit. |
| Do the same but reboot the machine | `make native-reboot` | Same as `native-rebuild`, then reboots. |
| Return to your original desktop | `make native-return` | Restores the login configuration and ends Dragonfruit. |
| Recover from a broken Dragonfruit login | `make native-recover` | Same recovery path from a TTY/SSH; no Dragonfruit session needed. |

The nested loop is the fastest way to work on apps and the compositor without
touching your login setup. `make dev` starts the compositor inside your
current Wayland session; `make dev-full` adds the real session services on a
private bus. Both build first and are safe to rerun.

The native loop replaces your current session. `make native` is the one-time
provisioning path and defaults to the **release** profile; use
`make native-debug` (or `NATIVE_ARGS=--debug`) only when you need the fast
iteration build. After that, use the smaller commands in the table.
`native-rebuild` also accepts `NATIVE_ARGS=--no-install` for binary-only
changes or `--no-logout` to build/refresh without ending the session; see
`make help` for the diagnostics commands.

The release profile matters on high-resolution/high-refresh panels: the 4K
damage/material path is CPU-bound enough that an unoptimized build can cap
animated clients well below the panel's refresh rate.

The native install also wires a dev-only capture service. On login it records
units, processes, DRM state, journals, and a render-stats dump under
`~/.local/state/dragonfruit/native-captures/`, then records a final snapshot
when the session ends. `native-recover`/`native-return` remove the capture
unit; the reports stay for comparison. `make native-capture` takes an
on-demand snapshot, and `make native-perf` adds a live client frame-rate probe
(`vkcube`/`glxgears`) so the refresh-rate path is measured, not guessed.

If the Dragonfruit login does not come up (for example a black screen), switch
to a text console with `Ctrl+Alt+F3`, log in, and run `make native-recover`
(or `bash scripts/native-session.sh recover`). It removes the dev preselect and
runtime environment, restores the saved display-manager files, stops the failed
session, and restarts the login manager from a text console — no Dragonfruit
session required.

The native session uses systemd user units and works from any normal user
account with sudo for the system install. On a non-systemd host, use the
nested `make dev` loop instead.

## Test

One command runs all tests — Rust unit tests plus the QML lint suite
(run via `ctest`):

```bash
make test
```

## Lint and gates

```bash
make lint    # cargo fmt --check, clippy -D warnings, qmllint, desktop-name gate
make check   # lint + tests + 100-cycle teardown soak
```

## Premium experience gate (T-17)

The full loop — window lifecycle, Spaces/Mission Control/app switch, a Flatpak
browser, materials in light/dark/reduced motion, absence/crash/leak/lock
robustness — is gated by the T-17 track
([design/tracks/17-premium-gate.md](docs/design/tracks/17-premium-gate.md)). The
reviewed sign-off is
[docs/captures/t17-premium-gate.md](docs/captures/t17-premium-gate.md); the
gate is agent-signed with the DRM and baseline-frame-budget rows left open on
the hardware rail (T-17.2/T-17.4).

```bash
make check                      # lint + tests + 100-cycle teardown soak (green)
make e2e                        # the foundation + conformance suites (green)
make t17-premium-gate-capture   # assembled-desktop still + sign-off transcript

# the per-unit evidence the report indexes
make t17-window-loop-capture
make t17-navigation-capture
make t17-flatpak-browser-capture
make t17-visual-floor-capture
make t17-robustness-matrix
make t17-leak-lock-soak
```

Requires a host Wayland session, `spectacle`, python3 + Pillow for the capture
targets; `make check`/`make e2e` are headless.

The **desktop-name gate** (`scripts/check-desktop-names.sh`) fails on
any hardcoded desktop name — `dragonfruit` is the only legal value
(`XDG_CURRENT_DESKTOP` is a public contract; see
[docs/ipc-versioning.md](docs/ipc-versioning.md)).

## The daily dev workflow: nested sessions

```bash
make dev      # = dragonfruit dev --nested
make demo     # nested loop demo: shell + Qt app + X11 app + checklist
```

The compositor opens as a window on your host Wayland session, creates
its own private socket, and tears everything down on exit — the host
session is never disturbed. `make demo` is the per-slice demo harness
(T-01.6a): it builds, launches the shell and two clients, and prints the
walkthrough checklist. With no host Wayland session (CI) it runs the same
target headless as a launch+teardown smoke. Extras:

```bash
dragonfruit dev --nested --launch APP   # run an app against the nested session
dragonfruit dev --demo --headless       # force the scripted half
make soak                               # 100 clean-exit cycles, zero strays
dragonfruit dev --soak 50               # same gate, headless (what CI runs)
```

Higher rungs of the testing ladder — dedicated-user real sessions, VMs,
hardware matrix — are in [docs/testing-ladder.md](docs/testing-ladder.md).

## CI

[.github/workflows/ci.yml](.github/workflows/ci.yml) runs on every PR:
`cargo fmt --check`, `clippy -D warnings`, Rust unit tests, the
desktop-name grep gate, the headless compositor smoke + teardown soak,
the CMake/Qt build, and QML lint. Build logs are uploaded as an
artifact on every run. The compositor's test suite runs on the
**headless backend** — no display required.

## Licensing

MIT, repo-wide — compositor, services, protocols, design system, shell,
and apps. See [LICENSE.md](LICENSE.md), [NOTICE](NOTICE) for third-party
attribution, and [docs/licensing.md](docs/licensing.md) for the policy.

## More docs

- [Design docs](docs/design/00-overview.md) — architecture, roadmap
  inputs, risks
- [Roadmap](docs/ROADMAP.md) — the strict work-unit sequence
- [IPC versioning policy](docs/ipc-versioning.md)
- [Testing ladder](docs/testing-ladder.md)
