// SPDX-License-Identifier: MIT
//! The Mission Control and hot corners adapter (T-15.5a).
//!
//! Mission Control and hot corners are **compositor-native** features: the
//! compositor detects a corner (`compositor/src/input/hot_corners.rs`) and
//! drives the single overview state machine
//! (`compositor/src/overview/mod.rs`); the shell learns about both over the
//! private `df_toplevel_manager` bridge (`hot_corner`, `overview_changed`).
//! This crate is the adapter over that host stack. It never re-implements a
//! corner detector or an overview transition — it reuses them by facing their
//! published state:
//!
//! * the trigger **configuration** (the four corner assignments, the dwell and
//!   inset, and the gesture-gating trio the compositor applies live), and
//! * the runtime **overview state** (open/closed, selection, Space/window
//!   counts),
//!
//! behind a [`MissionControlSource`] seam, with the trigger **event** stream
//! ([`HotCornerTrigger`]) and a pure [`MissionControlChange`] diff.
//!
//! # The read path
//!
//! 1. A [`MissionControlSource`] returns the raw [`MissionControlData`] once
//!    per [`MissionControlAdapter::refresh`] (or absence/error).
//! 2. [`MissionControlSnapshot::from_data`] names the corners and actions and
//!    folds the gating and runtime state into the typed snapshot the pane and
//!    tile draw.
//! 3. The adapter drives the shared subscription lifecycle, so a bridge that
//!    comes and goes re-subscribes and re-syncs with no user-visible error.
//!
//! # Read-only runtime, settingsd-owned configuration
//!
//! The compositor owns the one overview machine; the shell only mirrors it
//! ([03-workspaces.md](../../../docs/design/03-workspaces.md)).
//! The durable trigger choices are owned by `settingsd` and applied live by
//! the compositor, exactly as the input pointer keys are — the adapter is the
//! *projection*, not a second settings owner. A consumer that wants to change
//! an assignment writes the settings key; the compositor's policy updates;
//! the bridge publishes the new configuration; the adapter reports it.
//!
//! # Absence
//!
//! A missing bridge (the shell has no `df_toplevel_manager` global, or the
//! compositor is not reachable) is the adapter's `Unavailable` state and hides
//! the item. This is a normal state — trigger configuration still applies from
//! `settingsd`'s defaults and no session startup is blocked. A present bridge
//! that cannot be read is `Error`: visible and inert with the message.
//!
//! # Testing
//!
//! CI has no compositor, so the adapter is driven by [`MockMissionControl`]
//! over the source seam. `kill`/`restart` exercise absence and re-subscribe;
//! `push` and `trigger` drive the state and the event stream.

mod adapter;
mod model;
mod source;

pub use adapter::MissionControlAdapter;
pub use model::{
    GestureGating, HotCorner, HotCornerAction, HotCornerTrigger, MissionControlChange,
    MissionControlSnapshot, OverviewState,
};
pub use source::{MissionControlData, MissionControlSource, MockMissionControl};
