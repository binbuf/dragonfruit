# Design

Architecture documents for Dragonfruit. One topic per file. Agent sessions read
the docs relevant to their task before editing code and update them when
behaviour changes. Decisions that constrain later work go into `adr/` as
numbered records.

## Start here

| Document | What it answers |
|---|---|
| [STATUS.md](STATUS.md) | **What is actually implemented**, what is partial, what is design-only, and the current gap/roadmap register |
| [00-overview.md](00-overview.md) | Product thesis, accepted compromises, non-goals |
| [01-architecture.md](01-architecture.md) | Process model, ownership boundary, state ownership, IPC, repository layout |
| [02-compositor.md](02-compositor.md) | Compositor backends, input, renderer/effects, window model, protocols |
| [03-workspaces.md](03-workspaces.md) | Spaces, Mission Control, gestures |
| [04-shell.md](04-shell.md) | Menu bar, Dock, Applications drawer, Control Center, notifications, OSD |
| [05-window-decorations.md](05-window-decorations.md) | SSD/CSD policy, traffic lights, window menu |
| [06-global-menu.md](06-global-menu.md) | Menu broker, native/DBusMenu tiers, accelerators |
| [07-system-integration.md](07-system-integration.md) | Adapters, NetworkManager/BlueZ/PipeWire/UPower/UDisks, portals |
| [08-settings.md](08-settings.md) | Settings app, settingsd, pane catalog, absence matrix |
| [09-files.md](09-files.md) | Files app, files-core, trash, desktop icons |
| [10-design-system.md](10-design-system.md) | Components, tokens, typography, motion, quality gates |
| [11-session-and-dev-workflow.md](11-session-and-dev-workflow.md) | Session model, nested/native workflows, testing ladder |
| [12-packaging.md](12-packaging.md) | Fedora/Debian packaging and distribution |
| [22-total-experience.md](22-total-experience.md) | The whole-desktop target: shell, navigation, Settings, motion, HUDs, ownership |
| [23-macos-structural-surface-review.md](23-macos-structural-surface-review.md) | Measured window chrome, About, Spotlight Apps, Notification Center, Control Center and Finder anatomy |
| [settings-business-logic-map.md](settings-business-logic-map.md) | Every Settings pane/row to its owner and absence path |
| [distro-compatibility.md](distro-compatibility.md) | Fedora/Ubuntu/Debian/Arch host-service and packaging contract |
| [distro-adapter-contract.md](distro-adapter-contract.md) | Cross-distro adapter traits, command matrix, dependency sets, mock, and probe (T221) |
| [interface-naming-and-licensing.md](interface-naming-and-licensing.md) | Generic names, original assets, third-party substitutions |
| [motion-and-feedback.md](motion-and-feedback.md) | Motion language, HUDs, sound, cursor |
| [evidence-and-acceptance.md](evidence-and-acceptance.md) | Capture, measurement, and acceptance process |
| [experience-schema.md](experience-schema.md) | Machine-readable surface/state/axis/evidence manifest and per-surface thresholds |
| [14-risks.md](14-risks.md) | Risks, hard limits, IP constraints |
| [architecture-audit-native-rendering.md](architecture-audit-native-rendering.md) | The native DRM/material/shell-transport audit and its implementation record |
| [advisor-briefing.md](advisor-briefing.md) | Historical context pack; superseded where it conflicts with STATUS/audit |

The track documents in [tracks/](tracks/) are the design references for the
T-01…T-22 slices. They describe the target at the time the track was cut; use
`STATUS.md` for the current state. Track 22 (v0.2.0) is the total-experience
track: [tracks/22-total-experience-parity.md](tracks/22-total-experience-parity.md);
its executable task set is [tasks/0.2.0/](../tasks/0.2.0/README.md).

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
```

The generated `docs/INDEX.md` is a convenience listing; it is not the source of
truth and may lag a hand-added document until the next regeneration.
