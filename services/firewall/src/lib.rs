// SPDX-License-Identifier: MIT
//! The firewall adapter (T225).
//!
//! The Network pane and its Firewall sheet never shell out. They read one
//! [`FirewallAdapter`], which owns the host-manager detection and the two
//! explicit writes behind the [`FirewallSource`] seam:
//!
//! * **firewalld** over `firewall-cmd` (the Fedora family's D-Bus daemon; the
//!   CLI is the diagnostic/fallback transport used here),
//! * **ufw** over its CLI (Debian/Ubuntu),
//! * **nftables** over `nft`, detected but reported **Unavailable**: it has no
//!   service status to read and ADR 0229 forbids a first-party rule editor, so
//!   the pane never shows a fake Off toggle.
//!
//! A host with none of the three is [`FirewallData`]-absent: the pane shows
//! "No firewall detected" and offers no toggle. A manager that cannot report
//! status is `status_known: false`, which the view maps to Unavailable — never
//! Off. Absence is a normal state and never blocks session startup.

mod adapter;
mod command;
mod model;
mod source;

pub use adapter::{FirewallAccess, FirewallAdapter, FirewallResult};
pub use command::{
    detect_backend, parse_firewall_cmd_ports, parse_firewall_cmd_services, parse_ufw_status,
    CommandFirewall,
};
pub use model::FirewallSnapshot;
pub use source::{FirewallData, FirewallSource, FirewallWriteOutcome, MockFirewall};
