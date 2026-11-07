// SPDX-License-Identifier: MIT
//! Host detection and the live capability probe.
//!
//! [`DistroReport`] is the T221 live check: it reads `/etc/os-release`, maps
//! the host to a family, probes the matrix's commands, and prints both the
//! host capability state and the selected stub adapter's status. It never
//! executes a distro command; `command_exists` only checks the executable
//! bits on `PATH`, so the probe is safe in CI and on a read-only host.

use std::path::{Path, PathBuf};

use crate::capability::{
    Absence, BackendId, Capability, CapabilityInfo, CapabilityStatus, Unsupported,
};
use crate::distro::Distro;
use crate::matrix::{self, BackendSpec};
use crate::stub::StubBackend;

/// The host facts the probe needs. A fixture implements this to test the
/// probe without touching the real host.
pub trait HostProbe {
    /// The contents of `/etc/os-release`, when readable.
    fn os_release(&self) -> Option<String>;

    /// Whether an executable named `command` is on the search path.
    fn command_exists(&self, command: &str) -> bool;

    /// Whether an absolute path exists.
    fn path_exists(&self, path: &str) -> bool;
}

/// The live host probe.
#[derive(Debug, Clone)]
pub struct SystemProbe {
    root: PathBuf,
    search_path: Vec<PathBuf>,
}

impl Default for SystemProbe {
    fn default() -> Self {
        SystemProbe::new()
    }
}

impl SystemProbe {
    /// A probe reading the real host.
    pub fn new() -> Self {
        let search_path = std::env::var_os("PATH")
            .map(|path| std::env::split_paths(&path).collect())
            .unwrap_or_else(|| {
                [
                    "/usr/local/sbin",
                    "/usr/local/bin",
                    "/usr/sbin",
                    "/usr/bin",
                    "/sbin",
                    "/bin",
                ]
                .into_iter()
                .map(PathBuf::from)
                .collect()
            });
        SystemProbe {
            root: PathBuf::from("/"),
            search_path,
        }
    }

    /// A probe reading a fixture root. The search path still comes from the
    /// host; `command_exists` prefixes each entry with the root.
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        let mut probe = SystemProbe::new();
        probe.root = root.into();
        probe
    }

    /// A probe reading a fixture root and a fixture search path.
    pub fn with_root_and_path(root: impl Into<PathBuf>, search_path: Vec<PathBuf>) -> Self {
        SystemProbe {
            root: root.into(),
            search_path,
        }
    }

    /// The filesystem root this probe reads.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn rooted(&self, path: &Path) -> PathBuf {
        if self.root == Path::new("/") {
            return path.to_path_buf();
        }
        if path.is_absolute() {
            let relative = path.strip_prefix("/").unwrap_or(path);
            self.root.join(relative)
        } else {
            self.root.join(path)
        }
    }
}

impl HostProbe for SystemProbe {
    fn os_release(&self) -> Option<String> {
        std::fs::read_to_string(self.root.join("etc/os-release")).ok()
    }

    fn command_exists(&self, command: &str) -> bool {
        self.search_path
            .iter()
            .any(|directory| is_executable(&self.rooted(&directory.join(command))))
    }

    fn path_exists(&self, path: &str) -> bool {
        self.rooted(Path::new(path)).exists()
    }
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// One capability's detected host state and matrix row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedCapability {
    /// The capability.
    pub capability: Capability,
    /// Whether the host tools are present.
    pub status: CapabilityStatus,
    /// The matrix row, when the family is supported.
    pub spec: Option<&'static BackendSpec>,
}

/// The complete T221 probe report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistroReport {
    /// The detected host.
    pub distro: Distro,
    /// The selected backend.
    pub backend: BackendId,
    /// One entry per capability, in matrix order.
    pub capabilities: Vec<DetectedCapability>,
}

impl DistroReport {
    /// Probe the real host.
    pub fn detect() -> Self {
        DistroReport::detect_with(&SystemProbe::new())
    }

    /// Probe a host through `probe`.
    pub fn detect_with(probe: &dyn HostProbe) -> Self {
        let distro = probe
            .os_release()
            .map(|text| Distro::from_os_release(&text))
            .unwrap_or_else(Distro::unknown);
        let backend = BackendId::from(distro.family);
        let capabilities = Capability::ALL
            .into_iter()
            .map(|capability| {
                let spec = matrix::spec(capability, distro.family);
                let status = match spec {
                    None => CapabilityStatus::Unsupported(Unsupported {
                        capability,
                        backend,
                        command: "no backend",
                        reason: format!("unsupported distribution: {}", distro.display_name()),
                    }),
                    Some(spec) => detect_capability(probe, capability, backend, spec),
                };
                DetectedCapability {
                    capability,
                    status,
                    spec,
                }
            })
            .collect();
        DistroReport {
            distro,
            backend,
            capabilities,
        }
    }

    /// One capability's detection entry.
    pub fn capability(&self, capability: Capability) -> Option<&DetectedCapability> {
        self.capabilities
            .iter()
            .find(|entry| entry.capability == capability)
    }

