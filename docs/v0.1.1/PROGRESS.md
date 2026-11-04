# Progress notes

<!-- symphony:digest:start -->
## Key facts (maintained by symphony — do not edit)

- **T183 — T-21.1a chrome transport baseline and decision**: Opt-in shell transport counters (`DRAGONFRUIT_CHROME_STATS=1`) in; `FrameCommitGate` records scene frames, `grabWindow()` readbacks, and the
- **T184 — T-21.1b Chrome HiDPI and fractional scale**: Vendored `fractional-scale-v1.xml` and `viewporter.xml` (MIT) under; `shell/src/chromescale.{h,cpp}`: pure logical -> physical mapping,
- **T185a — T-21.1c close-out**: Final-tree gates: `make build`, `make qml-test` (79/79), `make e2e`; `FrameCommitGate` retired: `shell/src/framecommitgate.h` deleted, its CMake

## Task notes (maintained by symphony — do not edit)

- [T183](progress/T183.md) — Opt-in shell transport counters (`DRAGONFRUIT_CHROME_STATS=1`) in
- [T184](progress/T184.md) — Vendored `fractional-scale-v1.xml` and `viewporter.xml` (MIT) under
- [T185a](progress/T185a.md) — Final-tree gates: `make build`, `make qml-test` (79/79), `make e2e`
<!-- symphony:digest:end -->

Shared index for the symphony run. Each task session writes its own note file under the progress/
directory ("TNN.md") with what later tasks need to know: real paths, commands that work, contract
deviations, gotchas. Facts, not narrative. The harness keeps a generated "Key facts" digest and a link
to each task note in this file (between the symphony:digest markers); do not edit that block by hand.
