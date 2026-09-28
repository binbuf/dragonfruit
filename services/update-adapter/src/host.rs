// SPDX-License-Identifier: MIT
//! The live host-stack source: the host identity read and the distribution
//! update provider seam.
//!
//! [`HostSystem`] is the concrete [`SystemSource`] a session host runs. It has
//! two collaborators:
//!
//! * the host's **identity files** — `/etc/os-release`, `/proc`, and the DMI
//!   sysfs nodes — read directly and root-configurable so a fixture tree can
//!   drive the parser in tests. This is the About and General half and it is
//!   always read.
//! * an optional [`UpdateProvider`] — the distribution's own update provider
//!   (the `SystemProvider` of [08-settings.md]). The concrete provider is
//!   distro-specific and belongs with packaging
//!   ([12-packaging.md] puts it below the distro-agnostic interface), so this
//!   crate ships the seam and the mock; `HostSystem::new` runs with no provider
//!   and reports updates as absent until one is attached.
//!
//! Reuse, never reimplement: the adapter does not parse package-manager output
//! or resolve dependencies. It reads what the host publishes and forwards the
//! explicit check/install/reboot writes to the provider.
//!
//! [08-settings.md]: ../../../docs/design/08-settings.md
//! [12-packaging.md]: ../../../docs/design/12-packaging.md

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use dragonfruit_system_adapters::AdapterError;

use crate::source::{SystemData, SystemIdentity, SystemSource, UpdateData, UpdateOutcome};

/// The distribution's own update provider.
///
/// This is the `SystemProvider` interface of [08-settings.md], narrowed to the
/// update half and adapted to the adapter contract's synchronous, explicit
/// shape. A provider is owned by the host and reports its own state; the
/// adapter never opens a transaction itself.
///
/// [08-settings.md]: ../../../docs/design/08-settings.md
pub trait UpdateProvider {
    /// The provider's current state. `Ok(None)` means the provider has gone
    /// away; `Err` means it is present but could not be read.
    fn status(&mut self) -> Result<Option<UpdateData>, AdapterError>;

    /// Start a check for updates. One explicit request.
    fn start_check(&mut self) -> UpdateOutcome {
        UpdateOutcome::Absent
    }

    /// Start installing the available updates. One explicit request.
    fn start_install(&mut self) -> UpdateOutcome {
        UpdateOutcome::Absent
    }

    /// Restart the host to finish an update. One explicit request.
    fn start_reboot(&mut self) -> UpdateOutcome {
        UpdateOutcome::Absent
    }
}

/// The live host-stack source.
pub struct HostSystem {
    root: PathBuf,
    provider: Option<Box<dyn UpdateProvider>>,
}

impl Default for HostSystem {
    fn default() -> Self {
        HostSystem::new()
    }
}

impl HostSystem {
    /// A source reading the real host (`/`) with no update provider attached.
    pub fn new() -> Self {
        HostSystem {
            root: PathBuf::from("/"),
            provider: None,
        }
    }

    /// A source reading `root` instead of `/`, for tests.
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        HostSystem {
            root: root.into(),
            provider: None,
        }
    }

    /// Attach the distribution update provider.
    pub fn with_provider(mut self, provider: Box<dyn UpdateProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    /// The filesystem root this source reads.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Read the host identity from the configured root.
    pub fn read_identity(&self) -> SystemIdentity {
        let os_release = read_trimmed(self.root.join("etc/os-release"))
            .map(|text| parse_os_release(&text))
            .unwrap_or_default();
        let host_name = read_trimmed(self.root.join("etc/hostname"))
            .or_else(|| read_trimmed(self.root.join("proc/sys/kernel/hostname")))
            .unwrap_or_default();
        let kernel = read_trimmed(self.root.join("proc/sys/kernel/osrelease")).unwrap_or_default();
        let device_model = read_trimmed(self.root.join("sys/devices/virtual/dmi/id/product_name"))
            .unwrap_or_default();
        let serial = read_trimmed(self.root.join("sys/devices/virtual/dmi/id/product_serial"))
            .unwrap_or_default();
        let processor = read_trimmed(self.root.join("proc/cpuinfo"))
            .map(|text| parse_cpu_model(&text))
            .unwrap_or_default();
        let memory_bytes = read_trimmed(self.root.join("proc/meminfo"))
            .map(|text| parse_mem_total(&text))
            .unwrap_or(0);

        SystemIdentity {
            host_name,
            os_name: os_release
                .get("PRETTY_NAME")
                .or_else(|| os_release.get("NAME"))
                .cloned()
                .unwrap_or_default(),
            os_version: os_release.get("VERSION_ID").cloned().unwrap_or_default(),
            os_id: os_release.get("ID").cloned().unwrap_or_default(),
            kernel,
            architecture: std::env::consts::ARCH.to_owned(),
            device_model,
            processor,
            memory_bytes,
            serial,
        }
    }
}

