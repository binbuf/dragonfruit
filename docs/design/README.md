# Design

Architecture documents for Dragonfruit. One topic per file. Agent sessions read
the docs relevant to their task before editing code and update them when
behaviour changes. Decisions that constrain later work go into `adr/` as
numbered records.

**Release status (2026-11-25).** The stability set (T304–T348) is agent-done:
`make gates` and `make verify-stability` are green. The reference-hardware
daily-driver matrix is `OPEN` (0 PASS / 0 FAIL / 17 OPEN; no native intake), so the
README's daily-driver note stays. See [design 27 §6](27-stability-and-daily-driver.md#6-release-gate-and-status-transition),
[STATUS.md](STATUS.md), and [ADR 0359](adr/0359-stability-close-out-verdict.md).

## Start here

| Document | What it answers |
|---|---|
| [STATUS.md](STATUS.md) | **What is actually implemented**, what is partial, what is design-only, and the current gap/roadmap register |
| [Chrome performance priorities](STATUS.md#9-chrome-performance-priorities) | Ranked Dock, Menu Bar, and menu gaps with the evidence needed to close them |
| [00-overview.md](00-overview.md) | Product thesis, accepted compromises, non-goals |
| [01-architecture.md](01-architecture.md) | Process model, ownership boundary, state ownership, IPC, repository layout |
| [02-compositor.md](02-compositor.md) | Compositor backends, input, renderer/effects, window model, protocols |
| [03-workspaces.md](03-workspaces.md) | Spaces, Overview, gestures |
| [04-shell.md](04-shell.md) | Menu bar, Dock, Applications drawer, Quick Settings, notifications, OSD |
| [05-window-decorations.md](05-window-decorations.md) | SSD/CSD policy, traffic lights, window menu |
| [06-global-menu.md](06-global-menu.md) | Menu broker, native/DBusMenu tiers, accelerators |
| [07-system-integration.md](07-system-integration.md) | Adapters, NetworkManager/BlueZ/PipeWire/UPower/UDisks, portals |
| [08-settings.md](08-settings.md) | Settings app, settingsd, pane catalog, absence matrix |
| [09-files.md](09-files.md) | Files app, files-core, trash, desktop icons |
| [10-design-system.md](10-design-system.md) | Components, tokens, typography, motion, quality gates |
| [11-session-and-dev-workflow.md](11-session-and-dev-workflow.md) | Session model, nested/native workflows, testing ladder |
| [12-packaging.md](12-packaging.md) | Fedora/Debian packaging and distribution |
| [22-total-experience.md](22-total-experience.md) | The whole-desktop target: shell, navigation, Settings, motion, HUDs, ownership |
| [23-macos-structural-surface-review.md](23-macos-structural-surface-review.md) | Measured window chrome, About, Search and Applications, Notifications, Quick Settings and Files anatomy; decision in [ADR 0241](adr/0241-macos-structural-surface-review.md) |
| [24-system-settings.md](24-system-settings.md) | System Settings target: measured window/IA, every the reference design pane row, Linux adaptations, absence/empty states, 2026-11-14 findings register, and the T263–T284 task map in the `system-settings` set |
| [25-system-settings-visual-parity.md](25-system-settings-visual-parity.md) | The 2026-11-17 mockup target: reduced IA, chrome/density, per-category dispositions, Menu Bar Controls and OS glyph, and the T285–T302b session-sized task map in the `system-settings-2` set |
| [26-settings-logic.md](26-settings-logic.md) | Per-category behaviour logic: which rows exist on which hardware, and the deliberate divergences from the macOS reference; Energy is the first completed category |
| [27-stability-and-daily-driver.md](27-stability-and-daily-driver.md) | The daily-driver contract: lock/focus/session/idle/hardware/data/multi-monitor invariants, the T304–T348 stability task map, the reference-hardware matrix (`make t347-daily-driver-matrix`), and the checked deferred register (`scripts/check-deferred-register.py`). T348 close-out: agent gate green; T347 hardware matrix `OPEN`, so the daily-driver claim is not flipped ([ADR 0359](adr/0359-stability-close-out-verdict.md)). |
| [settings-business-logic-map.md](settings-business-logic-map.md) | Every Settings pane/row to its owner and absence path |
| [distro-compatibility.md](distro-compatibility.md) | Fedora/Ubuntu/Debian/Arch host-service and packaging contract |
| [distro-adapter-contract.md](distro-adapter-contract.md) | Cross-distro adapter traits, command matrix, dependency sets, mock, and probe (T221) |
| [interface-naming-and-licensing.md](interface-naming-and-licensing.md) | Generic names, original assets, third-party substitutions |
| [motion-and-feedback.md](motion-and-feedback.md) | Motion language, HUDs, sound, cursor |
| [evidence-and-acceptance.md](evidence-and-acceptance.md) | Capture, measurement, and acceptance process, including the native chrome performance gate |
| [experience-schema.md](experience-schema.md) | Machine-readable surface/state/axis/evidence manifest and per-surface thresholds |
| [14-risks.md](14-risks.md) | Risks, hard limits, IP constraints |
| [ipc-versioning.md](ipc-versioning.md) | Lockstep versioning for private protocols and D-Bus contracts |
| [private-protocols.md](private-protocols.md) | Private compositor↔shell protocol surfaces and trust model |
| [keymap.md](keymap.md) | Modifier-role mapping and global shortcut grammar |
| [settings-keys.md](settings-keys.md) | Every `org.dragonfruit.Settings1` key, owner, and consumer |
| [licensing.md](licensing.md) | Repo-wide licensing policy and third-party obligations |
| [testing-ladder.md](testing-ladder.md) | Nested, dedicated-session, VM, and native testing rungs |
| [runbook-drm-session.md](runbook-drm-session.md) | Real DRM/KMS session bring-up, validation, and teardown |
| [version-increment.md](version-increment.md) | Release version bump, tag, publish, and verify runbook |
| [xwayland-scaling.md](xwayland-scaling.md) | Integer-scaled Xwayland and fractional output policy |

The [whole-desktop design](22-total-experience.md) defines the cross-component
target. See the [current status register](STATUS.md) for implementation and
hardware evidence. Delivery units live in the local, gitignored
`docs/tasks/0.2.0/` index.

## Reading rules

1. **Topic docs describe the target; STATUS records the delta.** If they
   disagree, the topic doc is the intended design and STATUS is the current
   truth. Fix the topic doc in the same change that changes the behavior.
2. **Code is evidence, not a design document.** When a doc claims a feature,
   it should name the file, test, capture, or ADR that proves it. "Implemented"
   without evidence is a documentation bug.
3. **A checked task is not a hardware or human sign-off.** The roadmap's
   checkboxes mean agent-done at the task bar; the T-17 report and STATUS keep
   hardware/human rows separate.
4. **One decision, one ADR.** New behavior that constrains later work gets an
   ADR. ADR numbers are never reused; a correction gets a note in the record.

## ADR conventions

- `adr/0000-template.md` is the shape.
- Number sequentially from the highest existing id. Check for collisions
  before committing a new record.
- Status is `proposed`, `accepted`, `superseded`, or `rejected`.
- Superseded records stay in place with a pointer to the replacement; they are
  not deleted.
- If a record is renumbered, add a note at the top with the old id and update
  every reference.

## Documentation checks

There is no single doc-lint target yet. The audit used these checks; run them
after a docs change:

```bash
# local Markdown links resolve
python3 - <<'PY'
import glob, os, re
for f in glob.glob('docs/design/**/*.md', recursive=True):
    text = open(f, encoding='utf-8').read()
    for m in re.finditer(r'\[[^\]]*\]\(([^)]+)\)', text):
        link = m.group(1).split('#')[0]
        if not link or link.startswith(('http', 'mailto:')):
            continue
        target = os.path.normpath(os.path.join(os.path.dirname(f), link))
        if not os.path.exists(target):
            print(f, '->', m.group(1))
PY

# ADR ids are unique
ls docs/design/adr | sed -E 's/^([0-9]+).*/\1/' | sort | uniq -d

# every deferred row has its five fields and an open STATUS gap
python3 scripts/check-deferred-register.py
```

A local `docs/INDEX.md` (gitignored) is a convenience listing; it is not the
source of truth and may lag a hand-added document until the next regeneration.

Some docs intentionally point at gitignored local-only material
(`docs/tasks/`, `docs/design/tracks/`, `docs/captures/`, `docs/reference/`,
and the local roadmap files). Those links resolve only in a full local
checkout; they are not part of the checked-in design set.
