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
| Input methods | `text-input` / input-method protocols from the start | Early integration tests |
| Drag and drop, clipboard | Wayland DnD + data-control paths; files, images, text | Vertical slice |
| Screen sharing | Portal + PipeWire stream path | Infrastructure phase |
| Full-screen games | Direct scanout where possible; avoid compositor overhead | Performance phase |
| Accessibility | Component-level AT-SPI verification (see [10-design-system.md](10-design-system.md)) | Per component |
| Locking | Compositor-enforced fail-secure lock | Kill tests |
| Applications bending protocol expectations | Robustness-first handling; never crash on malformed requests | Protocol fuzzing |

Smithay and wlroots remove a huge amount of work, but neither turns a
compositor into a finished desktop. This is exactly why we reuse the entire
Linux plumbing layer and spend the engineering budget on the pieces users
actually perceive as *our* desktop.

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
