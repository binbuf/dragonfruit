// SPDX-License-Identifier: MIT
//! Third-party toolkit integration applied at session start.
//!
//! Wayland leaves client-side decorations to the client. Our own apps and
//! Qt/SSD clients already put the traffic lights on the left; GTK-family
//! clients (GTK 3/4, libadwaita) draw their own headerbar and read the
//! standard `gtk-decoration-layout` setting instead. This module applies the
//! same left-side layout to that shared toolkit setting so a cooperative CSD
//! app matches the desktop.
//!
//! This is a best-effort compatibility bridge, not compositor-side decoration
//! injection: apps that hard-code their own controls (Electron/Chromium, SDL,
//! some toolkits) keep them where they draw them.
//!
//! A GTK client may read the layout from any of three places, so all three
//! are written:
//!
//! * `$XDG_CONFIG_HOME/gtk-3.0/settings.ini` and `gtk-4.0/settings.ini`
//!   (the Xwayland/XSettings fallback),
//! * `$XDG_CONFIG_HOME/glib-2.0/settings/keyfile` (the GSettings keyfile
//!   backend),
//! * the live dconf value through `gsettings`, when it is available, so a
//!   Wayland GTK client sees the change without a new login.
//!
//! The INI/keyfile writers preserve every line and key they do not own and
//! replace the file atomically. A missing `gsettings` is not an error: the
//! file fallbacks still carry the layout for the next login.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long the live `gsettings` write may take before it is abandoned. The
/// session entry runs this before the session starts, so a broken bus must
/// never hold the login.
const GSETTINGS_TIMEOUT: Duration = Duration::from_secs(5);

/// The GTK window-control layout. The colon separates the left and right
/// sides, so close/minimize/maximize sit on the left and the right side is
/// empty.
pub const BUTTON_LAYOUT: &str = "close,minimize,maximize:";

/// The GTK setting key (`gtk-decoration-layout`) in `settings.ini`.
pub const GTK_SETTINGS_KEY: &str = "gtk-decoration-layout";
/// The INI group GTK reads.
pub const GTK_SETTINGS_GROUP: &str = "Settings";

/// The GSettings schema that owns the toolkit layout.
pub const GSETTINGS_SCHEMA: &str = "org.gnome.desktop.wm.preferences";
/// The GSettings key the GTK setting maps to.
pub const GSETTINGS_KEY: &str = "button-layout";

/// The GSettings keyfile group: the schema path with `/` instead of `.`.
pub fn gsettings_group() -> String {
    GSETTINGS_SCHEMA.replace('.', "/")
}

/// Where the sync wrote, and what happened to the live dconf value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncReport {
    /// Files whose contents changed. Already-correct files are not rewritten.
    pub files: Vec<PathBuf>,
    /// The live `gsettings` result.
    pub gsettings: GsettingsOutcome,
}

/// The outcome of the best-effort live GSettings write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GsettingsOutcome {
    /// `gsettings set` succeeded.
    Applied,
    /// No `gsettings` binary is installed.
    Unavailable,
    /// `gsettings` ran but failed; the string is its stderr/status summary.
    Failed(String),
}

impl SyncReport {
    /// A one-line human summary for the `--sync-toolkit` command.
    pub fn summary(&self) -> String {
        let files = if self.files.is_empty() {
            "no file changes".to_string()
        } else {
            self.files
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let gsettings = match &self.gsettings {
            GsettingsOutcome::Applied => "gsettings applied".to_string(),
            GsettingsOutcome::Unavailable => "gsettings unavailable (file fallback)".to_string(),
            GsettingsOutcome::Failed(message) => format!("gsettings failed: {message}"),
        };
        format!("{files}; {gsettings}")
    }
}

/// Resolve `$XDG_CONFIG_HOME`, falling back to `$HOME/.config`. `None` when
/// neither variable is set.
pub fn config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".config")))
}

/// Apply the left-side layout to every GTK configuration source under
/// `config_home`. Returns the files whose contents changed.
pub fn sync_window_controls(config_home: &Path) -> io::Result<Vec<PathBuf>> {
    let mut written = Vec::new();

    for (directory, file) in [("gtk-3.0", "settings.ini"), ("gtk-4.0", "settings.ini")] {
        let path = config_home.join(directory).join(file);
        if update_ini_file(&path, GTK_SETTINGS_GROUP, GTK_SETTINGS_KEY, BUTTON_LAYOUT)? {
            written.push(path);
        }
    }

    // The keyfile backend stores strings as GVariant text, so the layout is
    // quoted exactly as `gsettings` would write it.
    let keyfile = config_home
        .join("glib-2.0")
        .join("settings")
        .join("keyfile");
    let value = gvariant_string(BUTTON_LAYOUT);
    if update_ini_file(&keyfile, &gsettings_group(), GSETTINGS_KEY, &value)? {
        written.push(keyfile);
    }

    Ok(written)
}