impl SystemSource for HostSystem {
    fn read(&mut self) -> Result<Option<SystemData>, AdapterError> {
        let identity = self.read_identity();
        let updates = match self.provider.as_mut() {
            Some(provider) => provider.status()?,
            None => None,
        };

        let identity_blank = identity.os_name.is_empty()
            && identity.os_id.is_empty()
            && identity.host_name.is_empty()
            && identity.kernel.is_empty();
        if identity_blank && updates.is_none() {
            return Ok(None);
        }
        Ok(Some(SystemData { identity, updates }))
    }

    fn check(&mut self) -> UpdateOutcome {
        match self.provider.as_mut() {
            Some(provider) => provider.start_check(),
            None => UpdateOutcome::Absent,
        }
    }

    fn install(&mut self) -> UpdateOutcome {
        match self.provider.as_mut() {
            Some(provider) => provider.start_install(),
            None => UpdateOutcome::Absent,
        }
    }

    fn reboot(&mut self) -> UpdateOutcome {
        match self.provider.as_mut() {
            Some(provider) => provider.start_reboot(),
            None => UpdateOutcome::Absent,
        }
    }
}

fn read_trimmed(path: PathBuf) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// Parse the `KEY=value` shape of `/etc/os-release`, stripping one layer of
/// surrounding quotes and ignoring comments and blanks.
pub(crate) fn parse_os_release(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .or_else(|| {
                value
                    .strip_prefix('\'')
                    .and_then(|value| value.strip_suffix('\''))
            })
            .unwrap_or(value);
        map.insert(key.trim().to_owned(), value.to_owned());
    }
    map
}

/// The processor description from `/proc/cpuinfo`: the first `model name`
/// (x86), `Processor` (many ARM boards), or `Hardware` line.
pub(crate) fn parse_cpu_model(text: &str) -> String {
    let mut fallback = String::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match key {
            "model name" => return value.to_owned(),
            "Processor" | "Hardware" if fallback.is_empty() => {
                fallback = value.to_owned();
            }
            _ => {}
        }
    }
    fallback
}

/// The total physical memory from `/proc/meminfo`'s `MemTotal:` line, in
/// bytes; `0` when the line is missing or unparseable.
pub(crate) fn parse_mem_total(text: &str) -> u64 {
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if key.trim() != "MemTotal" {
            continue;
        }
        let Some(kib) = value
            .split_whitespace()
            .next()
            .and_then(|n| n.parse::<u64>().ok())
        else {
            return 0;
        };
        return kib.saturating_mul(1024);
    }
    0
}

/// A fixture-backed distribution update provider.
///
/// The mock is the CI path when a `HostSystem` test attaches a provider: it
/// serves [`UpdateData`] with no package manager on the machine, and
/// `kill`/`restart` exercise the provider going away and coming back.
#[derive(Debug, Clone, PartialEq)]
pub struct MockUpdateProvider {
    present: bool,
    data: UpdateData,
    failure: Option<AdapterError>,
    write_failure: Option<AdapterError>,
    statuses: u32,
    checks: u32,
    installs: u32,
    reboots: u32,
}

