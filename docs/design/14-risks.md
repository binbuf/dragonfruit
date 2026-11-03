# Risks, Hard Limits, and Constraints

## The biggest technical risk is not drawing macOS-looking controls

That part is relatively straightforward. The risk is making the underlying
environment boringly reliable:

| Risk | Mitigation | Where tested |
|---|---|---|
| GPU hotplug, driver failures | Smithay backend handling; hotplug re-init; fail-safe output config | VM + real hardware matrix |
| Suspend/resume | logind `PrepareForSleep` hooks; render quiesce; resume re-init | Soak tests (see [ROADMAP.md](../ROADMAP.md)) |
| Multiple displays, unusual DPI | Fractional scaling via `viewporter`/`fractional-scale`; per-display Spaces | Multi-monitor test machines |
| Xwayland applications | Explicit compatibility phase; tracked app zoo | Compatibility phase |
| Input methods | `text-input` / input-method protocols advertised; no shipped IME process yet (G-20) | Host IME integration is still open |
| Drag and drop, clipboard | Wayland DnD + data-control paths; files, images, text | Vertical slice |
| Screen sharing | Portal + PipeWire stream path | Infrastructure phase |
| Full-screen games | Direct scanout where possible; avoid compositor overhead | Performance phase |
| Accessibility | Apps live on AT-SPI; shell chrome keeps per-component semantics but is not on the bus yet (G-19; see [10-design-system.md](10-design-system.md)) | Apps live; shell component tests |
| Locking | Compositor-enforced fail-secure lock | Kill tests |
| Applications bending protocol expectations | Robustness-first handling; never crash on malformed requests | Protocol fuzzing |

Smithay and wlroots remove a huge amount of work, but neither turns a
compositor into a finished desktop. This is exactly why we reuse the entire
Linux plumbing layer and spend the engineering budget on the pieces users
actually perceive as *our* desktop.

## Current engineering risks (2026-11)

These are the risks the audit found in the actual tree, ordered by leverage.
The gap register in [STATUS.md](STATUS.md) carries the evidence and proposed
units.

| Risk | Why it is material | Current mitigation |
|---|---|---|
| **Shell chrome transport** — offscreen QML → `grabWindow()` → `wl_shm`, no buffer scale or frame callback | Every Dock/menu/popover frame pays a full readback + copy; chrome is soft on fractional-scale outputs; this is the biggest threat to the "premium" claim | P0 unit: physical-scale rendering, buffer pool, frame callbacks, no full-window re-render |
| **Fixed 60 Hz scheduling** — animation clock, timer, and degrade budget all 16 ms | 120/144/240 Hz panels miss deadlines or degrade against the wrong budget; VRR is not represented | P0 unit: per-output refresh deadline and VRR-aware budget |
| **No native visual evidence** — DRM capture explicitly fails | Nested captures cannot prove the material/HiDPI/cursor path users see | P0 unit: trusted KMS readback or external capture procedure |
| **Files feature gap** — no tabs/New Window/copy/move/undo/progress/Open With/menu model | The Finder design promises operations the UI does not expose; users hit the gap immediately | P1 Files-completeness track |
| **Compatibility closure** — DBusMenu clicks, hide verbs, XDnD, `_NET_FRAME_EXTENTS` | These are the classic "Linux desktop jank" moments; the architecture already has the models | P1 compatibility-closure track |
| **Session power/idle** — About/power actions are stubs; no idle service or logind suspend backend | A desktop that cannot reliably lock/suspend/log out is not a daily driver | P1 session-completion track |
| **Multi-GPU/rotated material** — sampled path disabled, feather fallback | Hybrid laptops and rotated displays lose the signature material | P2; never sample across GPUs |
| **Persistent-chrome quality floor** — Dock/menu bar can reach `Minimal` | The most visible surfaces can lose the material under load | P2 policy + cheaper 4K blur |
| **Packaging/hardware rail** — packages are placeholders; driver matrix/perf OPEN | No installable, verifiable product without them | Hardware rail + real packaging work |
| **Shell accessibility/IME** — chrome not on AT-SPI; no shipped input-method process | Screen-reader and CJK users cannot rely on the desktop even though apps are accessible | P2: bridge shell chrome, integrate an existing IME |

The documentation risk is now managed by the rule in
[STATUS.md](STATUS.md): a topic doc describes the target, STATUS records the
delta, and a claim without a test/capture/ADR is a documentation bug.

## Project risks

- **Upstream churn.** Smithay evolves; its API surface is not frozen. We pin
  per release, upgrade deliberately, and keep our compositor a thin policy
  layer over it so upstream changes stay mechanical.
- **Qt licensing.** Qt 6 open source is GPL/LGPL. If closed components or
  official support ever matter, commercial Qt licensing is a business
  decision to make **early**, before first-party app code sprawls.
- **Bus factor.** The estimates in [ROADMAP.md](../ROADMAP.md) assume one
  strong engineer. This docs-first, protocol-versioned design is the
  mitigation: decisions live here, not in one head.
- **Scope creep.** The demo-MVP/daily-driver split is the guardrail. A
  complete printer configuration page is never allowed to precede the
  30-second interaction loop.

## Hard limits (accepted compromises)

Two product promises are bounded by third-party applications and we accept
them from the outset:

1. **Traffic lights wherever the application permits native desktop
   decoration** — CSD applications draw their own titlebars; we never inject
   fake compositor titlebars above them (see
   [05-window-decorations.md](05-window-decorations.md)).
2. **Global menu wherever the application exports one** — we never remove an
   application's internal menu merely because global menu mode is on (see
   [06-global-menu.md](06-global-menu.md)).

## No live compositor handoff

Wayland clients are connected to a specific compositor; there is no
standardized mechanism to migrate live windows from GNOME (or any other
compositor) into ours. "Gracefully swap the DE in and out" is not our
architecture — nested development and a separate login session are (see
[11-session-and-dev-workflow.md](11-session-and-dev-workflow.md)).

## Intellectual property

The U.S. Copyright Office notes that original creative works — including
computer programs and illustrations — receive copyright protection, and
Apple's own intellectual-property guidelines explicitly restrict uses of its
trademarks; unauthorized commercial use of the Apple logo can constitute
infringement or unfair competition.

Therefore: **the behavioral model is extremely familiar while the actual
design assets are ours.**

```text
Good target:
macOS-like workflow
macOS-like spatial organization
macOS-like Dock behavior
macOS-like global-menu concept
macOS-like Settings information architecture
macOS-like Mission Control interaction
left-side three-button window controls

Avoid shipping:
Apple logo
Apple wallpapers
Apple icons
Apple sounds
Apple branding
pixel-copied proprietary artwork
an application actually branded "Finder"
a product marketed as macOS
```

For a commercial/public product, an IP lawyer reviews a near-exact visual
implementation before release.

## The design moving target

There is also a practical reason for our own visual system: the thing we
might be tempted to copy keeps moving. Apple's current release is again
changing window shapes, menu-bar iconography, and materials. Building against
"whatever macOS currently looks like" would make Apple an involuntary upstream
design dependency.

Our internal rule:

> **"Reproduce the interaction quality and mental model, not Apple's bitmap
> output."**

That gives us room to improve things where Linux differs.
