// SPDX-License-Identifier: MIT
//! The libinput keyboard, mouse, and trackpad adapter (T-15.4a).
//!
//! The Settings Keyboard/Mouse/Trackpad pane and the Control Center tile do
//! not read libinput themselves. They read an [`InputAdapter`], which holds the
//! last device inventory libinput reported and exposes the three-state contract
//! from `dragonfruit-system-adapters` ([07-system-integration.md]): available,
//! hidden when libinput is absent, or visible-and-inert on a read error.
//! Nothing above this crate sees a libinput type.
//!
//! # The read path
//!
//! 1. [`CommandLibinput`] runs the libinput control tool once per
//!    [`InputAdapter::refresh`] (`libinput list-devices`) and returns the raw
//!    [`InputData`] (or absence/error).
//! 2. [`crate::model`] classifies the devices into keyboards, mice, and
//!    trackpads and derives the typed [`InputSnapshot`] the UI draws.
//! 3. The adapter drives the shared subscription lifecycle, so a host stack
//!    that comes and goes re-subscribes and re-syncs with no user-visible
//!    error.
//!
//! # Read-only by design
//!
//! libinput keeps no persisted configuration and ships no setter, so this
//! adapter has no writes. The user's keyboard/pointer settings are owned by
//! `settingsd` and applied live by the compositor over the private
//! `df_toplevel_manager` bridge ([adr/0034]); the adapter is the *inventory*
//! half — which devices exist and what they can do. This keeps the design's
//! rule that keyboard settings go through the compositor API, never a system
//! daemon ([07-system-integration.md] principle 3).
//!
//! # Absence
//!
//! libinput being absent (not installed, or unable to reach a seat) is the
//! adapter's `Unavailable` state and hides the item. A session that runs
//! libinput but has **no recognized device** still answers `Available`, with
//! [`InputSnapshot::present`](crate::InputSnapshot::present) false; a consumer
//! hides the item then too. Neither blocks session startup.
//!
//! # Testing
//!
//! CI has no input hardware and usually no seat access, so the adapter is
//! driven by [`MockInput`] over a fixture ([`InputSource`] is the seam). The
//! live tool is a thin mechanical layer over that seam, and the tool's output
//! format is pinned by a captured fixture
//! (see [adr/0124](../../../docs/design/adr/0124-input-device-adapter.md)).
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md
//! [adr/0034]: ../../../docs/design/adr/0034-compositor-policy-via-shell-bridge.md

mod adapter;
mod libinput;
mod model;
mod source;

pub use adapter::InputAdapter;
pub use libinput::{CommandLibinput, LIBINPUT_BIN};
pub use model::{DeviceChange, DeviceKind, InputDevice, InputSnapshot};
pub use source::{InputData, InputDeviceData, InputSource, MockInput};
