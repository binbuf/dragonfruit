// SPDX-License-Identifier: MIT
//! Distribution detection and the support-tier vocabulary.
//!
//! Detection reads `/etc/os-release` and maps `ID` / `ID_LIKE` onto one of the
//! four supported families. It never hardcodes a distro list in the UI, and an
//! unrecognized host is [`DistroFamily::Unknown`] (best-effort), never a
//! guessed backend.

use std::collections::BTreeMap;
use std::path::Path;

/// A supported distribution family, or an unrecognized host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DistroFamily {
    /// Fedora and its Fedora-family derivatives.
    Fedora,
    /// Ubuntu and its Ubuntu-family derivatives.
    Ubuntu,
    /// Debian and its Debian-family derivatives.
    Debian,
    /// Arch Linux and its Arch-family derivatives.
    Arch,
    /// A host outside the supported matrix.
    Unknown,
}

impl DistroFamily {
    /// Every family, including the unrecognized host.
    pub const ALL: [DistroFamily; 5] = [
        DistroFamily::Fedora,
        DistroFamily::Ubuntu,
        DistroFamily::Debian,
        DistroFamily::Arch,
        DistroFamily::Unknown,
    ];

    /// The four families with a stub backend and a documented matrix.
    pub const SUPPORTED: [DistroFamily; 4] = [
        DistroFamily::Fedora,
        DistroFamily::Ubuntu,
        DistroFamily::Debian,
        DistroFamily::Arch,
    ];

    /// The stable id used by the wire, the matrix, and the probe.
    pub const fn id(self) -> &'static str {
        match self {
            DistroFamily::Fedora => "fedora",
            DistroFamily::Ubuntu => "ubuntu",
            DistroFamily::Debian => "debian",
            DistroFamily::Arch => "arch",
            DistroFamily::Unknown => "unknown",
        }
    }

    /// The human label.
    pub const fn label(self) -> &'static str {
        match self {
            DistroFamily::Fedora => "Fedora",
            DistroFamily::Ubuntu => "Ubuntu",
            DistroFamily::Debian => "Debian",
            DistroFamily::Arch => "Arch Linux",
            DistroFamily::Unknown => "Unsupported distribution",
        }
    }

    /// The support tier from [distro-compatibility.md] §2.
    pub const fn tier(self) -> SupportTier {
        match self {
            DistroFamily::Fedora | DistroFamily::Ubuntu | DistroFamily::Debian => {
                SupportTier::Tier1
            }
            DistroFamily::Arch => SupportTier::Tier2,
            DistroFamily::Unknown => SupportTier::Unsupported,
        }
    }

    /// The administrator group names to detect, in preference order. A host
    /// may carry more than one (Ubuntu/Debian expose `sudo` and `admin`).
    pub const fn admin_groups(self) -> &'static [&'static str] {
        match self {
            DistroFamily::Fedora | DistroFamily::Arch => &["wheel"],
            DistroFamily::Ubuntu | DistroFamily::Debian => &["sudo", "admin"],
            DistroFamily::Unknown => &[],
        }
    }

    /// Map an exact `ID` (and optional `ID_LIKE` tokens) onto a family.
    ///
    /// Ubuntu is checked before Debian so an Ubuntu derivative with
    /// `ID_LIKE="ubuntu debian"` stays Ubuntu. Derivatives inherit their
    /// upstream family through `ID_LIKE`.
    pub fn from_os_release(id: &str, id_like: &[String]) -> DistroFamily {
        let id = id.to_ascii_lowercase();
        match id.as_str() {
            "fedora" | "rhel" | "centos" | "rocky" | "almalinux" => DistroFamily::Fedora,
            "ubuntu" => DistroFamily::Ubuntu,
            "debian" => DistroFamily::Debian,
            "arch" | "archlinux" | "manjaro" | "endeavouros" => DistroFamily::Arch,
            _ => {
                let like: Vec<String> = id_like.iter().map(|s| s.to_ascii_lowercase()).collect();
                if like
                    .iter()
                    .any(|s| matches!(s.as_str(), "fedora" | "rhel" | "centos"))
                {
                    DistroFamily::Fedora
                } else if like.iter().any(|s| s == "ubuntu") {
                    DistroFamily::Ubuntu
                } else if like.iter().any(|s| s == "debian") {
                    DistroFamily::Debian
                } else if like.iter().any(|s| s == "arch") {
                    DistroFamily::Arch
                } else {
                    DistroFamily::Unknown
                }
            }
        }
    }

    /// Map an exact id without `ID_LIKE` fallback.
    pub fn from_id(id: &str) -> DistroFamily {
        DistroFamily::from_os_release(id, &[])
    }
}

