// SPDX-License-Identifier: GPL-3.0-or-later
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
            command_exists("iptables"),
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
            Err(error) if error.to_ascii_lowercase().contains("not running") => {
                // firewalld is installed but stopped: that is an honest Off,
                // not an unavailable status. The pane can start it.
                (false, true, None)
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
            rules_present: false,
            note,
        };
        if status_known {
            if let Ok(zone) = run("firewall-cmd", &["--get-default-zone"]) {
                let zone = zone.trim();
                if !zone.is_empty() {
                    data.default_zone = Some(zone.to_owned());
                }
            }
            if let Ok(ports) = run("firewall-cmd", &["--list-ports"]) {
                data.open_ports = parse_firewall_cmd_ports(&ports);
                data.rules_present = true;
            }
            if let Ok(services) = run("firewall-cmd", &["--list-services"]) {
                data.open_services = parse_firewall_cmd_services(&services);
                data.rules_present = true;
            }
        }
        Ok(Some(data))
    }

    fn read_ufw(&self) -> Result<Option<FirewallData>, AdapterError> {
        // The manager's own status first: it is authoritative and includes the
        // rule table. `sudo -n` never prompts; it only succeeds on a host that
        // already allows passwordless reads.
        let output = match run("ufw", &["status", "verbose"]) {
            Ok(output) => Some(output),
            Err(_) => run("sudo", &["-n", "ufw", "status", "verbose"]).ok(),
        };
        if let Some(output) = output {
            let (enabled, ports, services, note) = parse_ufw_status(&output);
            // A command that answered but did not report a state (the common
            // "you need to be root" text) is not authoritative; fall through
            // to the unprivileged state read below.
            if note.is_none() {
                return Ok(Some(FirewallData {
                    backend: FirewallBackend::Ufw,
                    enabled,
                    status_known: true,
                    default_zone: None,
                    open_ports: ports,
                    open_services: services,
                    rules_present: true,
                    note: None,
                }));
            }
        }

        // A normal user on a default Ubuntu install cannot run `ufw status`,
        // but the on/off state is still readable: `systemctl show` and
        // `/etc/ufw/ufw.conf` are unprivileged. Report that state and say
        // exactly which command owns the rules.
        let service_state = run(
            "systemctl",
            &["show", "-p", "ActiveState", "--value", "ufw"],
        )
        .ok();
        let conf = std::fs::read_to_string("/etc/ufw/ufw.conf").ok();
        let (enabled, status_known, note) =
            ufw_unprivileged_state(service_state.as_deref(), conf.as_deref());
        Ok(Some(FirewallData {
            backend: FirewallBackend::Ufw,
            enabled,
            status_known,
            default_zone: None,
            open_ports: Vec::new(),
            open_services: Vec::new(),
            rules_present: false,
            note,
        }))
    }

    fn read_nftables(&self) -> Result<Option<FirewallData>, AdapterError> {
        // `nft list ruleset` needs CAP_NET_ADMIN on most hosts. When it works
        // the live rules are visible; when it does not, a readable
        // `/etc/nftables.conf` still proves rules are configured. Either way
        // nftables has no service toggle.
        let live = run("nft", &["list", "ruleset"]).ok();
        let config = std::fs::read_to_string("/etc/nftables.conf").ok();
        let rules_present = live.as_deref().map(has_nft_rules).unwrap_or(false)
            || config.as_deref().map(has_nft_rules).unwrap_or(false);
        let note = if rules_present {
            "nftables rules are present; manage them with `sudo nft list ruleset`."
        } else {
            "nftables is present but reports no rules; manage it with `sudo nft list ruleset`."
        };
        Ok(Some(FirewallData {
            backend: FirewallBackend::Nftables,
            enabled: false,
            status_known: false,
            default_zone: None,
            open_ports: Vec::new(),
            open_services: Vec::new(),
            rules_present,
            note: Some(note.to_owned()),
        }))
    }

    fn read_iptables(&self) -> Result<Option<FirewallData>, AdapterError> {
        // iptables has no supported manager. Read a world-readable rules file
        // where the host ships one; otherwise say the rules cannot be read
        // unprivileged. Never report "No firewall detected".
        let rules_present = ["/etc/iptables/iptables.rules", "/etc/sysconfig/iptables"]
            .iter()
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .any(|text| has_iptables_rules(&text));
        let note = if rules_present {
            "iptables rules present; no supported manager. Inspect with `sudo iptables -S`."
        } else {
            "iptables is present; no supported manager and the rules cannot be read \
             without administrator rights. Inspect with `sudo iptables -S`."
        };
        Ok(Some(FirewallData {
            backend: FirewallBackend::Iptables,
            enabled: false,
            status_known: false,
            default_zone: None,
            open_ports: Vec::new(),
            open_services: Vec::new(),
            rules_present,
            note: Some(note.to_owned()),
        }))
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
            FirewallBackend::Iptables => self.read_iptables(),
            FirewallBackend::None => Ok(None),
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
            FirewallBackend::Nftables | FirewallBackend::Iptables => {
                FirewallWriteOutcome::Unsupported
            }
            FirewallBackend::None => FirewallWriteOutcome::Absent,
        }
    }

    fn open_port(&mut self, port: Port) -> FirewallWriteOutcome {
        match self.backend() {
            FirewallBackend::Firewalld => {
                self.write_firewalld(&[&format!("--add-port={}", port.id())])
            }
            FirewallBackend::Ufw => self.write_ufw(&["allow", &port.id()]),
            FirewallBackend::Nftables | FirewallBackend::Iptables => {
                FirewallWriteOutcome::Unsupported
            }
            FirewallBackend::None => FirewallWriteOutcome::Absent,
        }
    }

    fn close_port(&mut self, port: Port) -> FirewallWriteOutcome {
        match self.backend() {
            FirewallBackend::Firewalld => {
                self.write_firewalld(&[&format!("--remove-port={}", port.id())])
            }
            FirewallBackend::Ufw => self.write_ufw(&["delete", "allow", &port.id()]),
            FirewallBackend::Nftables | FirewallBackend::Iptables => {
                FirewallWriteOutcome::Unsupported
            }
            FirewallBackend::None => FirewallWriteOutcome::Absent,
        }
    }

    fn open_service(&mut self, service: &str) -> FirewallWriteOutcome {
        match self.backend() {
            FirewallBackend::Firewalld => {
                self.write_firewalld(&[&format!("--add-service={service}")])
            }
            FirewallBackend::Ufw => self.write_ufw(&["allow", service]),
            FirewallBackend::Nftables | FirewallBackend::Iptables => {
                FirewallWriteOutcome::Unsupported
            }
            FirewallBackend::None => FirewallWriteOutcome::Absent,
        }
    }

    fn close_service(&mut self, service: &str) -> FirewallWriteOutcome {
        match self.backend() {
            FirewallBackend::Firewalld => {
                self.write_firewalld(&[&format!("--remove-service={service}")])
            }
            FirewallBackend::Ufw => self.write_ufw(&["delete", "allow", service]),
            FirewallBackend::Nftables | FirewallBackend::Iptables => {
                FirewallWriteOutcome::Unsupported
            }
            FirewallBackend::None => FirewallWriteOutcome::Absent,
        }
    }
}

