# Dragonfruit

A modern Linux desktop environment inspired by the total macOS experience, built with Rust and Qt Quick.

## Why Dragonfruit?

Dragonfruit brings the elegance, meticulous aesthetics, and intuitive workflow of macOS to Linux — not just its glass, but the whole desktop: the Dock, menu bar, Overview, app switcher, Search, Applications drawer, System Settings, animations, HUDs, and the small behaviours that make them feel like one system.

**Beautiful by design.** A fluid Dock, elegant animations, beautiful materials, intuitive workspaces, and a carefully coordinated visual language.

**Total experience, not a theme.** The target is behavioural and perceptual parity: the same mental model, information architecture, motion language, and quality bar — reproduced with original names and assets. See [`docs/design/22-total-experience.md`](docs/design/22-total-experience.md).

**Linux first.** Dragonfruit integrates with your existing distro. We are semi-opinionated, meaning we trade off some of the macOS-inspired vertical integration for customizability that Linux users typically prefer. Our goal is for you to keep the applications, tools, and distro you trust. Fedora, Ubuntu, Debian, and Arch are first-class targets; the contract is in [`docs/design/distro-compatibility.md`](docs/design/distro-compatibility.md).

**Every setting has an owner.** System Settings is not a gallery of toggles. Every row maps to our compositor, shell, `settingsd`, a host daemon, a distro adapter, a portal, or an explicitly chosen third-party component. The map is [`docs/design/settings-business-logic-map.md`](docs/design/settings-business-logic-map.md).

**Built for performance.** A Rust/Smithay Wayland compositor, GPU accelerated shell, and a design focused on responsive rendering, efficient resource usage, and smooth animation.

**Designed for resilience.** The desktop shell and supporting services run independently. If something goes wrong, the blast radius is reduced keeping your desktop stable.

### Built on open source

Every component in the DE is a build vs buy argument. We try to "buy" (in this acquire for free) when there's a high quality open source project with permissive licensing. We tend to build when we can't find something that fits well into the project's goals.

| Project | How Dragonfruit uses it |
|---|---|
| **Smithay** (MIT) | Rust Wayland compositor building blocks |
| **Qt 6 / Qt Quick** (LGPL/commercial and module-dependent) | Shell, design system, Settings, and Files interfaces |
| **Wayland / Xwayland** | Native display protocols and X11 application compatibility |
| **Mesa, DRM/KMS, libinput** | Graphics, display hardware, and input |
| **NetworkManager** | Networking and Wi-Fi |
| **BlueZ** | Bluetooth |
| **PipeWire / WirePlumber** | Audio and multimedia |
| **UPower / UDisks2** | Power and storage integration |
| **systemd / logind** | Session and process management |
| **xdg-desktop-portal** | Sandboxed application integration |
| **Inter** (SIL OFL 1.1) | Typography |
| **Tabler Icons** (MIT) | Interface line icons |
| **Phosphor Icons** (MIT) | Filled interface and Settings glyphs |
| **Capitaine Cursors** (LGPL) | Cursor theme |
| **Wikimedia Commons** (asset-specific licenses) | Optional online wallpaper discovery |

Dragonfruit's original compositor, shell, services, protocols, and design-system code are intended to use the MIT license. Third-party components and assets retain their respective licenses.

## Total experience (v0.2.0)

The v0.2.0 track completes the core desktop experience:

- **System Settings, in detail** — every pane, row, state, search result, and
  admin prompt mapped to a real owner.
- **Dock, in full** — every click path, launch/quit, bounce, minimize/genie,
  magnification, auto-hide, stacks, Trash, badges, chooser, drag/drop,
  keyboard, and multi-display behaviour.
- **Menu bar, in full** — global menu, system menu, status items, menu-bar
  extras, clock/date, Quick Settings, overflow/notch, auto-hide.
- **Overview and workspaces** — live grid, spaces strip, drag between
  workspaces, gestures, hot corners, Desktop Reveal.
- **App switcher, Search, Applications drawer, Stage, tiling, Preview,
  login/lock/first-run.**
- **Animations and HUDs** — window lifecycle, menus, Mission Control,
  app switcher, volume/brightness/backlight/charging, notifications.
- **Cross-distro integration** — app discovery, startup apps, users/groups,
  updates, default apps, and admin auth on Fedora, Ubuntu, Debian, and Arch.

Start with:

- [`docs/design/22-total-experience.md`](docs/design/22-total-experience.md) — the target
- [`docs/design/settings-business-logic-map.md`](docs/design/settings-business-logic-map.md) — every Settings row to its owner
- [`docs/design/distro-compatibility.md`](docs/design/distro-compatibility.md) — the cross-distro contract
- [`docs/design/interface-naming-and-licensing.md`](docs/design/interface-naming-and-licensing.md) — generic names and third-party policy
- [`docs/design/motion-and-feedback.md`](docs/design/motion-and-feedback.md) — motion, HUDs, sound, cursor
- [`docs/design/evidence-and-acceptance.md`](docs/design/evidence-and-acceptance.md) — capture/measurement/acceptance
- [`docs/v0.2.0/ROADMAP.md`](docs/v0.2.0/ROADMAP.md) — the executable task sequence
- [`docs/tasks/0.2.0/`](docs/tasks/0.2.0/README.md) — one-session tasks T209–T262

Reference capture evidence and the expanded campaign plan live in the sibling
`macos-liquid-glass-review` repository (`docs/GOAL.md`,
`docs/CAPTURE-PLAN.md`, `docs/SURFACE-INVENTORY.md`).

## Repository layout

```text
compositor/        Rust + Smithay compositor
protocols/         private Wayland protocols + the df-ipc contract crate
shell/             Qt Quick shell: menu bar, Dock, Control Center, …
services/          settingsd, menu-broker, app-index, session
apps/              Settings, Files
portal/            xdg-desktop-portal-dragonfruit
packaging/         Fedora/Debian metadata + portable release bundle
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

## Getting Started

### Install a release build

Tagged releases publish a portable `tar.gz`, a Fedora RPM, a Debian binary
package, and a source tarball. They install alongside your current desktop:
they do **not** remove it, preselect a display-manager session, arm autologin,
or change default applications. Log out and choose **Dragonfruit** at the
login screen.

Fedora 44 (primary reference platform):

```bash
sudo dnf install ./dragonfruit-desktop-*.rpm
```

Debian/Ubuntu with Qt 6.11 available:

```bash
sudo apt install ./dragonfruit-desktop_*.deb
```

Portable bundle (the host must already provide Qt 6.11 and the runtime stack):

```bash
tar -xzf dragonfruit-*-linux-*.tar.gz
cd dragonfruit-*-linux-*
sudo ./install.sh
```

See [`packaging/README.md`](packaging/README.md) for the asset list, local
`make dist` targets, and the release workflow. These are early binary assets;
T164/T165 still own official distro packaging.

### Build

One command builds the full desktop — the Cargo workspace (compositor,
services, tools) and the CMake/Qt side (design system, shell, apps):

```bash
make build
```


### Run it

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

### Test

One command runs all tests — Rust unit tests plus the QML lint suite
(run via `ctest`):

```bash
make test
```

### Lint and gates

```bash
make lint    # cargo fmt --check, clippy -D warnings, qmllint, desktop-name gate
make check   # lint + tests + 100-cycle teardown soak
```

### The daily dev workflow: nested sessions

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

### CI

[.github/workflows/ci.yml](.github/workflows/ci.yml) runs on every PR:
`cargo fmt --check`, `clippy -D warnings`, Rust unit tests, the
desktop-name grep gate, the headless compositor smoke + teardown soak,
the CMake/Qt build, and QML lint. Build logs are uploaded as an
artifact on every run. The compositor's test suite runs on the
**headless backend** — no display required.

[.github/workflows/release.yml](.github/workflows/release.yml) builds and
publishes the release assets on `v*` tags (or a manual dispatch). See
[`packaging/README.md`](packaging/README.md).

## Licensing

MIT, repo-wide — compositor, services, protocols, design system, shell,
and apps. See [LICENSE.md](LICENSE.md), [NOTICE](NOTICE) for third-party
attribution, and [docs/licensing.md](docs/licensing.md) for the policy.

## More docs

- [Design docs](docs/design/00-overview.md) — architecture, roadmap
  inputs, risks
- [Total experience target](docs/design/22-total-experience.md) — the whole-desktop goal
- [Settings business-logic map](docs/design/settings-business-logic-map.md) — every row to its owner
- [Cross-distro contract](docs/design/distro-compatibility.md) — Fedora/Ubuntu/Debian/Arch
- [Naming and licensing](docs/design/interface-naming-and-licensing.md) — generic names, assets, third-party policy
- [Motion and feedback](docs/design/motion-and-feedback.md) — animations, HUDs, sound, cursor
- [Evidence and acceptance](docs/design/evidence-and-acceptance.md) — capture, measurement, gates
- [Roadmap](docs/ROADMAP.md) — the strict work-unit sequence
- [v0.2.0 roadmap](docs/v0.2.0/ROADMAP.md) — total-experience task set
- [IPC versioning policy](docs/ipc-versioning.md)
- [Testing ladder](docs/testing-ladder.md)
