// SPDX-License-Identifier: MIT
//! Launch-token trust model for the private shell protocols (T-07).
//!
//! Chrome surfaces, window/workspace control, and output management are a
//! privilege of a fixed set of trusted session processes, not a public
//! extension surface ([02-compositor.md](../../.docs/design/02-compositor.md)).
//! Each trusted process is provisioned with a **one-time launch token
//! out-of-band at startup** (via the session environment, T-24): the shell,
//! and — when desktop icons ship — the Files desktop surface (T-19).
//!
//! Properties enforced here:
//!
//! * **One-time.** A token is consumed by the first successful handshake;
//!   replaying it is refused (FR-5).
//! * **Per-boot.** Every token is bound to the compositor boot that minted
//!   it; a token captured in a previous session is refused (FR-5).
//! * **Versioned.** The handshake compares lockstep versions and refuses
//!   mismatched sets (FR-6).
//! * **Role-scoped.** A token grants the role it was minted for, so the
//!   desktop-icons surface can be admitted without the shell's full set.
//!
//! The model is deliberately pure (no Wayland types) so the refusal matrix
//! is unit-testable without a live compositor.

use std::collections::HashMap;

/// Environment variable the session manager uses to hand a process its
/// launch token (T-24). The compositor also writes the shell's token to
/// `$XDG_RUNTIME_DIR/<socket>.launch-token` for the dev tool.
pub const TOKEN_ENV: &str = "DRAGONFRUIT_LAUNCH_TOKEN";

/// Comma-separated list of pre-minted tokens, used by the session manager
/// and the conformance tests to provision more than one trusted process.
pub const TOKENS_ENV: &str = "DRAGONFRUIT_LAUNCH_TOKENS";

/// Suffix of the per-session token hand-off file.
pub const TOKEN_FILE_SUFFIX: &str = ".launch-token";

/// How many random bytes a launch token carries.
pub const TOKEN_BYTES: usize = 32;

/// A trusted session process role. Tokens are minted per role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrustedRole {
    /// The Qt Quick shell: menu bar, Dock, Control Center, OSD, overview.
    Shell,
    /// The Files desktop-icon surface (T-19, later).
    DesktopIcons,
}

impl TrustedRole {
    /// The role a token is for, as a stable label for logs.
    pub const fn name(self) -> &'static str {
        match self {
            TrustedRole::Shell => "shell",
            TrustedRole::DesktopIcons => "desktop-icons",
        }
    }
}

/// Why a handshake was refused. The numeric codes are part of the
/// `df_core.refused` event contract and are additive-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// No token was presented.
    NotProvisioned,
    /// The token is unknown, or was minted by a different boot.
    InvalidToken,
    /// The one-time token was already redeemed.
    TokenConsumed,
    /// The peer's lockstep set does not match ours.
    VersionMismatch,
    /// This client already authenticated.
    AlreadyAuthenticated,
}

impl Refusal {
    /// The stable wire code (see `dragonfruit-core.xml`).
    pub const fn code(self) -> u32 {
        match self {
            Refusal::InvalidToken => 0,
            Refusal::TokenConsumed => 1,
            Refusal::VersionMismatch => 2,
            Refusal::NotProvisioned => 3,
            Refusal::AlreadyAuthenticated => 4,
        }
    }

    /// A short label for the audit log.
    pub const fn name(self) -> &'static str {
        match self {
            Refusal::InvalidToken => "invalid-token",
            Refusal::TokenConsumed => "token-consumed",
            Refusal::VersionMismatch => "version-mismatch",
            Refusal::NotProvisioned => "not-provisioned",
            Refusal::AlreadyAuthenticated => "already-authenticated",
        }
    }

    /// The human-readable detail sent in `refused` and written to the log.
    pub const fn message(self) -> &'static str {
        match self {
            Refusal::InvalidToken => "launch token is unknown or was minted by a previous boot",
            Refusal::TokenConsumed => "launch token was already redeemed",
            Refusal::VersionMismatch => {
                "lockstep version mismatch; compositor, shell, and protocols must ship as one set"
            }
            Refusal::NotProvisioned => "no launch token was provisioned for this process",
            Refusal::AlreadyAuthenticated => "this client already authenticated",
        }
    }
}

/// A one-time launch token. The value is redacted from `Debug` output so a
/// token can never leak through a log line.
#[derive(Clone, PartialEq, Eq)]
pub struct LaunchToken {
    value: [u8; TOKEN_BYTES],
    role: TrustedRole,
}

impl std::fmt::Debug for LaunchToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LaunchToken")
            .field("role", &self.role)
            .field("value", &"<redacted>")
            .finish()
    }
}

impl LaunchToken {
    /// The role this token admits.
    pub const fn role(&self) -> TrustedRole {
        self.role
    }

    /// The raw bytes (for writing the hand-off file). Never log these.
    pub fn as_bytes(&self) -> &[u8; TOKEN_BYTES] {
        &self.value
    }