/// The manager precedence: firewalld, then ufw, then nftables, then iptables,
/// then none. iptables is named but unsupported: it is read-only/delegate.
pub fn detect_backend(
    has_firewalld: bool,
    has_ufw: bool,
    has_nft: bool,
    has_iptables: bool,
) -> FirewallBackend {
    if has_firewalld {
        FirewallBackend::Firewalld
    } else if has_ufw {
        FirewallBackend::Ufw
    } else if has_nft {
        FirewallBackend::Nftables
    } else if has_iptables {
        FirewallBackend::Iptables
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

/// The unprivileged ufw state: `(enabled, status_known, note)`.
///
/// `service_state` is `systemctl show -p ActiveState --value ufw`; `conf` is
/// `/etc/ufw/ufw.conf`. The on/off state is readable from either, but the rule
/// table is not, so the note names the exact command that owns it.
pub fn ufw_unprivileged_state(
    service_state: Option<&str>,
    conf: Option<&str>,
) -> (bool, bool, Option<String>) {
    let conf_enabled = conf.and_then(parse_ufw_conf);
    let service_active = service_state
        .map(|state| state.trim().to_ascii_lowercase())
        .and_then(|state| match state.as_str() {
            "active" => Some(true),
            "inactive" | "failed" => Some(false),
            _ => None,
        });
    if conf_enabled.is_none() && service_active.is_none() {
        return (
            false,
            false,
            Some(
                "ufw status is not readable without administrator rights; \
                 manage it with `sudo ufw status verbose`."
                    .to_owned(),
            ),
        );
    }
    let enabled = match (conf_enabled, service_active) {
        (Some(conf), Some(active)) => conf && active,
        (Some(conf), None) => conf,
        (None, Some(active)) => active,
        (None, None) => false,
    };
    (
        enabled,
        true,
        Some(
            "ufw rules need administrator rights; manage them with \
             `sudo ufw status verbose`."
                .to_owned(),
        ),
    )
}

/// Parse `ENABLED=yes|no` out of `/etc/ufw/ufw.conf`.
pub fn parse_ufw_conf(text: &str) -> Option<bool> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if !key.trim().eq_ignore_ascii_case("ENABLED") {
            continue;
        }
        return match value.trim().to_ascii_lowercase().as_str() {
            "yes" | "true" | "1" => Some(true),
            "no" | "false" | "0" => Some(false),
            _ => None,
        };
    }
    None
}

