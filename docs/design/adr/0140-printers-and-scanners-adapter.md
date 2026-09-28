# 0140 — The Printers and Scanners adapter projects the host stack

- **Status:** Accepted (T-15.12a)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settings.md](../08-settings.md),
  ADR [0119](0119-storage-adapter-absence-and-mount-outcomes.md),
  ADR [0128](0128-battery-power-profiles-adapter.md),
  ADR [0138](0138-users-and-groups-adapter.md)

## Context

T-15.12 splits the macOS Printers & Scanners surface into an adapter (T-15.12a)
and a Settings pane plus Control Center tile (T-15.12b). [07-system-integration.md]
routes the surface to "CUPS + SANE". CUPS owns the print queues; SANE owns the
scanner devices. The task requires state, events, and absent-daemon behavior,
unit-tested against a mock, and forbids reimplementing printing or scanning.
CUPS ships no stable machine-readable local API, and the design's guardrail says
to "ship minimal IPP print + queue status first".

## Decision

- **A new crate, `dragonfruit-printer-adapter` (`services/printer-adapter`).**
  It implements the T-07 adapter contract (`Adapter`, `AdapterState`,
  `Subscription`) behind its own `PrintSource` seam, exactly like the other
  T-15 adapters. `AdapterId::PRINTER` (id `printer`) joins the shared ids.
- **The printers half reuses the CUPS client tools.** `HostPrint` runs one
  `lpstat -l -t` per `read` (scheduler state, default destination, device URIs,
  accepting state, queues with their long info, and queued jobs) and parses it
  in one place, `printers_from_lpstat`, pinned by a fixture. The writes are one
  tool call each: `lpadmin -d` for the default destination,
  `cupsaccept`/`cupsreject` for the accepting flag, and `cancel` for a job. This
  is the same reuse the audio adapter makes of `pw-dump`/`wpctl` and the input
  adapter of `libinput list-devices`; the project links no CUPS or SANE
  library.
- **The scanners half reuses `scanimage -L`.** Each
  ``device `…' is a …`` line becomes a device with its verbatim description and
  a derived physical form (flatbed/sheet-fed/handheld), classified by
  `ScannerKind::from_description`; an unrecognized description stays `Unknown`.
- **Absence is layered and per stack.** The adapter is `Unavailable` only when
  neither CUPS nor SANE is reachable. CUPS answering with no queue is
  `Available` with an empty printer list (`PrintData::printers` is
  `Some(vec![])`); a running host with no SANE is `Available` with
  `scanners: None`, so only the scanner half disables. A present stack that
  cannot be read is `Error`, visible and inert with the message.
- **Explicit writes, no invented snapshots.** `set_default_printer`,
  `set_printer_accepting_jobs`, and `cancel_job` answer
  `Applied`/`Denied`/`Absent`/`Failed`; CUPS publishes the result and the host
  re-reads. A policy refusal is surfaced per action without degrading the read
  state.
- **One event stream.** `PrintSnapshot::changes(previous)` is a pure diff — the
  daemons' presence, queues added/removed/edited, the default destination,
  scanners added/removed/edited, and jobs entering/leaving a queue — queued by
  the adapter and drained through `drain_changes`.

Rejected: linking libcups/libsane (the project's adapters stay thin over the
tools the stacks ship); reimplementing IPP in Rust (the CUPS tools already speak
it and the queue state is not the project's to own); hiding the whole item when
one stack is missing (absence is layered, and printers and scanners are
independent); treating a missing tool as an error (it is a normal absence).

## Consequences

- `dragonfruit-printer-adapter` is a workspace member whose dependencies are the
  adapter contract and `serde`. CI drives it with `MockPrint`; no CUPS, SANE,
  shell, or hardware is involved. `make e2e` runs
  `cargo test -p dragonfruit-printer-adapter`.
- `HostPrint` is the live `PrintSource`. Its read is best-effort over the tools'
  human-oriented output; the parser is total and pinned by a fixture, and the
  mock is the contract's CI path.
- Adding a printer (discovery + `lpadmin -p …`) is not in this adapter; the
  T-15.12b pane must decide its add flow and may extend the adapter rather than
  reimplementing CUPS. T-15.12b declares any settingsd presentation preferences
  and wires the pane/tile; it must not add a second print or scan
  implementation.