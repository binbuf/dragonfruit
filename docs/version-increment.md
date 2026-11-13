# Version increment and release runbook

How to cut a Dragonfruit release: choose the version, update the one product
version everywhere it is declared, write the changelog, run the release gate,
push `main`, tag, push the tag, and verify the published assets.

This is a release procedure, not a branching model. Releases are cut from
`main`; the tag is the release. `scripts/package.sh` derives the package
version from the tag, and
[`.github/workflows/release.yml`](../.github/workflows/release.yml) publishes
the assets when a `v*` tag is pushed.

Related: [`packaging/README.md`](../packaging/README.md),
[`docs/ipc-versioning.md`](ipc-versioning.md), and
[`AGENTS.md`](../AGENTS.md) (staging rules).

## 1. What the version is — and what it is not

There is **one product version**. It must agree in all of these places on the
commit that is tagged:

| Where | What | Changes at release? |
|---|---|---|
| `Cargo.toml` → `[workspace.package] version` | Rust workspace version; every crate inherits it | **Yes** |
| `Cargo.toml` and member `Cargo.toml` files → `{ path = "...", version = "..." }` | Requirements on sibling workspace crates | **Yes**, all of them |
| `Cargo.lock` | Locked version of every workspace package | **Yes**, regenerate |
| `CMakeLists.txt` → `project(dragonfruit VERSION ...)` | Qt/CMake side of the lockstep product | **Yes** |
| Git tag `vX.Y.Z` | Source of truth for release packages and the workflow | **Yes** |

The same version is embedded in some binaries. For example the portal exposes
its build version as `org.dragonfruit.Portal1.Version`, and the notification
service returns `env!("CARGO_PKG_VERSION")` from `GetServerInformation`, so a
stale Cargo version is user-visible even when the package filename is correct.

Do **not** bump these during a product release:

| Do not touch | Why |
|---|---|
| `packaging/debian/control.in` (`Version: @VERSION@`) | Substituted from `--version` at package time |
| `packaging/fedora/dragonfruit.spec` (`%{df_version}`) | Passed by `scripts/package.sh` |
| `qt_add_qml_module(... VERSION 1.0)` in shell, apps, design-system | QML module/API version, not the product version |
| `design-system/tokens/tokens.json` → `"version"` | Token-schema revision, not the product version |
| `LOCKSTEP_VERSION` in `protocols/df-ipc/src/lib.rs` | Private-protocol set version; bump only for a protocol-set change, in the same commit as the XMLs (see `docs/ipc-versioning.md`) |
| `SCHEMA_VERSION` in `services/settingsd/src/schema.rs` | Settings schema revision; bump only when keys/migrations change |
| `STATE_VERSION` in `services/session/src/dev_session.rs` | Dev-session state-file schema |
| `rust-toolchain.toml`, Smithay pin, Qt floor | Toolchain/dependency decisions, not the product version |

## 2. Choose the number

