# 0246 — About uses one Settings identity and two surfaces

Status: accepted (2026-11-09)

## Context

The General pane had an About dialog and a second More Info dialog, while the
system menu's About action logged a stub. The structural review measures a
280×503 pt utility panel and a separate 740×820 pt Settings page.

## Decision

- The Settings executable owns both surfaces. `--about-system` opens its
  utility window; More Info reveals the regular Settings window at the measured
  size and opens General → About in place. The General row opens the page.
- The existing Updates identity view remains the one machine identity source.
  Startup disk and free space use the existing General host snapshot; the
  startup-disk row prefers the root-volume label, then the parent disk, root
  device, and mount point. A missing field is labelled “Not available”;
  warranty and parts rows have no Linux owner and are omitted.
- The compact surface uses a dedicated near-opaque utility gradient and its
  own shadow. It does not inherit popup glass. Reduced transparency forces an
  opaque backing; increased contrast strengthens its edge.
- Original vector artwork scales with the output. The regulatory action
  reports absent host certification data instead of fabricating it.

## Consequences

The system menu may launch another process of the same Settings executable
when a full Settings window is already running. A future single-instance
handoff may reuse the running process without changing the two QML surfaces.
The live system-menu journey and dark surfaces are captured. Long/localized
names are captured at 1× and tested at 1×/1.5×/2×. Controlled material fit over
the 21 reference backgrounds and a live localized host remain open evidence
items.
