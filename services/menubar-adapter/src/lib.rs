// SPDX-License-Identifier: MIT
//! The Menu Bar configuration adapter (T-15.9a).
//!
//! Menu Bar configuration is **shell-native**. There is no external daemon to
//! wrap: the shell's `MenuBar` (`shell/menubar/MenuBar.qml`) owns the chrome,
//! the status-item model, and the clock; the menu-broker
//! (`services/menu-broker`) resolves the focused app's global menu; and
//! `settingsd` owns the durable preferences both read. This crate is the
//! adapter over that host stack. It never re-renders the bar or re-resolves a
//! menu — it faces the state and configuration those components already
//! publish:
//!
//! * the effective **auto-hide** mode and whether the bar is hidden right now,
//! * the **background** and **global application-menu** flags,
//! * the **clock** display options, and
//! * the live **availability of each control** (Wi-Fi, Bluetooth, volume,
//!   battery, Focus, accessibility) as the shell's status row shows it,
//!
//! behind a [`MenuBarSource`] seam, with a pure [`MenuBarChange`] diff as the
//! event stream.
//!
//! # The read path
//!
//! 1. A [`MenuBarSource`] returns the raw [`MenuBarData`] once per
//!    [`MenuBarAdapter::refresh`] (or absence/error).
//! 2. [`MenuBarSnapshot::from_data`] types the auto-hide mode, carries the
//!    clock options, folds the per-control availability into stable slots, and
//!    names the flags the pane and tile draw.
//! 3. The adapter drives the shared subscription lifecycle, so a host stack
//!    that comes and goes re-subscribes and re-syncs with no user-visible
//!    error.
//!
//! # Read-only runtime, settingsd-owned preferences
//!
//! The durable preferences are `settingsd`'s (the `menu.*` and clock keys
//! T-15.9b declares) and the shell renders the bar. A consumer that wants to
//! change a mode or a toggle writes the settings key; the shell applies it
//! live; the bridge publishes the new state; the adapter reports it. There is
//! no write method here, exactly as `dragonfruit-overview` and
//! `dragonfruit-lock-adapter` have none.
//!
//! # Absence
//!
//! A missing menu-bar bridge is the adapter's `Unavailable` state and hides
//! the item. This is a normal state — the menu bar still renders from the
//! shell's built-in defaults and no session startup is blocked. A present
//! bridge that cannot be read is `Error`: visible and inert with the message.
//! A single absent *control* is not an adapter error: it is an absent
//! [`MenuBarControlState`] slot inside an `Available` snapshot, so one daemon
//! (say Bluetooth) going away hides only its slot, not the adapter.
//!
//! # Testing
//!
//! CI has no shell, so the adapter is driven by [`MockMenuBar`] over the source
//! seam. `kill`/`restart` exercise absence and re-subscribe; `push` drives the
//! configuration and the control availability.
//!
//! # Not modelled yet
//!
//! The richer macOS `Clock Options...` rows (day of week, 24-hour, flashing
//! separators) and the Apple-only controls (AirDrop, Screen Mirroring) are
//! deliberately omitted: the shell clock renders only the date and seconds
//! today, and the others have no Linux equivalent. They are recorded as
//! follow-ups rather than shipped as dead controls.

mod adapter;
mod model;
mod source;

pub use adapter::MenuBarAdapter;
pub use model::{
    ClockOption, ClockOptions, MenuBarAutoHide, MenuBarChange, MenuBarControl, MenuBarSnapshot,
};
pub use source::{MenuBarControlState, MenuBarData, MenuBarSource, MockMenuBar};

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_system_adapters::Adapter;

    #[test]
    fn the_adapter_reports_the_menu_bar_slot() {
        let adapter = MenuBarAdapter::new(MockMenuBar::absent());
        assert_eq!(
            <MenuBarAdapter<MockMenuBar> as Adapter>::id(&adapter),
            dragonfruit_system_adapters::AdapterId::MENU_BAR
        );
    }

    #[test]
    fn absence_is_a_normal_state() {
        let mut adapter = MenuBarAdapter::new(MockMenuBar::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn the_control_ids_are_the_shell_status_item_ids() {
        assert_eq!(MenuBarControl::Wifi.id(), "wifi");
        assert_eq!(MenuBarControl::Sound.id(), "volume");
        assert_eq!(MenuBarControl::Focus.id(), "focus");
        assert_eq!(MenuBarControl::Accessibility.id(), "accessibility");
        assert_eq!(MenuBarControl::Clock.id(), "clock");
    }
}
