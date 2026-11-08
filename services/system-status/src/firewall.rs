// SPDX-License-Identifier: MIT
//! The firewall half of the bridge host (T225).
//!
//! The firewall adapter owns detection (firewalld/ufw/nftables) and the
//! explicit writes. This module projects its typed snapshot to the flat JSON
//! the Network pane and its sheet draw, and maps each write to a report. A
//! manager that cannot report status (nftables) is `state: "unavailable"` with
//! `detected: true` and `statusKnown: false`; the pane never shows a fake Off
//! toggle. No manager at all is `state: "unavailable"` with `detected: false`.

use dragonfruit_distro_adapters::{Port, Protocol};
use dragonfruit_firewall::{FirewallAdapter, FirewallResult, FirewallSource};
use dragonfruit_system_adapters::Adapter;
use serde_json::{json, Value};

const KIND_FIREWALL: &str = "firewall";

/// The bridge host for the firewall adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct FirewallHost<S> {
    adapter: FirewallAdapter<S>,
}

impl<S: FirewallSource> FirewallHost<S> {
    /// A host over a firewall source.
    pub fn new(source: S) -> Self {
        FirewallHost {
            adapter: FirewallAdapter::new(source),
        }
    }

    /// Re-read the manager once.
    pub fn refresh(&mut self) {
        self.adapter.refresh();
    }

    /// The firewall view the pane renders.
    pub fn view(&self) -> Value {
        firewall_view(&self.adapter)
    }

    /// The view as JSON (the D-Bus `State()` payload).
    pub fn state(&self) -> String {
        self.view().to_string()
    }

    /// Enable or disable the firewall.
    pub fn set_enabled(&mut self, enabled: bool) -> Value {
        firewall_report(&self.adapter.set_enabled(enabled))
    }

    /// Open a port by number and protocol id.
    pub fn open_port(&mut self, port: u16, protocol: &str) -> Value {
        match protocol_from_id(protocol) {
            Some(protocol) => firewall_report(&self.adapter.open_port(Port::new(port, protocol))),
            None => {
                json!({ "outcome": "failed", "error": format!("unknown protocol: {protocol}") })
            }
        }
    }

    /// Close a port by number and protocol id.
    pub fn close_port(&mut self, port: u16, protocol: &str) -> Value {
        match protocol_from_id(protocol) {
            Some(protocol) => firewall_report(&self.adapter.close_port(Port::new(port, protocol))),
            None => {
                json!({ "outcome": "failed", "error": format!("unknown protocol: {protocol}") })
            }
        }
    }

    /// Open a named service.
    pub fn open_service(&mut self, service: &str) -> Value {
        firewall_report(&self.adapter.open_service(service))
    }

    /// Close a named service.
    pub fn close_service(&mut self, service: &str) -> Value {
        firewall_report(&self.adapter.close_service(service))
    }

    /// The adapter, for tests.
    pub fn adapter(&self) -> &FirewallAdapter<S> {
        &self.adapter
    }

    /// The adapter, mutably.
    pub fn adapter_mut(&mut self) -> &mut FirewallAdapter<S> {
        &mut self.adapter
    }
}

/// Build the firewall view from an adapter.
pub fn firewall_view<S: FirewallSource>(adapter: &FirewallAdapter<S>) -> Value {
    let read_only = adapter.is_read_only();
    let read_only_note = adapter.degradation_note().unwrap_or_default();
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({
            "kind": KIND_FIREWALL,
            "state": "unavailable",
            "detected": false,
            "backend": "none",
            "backendLabel": "No firewall detected",
            "statusKnown": false,
            "enabled": Value::Null,
            "portCount": 0,
            "serviceCount": 0,
            "ports": [],
            "services": [],
            "defaultZone": "",
            "note": "",
            "readOnly": read_only,
            "readOnlyNote": read_only_note,
        });
    }
    if let Some(error) = state.error() {
        // A manager that answered but could not be read stays visible and
        // inert: `detected` is true, the status is unknown (never a fake Off),
        // and the pane disables the toggle and the rule sheet.
        return json!({
            "kind": KIND_FIREWALL,
            "state": "error",
            "detected": true,
            "backend": "unknown",
            "backendLabel": "Firewall",
            "statusKnown": false,
            "enabled": Value::Null,
            "label": "Unavailable",
            "defaultZone": "",
            "portCount": 0,
            "serviceCount": 0,
            "ports": [],
            "services": [],
            "note": error.message(),
            "error": error.message(),
            "readOnly": read_only,
            "readOnlyNote": read_only_note,
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_FIREWALL, "state": "unavailable" });
    };
    let ports: Vec<Value> = snapshot
        .open_ports
        .iter()
        .map(|port| {
            json!({
                "port": port.port,
                "protocol": port.protocol.id(),
                "id": port.id(),
                "label": format!("{}/{}", port.port, port.protocol.id()),
            })
        })
        .collect();
    json!({
        "kind": KIND_FIREWALL,
        "state": if snapshot.status_known { "available" } else { "unavailable" },
        "detected": true,
        "backend": snapshot.backend.id(),
        "backendLabel": snapshot.backend.label(),
        "statusKnown": snapshot.status_known,
        "enabled": if snapshot.status_known { Value::Bool(snapshot.enabled) } else { Value::Null },
        "label": snapshot.label(),
        "defaultZone": snapshot.default_zone.clone().unwrap_or_default(),
        "portCount": snapshot.port_count(),
        "serviceCount": snapshot.service_count(),
        "ports": ports,
        "services": snapshot.open_services,
        "note": snapshot.note.clone().unwrap_or_default(),
        "readOnly": read_only,
        "readOnlyNote": read_only_note,
    })
}