Use [Semantic Versioning](https://semver.org/spec/v2.0.0.html), with the usual
pre-1.0 allowance: while the product is `0.x`, a **minor** bump may contain
breaking user-visible changes, and a **patch** bump is for compatible fixes.
Additive features go in a minor bump. Reserve the major bump for the first
release that promises a stable surface.

Tags are `vX.Y.Z` (`v0.2.0`). Never tag without the `v`; the workflow strips
it for package metadata (`${GITHUB_REF_NAME#v}`).

### Pre-release tags

`scripts/package.sh` accepts `v0.3.0-rc.1` and replaces the hyphen with `~` for
RPM (and the Arch package, which reuses the RPM-sanitised string):
`0.3.0~rc.1`. RPM sorts that before `0.3.0`; verify the Arch ordering with
`vercmp` before publishing. The DEB path does not rewrite the hyphen, so a DEB
built from `0.3.0-rc.1` sorts *after* `0.3.0` in `dpkg` terms. Until that is
fixed, prefer a manual dispatch for an RC build (no publish), and mark any
GitHub pre-release manually — the workflow does not set `prerelease`. Do not
publish an RC whose upgrade path is wrong.

## 3. Preflight

Start from a clean, current `main`:

```sh
git switch main
git fetch origin
git pull --ff-only origin main
git status --porcelain=v1 --untracked-files=all   # must be empty
git log --oneline --decorate -5
git tag --list 'v*' --sort=-v:refname
git remote -v
```

Confirm `origin` is the repository that should receive the release: the
tag-triggered workflow publishes to the repository that hosts it, and `gh`
operates on the current remote. If `origin` is a personal fork, stop and
switch to the canonical repository before tagging.

Confirm the last `main` CI run is green before adding the release commit:

```sh
gh run list --workflow=ci.yml --branch main --limit 3
```

Pick the old and new versions (the examples below use `0.1.0` → `0.2.0`):

```sh
OLD=0.1.0
NEW=0.2.0
```

## 4. Write the changelog

`CHANGELOG.md` lives at the repository root. If it does not exist yet, create
it with this shape (newest release first, newest entries at the top of each
section):

```markdown
# Changelog

All notable user-visible changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-11-12

### Added

### Changed

### Fixed

### Removed

### Security
```

Draft from history since the previous tag (on a first release, use the whole
history and summarize by track rather than pasting every commit):

```sh
git log --oneline --no-merges --first-parent v0.1.0..HEAD   # previous tag exists
git log --oneline --no-merges --first-parent                # first release
```

Write for users and packagers, not for the task tracker. Group under
**Added / Changed / Fixed / Removed / Security** and call out anything that
changes expectations:

- user-visible behaviour and new surfaces;
- breaking changes and migrations;
- packaging, dependency-floor, or session-entry changes;
- `LOCKSTEP_VERSION` and settings-schema revisions;
- known limitations that are deliberate for this release.

Keep the empty `## [Unreleased]` heading at the top for the next cycle. Do not
rewrite entries from already-published versions.

## 5. Update the version everywhere

### 5.1 Workspace version

Edit `Cargo.toml`:

```toml
[workspace.package]
version = "0.2.0"
```

Every workspace member uses `version.workspace = true`, so there is no
per-crate version to edit.

### 5.2 Sibling path-dependency requirements

Every `{ path = "...", version = "..." }` requirement must match the workspace
version. These live in the root manifest and in the member manifests that
depend on sibling crates.

```sh
export OLD=0.1.0 NEW=0.2.0
git ls-files '*Cargo.toml' | xargs perl -pi -e \
  's{(path = "[^"]+", version = ")\Q$ENV{OLD}\E(")}{$1$ENV{NEW}$2}g'
```

Review the result before continuing:

```sh
git diff -- '*Cargo.toml'
```

Only `path`/`version` pairs should move. **Do not** use a blind
`sed s/0.1.0/0.2.0/`: `smithay-drm-extras = { version = "0.1.0", ... }` is a
third-party dependency that happens to share the current product version.

### 5.3 CMake project version

Edit `CMakeLists.txt`:

```cmake
project(dragonfruit VERSION 0.2.0 LANGUAGES C CXX)
```

The `VERSION 1.0` arguments inside `qt_add_qml_module(...)` stay as-is; those
are QML module API versions.

### 5.4 Regenerate the lockfile

```sh
cargo check --workspace
```

`Cargo.lock` should change only in the `version =` lines for workspace
packages (`df-ipc`, `dragonfruit-*`). Inspect:

```sh
git diff -- Cargo.lock
```

### 5.5 Verify consistency

These must all agree with `NEW`:

```sh
grep -n '^version = ' Cargo.toml
grep -n '^project(dragonfruit VERSION' CMakeLists.txt
git grep -n -E 'path = "[^"]+", version = "' -- '*Cargo.toml'
cargo metadata --no-deps --format-version 1 \
  | python3 -c 'import json,sys; print(sorted({p["version"] for p in json.load(sys.stdin)["packages"]}))'
```

The last command should print exactly `['0.2.0']`. The only remaining
`0.1.0` strings should belong to third-party crates:

```sh
git grep -n '0\.1\.0' -- '*Cargo.toml' CMakeLists.txt Cargo.lock
```

## 6. Run the release gate

Run on the final tree, before the release commit is tagged. The release gate is
serial on purpose:

```sh
make lint      # fmt, clippy, QML lint, naming/assets/i18n/design checks
make gates     # e2e + serial qml-test, plus all non-built gate checks
```

Optional local packaging smoke (the authoritative package builds are in CI):

```sh
make dist VERSION="$NEW" FORMATS=tar
```

On a Linux host with `dpkg-deb`, `rpmbuild`, `bsdtar`, and `zstd`, `make dist
VERSION="$NEW"` builds the full local set for inspection. Never commit
`dist/`, `build/`, or capture output (see `.gitignore` and `AGENTS.md`). After
the gate, `git status --short` should show only the intended release edits; if
a test or capture target left an untracked file, add an ignore rule rather than
staging it.

If anything fails, fix it on `main` before tagging. Do not tag a commit that
has not passed the gate.

## 7. Commit and push `main`

Stage explicitly, per `AGENTS.md`:

```sh
git status --porcelain=v1 --untracked-files=all
git add -u
git add CHANGELOG.md          # only when creating it for the first time
git diff --cached --stat
git diff --cached
git commit -m "release: $NEW"
```

The commit should contain the changelog and every version change from §5 —
nothing generated, no captures, no `dist/`.

Push `main` (or open and merge the release PR if the branch is protected):

```sh
git push origin main
```

Wait for CI on the release commit:

```sh
gh run list --workflow=ci.yml --branch main --limit 1
gh run watch <run-id> --exit-status
```

The commit that gets tagged must already be on `origin/main` and green. If CI
fails, fix it and push a follow-up commit **before** tagging.

## 8. Tag and push the tag

Confirm the working tree is clean and local `main` is exactly `origin/main`:

```sh
git switch main
git pull --ff-only origin main
test -z "$(git status --porcelain)"
test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)"
```

If `main` moved after the validated release commit, do not tag the new tip
until CI is green for it; otherwise tag the validated commit explicitly and
confirm it is reachable from `origin/main`.

Create an annotated tag on that commit:

```sh
git tag -a "v$NEW" -m "Dragonfruit $NEW"
git show --no-patch --decorate "v$NEW"
```

If the project signs tags, use `git tag -s` instead and verify with
`git tag -v "v$NEW"`. Never put secrets or signing keys in the repository.

Push the exact tag — **not** `git push --tags`, which can publish stale local
tags:

```sh
git push origin "v$NEW"
```

Pushing the tag triggers the release workflow. Never move or re-push a tag
once a GitHub release has been published (see §11).

## 9. Watch the release workflow

```sh
gh run list --workflow=release.yml --limit 3
gh run watch <run-id> --exit-status
gh release view "v$NEW"
```

The workflow builds:

- Debian 13: portable `tar.gz` + binary `.deb`
- Fedora 43: binary `.rpm`
- Arch: binary `.pkg.tar.zst`
- source `tar.gz` (`git archive`)
- `SHA256SUMS`

Each package job installs its own output and fails if the session entry is
missing or an ELF has an unresolved shared library. The publish job runs only
for tags; a manual dispatch leaves artifacts on the workflow run and creates
no release.

For a dry run before tagging:

```sh
gh workflow run release.yml --ref main -f version="$NEW"
```

### Verify the published assets

```sh
mkdir -p /tmp/dragonfruit-release
gh release download "v$NEW" --dir /tmp/dragonfruit-release
cd /tmp/dragonfruit-release
sha256sum -c SHA256SUMS          # Linux
shasum -a 256 -c SHA256SUMS      # macOS
```

Check the package versions, not just the filenames:

```sh
dpkg-deb -f ./*.deb Version                       # 0.2.0
rpm -qp --qf '%{VERSION}\n' ./*.rpm               # 0.2.0
bsdtar -xOf ./*.pkg.tar.zst .PKGINFO | grep '^pkgver'   # 0.2.0-1
```

Also confirm the release is attached to the tagged commit and that the tag
still resolves to `origin/main`:

```sh
gh release view "v$NEW" --json tagName,targetCommitish,assets \
  --jq '{tag: .tagName, target: .targetCommitish, assets: [.assets[].name]}'
test "$(git rev-parse "v$NEW^{commit}")" = "$(git rev-parse origin/main)"
```

### Curate the release notes

The workflow uses `generate_release_notes: true`, so GitHub writes notes from
commits and pull requests. Those are usually less useful than the changelog.
After publishing, prepend the `CHANGELOG.md` section to the release body:

```sh
# The "## [0.2.0]" section, up to the next "## [" release heading.
awk -v v="$NEW" '
  index($0, "## [" v "]") == 1 { in_section = 1 }
  in_section && index($0, "## [") == 1 && index($0, "## [" v "]") != 1 { exit }
  in_section { print }
' CHANGELOG.md > /tmp/release-notes.md

printf '\n' >> /tmp/release-notes.md
gh release view "v$NEW" --json body --jq .body >> /tmp/release-notes.md
gh release edit "v$NEW" --notes-file /tmp/release-notes.md
```

If pre-release, also mark it in GitHub (the workflow does not infer this):

```sh
gh release edit "v$NEW" --prerelease
```

## 10. After the release

- Leave `Cargo.toml`, the path pins, and `CMakeLists.txt` at the **released**
  version until the next release commit. Do not carry a speculative next
  version: `scripts/package.sh` derives package metadata from the tag, and a
  working-tree version that is ahead of the latest tag makes local builds and
  `env!("CARGO_PKG_VERSION")` report a version that does not exist yet.
- Keep the new `## [Unreleased]` section at the top of `CHANGELOG.md`; add to
  it as work lands.
- If the release changes the roadmap state, update
  [`docs/design/STATUS.md`](design/STATUS.md) and the relevant design doc in
  the same follow-up change. Do not mechanically rewrite every historical
  `v0.2.0` reference.
- Announce only after the release assets and checksums are verified.

## 11. Rollback and recovery

| Point of failure | Action |
|---|---|
| Gate/CI fails before tagging | Fix on `main`; the tag does not exist yet, so there is nothing to undo |
| Wrong commit tagged, workflow not published | `git tag -d v0.2.0` and `git push origin :refs/tags/v0.2.0`, fix `main`, recreate the tag |
| Release workflow fails before `publish` | No GitHub release exists. Delete the remote tag with `git push origin :refs/tags/v0.2.0`, fix, re-tag; artifacts on the failed run are diagnostic only |
| GitHub release published but an asset is broken | **Do not move the tag.** Hide the release (`gh release edit v0.2.0 --draft`) and cut a patch release (`v0.2.1`) from a fixed `main` |
| Version mismatch found after publish | Same as above: patch release, never re-tag or force-push |
| Wrong version in local `make dist` output | Pass `VERSION=...` to `make dist`, or tag first; `scripts/package.sh` uses the tag, not `Cargo.toml` |

Never force-push `main`, delete a published tag, or reuse a version number.
Only fast-forward fixes and patch releases from here.

## 12. First tagged release

If no `v*` tag exists yet, the first release is also the first time
`scripts/package.sh` finds an exact tag. Before that, `make dist` falls back to
a `git describe`/commit-derived version, not the Cargo version. For a local
preview before the first tag, always pass the intended version explicitly:

```sh
make dist VERSION=0.2.0 FORMATS=tar
```

Make the first tagged commit's Cargo/CMake version and the tag agree, then
push the tag and let the workflow build the named assets.

## Quick checklist

```text
[ ] main is clean, current, and green in CI
[ ] version chosen (SemVer; tag has the leading v)
[ ] CHANGELOG.md section written; [Unreleased] kept
[ ] Cargo workspace version updated
[ ] every path-dependency version requirement updated
[ ] Cargo.lock regenerated (workspace versions only)
[ ] CMake project VERSION updated
[ ] consistency checks pass (one product version everywhere)
[ ] make lint && make gates pass on the final tree
[ ] release commit pushed to main; CI green
[ ] annotated tag created on the pushed commit
[ ] exact tag pushed (not --tags); release workflow green
[ ] release assets + SHA256SUMS downloaded and verified
[ ] package metadata reports the tag version
[ ] GitHub release body curated; pre-release flag set if needed
[ ] tree left at the released version; [Unreleased] ready for next cycle
```
