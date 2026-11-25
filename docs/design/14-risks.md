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
| Input methods | `text-input` / input-method protocols advertised; no shipped IME process yet (G-20; [deferred register](27-stability-and-daily-driver.md#7-deferred-register)). Promote when a CJK user joins the reference set or a distro requires it. | Host IME integration is still open |
| Drag and drop, clipboard | Wayland DnD + data-control paths; files, images, text | Vertical slice |
| Screen sharing | Portal + PipeWire stream path | Infrastructure phase |
| Full-screen games | Direct scanout where possible; avoid compositor overhead | Performance phase |
| Accessibility | Apps live on AT-SPI; shell chrome keeps per-component semantics but is not on the bus yet (G-19; [deferred register](27-stability-and-daily-driver.md#7-deferred-register); see [10-design-system.md](10-design-system.md)). Promote when a screen-reader user joins the reference set. | Apps live; shell component tests |
| Locking | Compositor-enforced fail-secure lock | Kill tests |
| Applications bending protocol expectations | Robustness-first handling; never crash on malformed requests | Protocol fuzzing |

Smithay and wlroots remove a huge amount of work, but neither turns a
compositor into a finished desktop. This is exactly why we reuse the entire
Linux plumbing layer and spend the engineering budget on the pieces users
actually perceive as *our* desktop.

## Current engineering risks

| Risk | Current boundary | Remaining exposure |
|---|---|---|
| Chrome input and presentation | Physical-scale frames, DMA-BUF and pooled shared-memory paths, per-scene producers and guarded popup commits | Shared GUI-thread work, large menu models, strip-gated popup pacing and fallback readback can still raise tail latency. |
| Mixed refresh, scale and adaptive sync | The compositor tracks per-output presentation deadlines and adapts when DRM reports a usable adaptive-sync range. | Shared shell raster scale and popup placement need mixed-output coverage. |
| Native performance and fullscreen | Earlier captures cover native primary-plane material on a single output. The compositor reports scanout candidates separately from actual scanout. | The current menu workload lacks native latency acceptance. Driver fallbacks, cursor and overlay planes, and actual direct scanout require verification. |
| Shell recovery | The compositor preserves the session when the shell exits, and the supervisor starts a new shell. | A shared GUI-thread hang or shell crash affects both strips. Faster recovery and independent fault containment need separate evidence. |
| Multi-GPU and rotated displays | The sampled material path has a single-GPU, normal-transform boundary. | Supported fallback behavior and the visual floor need hardware validation. |
| Compatibility and usability | Compatibility layers, menu projections and per-component keyboard behavior are implemented. | Titlebar menu labels, desktop accessibility, input methods and other product gaps remain in the status register. |

The accepted deferrals are not silent: [design 27 §7](27-stability-and-daily-driver.md#7-deferred-register)
is the one register. Every row names a reason, a user impact, an owner, a
promotion trigger, and an open STATUS gap row in [STATUS.md](STATUS.md).
`scripts/check-deferred-register.py` (part of `make verify-stability`) fails on
a missing field, a placeholder owner, a duplicate, or an unknown/resolved
STATUS row. An item that is not in the table is a bug, not an undocumented
decision. [ADR 0357](adr/0357-deferred-register-contract.md) records the rule.

The [ranked performance gaps](STATUS.md#9-chrome-performance-priorities)
link each risk to an acceptance condition. The
[evidence contract](evidence-and-acceptance.md#54-chrome-performance) defines
the native timing and workload requirements.

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
3. **AirDrop/Continuity/Handoff and reference account services are omitted** —
   there is no permissively licensed drop-in with the same UX, and iCloud,
   Siri, Apple Intelligence, Screen Time, Wallet, Game Center, Find My, and App
   Store are not Linux services. Optional LocalSend/KDE Connect integration
   can be evaluated separately (see
   [interface-naming-and-licensing.md](interface-naming-and-licensing.md)).

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