/// Build the view from an already-decoded snapshot (pure half).
pub fn firewall_snapshot_view(snapshot: &dragonfruit_firewall::FirewallSnapshot) -> Value {
    let ports: Vec<Value> = snapshot
        .open_ports
        .iter()
        .map(|port| {
            json!({
                "port": port.port,
                "protocol": port.protocol.id(),
                "id": port.id(),
                "label": format!("{}/{}", port.port, port.protocol.id()),
            })
        })
        .collect();
    json!({
        "kind": KIND_FIREWALL,
        "state": if snapshot.status_known { "available" } else { "unavailable" },
        "detected": true,
        "backend": snapshot.backend.id(),
        "backendLabel": snapshot.backend.label(),
        "statusKnown": snapshot.status_known,
        "enabled": if snapshot.status_known { Value::Bool(snapshot.enabled) } else { Value::Null },
        "label": snapshot.label(),
        "defaultZone": snapshot.default_zone.clone().unwrap_or_default(),
        "portCount": snapshot.port_count(),
        "serviceCount": snapshot.service_count(),
        "ports": ports,
        "services": snapshot.open_services,
        "note": snapshot.note.clone().unwrap_or_default(),
        "readOnly": false,
        "readOnlyNote": "",
    })
}

/// The JSON report for a [`FirewallResult`].
pub fn firewall_report(result: &FirewallResult) -> Value {
    match result {
        FirewallResult::Applied => json!({ "outcome": "applied" }),
        FirewallResult::Denied { note } => json!({ "outcome": "denied", "note": note }),
        FirewallResult::Absent => json!({ "outcome": "absent" }),
        FirewallResult::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
        FirewallResult::Unsupported => json!({ "outcome": "unsupported" }),
    }
}

fn protocol_from_id(id: &str) -> Option<Protocol> {
    match id {
        "tcp" => Some(Protocol::Tcp),
        "udp" => Some(Protocol::Udp),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_firewall::MockFirewall;

    #[test]
    fn an_absent_manager_is_detected_false() {
        let mut host = FirewallHost::new(MockFirewall::absent());
        host.refresh();
        let view = host.view();
        assert_eq!(view["kind"], "firewall");
        assert_eq!(view["state"], "unavailable");
        assert_eq!(view["detected"], false);
        assert_eq!(view["backendLabel"], "No firewall detected");
        assert_eq!(view["enabled"], Value::Null);
    }

    #[test]
    fn an_nftables_manager_is_detected_but_unavailable() {
        let mut host = FirewallHost::new(MockFirewall::nftables_unavailable());
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "unavailable");
        assert_eq!(view["detected"], true);
        assert_eq!(view["backend"], "nftables");
        assert_eq!(view["statusKnown"], false);
        assert_eq!(view["enabled"], Value::Null);
        assert_eq!(host.set_enabled(true)["outcome"], "unsupported");
    }

    #[test]
    fn an_unreadable_manager_is_unavailable_not_off() {
        let mut host = FirewallHost::new(MockFirewall::failing("firewall-cmd: not authorized"));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "error");
        assert_eq!(view["detected"], true);
        assert_eq!(view["statusKnown"], false);
        assert_eq!(view["enabled"], Value::Null);
        assert_eq!(view["label"], "Unavailable");
        assert_eq!(view["note"], "firewall-cmd: not authorized");
    }

    #[test]
    fn the_view_carries_the_rules_and_state() {
        let mut host = FirewallHost::new(MockFirewall::present());
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["backend"], "firewalld");
        assert_eq!(view["enabled"], true);
        assert_eq!(view["label"], "On");
        assert_eq!(view["defaultZone"], "public");
        assert_eq!(view["portCount"], 2);
        assert_eq!(view["serviceCount"], 2);
        assert_eq!(view["ports"][0]["id"], "22/tcp");
        assert_eq!(view["services"][0], "ssh");
    }

    #[test]
    fn writes_apply_and_are_reported() {
        let mut host = FirewallHost::new(MockFirewall::present());
        host.refresh();
        assert_eq!(host.set_enabled(false)["outcome"], "applied");
        assert_eq!(host.open_port(8080, "tcp")["outcome"], "applied");
        assert_eq!(host.close_port(22, "tcp")["outcome"], "applied");
        assert_eq!(host.open_service("mdns")["outcome"], "applied");
        assert_eq!(host.close_service("https")["outcome"], "applied");
        assert_eq!(host.open_port(1, "sctp")["outcome"], "failed");
    }

    #[test]
    fn a_denied_write_degrades_to_read_only() {
        let mut host = FirewallHost::new(MockFirewall::present());
        host.refresh();
        host.adapter_mut()
            .source_mut()
            .deny_writes("not authorized");
        assert_eq!(host.set_enabled(true)["outcome"], "denied");
        let view = host.view();
        assert_eq!(view["readOnly"], true);
        assert_eq!(view["readOnlyNote"], "not authorized");
    }
}
