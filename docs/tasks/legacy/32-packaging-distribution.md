# T-32 — Packaging and Distribution

| | |
|---|---|
| **Phase** | Cross-cutting (starts Phase 1 with CI builds; gates Phase 5+ real-session testing) |
| **Area** | `packaging/fedora/` + `packaging/debian/` (later) |
| **Depends on** | [T-01](01-repo-scaffolding-ci-licensing.md) (licensing) · [T-24](24-session-lifecycle.md) (units) · everything shippable |
| **Blocks** | First public release · COPR · Fedora Spin |
| **Estimate** | L (Fedora) + L (Debian, later) |
| **Design docs** | [12-packaging.md](../../design/12-packaging.md) · [01-architecture.md](../../design/01-architecture.md) |

## Summary

Fedora 44 packaging as multiple RPMs under the `dragonfruit` namespace,
parallel-installable beside GNOME; COPR distribution first, official repos
when stable, a Fedora Spin when genuinely stable; Debian/Ubuntu later —
mostly a packaging exercise thanks to the `SystemProvider` trait and the
daemon abstraction boundary.

## Background

Fedora is the reference platform; very little of the desktop knows it is
running on Fedora
([12-packaging.md](../../design/12-packaging.md)). Fedora's official Sway,
COSMIC, Budgie, and Xfce spins prove both the distribution and
parallel-install models.

## Scope

### Fedora packaging (in scope)

1. **RPM set** (from
   [12-packaging.md](../../design/12-packaging.md)):
   ```text
   dragonfruit-compositor
   dragonfruit-shell
   dragonfruit-settingsd
   dragonfruit-settings
   dragonfruit-files
   dragonfruit-portal
   dragonfruit-session
   dragonfruit-desktop        # meta-package
   ```
   Contents per the packaging table: compositor binary + protocol XMLs +
   generated bindings; shell process; `settingsd` + `menu-broker` +
   `app-index` (split later if warranted); first-party apps + design
   system assets; `xdg-desktop-portal-dragonfruit` + `portals.conf`;
   Wayland-session desktop file + systemd user units + target; meta
   requiring the rest.
2. **Parallel-install rules**:
   - Everything under the `dragonfruit` namespace: session file
     `dragonfruit.desktop`, D-Bus names `org.dragonfruit.*`, icon theme
     and settings paths — **zero file conflicts with GNOME or any other
     DE**.
   - `portals.conf` restricts backends to
     `xdg-desktop-portal-dragonfruit` plus the generic/GTK backends, so
     Flatpak apps work out of the box.
   - Compositor, shell, and protocol XMLs ship as a **lockstep set**:
     same `%version`; the compositor refuses mismatched protocol versions
     at handshake.
   - **No package requires, recommends, or obsoletes anything GNOME.**
   - **Default handlers**: register `dragonfruit-files.desktop` as the
     default `inode/directory` handler, so folder-opening from any app or
     portal routes to Files ([09-files.md](../../design/09-files.md)).
3. **Distribution path**:
   1. **COPR** for early testers.
   2. **Official repositories** once stable.
   3. **A Fedora Spin** once genuinely stable (the Sway/COSMIC precedent).
4. **Repo hygiene**: spec files reviewed per Fedora guidelines; licenses
   (T-01 decision) map cleanly into `License:` tags; the MIT protocol
   XMLs stay freely implementable by third parties.

### Debian/Ubuntu (later — plan now)

- Most of the architecture is unchanged: NetworkManager, BlueZ, PipeWire,
  UPower, D-Bus, Wayland provide the abstraction boundary; the work is
  packaging, dependency versions, defaults, and distro-specific
  administration.
- `DebianProvider` implements the `SystemProvider` trait
  ([08-settings.md](../../design/08-settings.md)); everything above the
  interface is identical.
- Layout: `packaging/debian/`.

### Out of scope

- Any greeter/display-manager packaging (GDM reused —
  [01-architecture.md](../../design/01-architecture.md)).
- Non-Fedora/non-Debian families (arch etc.) — community, later.

## Requirements

- FR-1: `sudo dnf install dragonfruit-desktop` installs the full desktop
  on a stock Fedora 44 GNOME machine **without removing or conflicting
  with GNOME**; logout shows both sessions in GDM.
- FR-2: Each RPM contains exactly its table contents; no cross-package
  file overlap; namespace audit passes (no paths outside the dragonfruit
  namespace except spec-mandated freedesktop locations like
  `wayland-sessions`).
- FR-3: Lockstep enforcement: all protocol-carrying packages share
  `%version`; a mixed-version install fails at handshake with a clear
  error (test installs old shell RPM against new compositor RPM).
- FR-4: `portals.conf` shipped and effective (Flatpak browser routes to
  our backend — with T-27).
- FR-5: Files is the default `inode/directory` handler after install.
- FR-6: COPR repo builds all packages from the monorepo tag; install
  instructions documented.
- FR-7: `DebianProvider` stub + packaging plan documented (trait from
  T-15; Debian builds deferred by design).

## Acceptance criteria

- [ ] Clean install / upgrade / uninstall on Fedora 44 VM: no conflicts,
      no leftovers, GNOME untouched.
- [ ] Mixed-version lockstep refusal test green.
- [ ] COPR install path exercised end-to-end by a non-developer.
- [ ] Default-handler and portal routing verified post-install.

## Test plan

- VM install matrices (stock GNOME box, minimal box); upgrade across two
  release tags; uninstall cleanliness.
- COPR scratch builds on tag; kinit-less smoke install.

## Risks / open questions

- Fedora review process timelines (official repos) — start early, expect
  iteration.
- Qt 6 version skew between Fedora 44 and Debian stable — keep QML code
  on the oldest supported Qt minor; the `SystemProvider` split keeps the
  rest identical.