impl MockUpdateProvider {
    /// A provider reporting `data`.
    pub fn present(data: UpdateData) -> Self {
        MockUpdateProvider {
            present: true,
            data,
            failure: None,
            write_failure: None,
            statuses: 0,
            checks: 0,
            installs: 0,
            reboots: 0,
        }
    }

    /// A provider that is present but fails every status read.
    pub fn failing(message: impl Into<String>) -> Self {
        MockUpdateProvider {
            present: true,
            data: UpdateData::default(),
            failure: Some(AdapterError::new(message)),
            write_failure: None,
            statuses: 0,
            checks: 0,
            installs: 0,
            reboots: 0,
        }
    }

    /// A provider whose writes fail.
    pub fn fail_writes(mut self, message: impl Into<String>) -> Self {
        self.write_failure = Some(AdapterError::new(message));
        self
    }

    /// Publish fresh provider state.
    pub fn push(&mut self, data: UpdateData) {
        self.present = true;
        self.failure = None;
        self.data = data;
    }

    /// The provider goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The provider comes back.
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// The provider's current state.
    pub fn data(&self) -> &UpdateData {
        &self.data
    }

    /// How many status reads the provider served.
    pub fn statuses(&self) -> u32 {
        self.statuses
    }

    /// How many checks the provider accepted.
    pub fn checks(&self) -> u32 {
        self.checks
    }

    /// How many installs the provider accepted.
    pub fn installs(&self) -> u32 {
        self.installs
    }

    /// How many reboots the provider accepted.
    pub fn reboots(&self) -> u32 {
        self.reboots
    }

    fn outcome(&self) -> UpdateOutcome {
        if !self.present {
            return UpdateOutcome::Absent;
        }
        if let Some(error) = &self.write_failure {
            return UpdateOutcome::Failed(error.clone());
        }
        UpdateOutcome::Applied
    }
}

