# Progress Notes

Working notes for subsequent tasks — environment quirks, decisions made
beyond the design docs, and follow-ups discovered during implementation.
Newest entries last. Update this file whenever a task teaches something
the next task needs to know.

## Environment / toolchain (learned during T-01)

- **Qt/CMake toolchain lives at `~/.local/df-toolchain/usr`** (Qt 6.11,
  CMake 4.3, Ninja 1.13). It is NOT on `PATH` by default; the Makefile
  auto-discovers it, but pass `PATH=~/.local/df-toolchain/usr/bin:$PATH`
  (and `LD_LIBRARY_PATH=~/.local/df-toolchain/usr/lib64`) when driving
  cmake/ctest directly. `cmake` needs the LD_LIBRARY_PATH (librhash).
- **The host is Fedora 44 with a KDE Wayland session** (`wayland-0`), not
  GNOME — the nested backend works fine against it; design docs' "nested
  in GNOME" examples are host-agnostic in practice.
- **`libxkbcommon.so` (linker name) is missing** on this machine — only
  `libxkbcommon.so.0` exists (no `-devel` package, no sudo). Workaround
  in use: `RUSTFLAGS="-L ~/.local/lib"` with a `libxkbcommon.so` symlink
  pointing at the .so.0. Any `cargo build/test` that links the
  compositor needs this until the package is installed.
- rustfmt/clippy were not installed for the pinned toolchain; installed
  via `rustup component add rustfmt clippy`.

## T-01 — repo scaffolding, CI, licensing, nested workflow

**State: complete** (CI awaiting first green run — see below). All
gates verified locally: `cargo build/test/clippy -D warnings/fmt
--check`, CMake build + 8/8 qmllint ctests, `scripts/check-desktop-names.sh`,
100-cycle headless soak, 100 consecutive clean nested `dragonfruit dev
--nested` exits.

Follow-ups for later tasks:

- **T-02 must harden nested teardown.** Observed once in ~300 nested
  cycles: the compositor died silently mid-cycle and its Wayland socket
  survived (no stderr, no clean-exit print). Direct compositor cycling
  was 100/100 clean both before and after, so it is a rare race, likely
  in the winit teardown path. Actions: (1) RAII/panic-guard the socket
  cleanup instead of relying on the drop chain at the end of
  `compositor_loop` (compositor/src/main.rs), (2) reproduce under the
  real T-02 event loop, (3) consider a `Drop` guard in the dev tool too
  for the `--launch`ed children.
- **The skeleton compositor pump loop polls at 50 ms** — fine for T-01,
  but T-02 replaces it with an fd-driven calloop loop; don't build on
  `compositor_loop`'s structure, build on its contracts (socket print
  format, teardown verification, lockstep assert).
- **`cargo run` swallows signal-context detail**: when the process group
  gets SIGTERM (e.g. `timeout` or Ctrl-C under `make`), the dev tool's
  stale `SIGNALLED` flag used to make `shutdown_child` SIGKILL a
  compositor that was already exiting cleanly → silent exit 71. Fixed
  (flag cleared before teardown; final wait-status check), but keep the
  lesson: teardown code must distinguish the initiating signal from new
  signals. The Foundation phase-exit soak should include nested runs,
  not just headless, to keep exercising this.
- **CI (`.github/workflows/ci.yml`) has never run on GitHub.** It needs
  a first observed green run on a PR — including the Qt 6.11 install via
  `jurplel/install-qt-action@v4` (`version: "6.11.*"`, unverified on
  ubuntu-latest) and the rust-toolchain pin (1.98.1, matching local).
  If the Qt action can't provide 6.11, either mirror the df-toolchain
  approach in CI or lower `find_package(Qt6 6.11 ...)` — decide then,
  with a note here.
- **Naming gate exemptions**: lines carrying `df-allow-desktop-name` are
  exempt from `scripts/check-desktop-names.sh` (used by the negative
  D-Bus name test in df-ipc). Keep exemptions rare.
- **Smithay =0.7.0, calloop =0.14.4, wayland-server =0.31.10** are exact
  pins in the root Cargo.toml; upgrades are deliberate events
  (14-risks.md). df-ipc is std-only by policy (docs/licensing.md) —
  keep it that way.
- **Licensing is recorded but not human-reviewed** — the release-blocker
  signoff in 14-risks.md is still open. MIT/Apache for compositor +
  services + tools, MIT for protocol XMLs (enforced by the df-ipc test),
  LGPL-3.0-or-later for design-system, GPL-3.0-or-later for shell/apps.
  Full texts in LICENSES/; pointer LICENSE in every package dir.

## Conventions established in T-01 (follow in all later tasks)

- Every source file carries an SPDX header matching its directory's
  LICENSE file.
- Constants that are contracts (desktop name, lockstep version, D-Bus
  names) live in `df-ipc` and nowhere else.
- QML lint runs as ctest (`df_qml_lint` in the root CMakeLists) — every
  new QML module registers its files there.
- `make check` is the local CI-equivalent gate; run it before every
  commit.
