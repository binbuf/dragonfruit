# Dragonfruit

Dragonfruit is a Linux desktop environment: a Rust/Smithay compositor, a
Qt Quick (QML) shell and first-party apps, and small Rust services over
D-Bus — macOS-inspired interaction, original assets and code, built on
the Linux stack (Wayland, logind, NetworkManager, PipeWire, …).

**Status: pre-alpha.** The repository is at T-01 (scaffolding, CI,
licensing, nested dev workflow) of the
[task index](.docs/tasks/00-index.md); the design docs in
[.docs/design/](.docs/design/) are the source of truth.

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
                 libxkbcommon-devel wayland-devel
```

(If `cmake`/`ninja` live in a local Qt toolchain prefix, the Makefile
finds them at `$DF_TOOLCHAIN` or `~/.local/df-toolchain/usr`.)

## Build

One command builds the full desktop — the Cargo workspace (compositor,
services, tools) and the CMake/Qt side (design system, shell, apps):

```bash
make build
```

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

The **desktop-name gate** (`scripts/check-desktop-names.sh`) fails on
any hardcoded desktop name — `dragonfruit` is the only legal value
(`XDG_CURRENT_DESKTOP` is a public contract; see
[docs/ipc-versioning.md](docs/ipc-versioning.md)).

## The daily dev workflow: nested sessions

```bash
make dev      # = dragonfruit dev --nested
```

The compositor opens as a window on your host Wayland session, creates
its own private socket, and tears everything down on exit — the host
session is never disturbed. Extras:

```bash
dragonfruit dev --nested --launch APP   # run an app against the nested session
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

MIT OR Apache-2.0 for the compositor and services, **MIT for the
private protocol XMLs**, LGPL-3.0-or-later for the design system,
GPL-3.0-or-later for the shell and apps. See
[LICENSE.md](LICENSE.md) and [docs/licensing.md](docs/licensing.md).

## More docs

- [Design docs](.docs/design/00-overview.md) — architecture, roadmap
  inputs, risks
- [Roadmap](.docs/ROADMAP.md) and [task index](.docs/tasks/00-index.md)
- [IPC versioning policy](docs/ipc-versioning.md)
- [Testing ladder](docs/testing-ladder.md)
