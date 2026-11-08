// SPDX-License-Identifier: MIT
//! The live firewall transport: manager detection and the command wrappers.
//!
//! Reads never prompt: they call the manager's own status command as the
//! signed-in user. Writes go through `pkexec` when it is installed, so the
//! shell's polkit agent owns the prompt; a refusal is reported as a denial,
//! never as a fake success. nftables is detected but has no enable/disable
//! state and no first-party rule editor (ADR 0229), so its status is
//! `status_known: false` (Unavailable) and its writes are Unsupported.

use std::process::Command;

use dragonfruit_distro_adapters::{FirewallBackend, Port, Protocol};
use dragonfruit_system_adapters::AdapterError;

use crate::source::{FirewallData, FirewallSource, FirewallWriteOutcome};

/// The live command-backed firewall source.
#[derive(Debug, Default, Clone, Copy)]
pub struct CommandFirewall;

impl CommandFirewall {
    /// A source that probes the host on demand.
    pub const fn new() -> Self {
        CommandFirewall
    }

    fn backend(&self) -> FirewallBackend {
        detect_backend(
            command_exists("firewall-cmd"),
            command_exists("ufw"),
            command_exists("nft"),
        )
    }

    fn read_firewalld(&self) -> Result<Option<FirewallData>, AdapterError> {
        let state = run("firewall-cmd", &["--state"]);
        let (enabled, status_known, note) = match state {
            Ok(output) => {
                let trimmed = output.trim();
                if trimmed.contains("running") && !trimmed.contains("not running") {
                    (true, true, None)
                } else if trimmed.contains("not running") {
                    (false, true, None)
                } else {
                    (
                        false,
                        false,
                        Some("firewalld did not report a running state.".to_owned()),
                    )
                }
            }
            Err(error) => (
                false,
                false,
                Some(format!("firewalld status is unavailable: {error}")),
            ),
        };
        let mut data = FirewallData {
            backend: FirewallBackend::Firewalld,
            enabled,
            status_known,
            default_zone: None,
            open_ports: Vec::new(),
            open_services: Vec::new(),
            note,
        };
        if status_known && enabled {
            if let Ok(zone) = run("firewall-cmd", &["--get-default-zone"]) {
                let zone = zone.trim();
                if !zone.is_empty() {
                    data.default_zone = Some(zone.to_owned());
                }
            }
            if let Ok(ports) = run("firewall-cmd", &["--list-ports"]) {
                data.open_ports = parse_firewall_cmd_ports(&ports);
            }
            if let Ok(services) = run("firewall-cmd", &["--list-services"]) {
                data.open_services = parse_firewall_cmd_services(&services);
            }
        }
        Ok(Some(data))
    }

    fn read_ufw(&self) -> Result<Option<FirewallData>, AdapterError> {
        let output = match run("ufw", &["status", "verbose"]) {
            Ok(output) => output,
            Err(first) => match run("sudo", &["-n", "ufw", "status", "verbose"]) {
                Ok(output) => output,
                Err(_) => {
                    return Ok(Some(FirewallData {
                        backend: FirewallBackend::Ufw,
                        enabled: false,
                        status_known: false,
                        default_zone: None,
                        open_ports: Vec::new(),
                        open_services: Vec::new(),
                        note: Some(format!(
                            "ufw status requires administrator rights: {first}"
                        )),
                    }))
                }
            },
        };
        let (enabled, ports, services, note) = parse_ufw_status(&output);
        Ok(Some(FirewallData {
            backend: FirewallBackend::Ufw,
            enabled,
            status_known: note.is_none(),
            default_zone: None,
            open_ports: ports,
            open_services: services,
            note,
        }))
    }

