# Progress notes

<!-- symphony:digest:start -->
## Key facts (maintained by symphony — do not edit)

- **T183 — T-21.1a chrome transport baseline and decision**: Opt-in shell transport counters (`DRAGONFRUIT_CHROME_STATS=1`) in; `FrameCommitGate` records scene frames, `grabWindow()` readbacks, and the
- **T184 — T-21.1b Chrome HiDPI and fractional scale**: Vendored `fractional-scale-v1.xml` and `viewporter.xml` (MIT) under; `shell/src/chromescale.{h,cpp}`: pure logical -> physical mapping,
- **T185a — T-21.1c close-out**: Final-tree gates: `make build`, `make qml-test` (79/79), `make e2e`; `FrameCommitGate` retired: `shell/src/framecommitgate.h` deleted, its CMake
- **T185a1 — T-21.1c close-out verification**: Verified the committed T185a tree (`c6c87e9`; re-checked at the docs-only; `make build` — pass, 0.35 s.
- **T186a — T-21.1d GPU probe and render-control spike**: `shell/src/gpuchromecapabilities.{h,cpp}`: pure linux-dmabuf feedback-table; `shell/src/gpuchromeprobe.{h,cpp}`: read-only runtime probe. It binds
- **T186b — T-21.1d dmabuf export and single-buffer Dock commit**: `shell/src/chromedmabuf.{h,cpp}`: pure `ChromeDmabufFrame` validation; `shell/src/chromedmabufexport.{h,cpp}`: EGLImage from the FBO texture +
- **T186c — T-21.1d dmabuf ring, sync, and fallback**: `shell/src/chromedmabuf.{h,cpp}`: `ChromeDmabufRing` (capacity 3): acquire/; `GpuChromeFrameProducer` owns one FBO/export per ring slot. `exportFrame()`
- **T186d — T-21.1d GPU transport: rollout, stats, and docs**: `ShellController::commitChromeScene()` is the single commit path for every; `ShellProtocol::commitDmabufFor(label, ...)` and `commitLockDmabuf(id, ...)`
- **T187a — T-21.2a per-output refresh model (pure)**: New pure library module `compositor/src/refresh.rs`, exported as; `OutputRefresh { refresh_mhz, period, deadline, source, vrr }`,
- **T187b — T-21.2a refresh clock wiring**: `AnimationClock` now owns `frame_interval` (default `FRAME_INTERVAL`), with; `DfState` owns `RefreshSchedule` and an optional `DRAGONFRUIT_REFRESH_HZ`
- **T188a — T-21.2b per-output presentation clock (pure)**: New pure library module `compositor/src/presentation.rs`, exported as; `PresentationClock { last_presented, next_deadline, period, refresh_hz, vrr }`
- **T188b — T-21.2b per-output frame requests and earliest-deadline arming**: New pure library module `compositor/src/frame_scheduler.rs` (exported as; `FrameScheduler` keyed by output name with independent `urgent` and
- **T188c — T-21.2b VRR, chrome callbacks, and mixed-refresh evidence**: `PresentationClock` (T188a) now owns a backend-supplied `VrrRange` plus the; `DfState` owns `output_vrr`. `refresh_for_output`, `on_output_mode_changed`,

## Task notes (maintained by symphony — do not edit)

- [T183](progress/T183.md) — Opt-in shell transport counters (`DRAGONFRUIT_CHROME_STATS=1`) in
- [T184](progress/T184.md) — Vendored `fractional-scale-v1.xml` and `viewporter.xml` (MIT) under
- [T185a](progress/T185a.md) — Final-tree gates: `make build`, `make qml-test` (79/79), `make e2e`
- [T185a1](progress/T185a1.md) — Verified the committed T185a tree (`c6c87e9`; re-checked at the docs-only
- [T186a](progress/T186a.md) — `shell/src/gpuchromecapabilities.{h,cpp}`: pure linux-dmabuf feedback-table
- [T186b](progress/T186b.md) — `shell/src/chromedmabuf.{h,cpp}`: pure `ChromeDmabufFrame` validation
- [T186c](progress/T186c.md) — `shell/src/chromedmabuf.{h,cpp}`: `ChromeDmabufRing` (capacity 3): acquire/
- [T186d](progress/T186d.md) — `ShellController::commitChromeScene()` is the single commit path for every
- [T187a](progress/T187a.md) — New pure library module `compositor/src/refresh.rs`, exported as
- [T187b](progress/T187b.md) — `AnimationClock` now owns `frame_interval` (default `FRAME_INTERVAL`), with
- [T188a](progress/T188a.md) — New pure library module `compositor/src/presentation.rs`, exported as
- [T188b](progress/T188b.md) — New pure library module `compositor/src/frame_scheduler.rs` (exported as
- [T188c](progress/T188c.md) — `PresentationClock` (T188a) now owns a backend-supplied `VrrRange` plus the
<!-- symphony:digest:end -->

Shared index for the symphony run. Each task session writes its own note file under the progress/
directory ("TNN.md") with what later tasks need to know: real paths, commands that work, contract
deviations, gotchas. Facts, not narrative. The harness keeps a generated "Key facts" digest and a link
to each task note in this file (between the symphony:digest markers); do not edit that block by hand.
