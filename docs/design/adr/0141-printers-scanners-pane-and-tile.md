# 0141 — The Printers & Scanners pane and tile ride the bridge host

- **Status:** Accepted (T-15.12b)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settings.md](../08-settings.md),
  ADR [0140](0140-printers-and-scanners-adapter.md),
  ADR [0139](0139-users-groups-pane-and-tile.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md)

## Context

T-15.12a landed `dragonfruit-printer-adapter` (CUPS queues + SANE devices over
the host stack). T-15.12b must ship the Settings pane and the Control Center
tile as one functional unit, with every control applying live. The queue
operations are explicit actions over CUPS; the pane's paper-size choice has no
CUPS client-tool equivalent. As with Users & Groups (ADR 0139), the C++/QML
side never links the Rust adapter: it decodes the bridge host's JSON view.

The Control Center panel already held fourteen tiles at the ceiling of its
fixed 360×1160 surface; a fifteenth had to fit without clipping and without a
scrolling panel (the compositor forwards no pointer-axis events).

## Decision

- **One new bridge interface, no new daemon.** `services/system-status` serves
  `org.dragonfruit.SystemStatus1.Printers` at the shared object path. Its
  `PrintersHost<HostPrint>` wraps the T-15.12a adapter and projects one flat
  `printers` view (queues, scanners, counts, default destination) plus the
  three explicit writes (`SetDefaultPrinter`, `SetPrinterAcceptingJobs`,
  `CancelJob`). Each write returns the `applied`/`denied`/`absent`/`failed`
  report and the host re-reads; no snapshot is invented.
- **One settingsd key** (`printers.defaultPaperSize`, revision 17, allowed
  `us-letter`/`us-legal`/`a3`/`a4`/`a5`). The queues, the system default
  destination, and the jobs are CUPS state; the scanner inventory is SANE
  state. Those are host reads plus explicit actions, not durable preferences.
  A system-wide default paper size has no CUPS client-tool equivalent, so the
  pane stores it as a durable preference. The tile is read-only.
- **Absence is layered and visible.** The view is `unavailable` only when
  neither CUPS nor SANE is reachable; a host with no SANE is `available` with
  `scannersAvailable: false` (only the scanner note shows), and a CUPS that
  answers with no queue is `available` with an empty printer list. The pane
  shows a one-line absence note plus a per-half note; the tile hides on
  `unavailable` or when the host has neither a queue nor a device.
- **The Control Center tile is a read-only summary.** It carries the `printer`
  glyph, the live count label, and a `Printers & Scanners Settings…` link. It
  has no write; the queue writes live in the pane.
- **The panel compacts rather than scrolls.** To fit the fifteenth tile the
  content gap moves from `xs` to `xxs` and the new tile's internal gap from
  `sm` to `xs`; the fixed surface holds every tile. A scrolling panel was
  rejected (no pointer-axis forwarding, ADR 0139).
- **Reference deviations, once.** AirPrint wording is dropped (generic
  IPP/DNS-SD); the per-printer disclosure opens a detail dialog (status,
  model/location, accept-jobs, jobs with Cancel, set-default); `Last Printer
  Used` is the popup's value when CUPS names no default (clearing a default is
  not a CUPS client-tool operation, so it is state, not a choice);
  `Add Printer, Scanner, or Fax…` is not shown yet — there is no CUPS discovery
  seam in the adapter, and a disabled button would be a dead control. Adding a
  printer (discovery + `lpadmin -p …`) is recorded as a follow-up and belongs
  in the adapter.

Rejected: a second CUPS/SANE implementation in the shell (the adapter owns it);
Hiding the whole item on a SANE absence (the print half is independent); a
disabled `Add Printer, Scanner, or Fax…` row (dead control).

## Consequences

- `PrintersInterface` joins `interface_names()` (now ten); `main.rs` gains
  `--print-printers`; `services/system-status` depends on
  `dragonfruit-printer-adapter`.
- `apps/settings/PrintersClient` is the pane's seam (`DF_PRINTERS_FIXTURE`
  selects the in-process mock); `shell/src/systemstatusclient` and
  `systemstatusmodel` gain the read-only printers path.
- The design-system `Icon` gains painted `printer` and `scanner` glyphs.
- The Control Center panel stays a non-scrolling fixed surface at 360×1160.
- CUPS authorizes through its policy/polkit; a refusal surfaces as `denied`
  without degrading the read state, and the pane keeps working on the last
  read.
- Adding a printer (CUPS discovery + `lpadmin -p …`) remains a follow-up; the
  pane ships without an Add affordance until the adapter exposes discovery.