/// [`sync_window_controls`] for the process environment, plus the live
/// `gsettings` write. A missing config home is an error; a missing or failing
/// `gsettings` is reported in the result, never fatal.
pub fn sync_from_env() -> io::Result<SyncReport> {
    let Some(home) = config_home() else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "neither XDG_CONFIG_HOME nor HOME is set",
        ));
    };
    let files = sync_window_controls(&home)?;
    Ok(SyncReport {
        files,
        gsettings: apply_gsettings(),
    })
}

/// Write the live GSettings value through the standard `gsettings` tool. This
/// is what makes a GTK client on a dconf-backed Wayland session pick the
/// change up without a new login.
fn apply_gsettings() -> GsettingsOutcome {
    let mut child = match Command::new("gsettings")
        .args(["set", GSETTINGS_SCHEMA, GSETTINGS_KEY, BUTTON_LAYOUT])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return GsettingsOutcome::Unavailable
        }
        Err(error) => return GsettingsOutcome::Failed(error.to_string()),
    };

    let deadline = Instant::now() + GSETTINGS_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = match child.wait_with_output() {
                    Ok(output) => output,
                    Err(error) => return GsettingsOutcome::Failed(error.to_string()),
                };
                if output.status.success() {
                    return GsettingsOutcome::Applied;
                }
                let stderr = String::from_utf8_lossy(&output.stderr);
                let message = stderr.trim();
                return GsettingsOutcome::Failed(if message.is_empty() {
                    format!("exit status {}", output.status)
                } else {
                    message.to_string()
                });
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return GsettingsOutcome::Failed(format!(
                    "timed out after {}s",
                    GSETTINGS_TIMEOUT.as_secs()
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return GsettingsOutcome::Failed(error.to_string());
            }
        }
    }
}

/// Update one key in one INI group, preserving every other line. The file is
/// created when missing, and only rewritten when the value actually changes.
fn update_ini_file(path: &Path, group: &str, key: &str, value: &str) -> io::Result<bool> {
    let original = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let updated = update_ini_text(&original, group, key, value);
    if updated == original {
        return Ok(false);
    }
    write_atomically(path, &updated)?;
    Ok(true)
}

/// The pure INI edit: replace `key` in `[group]`, add it to an existing group,
/// or append the group. Comments, ordering, and unrelated keys are untouched.
fn update_ini_text(contents: &str, group: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = contents.lines().map(str::to_owned).collect();
    let header = format!("[{group}]");
    let new_line = format!("{key}={value}");

    let mut in_group = false;
    let mut group_found = false;
    let mut key_line = None;
    let mut group_end = None;

    for (index, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            if in_group {
                group_end = Some(index);
                break;
            }
            in_group = line == header;
            group_found |= in_group;
            continue;
        }
        if in_group {
            if let Some((name, _)) = line.split_once('=') {
                if name.trim() == key {
                    key_line = Some(index);
                    break;
                }
            }
        }
    }

    if let Some(index) = key_line {
        lines[index] = new_line;
    } else if group_found {
        // Insert before any blank separator that trails the group, matching
        // how hand-written INI files keep a blank line between sections.
        let mut at = group_end.unwrap_or(lines.len());
        while at > 0 && lines[at - 1].trim().is_empty() {
            at -= 1;
        }
        lines.insert(at, new_line);
    } else {
        if !lines.is_empty() && !lines.last().is_some_and(|line| line.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(header);
        lines.push(new_line);
    }

    let mut updated = lines.join("\n");
    if !updated.is_empty() {
        updated.push('\n');
    }
    updated
}

/// A GVariant string literal for the GSettings keyfile format.
fn gvariant_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('\'');
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\'' => escaped.push_str("\\'"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped.push('\'');
    escaped
}