    fn read_nftables(&self) -> Result<Option<FirewallData>, AdapterError> {
        match run("nft", &["list", "ruleset"]) {
            Ok(_) => Ok(Some(FirewallData {
                backend: FirewallBackend::Nftables,
                enabled: false,
                status_known: false,
                default_zone: None,
                open_ports: Vec::new(),
                open_services: Vec::new(),
                note: Some(
                    "nftables has no service status; enable/disable is managed by the host."
                        .to_owned(),
                ),
            })),
            Err(error) => Err(AdapterError::new(format!("nftables: {error}"))),
        }
    }

    fn write_firewalld(&self, args: &[&str]) -> FirewallWriteOutcome {
        match privileged_run("firewall-cmd", args) {
            Ok(_) => FirewallWriteOutcome::Applied,
            Err(error) => classify_write_error(error),
        }
    }

    fn write_ufw(&self, args: &[&str]) -> FirewallWriteOutcome {
        match privileged_run("ufw", args) {
            Ok(_) => FirewallWriteOutcome::Applied,
            Err(error) => classify_write_error(error),
        }
    }
}

impl FirewallSource for CommandFirewall {
    fn read(&mut self) -> Result<Option<FirewallData>, AdapterError> {
        match self.backend() {
            FirewallBackend::Firewalld => self.read_firewalld(),
            FirewallBackend::Ufw => self.read_ufw(),
            FirewallBackend::Nftables => self.read_nftables(),
            FirewallBackend::None | FirewallBackend::Iptables => Ok(None),
        }
    }

    fn set_enabled(&mut self, enabled: bool) -> FirewallWriteOutcome {
        match self.backend() {
            FirewallBackend::Firewalld => {
                let action = if enabled { "start" } else { "stop" };
                match privileged_run("systemctl", &[action, "firewalld"]) {
                    Ok(_) => FirewallWriteOutcome::Applied,
                    Err(error) => classify_write_error(error),
                }
            }
            FirewallBackend::Ufw => {
                let args: &[&str] = if enabled {
                    &["--force", "enable"]
                } else {
                    &["--force", "disable"]
                };
                self.write_ufw(args)
            }
            FirewallBackend::Nftables => FirewallWriteOutcome::Unsupported,
            FirewallBackend::None | FirewallBackend::Iptables => FirewallWriteOutcome::Absent,
        }
    }

    fn open_port(&mut self, port: Port) -> FirewallWriteOutcome {
        match self.backend() {
            FirewallBackend::Firewalld => {
                self.write_firewalld(&[&format!("--add-port={}", port.id())])
            }
            FirewallBackend::Ufw => self.write_ufw(&["allow", &port.id()]),
            FirewallBackend::Nftables => FirewallWriteOutcome::Unsupported,
            FirewallBackend::None | FirewallBackend::Iptables => FirewallWriteOutcome::Absent,
        }
    }

    fn close_port(&mut self, port: Port) -> FirewallWriteOutcome {
        match self.backend() {
            FirewallBackend::Firewalld => {
                self.write_firewalld(&[&format!("--remove-port={}", port.id())])
            }
            FirewallBackend::Ufw => self.write_ufw(&["delete", "allow", &port.id()]),
            FirewallBackend::Nftables => FirewallWriteOutcome::Unsupported,
            FirewallBackend::None | FirewallBackend::Iptables => FirewallWriteOutcome::Absent,
        }
    }

    fn open_service(&mut self, service: &str) -> FirewallWriteOutcome {
        match self.backend() {
            FirewallBackend::Firewalld => {
                self.write_firewalld(&[&format!("--add-service={service}")])
            }
            FirewallBackend::Ufw => self.write_ufw(&["allow", service]),
            FirewallBackend::Nftables => FirewallWriteOutcome::Unsupported,
            FirewallBackend::None | FirewallBackend::Iptables => FirewallWriteOutcome::Absent,
        }
    }