/// The support tier from [distro-compatibility.md] §2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportTier {
    /// CI install test, full feature matrix, release blocker.
    Tier1,
    /// CI install test, core feature matrix, documented gaps.
    Tier2,
    /// Best-effort; community-tested derivative.
    Tier3,
    /// No systemd/logind, Wayland/DRM, or the shared host stack.
    Unsupported,
}

impl SupportTier {
    /// The stable id used by the probe and diagnostics.
    pub const fn id(self) -> &'static str {
        match self {
            SupportTier::Tier1 => "tier-1",
            SupportTier::Tier2 => "tier-2",
            SupportTier::Tier3 => "tier-3",
            SupportTier::Unsupported => "unsupported",
        }
    }

    /// The human label.
    pub const fn label(self) -> &'static str {
        match self {
            SupportTier::Tier1 => "Tier 1",
            SupportTier::Tier2 => "Tier 2",
            SupportTier::Tier3 => "Tier 3",
            SupportTier::Unsupported => "Unsupported",
        }
    }
}

/// One host's distribution identity, read from `/etc/os-release`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Distro {
    /// The mapped family.
    pub family: DistroFamily,
    /// `ID` (`fedora`, `ubuntu`, `debian`, `arch`, …).
    pub id: String,
    /// `NAME`.
    pub name: String,
    /// `VERSION_ID`.
    pub version: String,
    /// `PRETTY_NAME`.
    pub pretty_name: String,
    /// `ID_LIKE` tokens.
    pub id_like: Vec<String>,
    /// Whether the host is a derivative of its mapped upstream family
    /// (`bazzite`, `pop`, `manjaro`, `rhel`, …). A derivative uses the
    /// upstream backend but is community-tested Tier 3.
    pub derivative: bool,
}

impl Distro {
    /// The honest "no `/etc/os-release`" host.
    pub fn unknown() -> Self {
        Distro {
            family: DistroFamily::Unknown,
            id: String::new(),
            name: String::new(),
            version: String::new(),
            pretty_name: String::new(),
            id_like: Vec::new(),
            derivative: false,
        }
    }

    /// A synthetic identity for a family, used by the stub constructors and
    /// the fixture tests.
    pub fn for_family(family: DistroFamily) -> Self {
        Distro {
            family,
            id: family.id().to_owned(),
            name: family.label().to_owned(),
            version: String::new(),
            pretty_name: family.label().to_owned(),
            id_like: Vec::new(),
            derivative: false,
        }
    }

    /// Detect the host from `/etc/os-release`.
    pub fn detect() -> Self {
        Distro::detect_from_root(Path::new("/"))
    }

    /// Detect the host from `root/etc/os-release`; a missing file is an
    /// unknown host, not an error.
    pub fn detect_from_root(root: &Path) -> Self {
        std::fs::read_to_string(root.join("etc/os-release"))
            .map(|text| Distro::from_os_release(&text))
            .unwrap_or_else(|_| Distro::unknown())
    }

    /// Parse `/etc/os-release` text into a distribution identity.
    pub fn from_os_release(text: &str) -> Self {
        let map = parse_os_release(text);
        let id = map.get("ID").cloned().unwrap_or_default();
        let id_like: Vec<String> = map
            .get("ID_LIKE")
            .map(|value| value.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default();
        let family = DistroFamily::from_os_release(&id, &id_like);
        let derivative = family != DistroFamily::Unknown && !is_upstream_id(&id, family);
        Distro {
            family,
            id,
            name: map
                .get("NAME")
                .cloned()
                .or_else(|| map.get("PRETTY_NAME").cloned())
                .unwrap_or_default(),
            version: map.get("VERSION_ID").cloned().unwrap_or_default(),
            pretty_name: map
                .get("PRETTY_NAME")
                .cloned()
                .or_else(|| map.get("NAME").cloned())
                .unwrap_or_default(),
            id_like,
            derivative,
        }
    }

    /// The support tier, with derivatives at Tier 3 and Ubuntu 26.04+
    /// demoted to Tier 2 as the compatibility document specifies.
    pub fn tier(&self) -> SupportTier {
        if self.derivative {
            return SupportTier::Tier3;
        }
        if self.family == DistroFamily::Ubuntu {
            let major = self
                .version
                .split('.')
                .next()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(0);
            if major >= 26 {
                return SupportTier::Tier2;
            }
        }
        self.family.tier()
    }

    /// Whether the host is one of the four supported families.
    pub fn is_supported(&self) -> bool {
        self.family != DistroFamily::Unknown
    }

    /// The administrator groups to detect on this host.
    pub fn admin_groups(&self) -> &'static [&'static str] {
        self.family.admin_groups()
    }