/// Replace `path` atomically. A symlinked file (a dotfile-manager setup) is
/// written through the link instead of replacing it.
fn write_atomically(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return fs::write(path, contents);
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings");
    let temporary = path.with_file_name(format!(".{file_name}.tmp-{}", std::process::id()));
    fs::write(&temporary, contents)?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "dragonfruit-toolkit-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create the scratch directory");
        path
    }

    #[test]
    fn the_layout_puts_all_three_controls_on_the_left() {
        assert_eq!(BUTTON_LAYOUT, "close,minimize,maximize:");
        assert!(BUTTON_LAYOUT.ends_with(':'), "the colon ends the left side");
    }

    #[test]
    fn an_existing_key_is_replaced_and_unrelated_lines_survive() {
        let original = "[Settings]\n# keep me\ngtk-theme-name=Breeze\ngtk-decoration-layout=icon:minimize,maximize,close\ngtk-font-name=Noto Sans\n";
        let updated = update_ini_text(
            original,
            GTK_SETTINGS_GROUP,
            GTK_SETTINGS_KEY,
            BUTTON_LAYOUT,
        );
        assert_eq!(
            updated,
            "[Settings]\n# keep me\ngtk-theme-name=Breeze\ngtk-decoration-layout=close,minimize,maximize:\ngtk-font-name=Noto Sans\n"
        );
    }

    #[test]
    fn a_missing_key_is_added_to_the_existing_group() {
        let original = "[Settings]\ngtk-theme-name=Breeze\n\n[Other]\nkey=value\n";
        let updated = update_ini_text(
            original,
            GTK_SETTINGS_GROUP,
            GTK_SETTINGS_KEY,
            BUTTON_LAYOUT,
        );
        assert_eq!(
            updated,
            "[Settings]\ngtk-theme-name=Breeze\ngtk-decoration-layout=close,minimize,maximize:\n\n[Other]\nkey=value\n"
        );
    }

    #[test]
    fn a_missing_group_is_appended() {
        let updated = update_ini_text(
            "[Other]\nkey=value\n",
            GTK_SETTINGS_GROUP,
            GTK_SETTINGS_KEY,
            BUTTON_LAYOUT,
        );
        assert_eq!(
            updated,
            "[Other]\nkey=value\n\n[Settings]\ngtk-decoration-layout=close,minimize,maximize:\n"
        );
    }

    #[test]
    fn a_repeat_write_is_idempotent() {
        let original = update_ini_text("", GTK_SETTINGS_GROUP, GTK_SETTINGS_KEY, BUTTON_LAYOUT);
        let again = update_ini_text(
            &original,
            GTK_SETTINGS_GROUP,
            GTK_SETTINGS_KEY,
            BUTTON_LAYOUT,
        );
        assert_eq!(again, original);
    }

    #[test]
    fn the_keyfile_value_is_a_quoted_gvariant_string() {
        assert_eq!(gvariant_string(BUTTON_LAYOUT), "'close,minimize,maximize:'");
        assert_eq!(gvariant_string("a'b\\c"), "'a\\'b\\\\c'");
    }

    #[test]
    fn sync_writes_all_three_sources_and_preserves_other_keys() {
        let home = scratch("sync");
        let gtk3 = home.join("gtk-3.0").join("settings.ini");
        let gtk4 = home.join("gtk-4.0").join("settings.ini");
        let keyfile = home.join("glib-2.0").join("settings").join("keyfile");

        fs::create_dir_all(gtk3.parent().unwrap()).expect("gtk3 dir");
        fs::write(&gtk3, "[Settings]\ngtk-theme-name=Breeze\ngtk-decoration-layout=icon:minimize,maximize,close\n")
            .expect("seed gtk3");
        fs::create_dir_all(keyfile.parent().unwrap()).expect("keyfile dir");
        fs::write(
            &keyfile,
            format!(
                "[{}]\nbutton-layout='icon:minimize,maximize,close'\n",
                gsettings_group()
            ),
        )
        .expect("seed keyfile");

        let written = sync_window_controls(&home).expect("sync succeeds");
        assert_eq!(written, vec![gtk3.clone(), gtk4.clone(), keyfile.clone()]);

        let gtk3_text = fs::read_to_string(&gtk3).expect("read gtk3");
        assert!(gtk3_text.contains("gtk-theme-name=Breeze"), "{gtk3_text}");
        assert!(
            gtk3_text.contains("gtk-decoration-layout=close,minimize,maximize:"),
            "{gtk3_text}"
        );

        let gtk4_text = fs::read_to_string(&gtk4).expect("read gtk4");
        assert!(
            gtk4_text.contains("[Settings]\ngtk-decoration-layout=close,minimize,maximize:"),
            "{gtk4_text}"
        );

        let keyfile_text = fs::read_to_string(&keyfile).expect("read keyfile");
        assert!(
            keyfile_text.contains(&format!("[{}]", gsettings_group())),
            "{keyfile_text}"
        );
        assert!(
            keyfile_text.contains("button-layout='close,minimize,maximize:'"),
            "{keyfile_text}"
        );

        // A second sync has nothing left to write.
        assert!(sync_window_controls(&home).expect("second sync").is_empty());

        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn sync_creates_missing_files() {
        let home = scratch("create");
        let written = sync_window_controls(&home).expect("sync succeeds");
        assert_eq!(written.len(), 3);
        for path in written {
            assert!(path.is_file(), "{} exists", path.display());
        }
        fs::remove_dir_all(&home).ok();
    }
}
