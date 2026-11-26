// SPDX-License-Identifier: GPL-3.0-or-later
//! The distribution group provider (T279).
//!
//! AccountsService owns the user list but has no group API, so the group half
//! of Users & Groups projects the host's own group database. This provider is
//! the real (non-mock) implementation of the [`GroupProvider`] seam:
//!
//! * **read** — `getent group` when the host ships it (NSS, so directory groups
//!   are included), falling back to `/etc/group`;
//! * **write** — the distribution's `groupadd`, `groupdel`, and `gpasswd`
//!   commands, raised through `pkexec` so the host polkit authority owns the
//!   prompt. A missing `pkexec` is a visible denial, never a silent edit.
//!
//! The provider never edits `/etc/group` itself, never writes a shadow file,
//! and never caches a snapshot: the adapter re-reads after every write.
//!
//! [`GroupProvider`]: crate::GroupProvider

use std::path::{Path, PathBuf};

use dragonfruit_system_adapters::AdapterError;

use crate::accounts::GroupProvider;
use crate::source::{AccountOutcome, GroupData};

/// The real distribution group provider.
#[derive(Debug, Clone)]
pub struct FileGroupProvider {
    root: PathBuf,
    privilege: String,
}

impl Default for FileGroupProvider {
    fn default() -> Self {
        FileGroupProvider::new()
    }
}

impl FileGroupProvider {
    /// A provider over the live host.
    pub fn new() -> Self {
        FileGroupProvider {
            root: PathBuf::from("/"),
            privilege: "pkexec".to_owned(),
        }
    }

    /// A provider over a test root, with the privilege command it would run.
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        FileGroupProvider {
            root: root.into(),
            privilege: "pkexec".to_owned(),
        }
    }

    /// Override the privilege-raising command (tests use an empty string to
    /// exercise the absent-privilege path).
    pub fn with_privilege(mut self, command: impl Into<String>) -> Self {
        self.privilege = command.into();
        self
    }

    fn group_file(&self) -> PathBuf {
        self.root.join("etc/group")
    }

    fn run(&self, command: &str, args: &[&str]) -> AccountOutcome {
        if command.is_empty() {
            return AccountOutcome::Denied(
                "the host has no polkit privilege helper (pkexec)".to_owned(),
            );
        }
        let output = std::process::Command::new(command).args(args).output();
        match output {
            Ok(output) if output.status.success() => AccountOutcome::Applied,
            Ok(output) => {
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                classify_failure(&text)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => AccountOutcome::Absent,
            Err(error) => AccountOutcome::Failed(AdapterError::new(error.to_string())),
        }
    }

    fn privileged(&self, command: &str, args: &[&str]) -> AccountOutcome {
        let mut argv: Vec<&str> = Vec::with_capacity(args.len() + 1);
        argv.push(command);
        argv.extend_from_slice(args);
        self.run(&self.privilege, &argv)
    }
}

impl GroupProvider for FileGroupProvider {
    fn status(&mut self) -> Result<Option<Vec<GroupData>>, AdapterError> {
        let live = self.root == Path::new("/");
        let text = match read_group_database(&self.group_file(), live) {
            Some(text) => text,
            None => return Ok(None),
        };
        Ok(Some(parse_groups(&text)))
    }

    fn create_group(&mut self, name: &str) -> AccountOutcome {
        if !valid_group_name(name) {
            return AccountOutcome::Denied(format!("invalid group name: {name}"));
        }
        self.privileged("groupadd", &["--", name])
    }

    fn delete_group(&mut self, name: &str) -> AccountOutcome {
        if !valid_group_name(name) {
            return AccountOutcome::Denied(format!("invalid group name: {name}"));
        }
        self.privileged("groupdel", &["--", name])
    }

    fn set_members(&mut self, name: &str, members: &[String]) -> AccountOutcome {
        if !valid_group_name(name) {
            return AccountOutcome::Denied(format!("invalid group name: {name}"));
        }
        let list = members.join(",");
        self.privileged("gpasswd", &["-M", list.as_str(), "--", name])
    }
}

/// A group name the host tools accept: no leading dash, no path or whitespace.
fn valid_group_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name.len() <= 32
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.')
}

