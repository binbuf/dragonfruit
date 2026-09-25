// SPDX-License-Identifier: MIT
//! T-10.5 acceptance: the Files performance budgets are measured headlessly and
//! recorded as raw numbers ([09-files.md#performance-budgets]).
//!
//! Two budgets are covered here:
//!
//! * **Warm 1k-item folder < 50 ms to first frame**, with the listing streaming
//!   in. The measurement is begin → first non-empty batch applied.
//! * **100k-item listing with windowed delivery**: the incremental delta ABI
//!   must send each streamed node exactly once (no whole-snapshot re-delivery),
//!   and a scroll sweep over the loaded model must read a viewport well within
//!   one 60 Hz frame.
//!
//! The Qt side (windowed delegate virtualization) is measured by the
//! `tst_files_performance` runner; the numbers are recorded together under
//! `docs/captures/t10-files-perf.txt`. Run this test with `--nocapture` to see
//! the raw values.
//!
//! [09-files.md#performance-budgets]: ../../../docs/design/09-files.md

use std::sync::Arc;
use std::time::{Duration, Instant};

use dragonfruit_files_core::ffi::{
    df_files_begin_synthetic, df_files_delta_free, df_files_free, df_files_poll_delta,
    DF_FILES_STATUS_DONE, DF_FILES_STATUS_ERROR,
};
use dragonfruit_files_core::{
    DirectoryModel, ListingHandle, Location, StdFsSource, SyntheticSource,
};

/// One 60 Hz frame, the T-10.5 scroll budget.
const ONE_FRAME_US: u64 = 1_000_000 / 60;
/// The T-10.5 warm-open budget.
const OPEN_BUDGET_MS: u64 = 50;

/// Apply events until the model completes (or the timeout fires).
fn drain_to_completion(model: &mut DirectoryModel, handle: &ListingHandle) {
    let timeout = Duration::from_secs(60);
    while !model.is_complete() {
        match handle.recv_timeout(timeout) {
            Some(event) => {
                model.apply(event);
            }
            None => panic!("listing stalled in state {:?}", model.state()),
        }
    }
}

#[test]
fn a_warm_1k_folder_paints_below_50ms() {
    const FILES: usize = 1_000;
    const BATCH: usize = 256;

    // Warm up the allocator and the worker-spawn path once, so the measured
    // run is the "warm cache" number the budget names.
    {
        let mut warm = DirectoryModel::with_batch_size(BATCH);
        let handle = warm.begin(
            Arc::new(SyntheticSource::new(FILES)),
            Location::file("/synthetic"),
        );
        drain_to_completion(&mut warm, &handle);
    }

    let mut model = DirectoryModel::with_batch_size(BATCH);
    let start = Instant::now();
    let handle = model.begin(
        Arc::new(SyntheticSource::new(FILES)),
        Location::file("/synthetic"),
    );
    let first = handle
        .recv_timeout(Duration::from_secs(5))
        .expect("first batch");
    model.apply(first);
    let elapsed = start.elapsed();

    let elapsed_ms = elapsed.as_secs_f64() * 1_000.0;
    println!(
        "T-10.5 warm 1k open: first_frame={:.2} ms (budget < {OPEN_BUDGET_MS} ms), \
         first_batch_rows={}",
        elapsed_ms,
        model.len()
    );
    assert!(model.is_streaming(), "the first batch streams");
    assert!(!model.is_empty());
    assert!(
        elapsed_ms < OPEN_BUDGET_MS as f64,
        "first frame took {elapsed_ms:.2} ms, budget {OPEN_BUDGET_MS} ms"
    );
    model.check_consistent().expect("consistent");
}