impl UpdateProvider for MockUpdateProvider {
    fn status(&mut self) -> Result<Option<UpdateData>, AdapterError> {
        self.statuses += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone()))
    }

    fn start_check(&mut self) -> UpdateOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.checks += 1;
        }
        outcome
    }

    fn start_install(&mut self) -> UpdateOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.installs += 1;
        }
        outcome
    }

    fn start_reboot(&mut self) -> UpdateOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.reboots += 1;
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{UpdateItem, UpdatePhase, UpdateSeverity};

    #[test]
    fn os_release_parses_quotes_and_comments() {
        let map = parse_os_release(
            "# comment\nNAME=\"Dragonfruit Linux\"\nID=dragonfruit\nVERSION_ID='44'\nBAD\n",
        );
        assert_eq!(
            map.get("NAME").map(String::as_str),
            Some("Dragonfruit Linux")
        );
        assert_eq!(map.get("ID").map(String::as_str), Some("dragonfruit"));
        assert_eq!(map.get("VERSION_ID").map(String::as_str), Some("44"));
        assert!(!map.contains_key("BAD"));
    }

    #[test]
    fn cpu_model_parses_x86_and_arm() {
        assert_eq!(
            parse_cpu_model("processor\t: 0\nmodel name\t: Example CPU 9000\n"),
            "Example CPU 9000"
        );
        assert_eq!(
            parse_cpu_model("Processor\t: ARMv8 Processor\n"),
            "ARMv8 Processor"
        );
        assert_eq!(parse_cpu_model("Hardware\t: Board\n"), "Board");
        assert_eq!(parse_cpu_model(""), "");
    }

    #[test]
    fn mem_total_parses_kib_to_bytes() {
        assert_eq!(
            parse_mem_total("MemTotal:       16384000 kB\nMemFree: 1 kB\n"),
            16384000 * 1024
        );
        assert_eq!(parse_mem_total("MemFree: 1 kB\n"), 0);
        assert_eq!(parse_mem_total(""), 0);
    }

    fn fixture_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "dragonfruit-update-adapter-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["etc", "proc/sys/kernel", "sys/devices/virtual/dmi/id"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
        std::fs::write(
            root.join("etc/os-release"),
            "NAME=\"Dragonfruit Linux\"\nPRETTY_NAME=\"Dragonfruit Linux 44\"\nID=dragonfruit\nVERSION_ID=\"44\"\n",
        )
        .unwrap();
        std::fs::write(root.join("etc/hostname"), "fixture\n").unwrap();
        std::fs::write(root.join("proc/sys/kernel/osrelease"), "6.12.0\n").unwrap();
        std::fs::write(
            root.join("proc/cpuinfo"),
            "processor\t: 0\nmodel name\t: Fixture CPU\n",
        )
        .unwrap();
        std::fs::write(root.join("proc/meminfo"), "MemTotal:       16384000 kB\n").unwrap();
        std::fs::write(
            root.join("sys/devices/virtual/dmi/id/product_name"),
            "Fixture Book\n",
        )
        .unwrap();
        std::fs::write(
            root.join("sys/devices/virtual/dmi/id/product_serial"),
            "FIXTURE-1\n",
        )
        .unwrap();
        root
    }

    #[test]
    fn the_identity_read_follows_the_root() {
        let root = fixture_root();
        let system = HostSystem::with_root(&root);
        let identity = system.read_identity();
        assert_eq!(identity.host_name, "fixture");
        assert_eq!(identity.os_name, "Dragonfruit Linux 44");
        assert_eq!(identity.os_id, "dragonfruit");
        assert_eq!(identity.os_version, "44");
        assert_eq!(identity.kernel, "6.12.0");
        assert_eq!(identity.device_model, "Fixture Book");
        assert_eq!(identity.processor, "Fixture CPU");
        assert_eq!(identity.memory_bytes, 16384000 * 1024);
        assert_eq!(identity.serial, "FIXTURE-1");
        assert_eq!(identity.memory_label(), "16 GB");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unreadable_root_is_a_normal_absence() {
        let root = std::env::temp_dir().join(format!(
            "dragonfruit-update-adapter-missing-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut system = HostSystem::with_root(&root);
        assert_eq!(system.read(), Ok(None));
        assert_eq!(system.check(), UpdateOutcome::Absent);
    }

    #[test]
    fn a_blank_identity_still_answers_when_a_provider_is_present() {
        let root = std::env::temp_dir().join(format!(
            "dragonfruit-update-adapter-blank-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let mut system = HostSystem::with_root(&root).with_provider(Box::new(
            MockUpdateProvider::present(UpdateData {
                phase: UpdatePhase::UpToDate,
                ..UpdateData::default()
            }),
        ));
        let data = system.read().unwrap().unwrap();
        assert!(data.identity.host_name.is_empty());
        assert!(data.updates_available());
    }

    #[test]
    fn the_provider_status_rides_the_read_and_the_writes() {
        let root = fixture_root();
        let mut system = HostSystem::with_root(&root).with_provider(Box::new(
            MockUpdateProvider::present(UpdateData {
                phase: UpdatePhase::Available,
                updates: vec![UpdateItem {
                    id: "glibc".to_owned(),
                    severity: UpdateSeverity::Security,
                    ..UpdateItem::default()
                }],
                last_checked_ms: Some(5),
                message: None,
            }),
        ));
        let data = system.read().unwrap().unwrap();
        assert_eq!(data.identity.host_name, "fixture");
        assert_eq!(data.updates.as_ref().unwrap().phase, UpdatePhase::Available);
        assert_eq!(system.check(), UpdateOutcome::Applied);
        assert_eq!(system.install(), UpdateOutcome::Applied);
        assert_eq!(system.reboot(), UpdateOutcome::Applied);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_failing_provider_is_an_error_not_absence() {
        let root = fixture_root();
        let mut system = HostSystem::with_root(&root)
            .with_provider(Box::new(MockUpdateProvider::failing("provider: timeout")));
        let error = system.read().unwrap_err();
        assert_eq!(error.message(), "provider: timeout");
        let _ = std::fs::remove_dir_all(&root);
    }
}