    /// The host capabilities the probe found available.
    pub fn available(&self) -> Vec<Capability> {
        self.capabilities
            .iter()
            .filter(|entry| entry.status.is_available())
            .map(|entry| entry.capability)
            .collect()
    }

    /// The selected stub adapter's status for `capability`.
    pub fn adapter_status(&self, capability: Capability) -> CapabilityStatus {
        StubBackend::new(self.distro.clone()).unsupported_status(capability)
    }

    /// The live-check transcript.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("dragonfruit distro probe (T221)\n");
        out.push_str(&format!(
            "distro: {} ({}, {})\n",
            self.distro.display_name(),
            self.distro.family.id(),
            self.distro.tier().label()
        ));
        out.push_str(&format!(
            "backend: {} (stub; T254-T259 implement the concrete backend)\n",
            self.backend
        ));
        out.push_str(&format!(
            "host capabilities: {}/{}\n",
            self.available().len(),
            self.capabilities.len()
        ));
        out.push_str(&format!(
            "{:<16} {:<12} {:<14} {}\n",
            "capability", "host", "adapter", "command/API"
        ));
        for entry in &self.capabilities {
            let adapter = self.adapter_status(entry.capability);
            let api = entry
                .spec
                .map(|spec| spec.api)
                .unwrap_or("no backend for this distribution");
            out.push_str(&format!(
                "{:<16} {:<12} {:<14} {}\n",
                entry.capability.id(),
                entry.status.state().id(),
                adapter.state().id(),
                api
            ));
            if !entry.status.reason().is_empty() {
                out.push_str(&format!(
                    "{:<16} {:<12} {:<14} {}\n",
                    "",
                    "",
                    "",
                    entry.status.reason()
                ));
            }
        }
        out
    }
}

fn detect_capability(
    probe: &dyn HostProbe,
    capability: Capability,
    backend: BackendId,
    spec: &'static BackendSpec,
) -> CapabilityStatus {
    if spec.is_builtin() {
        return CapabilityStatus::Available(CapabilityInfo {
            capability,
            backend,
            command: spec.command,
            api: spec.api,
        });
    }
    if spec
        .probe
        .iter()
        .any(|command| probe.command_exists(command))
    {
        return CapabilityStatus::Available(CapabilityInfo {
            capability,
            backend,
            command: spec.command,
            api: spec.api,
        });
    }
    CapabilityStatus::Absent(Absence {
        capability,
        missing: spec.packages.to_vec(),
        reason: format!("none of {} is installed", spec.probe.join(", ")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distro::DistroFamily;

    struct Fixture {
        release: Option<String>,
        commands: Vec<&'static str>,
    }

    impl HostProbe for Fixture {
        fn os_release(&self) -> Option<String> {
            self.release.clone()
        }

        fn command_exists(&self, command: &str) -> bool {
            self.commands.contains(&command)
        }

        fn path_exists(&self, _path: &str) -> bool {
            false
        }
    }

    fn fedora() -> Fixture {
        Fixture {
            release: Some("ID=fedora\nVERSION_ID=44\nPRETTY_NAME=\"Fedora Linux 44\"\n".to_owned()),
            commands: vec![
                "dnf5",
                "firewall-cmd",
                "useradd",
                "timedatectl",
                "localectl",
                "xdg-mime",
            ],
        }
    }

    #[test]
    fn a_supported_host_reports_the_detected_capabilities() {
        let report = DistroReport::detect_with(&fedora());
        assert_eq!(report.distro.family, DistroFamily::Fedora);
        assert_eq!(report.backend, BackendId::FEDORA);
        assert_eq!(report.capabilities.len(), Capability::ALL.len());
        assert!(report
            .capability(Capability::Updates)
            .unwrap()
            .status
            .is_available());
        assert!(report
            .capability(Capability::Firewall)
            .unwrap()
            .status
            .is_available());
        // gdm/sddm are not in the fixture command list.
        assert!(report
            .capability(Capability::Autologin)
            .unwrap()
            .status
            .is_absent());
        assert!(report.available().contains(&Capability::AppDiscovery));
        let text = report.render();
        assert!(text.contains("Fedora Linux 44"));
        assert!(text.contains("updates"));
        assert!(text.contains("dnf5 check-update --json"));
    }

    #[test]
    fn an_unknown_host_is_unsupported_and_still_renders() {
        let report = DistroReport::detect_with(&Fixture {
            release: Some("ID=void\nPRETTY_NAME=\"Void Linux\"\n".to_owned()),
            commands: vec![],
        });
        assert_eq!(report.distro.family, DistroFamily::Unknown);
        assert!(report
            .capability(Capability::Updates)
            .unwrap()
            .status
            .is_unsupported());
        assert!(report.render().contains("unsupported distribution"));
    }

    #[test]
    fn a_missing_os_release_is_an_unknown_host() {
        let report = DistroReport::detect_with(&Fixture {
            release: None,
            commands: vec![],
        });
        assert_eq!(report.distro.family, DistroFamily::Unknown);
        assert_eq!(report.capabilities.len(), Capability::ALL.len());
    }
}