/// Drive the incremental C ABI over a 100k synthetic listing and assert that
/// the stream sends each node exactly once. Returns (rows delivered, elapsed).
unsafe fn incremental_stream(count: u32, batch: u32) -> (u64, Duration) {
    use std::ffi::CString;

    let uri = CString::new("file:///synthetic").expect("uri");
    let session = df_files_begin_synthetic(uri.as_ptr(), count, batch);
    assert!(!session.is_null(), "synthetic session");
    let start = Instant::now();
    let mut rows: u64 = 0;
    let mut last_seen: Option<u32> = None;
    loop {
        let delta = df_files_poll_delta(session, 5_000);
        assert!(!delta.is_null(), "poll returned null");
        let status = (*delta).status;
        assert_ne!(status, DF_FILES_STATUS_ERROR, "listing failed");
        if (*delta).reset != 0 {
            // The synthetic listing never edits the model, so streaming delta
            // only; a reset here would mean the windowed contract regressed.
            panic!("synthetic streaming must not reset");
        }
        // Within one delta the ranks are strictly ascending.
        for index in 0..(*delta).row_count as usize {
            let rank = (*delta).rows.add(index).read().rank;
            if let Some(previous) = last_seen {
                assert!(rank > previous, "delta ranks must ascend");
            }
            last_seen = Some(rank);
            rows += 1;
        }
        df_files_delta_free(delta);
        if status == DF_FILES_STATUS_DONE {
            break;
        }
    }
    let elapsed = start.elapsed();
    df_files_free(session);
    (rows, elapsed)
}

#[test]
fn a_100k_stream_delivers_each_node_once() {
    const COUNT: u32 = 100_000;
    const BATCH: u32 = 512;

    let (rows, elapsed) = unsafe { incremental_stream(COUNT, BATCH) };
    let elapsed_ms = elapsed.as_secs_f64() * 1_000.0;
    println!(
        "T-10.5 100k stream: rows_delivered={rows} (windowed, not {count}x snapshots), \
         elapsed={elapsed_ms:.1} ms, batch={BATCH}",
        count = COUNT
    );
    assert_eq!(
        rows,
        u64::from(COUNT),
        "each node is delivered exactly once"
    );
}

#[test]
fn a_100k_scroll_viewport_fits_one_frame() {
    const FILES: usize = 100_000;
    const BATCH: usize = 512;
    const VIEWPORT: usize = 40;
    const SWEEP: usize = 250;

    let mut model = DirectoryModel::with_batch_size(BATCH);
    let handle = model.begin(
        Arc::new(SyntheticSource::new(FILES)),
        Location::file("/synthetic"),
    );
    drain_to_completion(&mut model, &handle);
    assert_eq!(model.len(), FILES);
    model.check_consistent().expect("consistent");

    // A scroll sweep: read a 40-row viewport at `SWEEP` positions spread over
    // the 100k list. This is the model-side cost of a frame; Qt's delegate
    // virtualization keeps the painted work bounded by the viewport too.
    let stride = (FILES - VIEWPORT) / SWEEP;
    let start = Instant::now();
    let mut checked = 0usize;
    for step in 0..SWEEP {
        let first = step * stride;
        for rank in first..first + VIEWPORT {
            let node = model.ordered_node(rank).expect("viewport row");
            checked += node.display_name().len();
        }
    }
    let elapsed = start.elapsed();
    let per_viewport_us = elapsed.as_micros() as f64 / SWEEP as f64;
    println!(
        "T-10.5 100k scroll: viewport={VIEWPORT} rows, sweeps={SWEEP}, \
         per_viewport={per_viewport_us:.1} us (budget < {ONE_FRAME_US} us), \
         total={:.2} ms, names_seen={checked}",
        elapsed.as_secs_f64() * 1_000.0
    );
    assert!(
        per_viewport_us < ONE_FRAME_US as f64,
        "a viewport read took {per_viewport_us:.1} us, budget {ONE_FRAME_US} us"
    );
}

#[test]
fn the_real_fallback_still_streams_a_warm_folder() {
    // A belt-and-braces check that the budget test's synthetic source is not
    // masking a real-backend regression: list a real 1k tree too.
    const FILES: usize = 1_000;
    let dir = tempfile::tempdir().expect("temp dir");
    for index in 0..FILES {
        std::fs::write(dir.path().join(format!("file-{index:05}.txt")), b"x").expect("write");
    }
    let mut model = DirectoryModel::with_batch_size(256);
    let start = Instant::now();
    let handle = model.begin(Arc::new(StdFsSource::new()), Location::file(dir.path()));
    let first = handle
        .recv_timeout(Duration::from_secs(5))
        .expect("first batch");
    model.apply(first);
    let elapsed_ms = start.elapsed().as_secs_f64() * 1_000.0;
    println!("T-10.5 real 1k open: first_frame={elapsed_ms:.2} ms");
    assert!(!model.is_empty());
}