    /// Lowercase hex encoding, the form passed through the environment.
    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(TOKEN_BYTES * 2);
        for byte in self.value {
            out.push_str(&format!("{byte:02x}"));
        }
        out
    }

    /// Parse a hex token (the form found in `DRAGONFRUIT_LAUNCH_TOKEN`).
    pub fn parse_hex(hex: &str) -> Option<[u8; TOKEN_BYTES]> {
        if hex.len() != TOKEN_BYTES * 2 {
            return None;
        }
        let mut value = [0u8; TOKEN_BYTES];
        for (index, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
            let text = std::str::from_utf8(chunk).ok()?;
            value[index] = u8::from_str_radix(text, 16).ok()?;
        }
        Some(value)
    }
}

/// A minted token's record.
#[derive(Debug, Clone, Copy)]
struct TokenRecord {
    role: TrustedRole,
    boot_id: u64,
    consumed: bool,
}

/// The compositor's token store. One per session ("boot"); a new store
/// invalidates every token from a previous one.
#[derive(Debug)]
pub struct TrustModel {
    boot_id: u64,
    tokens: HashMap<[u8; TOKEN_BYTES], TokenRecord>,
    authenticated: usize,
}

impl Default for TrustModel {
    fn default() -> Self {
        TrustModel::new()
    }
}

impl TrustModel {
    /// A fresh store with a new boot id; all previous tokens are invalid.
    pub fn new() -> Self {
        TrustModel::with_boot_id(random_u64())
    }

    /// A store with an explicit boot id (tests, deterministic replay).
    pub fn with_boot_id(boot_id: u64) -> Self {
        TrustModel {
            boot_id,
            tokens: HashMap::new(),
            authenticated: 0,
        }
    }

    /// This store's boot id.
    pub const fn boot_id(&self) -> u64 {
        self.boot_id
    }

    /// Mint a fresh one-time token for `role`.
    pub fn mint(&mut self, role: TrustedRole) -> LaunchToken {
        let value = random_bytes::<TOKEN_BYTES>();
        self.insert(value, role);
        LaunchToken { value, role }
    }

    /// Mint a token with a caller-chosen value (session provisioning and
    /// deterministic tests). The value must be unique; a collision would
    /// silently replace an outstanding token, so callers must not reuse one.
    pub fn mint_with_value(&mut self, role: TrustedRole, value: [u8; TOKEN_BYTES]) -> LaunchToken {
        self.insert(value, role);
        LaunchToken { value, role }
    }

    fn insert(&mut self, value: [u8; TOKEN_BYTES], role: TrustedRole) {
        self.tokens.insert(
            value,
            TokenRecord {
                role,
                boot_id: self.boot_id,
                consumed: false,
            },
        );
    }

    /// Redeem a token. On success the token is consumed and the granted
    /// role returned; every failure path returns a [`Refusal`] without
    /// consuming the token (except an already-consumed replay, which is
    /// itself the refusal).
    pub fn authenticate(
        &mut self,
        token: &[u8],
        lockstep_version: u32,
    ) -> Result<TrustedRole, Refusal> {
        // Version is checked before the token so a misconfigured shell does
        // not burn its one-time token.
        if lockstep_version != df_ipc::LOCKSTEP_VERSION {
            return Err(Refusal::VersionMismatch);
        }
        if token.is_empty() {
            return Err(Refusal::NotProvisioned);
        }
        let Ok(value) = <[u8; TOKEN_BYTES]>::try_from(token) else {
            return Err(Refusal::InvalidToken);
        };
        let Some(record) = self.tokens.get(&value) else {
            return Err(Refusal::InvalidToken);
        };
        // A token from a previous boot is not in this store at all, but the
        // boot check makes the property explicit and guards a reused store.
        if record.boot_id != self.boot_id {
            return Err(Refusal::InvalidToken);
        }
        if record.consumed {
            return Err(Refusal::TokenConsumed);
        }
        let role = record.role;
        if let Some(record) = self.tokens.get_mut(&value) {
            record.consumed = true;
        }
        self.authenticated += 1;
        Ok(role)
    }

    /// How many tokens have been successfully redeemed.
    pub const fn authenticated_count(&self) -> usize {
        self.authenticated
    }

    /// Number of outstanding (minted) tokens.
    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }
}

/// Random bytes from the OS. Falls back to a time/pid/address mix only if
/// `/dev/urandom` is unavailable, so the value is never silently zero.
fn random_bytes<const N: usize>() -> [u8; N] {
    use std::io::Read;
    let mut buf = [0u8; N];
    if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
        if file.read_exact(&mut buf).is_ok() {
            return buf;
        }
    }
    let mut state = random_u64() ^ 0x9e37_79b9_7f4a_7c15;
    for chunk in buf.chunks_mut(8) {
        state = splitmix64(state);
        let bytes = state.to_le_bytes();
        let len = chunk.len();
        chunk.copy_from_slice(&bytes[..len]);
    }
    buf
}

