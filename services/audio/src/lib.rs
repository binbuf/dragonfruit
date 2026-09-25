// SPDX-License-Identifier: MIT
//! The PipeWire/WirePlumber audio adapter: the default sink's volume and mute,
//! and the per-sink list (T-07.3).
//!
//! The menu bar's volume item does not talk to PipeWire. It reads an
//! [`AudioAdapter`], which holds the last state the session manager pushed and
//! exposes the three-state contract from `dragonfruit-system-adapters`
//! ([07-system-integration.md]): available, hidden when the daemon is absent,
//! or visible-and-inert on a read error. Nothing above this crate sees a
//! `wpctl`/`pw-dump`/PipeWire type.
//!
//! # The read path
//!
//! 1. [`CommandAudio`] runs WirePlumber's `pw-dump` once per
//!    [`AudioAdapter::refresh`] and decodes the JSON graph into the raw
//!    [`AudioData`] (or absence/error).
//! 2. [`crate::model`] turns that into the typed [`AudioSnapshot`]: the sink
//!    list, the marked default sink, and its linear volume/mute.
//! 3. The adapter drives the shared subscription lifecycle, so a WirePlumber
//!    restart re-subscribes and re-syncs with no user-visible error.
//!
//! # The write path
//!
//! [`AudioAdapter::set_volume`] and [`AudioAdapter::set_mute`] are the two
//! writes, both over the same source seam (`wpctl` for the live source). They
//! are explicit user actions, never a poll, and they do not invent a snapshot:
//! the daemon pushes the resulting state and the host re-reads the adapter, so
//! the snapshot stays the single source of truth. Routing and device switching
//! are explicitly deferred (T-15).
//!
//! # Pinning the WirePlumber API
//!
//! There is no stable D-Bus volume API and the native `libpipewire` headers are
//! not part of the pinned toolchain, so the live source talks to WirePlumber
//! through the tools it ships and isolates the JSON/CLI churn in
//! [`crate::pw_dump`] ([adr/0028]). A later task can swap a native PipeWire
//! client in behind the same [`AudioSource`] trait without touching the
//! adapter, the model, or the shell.
//!
//! # Testing
//!
//! CI has no PipeWire, so the adapter is driven by [`MockAudio`] over a
//! captured fixture ([`AudioSource`] is the seam). The live CLI source is a
//! thin mechanical layer over that seam and is smoke-tested where a session is
//! present.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md
//! [adr/0028]: ../../../docs/design/adr/0028-audio-adapter-over-wireplumber-cli.md

mod adapter;
mod model;
mod pw_dump;
mod source;

pub use adapter::AudioAdapter;
pub use model::{AudioSnapshot, Sink};
pub use pw_dump::{CommandAudio, DEFAULT_SINK_TARGET, PW_DUMP_BIN, WPCTL_BIN};
pub use source::{AudioData, AudioSource, MockAudio, SetOutcome, SinkData};
