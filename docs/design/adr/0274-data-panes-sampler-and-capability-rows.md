# ADR 0274 — Data panes sample honestly; Displays rows come from one table

Status: accepted

## Context

Battery and Storage are data panes: their charts must show something true or
explain why they cannot. T273 found Battery drawing two permanently empty
chart frames (UPower exposes no charge history), Storage's chart missing the
free remainder and legend, and Displays still titled "Built-in Display" with
the advanced sheet duplicating the main brightness/rotation controls.

The Displays capability table already existed (ADR 0236), but only the
advanced sheet read it. The main pane had grown its own disabled rows, so a
capability could disagree with the sheet.

## Decision

- `services/power` owns a bounded, in-memory battery history sampler. It
  records one point per successful adapter read (same-instant reads replace,
  repeats inside 60 s are deduplicated, the ring holds ten days at one point a
  minute). History starts when the status service starts; an empty series is
  an explicit empty state with a refresh path, never a chart with no data.
  Screen-on usage has no session sampler, so it is one row naming that missing
  half rather than a second empty chart.
- `system-status` publishes the sampler as `history` on the battery view
  (`state`, `count`, `samples` with `secondsAgo`/`percent`, the range and
  interval). The pane reads it like every other adapter value.
- Displays reads its real outputs from the platform (`QGuiApplication`'s
  screens: name, logical size, refresh, primary). The pane's preview is
  proportional to the selected output, and a per-output list appears only
  when more than one exists. Writes remain the global `display.*` keys; the
  shell applies them to every output in this build, and per-output override
  keys are a later backend task.
- The capability table gains an `area` (`pane` or `advanced`) and every
  pane-level disabled row (Use as, auto-brightness, adaptive color, refresh,
  arrangement, Night Light, schedule, temperature, color profile, TV mode) as
  well as the advanced-only rows (resolution list, HDR, VRR, overscan). The
  main groups and the advanced sheet both render from it; the sheet never
  repeats brightness or rotation.
- Storage's stacked chart adds the free remainder and a legend, and its
  groups keep their titles with their own absence notes when UDisks2 is gone.
- Byte sizes are labelled GiB (binary) across Storage, About, and Startup
  Disk; memory labels stay the host's decimal vocabulary.

## Consequences

The Battery pane can be opened before the sampler has points and tells the
user what to do; the history survives no restart, which is explicit rather
than a fake provider. Displays can add an output without a pane edit, but a
disabled row still cannot become live until the compositor/colord half lands.
The capability table is the only place to promote one. Per-output scale
overrides need a settingsd key shape and a shell forwarder; until then the
global key is the compatibility path.
