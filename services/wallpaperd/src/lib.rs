// SPDX-License-Identifier: MIT
//! The wallpaper content provider (T-18.1a): the shipped original default
//! wallpaper plus Wikimedia Commons Featured Pictures (ADRs 0055 / 0094).
//!
//! The service resolves the shipped [`Default.jpg`](defaults) to a stable local
//! path, then, off the compositor/shell frame path, fetches the six
//! Featured-picture categories into a lazy local cache with attribution
//! metadata. It serves the catalogue over `org.dragonfruit.Wallpaper1` on the
//! user session bus; T-18.1b wires System Settings and the shell to it.
//!
//! # The seam
//!
//! [`ContentSource`] is the provider interface: one `catalogue` read per
//! category and one `download` per item. The live source is
//! [`WikipediaSource`] over an [`HttpClient`]; CI drives the same seam with
//! [`MockSource`], so no test touches the network.
//!
//! # Lazy by default, eager on demand
//!
//! [`Provider`] owns the pure state. [`ServiceState::refresh_if_needed`] locks
//! only for the state transitions and does the network work with the lock
//! released, so a session launch warms the cache in the background while the
//! desktop already renders the shipped default and no D-Bus read blocks on the
//! fetch. `Preload` calls the same method from the Wallpapers pane.

pub mod cache;
pub mod dbus;
pub mod defaults;
pub mod model;
pub mod provider;
pub mod source;
pub mod wikipedia;

pub use cache::{CacheLayout, Index, WEEK_SECS};
pub use defaults::{install_default, resolve_default, DefaultResolver};
pub use model::{Catalogue, Category, Status, WallpaperItem};
pub use provider::{
    fetch, now_secs, Provider, RefreshReport, ServiceState, WarmDecision, WarmReason,
};
pub use source::{ContentSource, HttpClient, MockSource, ParsedItem, UreqHttp, WikipediaSource};