/// Whether an nftables ruleset/config text contains anything beyond the
/// `flush ruleset` / table / chain scaffolding.
pub fn has_nft_rules(text: &str) -> bool {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower == "flush ruleset" || lower == "{" || lower == "}" {
            continue;
        }
        if lower.starts_with("table ")
            || lower.starts_with("chain ")
            || lower.starts_with("include ")
        {
            continue;
        }
        return true;
    }
    false
}

/// Whether an iptables rules file contains at least one append/insert rule.
pub fn has_iptables_rules(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        line.starts_with("-A ")
            || line.starts_with("-I ")
            || line.starts_with("-N ")
            || line.starts_with("-P ")
    })
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
    fn detection_prefers_firewalld_then_ufw_then_nftables_then_iptables() {
        assert_eq!(
            detect_backend(true, true, true, true),
            FirewallBackend::Firewalld
        );
        assert_eq!(
            detect_backend(false, true, true, true),
            FirewallBackend::Ufw
        );
        assert_eq!(
            detect_backend(false, false, true, true),
            FirewallBackend::Nftables
        );
        assert_eq!(
            detect_backend(false, false, false, true),
            FirewallBackend::Iptables
        );
        assert_eq!(
            detect_backend(false, false, false, false),
            FirewallBackend::None
        );
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
        let (enabled, ports, services, note) = parse_ufw_status("Status: inactive\n\n");
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

    #[test]
    fn ufw_conf_parses_the_enabled_flag() {
        assert_eq!(parse_ufw_conf("ENABLED=yes\n"), Some(true));
        assert_eq!(parse_ufw_conf("ENABLED=no\n"), Some(false));
        assert_eq!(parse_ufw_conf("# ENABLED=yes\n"), None);
        assert_eq!(parse_ufw_conf("LOGLEVEL=low\n"), None);
    }

    #[test]
    fn a_normal_user_ufw_read_gets_the_state_and_names_the_rules_command() {
        let (enabled, known, note) =
            ufw_unprivileged_state(Some("active\n"), Some("ENABLED=yes\n"));
        assert!(enabled);
        assert!(known);
        assert!(note.unwrap().contains("sudo ufw status verbose"));

        let (enabled, known, _) = ufw_unprivileged_state(Some("inactive\n"), Some("ENABLED=yes\n"));
        assert!(!enabled);
        assert!(known);

        let (enabled, known, note) = ufw_unprivileged_state(None, None);
        assert!(!enabled);
        assert!(!known);
        assert!(note.unwrap().contains("not readable"));
    }

    #[test]
    fn an_ufw_config_alone_still_reports_a_known_off_state() {
        let (enabled, known, note) = ufw_unprivileged_state(None, Some("ENABLED=no\n"));
        assert!(!enabled);
        assert!(known);
        assert!(note.is_some());
    }

    #[test]
    fn nftables_rule_detection_ignores_scaffolding() {
        assert!(!has_nft_rules("#!/usr/sbin/nft -f\nflush ruleset\n"));
        assert!(!has_nft_rules(
            "table inet filter {\n  chain input {\n  }\n}\n"
        ));
        assert!(has_nft_rules(
            "table inet filter {\n  chain input {\n    tcp dport 22 accept\n  }\n}\n"
        ));
        assert!(has_nft_rules(
            "add rule inet filter input tcp dport 22 accept\n"
        ));
    }

    #[test]
    fn iptables_rule_detection_counts_append_and_policy_rules() {
        assert!(!has_iptables_rules(
            "*filter\n:INPUT ACCEPT [0:0]\nCOMMIT\n"
        ));
        assert!(has_iptables_rules(
            "*filter\n:INPUT ACCEPT [0:0]\n-A INPUT -p tcp --dport 22 -j ACCEPT\nCOMMIT\n"
        ));
        assert!(has_iptables_rules("-P INPUT DROP\n"));
    }
}
