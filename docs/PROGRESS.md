# Progress Notes

Working notes for the plan in [ROADMAP.md](ROADMAP.md). The harness maintains the
"Key facts" digest near the top; each session appends a `## TNN` section with
environment quirks, decisions that go beyond the design docs, and follow-ups the
next task needs. Keep newest details last within a section.

The legacy (phase-based) plan's notes are archived at
[tasks/legacy/PROGRESS.md](tasks/legacy/PROGRESS.md); T-xx numbers there are
legacy numbers and do not match this plan.

## How this plan runs

- 168 one-session tasks; execution order is [ROADMAP.md](ROADMAP.md). Task ids
  `T01`–`T168` are positional; the stable design ids `T-01.1a` etc. are in each
  task file title and in [design/tracks/](design/tracks/).
- 158 nested/human tasks run first; the 10 `[hw]` tasks are Phase 18 (hardware
  rail) and need a seat, spare GPU or clean VM.
- Independent verify after every task: `make e2e`. Full gate: `make check`
  (lint + test + 100-cycle soak).
- Design references live in [design/](design/) and [design/tracks/](design/tracks/);
  a task's reference is inlined into its prompt.
- Harness install: `.symphony/` (gitignored). Config `.symphony/symphony.config.json`
  (provider `opencode`, model `openrouter/deepseek/deepseek-v4.1-flash`, jev enabled).
- Definition of done per task: building, headless test green in `make e2e`,
  reduced-motion/degrade variant where relevant, a capture under `docs/captures/`,
  and an explicit deferred list. Human sign-off is batched at track boundaries.

## Environment / toolchain

- Qt/CMake toolchain at `~/.local/df-toolchain/usr` (Qt 6.11, CMake 4.3,
  Ninja 1.13); the Makefile auto-discovers it. Pass
  `PATH=~/.local/df-toolchain/usr/bin:$PATH` and
  `LD_LIBRARY_PATH=~/.local/df-toolchain/usr/lib64` when driving cmake/ctest
  directly.
- Host is Fedora 44 with a KDE Wayland session (`wayland-0`); the nested backend
  works against it.
- `libxkbcommon.so` (linker name) is missing; symlink at
  `~/.local/lib/libxkbcommon.so` and `RUSTFLAGS="-L ~/.local/lib"` cover it.
  `~/.local/df-devroot/lib64` supplies libgbm/libseat/libinput/libudev for the
  DRM build.
- rustfmt/clippy are installed for the pinned toolchain.
- Smithay `=0.7.0`, calloop `=0.14.4`, wayland-server `=0.31.10` are exact pins;
  upgrades are deliberate events (design/14-risks.md). `df-ipc` is std-only by
  policy (docs/licensing.md).
- `make e2e` and `make check` are the scripted gates; `make demo` (T-01.6a)
  launches the nested loop for capture work.

## Landed foundation

Compositor core (nested/DRM/headless, calloop, damage-driven rendering), input
and shortcut engine, window model, Spaces, Xwayland, private shell protocols,
design system + gallery goldens, menu bar, Dock, overview state machine, dev
workflow (`make dev`/`e2e`/`soak`). Details and follow-ups are in
[tasks/legacy/PROGRESS.md](tasks/legacy/PROGRESS.md); the new plan builds on this
rather than rebuilding it.