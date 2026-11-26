# Contributing to Dragonfruit

Dragonfruit is an early-development Wayland desktop: a Rust/Smithay compositor,
a Qt Quick shell and first-party apps, and native Linux services. It is a
desktop environment, not a theme or a distribution.

This guide is the contract for changes. It is deliberately stricter than a
typical application project because a display server can end the user's
session, the shell is a visual product, and the project ships to several
distributions with different host services.

If you only read one section, read [Verify before opening a PR](#verify-before-opening-a-pr).

## Before you start

- **Discuss large changes first.** Open an issue before starting work on a new
  surface, a compositor behaviour, a protocol, a dependency, an asset, or a
  cross-component refactor. Small fixes, tests, and documentation corrections
  can go straight to a pull request.
- **One concern per pull request.** Keep mechanical moves separate from
  behaviour changes. A PR that touches the compositor, the shell, packaging,
  and the design tokens at once is very hard to review and easy to regress.
- **Read the design record for the area.** Start at
  [`docs/design/README.md`](docs/design/README.md), then the relevant topic doc.
  [`docs/design/STATUS.md`](docs/design/STATUS.md) records what is actually
  implemented versus design-only.
- **Search existing issues and PRs.** Duplicate work and duplicate reviews are
  expensive.
- **Do not hide unfinished work.** A disabled control, a stub that always
  succeeds, or a screenshot that implies a surface works are review blockers.
  Document the gap and its absence path instead.

## Set up a development environment

A Wayland host session is required for the daily nested workflow. The primary
development platform is Fedora; Debian, Ubuntu, and Arch are supported for
packaging and host-service behaviour.

The quick start in the [README](README.md#quick-start) installs the native
dependencies. The short version from a Fedora host is:

```bash
sudo dnf install rustc cargo qt6-qtdeclarative-devel qt6-qtwayland cmake ninja-build \
  libxkbcommon-devel wayland-devel libdrm-devel mesa-libgbm-devel \
  libinput-devel libseat-devel systemd-devel

git clone https://github.com/binbuf/dragonfruit.git
cd dragonfruit
make dev
```

The Qt 6 Wayland platform plugin (`qt6-qtwayland` on Fedora) is a separate
package on several distributions. `scripts/check-native-deps.sh` checks for it
without root, and `scripts/native-session.sh start` runs that check before it
arms a login.

`make dev` builds and runs Dragonfruit **in a window** on your existing session.
It is the safe default: your host desktop is never replaced, and teardown
checks for stray processes and sockets.

Useful variants:

```bash
make dev-full               # nested session with the real session services
make dev-full FIXTURES=1    # deterministic pane fixtures when hardware is absent
make demo                   # the vertical-slice walkthrough
make demo DEMO_ARGS=--headless
```

`make dev-full` runs on a private bus with scratch XDG directories, so it does
not read or overwrite your real settings.

### Toolchain and dependency pins

- Use the Rust toolchain pinned in [`rust-toolchain.toml`](rust-toolchain.toml).
  The workspace also declares an MSRV in [`Cargo.toml`](Cargo.toml); do not
  raise it without a separate, reviewed decision.
- Smithay is pinned exactly, and the Qt floor is deliberate. Do not bump the
  Smithay pin, the Rust toolchain, or the Qt minimum in a feature PR. Propose
  dependency and toolchain changes separately with a rationale and a license
  check.
- The Qt build floor is 6.8 even though CI uses a newer Qt. Do not use APIs
  that require more than the floor unless the packaging decision is updated in
  the same change.

### Real-session testing

Nested mode covers most day-to-day work. For DRM/KMS, input, suspend/resume,
multi-GPU, and real display-manager behaviour, use a dedicated user on a
second VT, a VM, or a spare machine. **Do not uninstall your host desktop**,
and do not treat `make native` as a casual test command:

```bash
make native          # replaces the current login selection
make native-recover  # recovery from a TTY/SSH
```

Read the [session and development workflow](docs/design/11-session-and-dev-workflow.md),
the [testing ladder](docs/design/testing-ladder.md), and the
[DRM session runbook](docs/design/runbook-drm-session.md) before touching
session lifecycle code.

## Know the tree

| Path | Owns |
|---|---|
| `compositor/` | Smithay Wayland compositor, window model, rendering, input, protocols |
| `shell/` | Menu bar, Dock, Overview, Quick Settings, notifications, switcher, OSD |
| `apps/` | First-party Settings and Files |
| `design-system/` | QML components, tokens, materials, gallery, shared visual language |
| `services/` | settingsd, session, app-index, menu-broker, system/distro adapters, portals support |
| `portal/` | `xdg-desktop-portal` backend |
| `protocols/` | Private Wayland protocols and the `df-ipc` contract crate |
| `tools/` | Development harness (`dragonfruit-dev`) |
| `packaging/` | Debian, Fedora, Arch, and portable packaging metadata |
| `scripts/` | Gates, generators, captures, and validation harnesses |
| `docs/design/` | The architecture, target behaviour, ADRs, and status register |
| `tests/` | Shared repository-level tests, including naming and assets |

The Rust workspace and the Qt/CMake side ship as one lockstep product. A change
that crosses the compositor/shell boundary usually needs both sides updated,
plus the private-protocol version if the wire contract changes.

## Change rules

### Design and scope

- The topic docs describe the **intended design**; `STATUS.md` records the
  **delta** from the current tree. If your change makes either false, update
  both in the same PR.
- New behaviour that constrains later work needs an ADR. Use
  [`docs/design/adr/0000-template.md`](docs/design/adr/0000-template.md), take
  the next free number, and never reuse a number.
- Keep the design system as the only visual implementation. A pane should not
  fork a shared component or grow a private widget vocabulary.
- Preserve the project's boundaries: per-process services, one settings owner,
  the compositor as the policy owner for its surfaces, and the shell as a
  client of the compositor's private protocols.
- Do not add a dependency, daemon, or optional service without an absence path.
  The desktop must still start and remain usable when a host service is
  missing.

### Rust

- Run `cargo fmt --all` and keep `cargo clippy --workspace --all-targets -- -D warnings`
  clean. CI treats warnings as errors.
- Prefer small, testable units over large event-loop bodies. The compositor
  already has conformance suites; put protocol and state-machine behaviour
  there rather than in a capture script.
- Do not hand-edit generated files. `compositor/src/design_tokens.rs`,
  `design-system/Theme.qml`, and the glyph registries are generated.
- Do not change `LOCKSTEP_VERSION`, the settings `SCHEMA_VERSION`, or the
  session `STATE_VERSION` unless the change genuinely alters that contract.
  Protocol and schema updates follow
  [`docs/design/ipc-versioning.md`](docs/design/ipc-versioning.md).

### Qt, QML, and the design system

- New QML must pass `qmllint` through the existing ctest targets.
- Use the shared components in `design-system/components/`. Do not copy a
  component into an app to get one different property.
- Never hardcode colours, radii, or animation durations in design-system QML.
  Values come from `design-system/tokens/tokens.json`. Regenerate with:

  ```bash
  ./scripts/gen-tokens.py
  ```

- Icons come from the vendored Tabler, Phosphor, brand, and WhiteSur registries.
  Do not reference an SVG that is not vendored, and do not hand-add a glyph to
  a generated registry. Regenerate with the matching `gen-*-glyphs.py` script.
- Wrap every user-visible string in `qsTr()`/`tr()`, and regenerate the catalog
  template:

  ```bash
  make i18n-update
  ```

- If you intentionally change the gallery's rendered output, regenerate the
  goldens with `make gallery-snapshot` and review the image diff. Do not update
  goldens just to make a failing test green.

### Strings, names, and user-facing text

Follow [`docs/design/interface-naming-and-licensing.md`](docs/design/interface-naming-and-licensing.md).
The project reproduces interaction quality, not another product's identity.
That means:

- Use the generic Dragonfruit terms in the naming map for UI, desktop files,
  packaging metadata, and screenshots.
- Never copy names, logos, symbols, sounds, wallpapers, or proprietary artwork
  from a commercial desktop.
- `make check-naming` scans user-facing QML/C++ strings, desktop entries, and
  packaging metadata. A new user-facing name may also need a
  `docs/design/naming-gate.json` entry and a regression test.

### Assets, dependencies, and licensing

First-party code is GPL-3.0-or-later licensed ([`LICENSE.md`](LICENSE.md)). A
contribution must be compatible with that.

- Every new asset (icon, font, sound, cursor, wallpaper, illustration) needs a
  source, a license, and a license-text entry in
  [`docs/design/asset-manifest.json`](docs/design/asset-manifest.json). Run
  `make check-assets`.
- AGPL, GPL-2.0-only, and other GPL-3.0-incompatible code must never be linked
  into first-party code. GPL-3.0-compatible dependencies are allowed when they
  are recorded in `NOTICE`. Copyleft data assets are allowed only when they are
  separable, loaded at runtime, and recorded with the existing
  notice/provenance rules.
- New Rust dependencies must be GPL-3.0 compatible, justified in the PR,
  and reflected in `Cargo.lock` and `NOTICE` where required. Do not hand-edit
  `Cargo.lock`.
- Do not commit screenshots, captures, recordings, logs, build outputs, or
  downloaded binaries. See [Generated and local-only files](#generated-and-local-only-files).
- By opening a pull request you confirm you have the right to submit the work
  under the project's GPL-3.0-or-later license. No separate CLA or DCO sign-off is
  required today.

### Tests

A bug fix needs a regression test that fails before the fix. New behaviour
needs a headless test. Failure and absence are normal states in this project,
so test them explicitly.

- Compositor and protocol changes belong in the existing conformance suites
  under `compositor/tests/`.
- Service and adapter changes need tests for the mock backend, the real-service
  path where available, and the absence path.
- QML changes need the relevant ctest QML suite. Visual changes also need the
  relevant `make *-capture` target and the states that apply: light/dark,
  1x/1.5x/2x, reduced motion, reduce transparency, increased contrast, and
  multi-display where the surface renders.
- Session, lock, and compositor lifecycle changes need the teardown/soak gates.
  Leaked sockets, processes, or runtime files are failures, not cleanup work
  for later.
- Never skip, weaken, or delete a test merely to make a PR pass. If a test is
  wrong, explain why in the PR and fix the test deliberately.

### Documentation and ADRs

- Update the relevant topic doc and `STATUS.md` in the same PR as the code.
  An "implemented" claim must name its evidence: a test, a capture, a file, or
  an ADR. "Implemented" without evidence is a documentation bug.
- Add an ADR for a decision that later work must respect. State the rejected
  alternatives and the consequences. Supersede old ADRs; never delete them.
- If you add a design document, add it to the table in
  [`docs/design/README.md`](docs/design/README.md).
- Captures and traces stay local under `docs/captures/` (gitignored). Record
  the rerun command and the observed result in the PR or design record. Attach
  images to the GitHub PR if they help review; do not commit them.
- Keep proposals, task lists, research notes, and diagnostics out of git.
  Permanent design material belongs in `docs/design/`; decisions belong in
  `docs/design/adr/`.
- After a docs change, run the Markdown-link and ADR-id checks documented in
  [`docs/design/README.md#documentation-checks`](docs/design/README.md#documentation-checks).
  The ADR uniqueness check in short form is:

  ```bash
  ls docs/design/adr | sed -E 's/^([0-9]+).*/\1/' | sort | uniq -d
  ```

## Verify before opening a PR

CI is the contract. Run the closest local equivalent before you ask for review:

```bash
make fmt          # rustfmt
make lint         # fmt check, clippy, QML lint, token/naming/i18n/icon gates
make test         # cargo test, QML/ctest suites, gallery visual regression
make verify-fast  # gate checks + end-to-end/conformance + parallel QML suites
```

`make verify-fast` is the normal per-change gate. It is slower than `make test`
because it runs the compositor conformance and demo harness.

Use the heavier targets when the change warrants them:

| Target | What it adds | When |
|---|---|---|
| `make check` | `lint` + `test` + 100-cycle teardown soak | Session, compositor lifecycle, or teardown changes |
| `make gates` | Gate checks + e2e + serial QML suites | Final tree before a release or a risky phase boundary |
| `make verify-settings` | Gate checks + QML suites only | Settings visual-parity changes with no Rust/compositor change |
| `make dist FORMATS=tar` | Packaged install tree | Packaging or install-layout changes |

Notes:

- `make gates` is intentionally serial and deterministic. Run it once on the
  final tree, not after every docs edit.
- The CI workflow runs on Ubuntu with the pinned Rust toolchain and a newer Qt
  than the floor. A green local Fedora run does not excuse breaking Debian or
  the Qt 6.8 floor.
- If a check cannot run on your machine (no host Wayland session, no DRM seat,
  no distro packaging tool), say so in the PR and state which rung of the
  testing ladder you did verify. Do not claim a hardware result you did not
  observe.
- `git diff --check` should be clean before you push.

### Generated and local-only files

Tracked generated files must be regenerated, reviewed, and committed with the
change that caused them:

- `design-system/Theme.qml`
- `compositor/src/design_tokens.rs`
- `design-system/BrandGlyphs.qml`
- `design-system/PhosphorGlyphs.qml`
- `design-system/TablerGlyphs.qml`
- generated app artwork under `assets/icons/apps/`
- `translations/dragonfruit.ts`

Everything else produced by a build, test, capture, or agent session is local:

- `build/`, `target/`, `dist/`, `build-release/`
- `docs/captures/`, `docs/tasks/`, `docs/ROADMAP.md`, `docs/v*/`
- `logs/`, `*.log`, `*.trace`, temporary screenshots and recordings
- `harness/`, `.symphony/`, `.stop`

Before committing, inspect the whole tree, including untracked files:

```bash
git status --porcelain=v1 --untracked-files=all
```

Stage explicit paths. Do not use `git add -A` or `git add .`.

## Commits and pull requests

### Branches and commits

Branch from `main`. Use a descriptive name such as `fix/dock-hover` or
`feat/settings-search`. Keep commits focused; a PR is squashed when merged.

Use an area prefix and an imperative summary:

```text
fix(compositor): repaint the Dock edge on scene-generation changes
design-system: keep pointer menu selection under the cursor
docs: explain the contribution gate
```

Keep the subject short. In the body, explain what changed, why, and how it was
verified. Reference the issue when one exists. Do not bump version numbers,
schema revisions, or dependency pins as incidental commit noise.

### Pull request description

Use this shape so reviewers can assess the change without guessing:

```markdown
## Summary
What changed, in one or two sentences.

## Why
The user-visible problem, design goal, or issue. Link the issue/ADR.

## Verification
Commands run and their result. State what could not be run and why.

## Evidence
Relevant tests, captures, or traces. Do not commit captures.

## Risks and follow-ups
Known gaps, `OPEN` rows, hardware not tested, or follow-up work.

## Checklist
- [ ] One concern; no unrelated changes
- [ ] `make lint` passes
- [ ] `make test` passes
- [ ] `make verify-fast` passes (or the reason it cannot)
- [ ] Regression/absence tests added or updated
- [ ] Design doc and `STATUS.md` updated
- [ ] ADR added for a lasting decision
- [ ] `qsTr()` + `make i18n-update` for new strings
- [ ] Tokens, icons, naming, and asset gates updated
- [ ] No captures, logs, build outputs, or other local artifacts
- [ ] Contribution is GPL-3.0 compatible and contains no proprietary assets
```

Draft PRs are welcome for early direction. Mark the PR ready only when the
description is accurate and the checks you claim are actually green.

### Review and merge

- CI must be green before merge. A red check is not waived because the change
  is small.
- Maintainers may ask you to split a PR, add an ADR, add an absence test, or
  remove unrelated cleanup. That is normal review, not a rejection.
- Keep the branch updated when `main` moves. Resolve conflicts in the branch;
  do not force a reviewer to reconstruct your intent.
- Squash merge is the default. The PR title becomes the commit subject, so
  make it match the commit style above.
- Do not merge your own PR unless a maintainer has explicitly asked you to.

## Reporting issues and security problems

A useful issue includes:

- the commit or release,
- the distribution and version,
- whether the session was nested, a dedicated VT, or native DRM,
- the exact steps to reproduce,
- expected versus actual behaviour,
- relevant logs with usernames, hostnames, addresses, and other personal data
  removed.

Do **not** open a public issue for a security vulnerability. Use GitHub's
private vulnerability reporting for the repository if it is available, or
contact a maintainer privately through their GitHub profile. Include the
affected component, reproduction steps, and impact. Never include secrets,
credentials, private keys, or personal data in a public issue, log, or capture.

## Collaboration and AI-assisted work

Be direct, respectful, and constructive. Harassment, personal attacks, and
repeatedly ignoring review guidance are not welcome, and maintainers may close
or block contributions that make the project harder to maintain.

If you use an AI assistant, you are still responsible for every line you
submit. Understand the code, run the checks yourself, do not fabricate logs or
verification claims, and do not submit unattributed code or assets whose
license you cannot establish. High-volume, low-effort PRs will be closed.