    fn close_service(&mut self, service: &str) -> FirewallWriteOutcome {
        match self.backend() {
            FirewallBackend::Firewalld => {
                self.write_firewalld(&[&format!("--remove-service={service}")])
            }
            FirewallBackend::Ufw => self.write_ufw(&["delete", "allow", service]),
            FirewallBackend::Nftables => FirewallWriteOutcome::Unsupported,
            FirewallBackend::None | FirewallBackend::Iptables => FirewallWriteOutcome::Absent,
        }
    }
}

/// The manager precedence: firewalld, then ufw, then nftables, then none.
pub fn detect_backend(has_firewalld: bool, has_ufw: bool, has_nft: bool) -> FirewallBackend {
    if has_firewalld {
        FirewallBackend::Firewalld
    } else if has_ufw {
        FirewallBackend::Ufw
    } else if has_nft {
        FirewallBackend::Nftables
    } else {
        FirewallBackend::None
    }
}

/// Parse `firewall-cmd --list-ports` (`22/tcp 53/udp`).
pub fn parse_firewall_cmd_ports(text: &str) -> Vec<Port> {
    let mut ports = Vec::new();
    for token in text.split_whitespace() {
        let Some((port, protocol)) = token.split_once('/') else {
            continue;
        };
        let Ok(port) = port.parse::<u16>() else {
            continue;
        };
        let Some(protocol) = protocol_from_id(protocol) else {
            continue;
        };
        let rule = Port::new(port, protocol);
        if !ports.contains(&rule) {
            ports.push(rule);
        }
    }
    ports.sort_by_key(|port| (port.port, port.protocol.id()));
    ports
}

/// Parse `firewall-cmd --list-services` (`ssh https dhcpv6-client`).
pub fn parse_firewall_cmd_services(text: &str) -> Vec<String> {
    let mut services: Vec<String> = text
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect();
    services.sort();
    services.dedup();
    services
}

/// Parse `ufw status verbose`.
///
/// Returns `(enabled, ports, services, note)`. A status the command did not
/// report (permission denied, unknown output) is `note: Some(...)` and must be
/// rendered Unavailable, never Off.
pub fn parse_ufw_status(text: &str) -> (bool, Vec<Port>, Vec<String>, Option<String>) {
    let mut enabled = false;
    let mut known = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(state) = trimmed.strip_prefix("Status:") {
            let state = state.trim().to_ascii_lowercase();
            if state.starts_with("active") {
                enabled = true;
                known = true;
            } else if state.starts_with("inactive") {
                enabled = false;
                known = true;
            }
        }
    }
    if !known {
        let note = if text.trim().is_empty() {
            "ufw did not report its status.".to_owned()
        } else {
            format!("ufw status is unavailable: {}", text.trim())
        };
        return (false, Vec::new(), Vec::new(), Some(note));
    }

    let mut ports = Vec::new();
    let mut services = Vec::new();
    let mut in_table = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("--") {
            in_table = true;
            continue;
        }
        if !in_table || trimmed.is_empty() {
            continue;
        }
        let mut columns = trimmed.split_whitespace();
        let Some(to) = columns.next() else {
            continue;
        };
        let Some(action) = columns.next() else {
            continue;
        };
        if !action.to_ascii_uppercase().starts_with("ALLOW") {
            continue;
        }
        // Strip the IPv6 annotation (`22/tcp (v6)`), which names the same rule.
        let to = to.strip_suffix("(v6)").unwrap_or(to);
        if to == "Anywhere" || to == "Anywhere (v6)" {
            continue;
        }
        if let Some((port_list, protocol)) = to.split_once('/') {
            let Some(protocol) = protocol_from_id(protocol) else {
                continue;
            };
            for port in port_list.split(',') {
                let Ok(port) = port.parse::<u16>() else {
                    continue;
                };
                let rule = Port::new(port, protocol);
                if !ports.contains(&rule) {
                    ports.push(rule);
                }
            }
        } else {
            let service = to.to_owned();
            if !services.contains(&service) {
                services.push(service);
            }
        }
    }
    ports.sort_by_key(|port| (port.port, port.protocol.id()));
    services.sort();
    (enabled, ports, services, None)
}

