// SPDX-License-Identifier: MIT
//! The General, About, and Updates adapter (T-15.10a).
//!
//! General, About, and Updates do not wrap an external desktop daemon the way
//! Bluetooth or UPower do: they project the **host stack**. That stack has two
//! halves, both reused, never reimplemented:
//!
//! * the host **identity** — the distribution release, kernel, architecture,
//!   DMI model/serial, processor, memory, and computer name the About and
//!   General rows draw ([`HostSystem::read_identity`]); and
//! * the distribution **update provider** — the check/install/reboot state
//!   behind the [`UpdateProvider`] seam, the `SystemProvider` of
//!   [08-settings.md] narrowed to updates. The concrete provider is
//!   distro-specific and belongs with packaging ([12-packaging.md]); this
//!   crate ships the seam and the mock.
//!
//! # The read path
//!
//! 1. A [`SystemSource`] returns the raw [`SystemData`] once per
//!    [`UpdateAdapter::refresh`] (or absence/error).
//! 2. [`SystemSnapshot::from_data`] carries the identity, types the update
//!    phase and list, and derives the labels and glyph the pane and tile draw.
//! 3. The adapter drives the shared subscription lifecycle, so a host stack
//!    that comes and goes re-subscribes and re-syncs with no user-visible
//!    error.
//!
//! # Absence is layered and normal
//!
//! The whole adapter is `Unavailable` only when neither the identity nor the
//! provider is reachable. A host that answers but runs **no update provider**
//! is `Available` with `updates: None`: only the update controls disable, and
//! the About/General rows stay live. That mirrors the battery adapter's two
//! per-daemon hide rules ([adr/0128]). A present provider that cannot be read
//! is `Error`, visible and inert with the message. Nothing blocks session
//! startup.
//!
//! # Explicit writes
//!
//! The pane's check, install, and reboot are explicit user actions over the
//! same seam ([`UpdateAdapter::check`]/[`install`](UpdateAdapter::install)/
//! [`reboot`](UpdateAdapter::reboot)). They invent no snapshot: the provider
//! publishes the resulting state and the host re-reads, so the snapshot stays
//! the single source of truth. Durable presentation preferences are
//! `settingsd`'s and land with T-15.10b.
//!
//! # Testing
//!
//! CI has no package manager and no provider, so the adapter is driven by
//! [`MockSystem`] over the source seam. `kill`/`restart` exercise absence and
//! re-subscribe; `push` drives the identity and the update state. [`HostSystem`]
//! reads a fixture root for the identity half, and [`MockUpdateProvider`]
//! proves the provider seam.
//!
//! [08-settings.md]: ../../../docs/design/08-settings.md
//! [12-packaging.md]: ../../../docs/design/12-packaging.md
//! [adr/0128]: ../../../docs/design/adr/0128-battery-power-profiles-adapter.md

mod adapter;
mod host;
mod model;
mod source;

pub use adapter::UpdateAdapter;
pub use host::{HostSystem, MockUpdateProvider, UpdateProvider};
pub use model::{SystemSnapshot, UpdateChange};
pub use source::{
    MockSystem, SystemData, SystemIdentity, SystemSource, UpdateData, UpdateItem, UpdateOutcome,
    UpdatePhase, UpdateSeverity,
};

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_system_adapters::Adapter;

    #[test]
    fn the_adapter_reports_the_updates_slot() {
        let adapter = UpdateAdapter::new(MockSystem::absent());
        assert_eq!(
            <UpdateAdapter<MockSystem> as Adapter>::id(&adapter),
            dragonfruit_system_adapters::AdapterId::UPDATES
        );
    }

    #[test]
    fn absence_is_a_normal_state() {
        let mut adapter = UpdateAdapter::new(MockSystem::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn the_phase_ids_are_stable() {
        assert_eq!(UpdatePhase::UpToDate.id(), "up-to-date");
        assert_eq!(UpdatePhase::RebootRequired.id(), "reboot-required");
        assert_eq!(UpdateSeverity::Security.id(), "security");
    }
}