/// A host refusal is a policy denial; everything else is a failure.
fn classify_failure(text: &str) -> AccountOutcome {
    let lowered = text.to_ascii_lowercase();
    if lowered.contains("not authorized")
        || lowered.contains("dismissed")
        || lowered.contains("authentication")
        || lowered.contains("permission denied")
        || lowered.contains("operation not permitted")
    {
        return AccountOutcome::Denied(text.trim().to_owned());
    }
    AccountOutcome::Failed(AdapterError::new(if text.trim().is_empty() {
        "the group provider refused the request".to_owned()
    } else {
        text.trim().to_owned()
    }))
}

/// Read the group database through NSS (`getent`) when the provider is live,
/// falling back to the local file. `None` means no group owner answered at all.
fn read_group_database(group_file: &Path, live: bool) -> Option<String> {
    if live {
        if let Ok(output) = std::process::Command::new("getent").arg("group").output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).into_owned();
                if !text.trim().is_empty() {
                    return Some(text);
                }
            }
        }
    }
    std::fs::read_to_string(group_file).ok()
}

/// Parse the `name:password:gid:members` lines the host publishes.
fn parse_groups(text: &str) -> Vec<GroupData> {
    let mut groups = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split(':');
        let Some(name) = fields.next() else { continue };
        let _password = fields.next();
        let Some(gid) = fields.next().and_then(|gid| gid.parse::<u32>().ok()) else {
            continue;
        };
        let members: Vec<String> = fields
            .next()
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|member| !member.is_empty())
            .map(str::to_owned)
            .collect();
        groups.push(GroupData {
            name: name.to_owned(),
            gid,
            members,
            // The conventional system range; a group in the user range is a
            // user-created group the pane may edit.
            system: gid < 1000,
        });
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_group_database_is_parsed_in_host_order() {
        let text = "wheel:x:10:dan,sam\nusers:x:100:dan\nroot:x:0:\n";
        let groups = parse_groups(text);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].name, "wheel");
        assert_eq!(groups[0].gid, 10);
        assert_eq!(groups[0].members, vec!["dan", "sam"]);
        // A gid in the reserved range is a system group the pane may not
        // delete; a user-range gid is editable.
        assert!(groups[0].system);
        assert_eq!(groups[1].members, vec!["dan"]);
        assert!(groups[2].system);
    }

    #[test]
    fn a_malformed_line_is_skipped_not_guessed() {
        let groups = parse_groups("broken\nwheel:x:10:dan\nno-gid:x::dan\n");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "wheel");
    }

    #[test]
    fn group_names_are_validated_before_a_write() {
        assert!(valid_group_name("devs"));
        assert!(valid_group_name("team_1"));
        assert!(!valid_group_name(""));
        assert!(!valid_group_name("-rf"));
        assert!(!valid_group_name("two words"));
        assert!(!valid_group_name("../etc"));
    }

    #[test]
    fn a_policy_refusal_is_a_denial() {
        assert!(matches!(
            classify_failure("Error executing command as another user: Not authorized"),
            AccountOutcome::Denied(_)
        ));
        assert!(matches!(
            classify_failure("groupadd: group already exists"),
            AccountOutcome::Failed(_)
        ));
    }

    #[test]
    fn a_missing_group_file_is_absent() {
        let mut provider = FileGroupProvider::with_root("/nonexistent-t279-root");
        assert!(provider.status().unwrap().is_none());
    }

    #[test]
    fn a_test_root_reads_its_own_group_file() {
        let root = std::env::temp_dir().join(format!("df-t279-groups-{}", std::process::id()));
        std::fs::create_dir_all(root.join("etc")).unwrap();
        std::fs::write(root.join("etc/group"), "devs:x:1200:dan\nwheel:x:10:dan\n").unwrap();
        let mut provider = FileGroupProvider::with_root(&root);
        let groups = provider.status().unwrap().unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "devs");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_write_without_a_privilege_helper_is_a_visible_denial() {
        let mut provider = FileGroupProvider::with_root("/").with_privilege("");
        assert!(matches!(
            provider.create_group("devs"),
            AccountOutcome::Denied(_)
        ));
    }
}