    /// The label the probe prints: `PRETTY_NAME`, then `NAME`, then `ID`.
    pub fn display_name(&self) -> String {
        for candidate in [&self.pretty_name, &self.name, &self.id] {
            if !candidate.is_empty() {
                return candidate.clone();
            }
        }
        "unknown".to_owned()
    }
}

fn is_upstream_id(id: &str, family: DistroFamily) -> bool {
    match family {
        DistroFamily::Fedora => id.eq_ignore_ascii_case("fedora"),
        DistroFamily::Ubuntu => id.eq_ignore_ascii_case("ubuntu"),
        DistroFamily::Debian => id.eq_ignore_ascii_case("debian"),
        DistroFamily::Arch => {
            id.eq_ignore_ascii_case("arch") || id.eq_ignore_ascii_case("archlinux")
        }
        DistroFamily::Unknown => false,
    }
}

/// Parse the `KEY=value` shape of `/etc/os-release`, stripping one layer of
/// surrounding quotes and ignoring comments and blanks.
pub fn parse_os_release(text: &str) -> BTreeMap<String, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn release(id: &str, id_like: &str, version: &str) -> Distro {
        Distro::from_os_release(&format!(
            "ID={id}\nID_LIKE=\"{id_like}\"\nVERSION_ID=\"{version}\"\nPRETTY_NAME=\"Test {id}\"\n"
        ))
    }

    #[test]
    fn the_four_supported_distros_map_to_their_families() {
        for (id, version, family) in [
            ("fedora", "44", DistroFamily::Fedora),
            ("ubuntu", "24.04", DistroFamily::Ubuntu),
            ("debian", "13", DistroFamily::Debian),
            ("arch", "", DistroFamily::Arch),
        ] {
            let host = release(id, "", version);
            assert_eq!(host.family, family);
            assert!(!host.derivative, "{id}");
        }
    }

    #[test]
    fn id_like_derivatives_inherit_their_upstream_family() {
        let bazzite = release("bazzite", "fedora", "44");
        assert_eq!(bazzite.family, DistroFamily::Fedora);
        assert!(bazzite.derivative);
        assert_eq!(bazzite.tier(), SupportTier::Tier3);

        let pop = release("pop", "ubuntu debian", "24.04");
        assert_eq!(pop.family, DistroFamily::Ubuntu);
        assert!(pop.derivative);
        assert_eq!(pop.tier(), SupportTier::Tier3);

        let mint = release("linuxmint", "ubuntu debian", "22");
        assert_eq!(mint.family, DistroFamily::Ubuntu);
        assert!(mint.derivative);

        let manjaro = release("manjaro", "arch", "");
        assert_eq!(manjaro.family, DistroFamily::Arch);
        assert!(manjaro.derivative);
        assert_eq!(manjaro.tier(), SupportTier::Tier3);
    }

    #[test]
    fn an_unknown_host_is_not_guessed() {
        let host = release("void", "void", "");
        assert_eq!(host.family, DistroFamily::Unknown);
        assert!(!host.is_supported());
        assert_eq!(host.tier(), SupportTier::Unsupported);
    }

    #[test]
    fn ubuntu_2604_is_tier_two() {
        assert_eq!(release("ubuntu", "", "24.04").tier(), SupportTier::Tier1);
        assert_eq!(release("ubuntu", "", "26.04").tier(), SupportTier::Tier2);
        assert_eq!(release("fedora", "", "44").tier(), SupportTier::Tier1);
        assert_eq!(release("arch", "", "").tier(), SupportTier::Tier2);
    }

    #[test]
    fn admin_groups_are_detected_not_hardcoded() {
        assert_eq!(DistroFamily::Fedora.admin_groups(), &["wheel"]);
        assert_eq!(DistroFamily::Arch.admin_groups(), &["wheel"]);
        assert_eq!(DistroFamily::Ubuntu.admin_groups(), &["sudo", "admin"]);
        assert_eq!(DistroFamily::Debian.admin_groups(), &["sudo", "admin"]);
        assert!(DistroFamily::Unknown.admin_groups().is_empty());
    }

    #[test]
    fn os_release_parsing_strips_quotes_and_comments() {
        let map =
            parse_os_release("# a comment\nNAME=\"Fedora Linux\"\nID='fedora'\nVERSION_ID=44\n");
        assert_eq!(map.get("NAME").unwrap(), "Fedora Linux");
        assert_eq!(map.get("ID").unwrap(), "fedora");
        assert_eq!(map.get("VERSION_ID").unwrap(), "44");
    }

    #[test]
    fn a_missing_file_is_an_unknown_host() {
        let root = std::env::temp_dir().join("dragonfruit-distro-missing-os-release");
        let _ = std::fs::remove_dir_all(&root);
        let host = Distro::detect_from_root(&root);
        assert_eq!(host.family, DistroFamily::Unknown);
    }
}
