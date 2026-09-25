// SPDX-License-Identifier: MIT
//! The session environment (T-12.1b).
//!
//! Every process the session starts inherits the same base environment:
//!
//! * `XDG_CURRENT_DESKTOP=dragonfruit` — the public desktop-name contract
//!   ([12-packaging.md](../../docs/design/12-packaging.md)).
//! * `XDG_SESSION_TYPE=wayland`.
//! * `WAYLAND_DISPLAY=<socket>` — the compositor's private socket, so clients
//!   and services all connect to this session.
//! * `DISPLAY=<:N>` — the Xwayland display, when one exists (Wayland-only
//!   sessions leave it unset).
//!
//! Trusted processes additionally get `DRAGONFRUIT_LAUNCH_TOKEN`:
//!
//! * the **compositor** is the token issuer: it pre-mints the token it finds
//!   in its environment instead of a random one, so the shell's token is the
//!   one the session chose;
//! * the **shell** presents the same token in the `df_core` handshake.
//!
//! The environment is data; [`SessionEnvironment::base`] and
//! [`SessionEnvironment::trusted`] return the exact pairs
//! [`crate::ServiceSpec::env`] carries, and [`crate::SessionPlan::with_environment`]
//! attaches them to a plan. The systemd user units in `services/session/units`
//! mirror the same contract (see
//! [11-session-and-dev-workflow.md](../../docs/design/11-session-and-dev-workflow.md)).
//! The contract is frozen in
//! [ADR 0065](../../docs/design/adr/0065-session-environment-and-units.md).

use std::path::Path;

/// `XDG_CURRENT_DESKTOP`; the value is `dragonfruit` (the public contract).
pub const CURRENT_DESKTOP: &str = "XDG_CURRENT_DESKTOP";
/// `XDG_SESSION_TYPE`; always `wayland`.
pub const SESSION_TYPE: &str = "XDG_SESSION_TYPE";
/// `WAYLAND_DISPLAY`; the compositor's private socket basename.
pub const WAYLAND_DISPLAY: &str = "WAYLAND_DISPLAY";
/// `DISPLAY`; the Xwayland display, when one exists.
pub const DISPLAY: &str = "DISPLAY";
/// `DRAGONFRUIT_LAUNCH_TOKEN`; the one-time private-protocol token.
pub const LAUNCH_TOKEN: &str = "DRAGONFRUIT_LAUNCH_TOKEN";

/// The session type every Dragonfruit process reports.
pub const WAYLAND_SESSION_TYPE: &str = "wayland";

/// The socket name the shipped systemd units use. A fixed, namespaced name
/// (never `wayland-0`) keeps the units and their `WAYLAND_DISPLAY` in lockstep.
pub const DEFAULT_SOCKET_NAME: &str = "dragonfruit-wayland";

/// How many random bytes a launch token carries; matches the compositor's
/// [`LaunchToken`](../../compositor/src/shell/trust.rs) width.
pub const TOKEN_BYTES: usize = 32;

/// The environment a session exports to its children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEnvironment {
    socket_name: String,
    x11_display: Option<String>,
    launch_token: Option<String>,
}

impl SessionEnvironment {
    /// A Wayland-only environment for `socket_name`, with no X11 display and
    /// no launch token yet.
    pub fn new(socket_name: impl Into<String>) -> Self {
        SessionEnvironment {
            socket_name: socket_name.into(),
            x11_display: None,
            launch_token: None,
        }
    }

    /// The environment for the socket name the shipped units use.
    pub fn default_socket() -> Self {
        SessionEnvironment::new(DEFAULT_SOCKET_NAME)
    }

    /// Attach an Xwayland `DISPLAY` (e.g. `:0`).
    pub fn with_x11_display(mut self, display: impl Into<String>) -> Self {
        self.x11_display = Some(display.into());
        self
    }

    /// Attach the session launch token (the compositor's and shell's shared
    /// one-time token). The caller mints it once with
    /// [`generate_launch_token`]; never log the value.
    pub fn with_launch_token(mut self, token: impl Into<String>) -> Self {
        self.launch_token = Some(token.into());
        self
    }

    /// Mint and attach a fresh launch token. This is the normal session-start
    /// path: the same value reaches the compositor and the shell.
    pub fn with_generated_token(self) -> Self {
        self.with_launch_token(generate_launch_token())
    }