/// A random `u64` for the boot id.
fn random_u64() -> u64 {
    use std::io::Read;
    if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
        let mut buf = [0u8; 8];
        if file.read_exact(&mut buf).is_ok() {
            return u64::from_le_bytes(buf);
        }
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let pid = std::process::id() as u64;
    let address = &nanos as *const u64 as u64;
    splitmix64(nanos ^ pid.rotate_left(17) ^ address.rotate_left(31))
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_value(byte: u8) -> [u8; TOKEN_BYTES] {
        [byte; TOKEN_BYTES]
    }

    #[test]
    fn valid_token_authenticates_once() {
        let mut trust = TrustModel::new();
        let token = trust.mint_with_value(TrustedRole::Shell, token_value(1));
        assert_eq!(
            trust.authenticate(token.as_bytes(), df_ipc::LOCKSTEP_VERSION),
            Ok(TrustedRole::Shell)
        );
        assert_eq!(trust.authenticated_count(), 1);
    }

    #[test]
    fn replayed_token_is_refused() {
        let mut trust = TrustModel::new();
        let token = trust.mint_with_value(TrustedRole::Shell, token_value(2));
        trust
            .authenticate(token.as_bytes(), df_ipc::LOCKSTEP_VERSION)
            .unwrap();
        assert_eq!(
            trust.authenticate(token.as_bytes(), df_ipc::LOCKSTEP_VERSION),
            Err(Refusal::TokenConsumed)
        );
    }

    #[test]
    fn token_from_a_previous_boot_is_invalid() {
        // Simulate a reboot: a fresh store with a different boot id never
        // knows the previous boot's token.
        let mut previous = TrustModel::with_boot_id(1);
        let old = previous.mint_with_value(TrustedRole::Shell, token_value(3));
        let mut current = TrustModel::with_boot_id(2);
        assert_eq!(
            current.authenticate(old.as_bytes(), df_ipc::LOCKSTEP_VERSION),
            Err(Refusal::InvalidToken)
        );
    }

    #[test]
    fn unknown_token_is_invalid() {
        let mut trust = TrustModel::new();
        assert_eq!(
            trust.authenticate(&token_value(9), df_ipc::LOCKSTEP_VERSION),
            Err(Refusal::InvalidToken)
        );
    }

    #[test]
    fn empty_token_is_not_provisioned() {
        let mut trust = TrustModel::new();
        assert_eq!(
            trust.authenticate(&[], df_ipc::LOCKSTEP_VERSION),
            Err(Refusal::NotProvisioned)
        );
    }

    #[test]
    fn wrong_version_is_refused_without_consuming_the_token() {
        let mut trust = TrustModel::new();
        let token = trust.mint_with_value(TrustedRole::Shell, token_value(4));
        assert_eq!(
            trust.authenticate(token.as_bytes(), df_ipc::LOCKSTEP_VERSION + 1),
            Err(Refusal::VersionMismatch)
        );
        // The token is still good: a corrected retry succeeds.
        assert_eq!(
            trust.authenticate(token.as_bytes(), df_ipc::LOCKSTEP_VERSION),
            Ok(TrustedRole::Shell)
        );
    }

    #[test]
    fn roles_are_distinct() {
        let mut trust = TrustModel::new();
        let shell = trust.mint_with_value(TrustedRole::Shell, token_value(5));
        let icons = trust.mint_with_value(TrustedRole::DesktopIcons, token_value(6));
        assert_eq!(
            trust.authenticate(icons.as_bytes(), df_ipc::LOCKSTEP_VERSION),
            Ok(TrustedRole::DesktopIcons)
        );
        assert_eq!(
            trust.authenticate(shell.as_bytes(), df_ipc::LOCKSTEP_VERSION),
            Ok(TrustedRole::Shell)
        );
    }

    #[test]
    fn hex_round_trips() {
        let mut trust = TrustModel::new();
        let token = trust.mint(TrustedRole::Shell);
        let hex = token.to_hex();
        assert_eq!(hex.len(), TOKEN_BYTES * 2);
        assert_eq!(LaunchToken::parse_hex(&hex), Some(*token.as_bytes()));
        assert_eq!(LaunchToken::parse_hex("nope"), None);
    }

    #[test]
    fn debug_redacts_the_token() {
        let mut trust = TrustModel::new();
        let token = trust.mint(TrustedRole::Shell);
        let rendered = format!("{token:?}");
        assert!(!rendered.contains(&token.to_hex()));
        assert!(rendered.contains("redacted"));
    }

    #[test]
    fn minted_tokens_are_unique() {
        let mut trust = TrustModel::new();
        let a = trust.mint(TrustedRole::Shell);
        let b = trust.mint(TrustedRole::Shell);
        assert_ne!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn refusal_codes_are_stable() {
        assert_eq!(Refusal::InvalidToken.code(), 0);
        assert_eq!(Refusal::TokenConsumed.code(), 1);
        assert_eq!(Refusal::VersionMismatch.code(), 2);
        assert_eq!(Refusal::NotProvisioned.code(), 3);
        assert_eq!(Refusal::AlreadyAuthenticated.code(), 4);
    }
}
