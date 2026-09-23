// SPDX-License-Identifier: MIT
//! `make demo` harness (T-01.6a): one command that builds, launches the
//! nested session with the shell and two clients (a Qt/Wayland app and an
//! X11 app), prints the demo checklist, and tears down deterministically.
//!
//! The same code path runs headless for CI: with no host Wayland session
//! (or an explicit `--headless`) it launches the compositor on the headless
//! backend, settles, asserts every child is still alive, tears down, and
//! asserts no socket leaked. That is the "scripted half" of the track demo
//! (docs/design/tracks/01-loop-v0-window-controls.md).

use std::path::PathBuf;

/// How long the scripted half lets the shell and clients map before it
/// checks that they are alive. Long enough for a cold Qt/Xwayland start on
/// CI, short enough not to matter.
pub const SCRIPTED_SETTLE: std::time::Duration = std::time::Duration::from_millis(3000);

/// Resolve the Qt/Wayland demo app: `DF_DEMO_QT_APP` if set, else the built
/// Settings placeholder, else Files. `None` when the Qt side is not built.
///
/// Paths are relative to the repository root, matching the Makefile's
/// `BUILD_DIR` default; run the harness from the repo root (`make demo`).
pub fn qt_app() -> Option<PathBuf> {
    env_app("DF_DEMO_QT_APP").or_else(|| {
        [
            "build/apps/settings/dragonfruit-settings",
            "build/apps/files/dragonfruit-files",
        ]
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
    })
}

/// Resolve the X11 demo app: `DF_DEMO_X11_APP` if set, else `xmessage` on
/// `PATH`. `None` when neither exists, in which case the X11 half is skipped
/// with a warning (the session is still valid; a Wayland-only demo is a
/// normal state).
pub fn x11_app() -> Option<PathBuf> {
    env_app("DF_DEMO_X11_APP").or_else(|| which("xmessage"))
}

/// The Qt QML import root the first-party apps need. The shell bakes this in
/// at compile time (`DF_QML_IMPORT_DIR`), but the apps do not, so the demo
/// exports `QML_IMPORT_PATH` for them. `DF_QML_IMPORT_PATH` overrides the
/// default `build/qml`.
pub fn qml_import_path() -> PathBuf {
    std::env::var_os("DF_QML_IMPORT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("build/qml"))
}

fn env_app(key: &str) -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os(key)?);
    path.is_file().then_some(path)
}

fn which(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

/// The printed checklist of steps the human performs at track sign-off. It
/// is printed in both modes (the acceptance is that `make demo` prints it);
/// `scripted` only changes the framing line.
pub fn checklist(
    socket_name: &str,
    qt: Option<&PathBuf>,
    x11: Option<&PathBuf>,
    scripted: bool,
) -> String {
    let mode = if scripted {
        "scripted half (headless — CI path)"
    } else {
        "nested — do the steps in the Dragonfruit window"
    };
    let qt_line = match qt {
        Some(path) => format!("Qt/Wayland app: {}", path.display()),
        None => "Qt/Wayland app: NOT FOUND — run `make build`".to_string(),
    };
    let x11_line = match x11 {
        Some(path) => format!("X11 app:        {}", path.display()),
        None => "X11 app:        not found (install xmessage or set DF_DEMO_X11_APP)".to_string(),
    };
    format!(
        "\
────────────────────────────────────────────────────────────────────────
 Dragonfruit — T-01 Loop v0 demo ({mode})
 socket {socket_name}
 {qt_line}
 {x11_line}

 Do this in the Dragonfruit window:
   [ ] 1. The menu bar (top) and Dock (bottom) are visible.
   [ ] 2. Click the Settings app in the Dock (or use the window already open).
   [ ] 3. The window appears with our titlebar and traffic lights.
   [ ] 4. Drag the window by its titlebar.
   [ ] 5. Double-click the titlebar to zoom; double-click again to unzoom.
   [ ] 6. Click the yellow light to minimize; restore it from the Dock.
   [ ] 7. Click the red light to close the window.
   [ ] 8. Right-click (or Control-click) the titlebar for the window menu;
          try Move to Space, Minimize, Zoom, and Close.
   [ ] 9. Repeat steps 4–8 on the X11 (xmessage) window.
   [ ]10. Close the Dragonfruit window to end the session.

 Ctrl-C quits; teardown is automatic and leaves the host session untouched.
────────────────────────────────────────────────────────────────────────
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checklist_names_every_scripted_step_and_menu_command() {
        let text = checklist("df-demo-1", None, None, false);
        for needle in [
            "traffic lights",
            "Drag",
            "Double-click",
            "yellow light",
            "red light",
            "Move to Space",
            "Minimize",
            "Zoom",
            "Close",
            "Close the Dragonfruit window",
        ] {
            assert!(
                text.contains(needle),
                "checklist is missing {needle:?}:\n{text}"
            );
        }
        assert!(text.contains("df-demo-1"));
        assert!(text.contains("nested"));
    }

    #[test]
    fn scripted_checklist_is_framed_as_the_ci_path() {
        let text = checklist("df-demo-1", None, None, true);
        assert!(text.contains("scripted half"), "{text}");
    }

    #[test]
    fn which_finds_a_real_binary_and_rejects_a_missing_one() {
        assert!(which("sh").is_some());
        assert!(which("dragonfruit-definitely-not-a-real-binary").is_none());
    }
}