    /// The compositor's private socket basename (the `WAYLAND_DISPLAY` value).
    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }

    /// The Xwayland `DISPLAY`, if any.
    pub fn x11_display(&self) -> Option<&str> {
        self.x11_display.as_deref()
    }

    /// The launch token, if any. Never log the returned value.
    pub fn launch_token(&self) -> Option<&str> {
        self.launch_token.as_deref()
    }

    /// The environment every session child gets, in a stable order.
    pub fn base(&self) -> Vec<(String, String)> {
        let mut env = vec![
            (
                CURRENT_DESKTOP.to_string(),
                df_ipc::DESKTOP_NAME.to_string(),
            ),
            (SESSION_TYPE.to_string(), WAYLAND_SESSION_TYPE.to_string()),
            (WAYLAND_DISPLAY.to_string(), self.socket_name.clone()),
        ];
        if let Some(display) = &self.x11_display {
            env.push((DISPLAY.to_string(), display.clone()));
        }
        env
    }

    /// The environment for a trusted process: [`SessionEnvironment::base`]
    /// plus `DRAGONFRUIT_LAUNCH_TOKEN` when a token is attached.
    pub fn trusted(&self) -> Vec<(String, String)> {
        let mut env = self.base();
        if let Some(token) = &self.launch_token {
            env.push((LAUNCH_TOKEN.to_string(), token.clone()));
        }
        env
    }

    /// Look up one variable in [`SessionEnvironment::base`].
    pub fn base_value(&self, key: &str) -> Option<String> {
        self.base()
            .into_iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// Read the compositor's Xwayland `DISPLAY` hand-off file, if present
    /// (`$XDG_RUNTIME_DIR/<socket>.x11-display`). The file is session state,
    /// written by the compositor and removed at teardown (T-06).
    pub fn x11_display_from_handoff(runtime_dir: &Path, socket_name: &str) -> Option<String> {
        let path = runtime_dir.join(format!("{socket_name}.x11-display"));
        let contents = std::fs::read_to_string(path).ok()?;
        let display = contents.trim();
        if display.is_empty() {
            None
        } else {
            Some(display.to_string())
        }
    }
}

/// A fresh random launch token, lowercase hex (64 characters). Read from the
/// OS; a time/pid fallback keeps the value non-empty if `/dev/urandom` is
/// unavailable, so a token is never silently the same on every boot.
pub fn generate_launch_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    fill_random(&mut bytes);
    let mut out = String::with_capacity(TOKEN_BYTES * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn fill_random(bytes: &mut [u8]) {
    use std::io::Read;
    if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
        if file.read_exact(bytes).is_ok() {
            return;
        }
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    let mut state = nanos ^ (std::process::id() as u64).rotate_left(17);
    for byte in bytes.iter_mut() {
        state = state
            .wrapping_add(0x9e37_79b9_7f4a_7c15)
            .rotate_left(13)
            .wrapping_mul(0xbf58_476d_1ce4_e5b9);
        *byte = (state >> 32) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_environment_sets_the_public_contract() {
        let env = SessionEnvironment::default_socket().with_x11_display(":0");
        assert_eq!(
            env.base_value(CURRENT_DESKTOP).as_deref(),
            Some("dragonfruit")
        );
        assert_eq!(env.base_value(SESSION_TYPE).as_deref(), Some("wayland"));
        assert_eq!(
            env.base_value(WAYLAND_DISPLAY).as_deref(),
            Some(DEFAULT_SOCKET_NAME)
        );
        assert_eq!(env.base_value(DISPLAY).as_deref(), Some(":0"));
        assert_eq!(env.base_value(LAUNCH_TOKEN), None, "not trusted by default");
    }

    #[test]
    fn a_wayland_only_session_leaves_display_unset() {
        let env = SessionEnvironment::default_socket();
        assert_eq!(env.base_value(DISPLAY), None);
    }

    #[test]
    fn trusted_environment_adds_the_token() {
        let env = SessionEnvironment::default_socket().with_launch_token("ab".repeat(32));
        assert_eq!(env.base_value(LAUNCH_TOKEN), None, "base stays untrusted");
        let trusted = env.trusted();
        assert!(trusted
            .iter()
            .any(|(key, value)| key == LAUNCH_TOKEN && value == &"ab".repeat(32)));
    }

    #[test]
    fn generated_tokens_are_64_hex_characters_and_unique() {
        let a = generate_launch_token();
        let b = generate_launch_token();
        assert_eq!(a.len(), TOKEN_BYTES * 2);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()), "{a}");
        assert_ne!(a, b, "a fresh token per call");
    }
}