fn protocol_from_id(id: &str) -> Option<Protocol> {
    match id {
        "tcp" => Some(Protocol::Tcp),
        "udp" => Some(Protocol::Udp),
        _ => None,
    }
}

fn command_exists(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|directory| {
        let candidate = directory.join(name);
        candidate.is_file()
    })
}

fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("{program}: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(if stderr.is_empty() {
            format!("{program} exited with {}", output.status)
        } else {
            stderr
        })
    }
}

fn privileged_run(program: &str, args: &[&str]) -> Result<String, String> {
    if command_exists("pkexec") {
        let mut all = Vec::with_capacity(args.len() + 1);
        all.push(program);
        all.extend_from_slice(args);
        run("pkexec", &all)
    } else {
        run(program, args)
    }
}

fn classify_write_error(error: String) -> FirewallWriteOutcome {
    let lower = error.to_ascii_lowercase();
    if lower.contains("not authorized")
        || lower.contains("permission denied")
        || lower.contains("access denied")
        || lower.contains("authentication")
        || lower.contains("dismissed")
    {
        FirewallWriteOutcome::Denied(error)
    } else {
        FirewallWriteOutcome::Failed(AdapterError::new(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_prefers_firewalld_then_ufw_then_nftables() {
        assert_eq!(
            detect_backend(true, true, true),
            FirewallBackend::Firewalld
        );
        assert_eq!(detect_backend(false, true, true), FirewallBackend::Ufw);
        assert_eq!(detect_backend(false, false, true), FirewallBackend::Nftables);
        assert_eq!(detect_backend(false, false, false), FirewallBackend::None);
    }

    #[test]
    fn firewall_cmd_ports_parse_and_dedup() {
        let ports = parse_firewall_cmd_ports("22/tcp 53/udp 443/tcp 22/tcp 8080/sctp");
        assert_eq!(
            ports,
            vec![
                Port::new(22, Protocol::Tcp),
                Port::new(53, Protocol::Udp),
                Port::new(443, Protocol::Tcp),
            ]
        );
    }

    #[test]
    fn firewall_cmd_services_parse_and_sort() {
        assert_eq!(
            parse_firewall_cmd_services("ssh https dhcpv6-client ssh"),
            vec!["dhcpv6-client", "https", "ssh"]
        );
    }

    #[test]
    fn ufw_status_parses_ports_services_and_state() {
        let text = "\
Status: active
Logging: on (low)
Default: deny (incoming), allow (outgoing), disabled (routed)
New profiles: skip

To                         Action      From
--                         ------      ----
22/tcp                     ALLOW IN    Anywhere
80,443/tcp                 ALLOW IN    Anywhere
53/udp                     ALLOW IN    Anywhere
Samba                      ALLOW IN    Anywhere
22/tcp (v6)                ALLOW IN    Anywhere (v6)
";
        let (enabled, ports, services, note) = parse_ufw_status(text);
        assert!(enabled);
        assert_eq!(note, None);
        assert_eq!(
            ports,
            vec![
                Port::new(22, Protocol::Tcp),
                Port::new(53, Protocol::Udp),
                Port::new(80, Protocol::Tcp),
                Port::new(443, Protocol::Tcp),
            ]
        );
        assert_eq!(services, vec!["Samba"]);
    }

    #[test]
    fn ufw_inactive_is_known_and_off() {
        let (enabled, ports, services, note) =
            parse_ufw_status("Status: inactive\n\n");
        assert!(!enabled);
        assert!(note.is_none());
        assert!(ports.is_empty());
        assert!(services.is_empty());
    }

    #[test]
    fn ufw_permission_denied_is_unavailable_not_off() {
        let (enabled, _, _, note) =
            parse_ufw_status("ERROR: You need to be root to run this script\n");
        assert!(!enabled);
        assert!(note.is_some());
        assert!(note.unwrap().contains("You need to be root"));
    }
}
