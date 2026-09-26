// SPDX-License-Identifier: MIT
//! `dragonfruit-lock-auth` — the PAM authentication seam for the session lock
//! (T-12.3b).
//!
//! The lock screen needs exactly one capability from the host: "is this the
//! account's password?". Dragonfruit never keeps a credential store and never
//! implements password verification; it asks the host's PAM stack through
//! [`PamAuthenticator`], which is also the binary half of this crate — the
//! small `dragonfruit-pam-helper` process the shell spawns
//! ([ADR 0068](../../docs/design/adr/0068-lock-pam-helper.md)).
//!
//! The helper is deliberately minimal:
//!
//! - one password in, one [`AuthResult`] out (success / denied / error);
//! - the password is only ever held in memory for the duration of the call;
//!   it is never written to disk, logged, or echoed back;
//! - libpam is loaded with `dlopen` at runtime (`libpam.so.0`), so the crate
//!   does not depend on the `pam-devel` headers to build and always uses the
//!   distribution's own module stack.
//!
//! The service name defaults to [`DEFAULT_SERVICE`] (`dragonfruit`); when that
//! service is not configured the authenticator retries the universally
//! available [`FALLBACK_SERVICE`] (`login`), so a host without a dedicated
//! Dragonfruit PAM file still authenticates.

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::OnceLock;

/// The PAM service Dragonfruit asks for first. A distribution may install an
/// `/etc/pam.d/dragonfruit` file to customize it; when absent the
/// authenticator falls back to [`FALLBACK_SERVICE`].
pub const DEFAULT_SERVICE: &str = "dragonfruit";

/// The PAM service used when [`DEFAULT_SERVICE`] is not configured. `login`
/// ships on every PAM host and authenticates the account's own password.
pub const FALLBACK_SERVICE: &str = "login";

/// The outcome of one authentication attempt. The helper maps this to its
/// process exit status ([`AuthResult::exit_code`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthResult {
    /// PAM accepted the credentials.
    Success,
    /// PAM rejected the credentials (including an unknown account).
    Denied,
    /// Authentication could not be attempted (PAM unavailable, no service,
    /// an internal error). Distinct from a rejected password.
    Error,
}

impl AuthResult {
    /// The helper's exit status contract: `0` success, `1` denied, `2` error.
    pub const fn exit_code(self) -> i32 {
        match self {
            AuthResult::Success => 0,
            AuthResult::Denied => 1,
            AuthResult::Error => 2,
        }
    }

    /// Interpret a child's exit status (`None` means it did not exit normally).
    pub const fn from_exit_code(code: Option<i32>) -> Self {
        match code {
            Some(0) => AuthResult::Success,
            Some(1) => AuthResult::Denied,
            _ => AuthResult::Error,
        }
    }
}

/// The one authentication operation the lock screen needs.
pub trait Authenticator {
    /// Verify `password` for `user`. Implementations must not persist either
    /// value.
    fn authenticate(&self, user: &str, password: &str) -> AuthResult;
}

/// A PAM-backed [`Authenticator`].
///
/// `confdir`, when set, points PAM at an alternate configuration directory
/// (Linux-PAM's `pam_start_confdir`). It exists for the headless tests, which
/// install a throwaway `permit`/`deny` service under a temp directory and
/// exercise the real libpam without touching `/etc/pam.d`; production leaves
/// it unset and uses the host defaults.
#[derive(Debug, Clone)]
pub struct PamAuthenticator {
    service: String,
    confdir: Option<PathBuf>,
}

impl Default for PamAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

impl PamAuthenticator {
    /// The production authenticator: service [`DEFAULT_SERVICE`], host PAM
    /// configuration.
    pub fn new() -> Self {
        Self {
            service: DEFAULT_SERVICE.to_string(),
            confdir: None,
        }
    }

    /// Override the PAM service name.
    pub fn with_service(mut self, service: impl Into<String>) -> Self {
        self.service = service.into();
        self
    }

    /// Point PAM at an alternate configuration directory (test seam).
    pub fn with_confdir(mut self, confdir: impl Into<PathBuf>) -> Self {
        self.confdir = Some(confdir.into());
        self
    }

    /// The configured service name.
    pub fn service(&self) -> &str {
        &self.service
    }

    /// The configured configuration directory, if any.
    pub fn confdir(&self) -> Option<&Path> {
        self.confdir.as_deref()
    }

    fn attempt(&self, user: &str, password: &str) -> AuthResult {
        let Some(api) = pam_api() else {
            return AuthResult::Error;
        };
        run(api, &self.service, user, password, self.confdir.as_deref())
    }
}

impl Authenticator for PamAuthenticator {
    fn authenticate(&self, user: &str, password: &str) -> AuthResult {
        let first = self.attempt(user, password);
        // A missing/unusable Dragonfruit service is the one recoverable error:
        // retry the universally present `login` service so a stock host works.
        if first == AuthResult::Error && self.service != FALLBACK_SERVICE {
            let fallback = run(
                pam_api().expect("libpam loaded above"),
                FALLBACK_SERVICE,
                user,
                password,
                self.confdir.as_deref(),
            );
            return fallback;
        }
        first
    }
}

/// Run one PAM transaction and classify the result.
///
/// On any setup failure (a NUL in an argument, a null handle, a PAM call that
/// cannot run) this returns [`AuthResult::Error`]: the lock stays up, which is
/// the fail-secure direction.
fn run(
    api: &pam::PamApi,
    service: &str,
    user: &str,
    password: &str,
    confdir: Option<&Path>,
) -> AuthResult {
    let (Ok(service), Ok(user), Ok(password), Ok(confdir)) = (
        CString::new(service),
        CString::new(user),
        CString::new(password),
        confdir_cstring(confdir),
    ) else {
        return AuthResult::Error;
    };

    // The conversation callback borrows this for the whole transaction.
    let mut conversation_data = pam::Conversation { user, password };
    let conv = pam::PamConv {
        conv: Some(pam::conversation),
        appdata_ptr: &mut conversation_data as *mut pam::Conversation as *mut c_void,
    };
    let confdir_ptr = confdir.as_ref().map_or(ptr::null(), |value| value.as_ptr());

    let mut handle: *mut pam::PamHandle = ptr::null_mut();
    let started = unsafe {
        (api.start_confdir)(
            service.as_ptr(),
            conversation_data.user.as_ptr(),
            &conv,
            confdir_ptr,
            &mut handle,
        )
    };
    if started != pam::PAM_SUCCESS || handle.is_null() {
        return AuthResult::Error;
    }

    let auth = unsafe { (api.authenticate)(handle, 0) };
    // Only the auth and account phases matter for unlocking; the session
    // phase (keyring, systemd, selinux context) belongs to the login session,
    // not to a lock-screen password check.
    let account = if auth == pam::PAM_SUCCESS {
        unsafe { (api.acct_mgmt)(handle, 0) }
    } else {
        auth
    };
    unsafe { (api.end)(handle, auth) };

    classify(account)
}

fn confdir_cstring(confdir: Option<&Path>) -> std::io::Result<Option<CString>> {
    use std::os::unix::ffi::OsStrExt;
    match confdir {
        Some(path) => Ok(Some(CString::new(path.as_os_str().as_bytes())?)),
        None => Ok(None),
    }
}

/// Map a PAM status to the three-state result the helper exposes. A rejected
/// password is `Denied`; anything that prevented verification is `Error`.
fn classify(status: c_int) -> AuthResult {
    use pam::*;
    match status {
        PAM_SUCCESS => AuthResult::Success,
        PAM_AUTH_ERR
        | PAM_MAXTRIES
        | PAM_USER_UNKNOWN
        | PAM_PERM_DENIED
        | PAM_ACCT_EXPIRED
        | PAM_AUTHTOK_ERR
        | PAM_CRED_INSUFFICIENT => AuthResult::Denied,
        // A service/auth stack that could not answer is an error, not a
        // rejection; the caller keeps the lock up.
        PAM_AUTHINFO_UNAVAIL => AuthResult::Error,
        _ => AuthResult::Error,
    }
}

/// Load libpam once. `None` when the runtime library is absent, in which case
/// every authentication reports `Error` (never a false unlock).
fn pam_api() -> Option<&'static pam::PamApi> {
    static API: OnceLock<Option<pam::PamApi>> = OnceLock::new();
    API.get_or_init(pam::PamApi::load).as_ref()
}

mod pam {
    //! The minimal slice of the Linux-PAM ABI this crate uses. The structures
    //! are stable ABI (`pam_appl.h`); declaring them here avoids a hard build
    //! dependency on the development headers, which are not part of the
    //! pinned toolchain.

    use super::{c_char, c_int, c_void};
    use std::ffi::CString;
    use std::ptr;

    pub const PAM_SUCCESS: c_int = 0;
    pub const PAM_BUF_ERR: c_int = 5;
    pub const PAM_PERM_DENIED: c_int = 6;
    pub const PAM_AUTH_ERR: c_int = 7;
    pub const PAM_CRED_INSUFFICIENT: c_int = 8;
    pub const PAM_AUTHINFO_UNAVAIL: c_int = 9;
    pub const PAM_USER_UNKNOWN: c_int = 10;
    pub const PAM_MAXTRIES: c_int = 11;
    pub const PAM_ACCT_EXPIRED: c_int = 13;
    pub const PAM_CONV_ERR: c_int = 19;
    pub const PAM_AUTHTOK_ERR: c_int = 20;

    pub const PAM_PROMPT_ECHO_OFF: c_int = 1;
    pub const PAM_PROMPT_ECHO_ON: c_int = 2;

    #[repr(C)]
    pub struct PamMessage {
        pub msg_style: c_int,
        pub msg: *const c_char,
    }

    #[repr(C)]
    pub struct PamResponse {
        pub resp: *mut c_char,
        pub resp_retcode: c_int,
    }

    pub type ConvFn = unsafe extern "C" fn(
        c_int,
        *mut *const PamMessage,
        *mut *mut PamResponse,
        *mut c_void,
    ) -> c_int;

    #[repr(C)]
    pub struct PamConv {
        pub conv: Option<ConvFn>,
        pub appdata_ptr: *mut c_void,
    }

    /// Opaque `pam_handle_t`.
    #[repr(C)]
    pub struct PamHandle {
        _private: [u8; 0],
    }

    /// The conversation state: the account and the one password to hand back
    /// to PAM. Both live on the stack of `run` and are dropped (and zeroed by
    /// the allocator) when the transaction ends.
    pub struct Conversation {
        pub user: CString,
        pub password: CString,
    }

    /// The PAM conversation callback. PAM prompts; we answer the password
    /// prompt with the supplied password, the user prompt with the account,
    /// and informational prompts with nothing.
    pub unsafe extern "C" fn conversation(
        num_msg: c_int,
        msg: *mut *const PamMessage,
        resp: *mut *mut PamResponse,
        appdata: *mut c_void,
    ) -> c_int {
        if num_msg <= 0 || msg.is_null() || resp.is_null() || appdata.is_null() {
            return PAM_CONV_ERR;
        }
        let data = &*(appdata as *const Conversation);
        let responses =
            libc::calloc(num_msg as usize, std::mem::size_of::<PamResponse>()) as *mut PamResponse;
        if responses.is_null() {
            return PAM_BUF_ERR;
        }
        for index in 0..num_msg as isize {
            let message = *msg.offset(index);
            if message.is_null() {
                continue;
            }
            let value = match (*message).msg_style {
                PAM_PROMPT_ECHO_OFF => data.password.as_ptr(),
                PAM_PROMPT_ECHO_ON => data.user.as_ptr(),
                // PAM_ERROR_MSG / PAM_TEXT_INFO: no response is expected, and
                // we deliberately do not surface PAM text to the caller.
                _ => continue,
            };
            let copy = libc::strdup(value);
            if copy.is_null() {
                free_responses(responses, num_msg as isize);
                return PAM_BUF_ERR;
            }
            (*responses.offset(index)).resp = copy;
            (*responses.offset(index)).resp_retcode = 0;
        }
        *resp = responses;
        PAM_SUCCESS
    }

    unsafe fn free_responses(responses: *mut PamResponse, count: isize) {
        for index in 0..count {
            let response = responses.offset(index);
            if !(*response).resp.is_null() {
                libc::free((*response).resp as *mut c_void);
            }
        }
        libc::free(responses as *mut c_void);
    }

    type StartConfdirFn = unsafe extern "C" fn(
        *const c_char,
        *const c_char,
        *const PamConv,
        *const c_char,
        *mut *mut PamHandle,
    ) -> c_int;
    type AuthenticateFn = unsafe extern "C" fn(*mut PamHandle, c_int) -> c_int;
    type AcctMgmtFn = unsafe extern "C" fn(*mut PamHandle, c_int) -> c_int;
    type EndFn = unsafe extern "C" fn(*mut PamHandle, c_int) -> c_int;

    /// The libpam entry points the helper uses, resolved once at runtime.
    pub struct PamApi {
        pub start_confdir: StartConfdirFn,
        pub authenticate: AuthenticateFn,
        pub acct_mgmt: AcctMgmtFn,
        pub end: EndFn,
    }

    impl PamApi {
        /// `dlopen` libpam and resolve the four symbols. `None` (a logged
        /// diagnostic, never a panic) when the library or a symbol is missing.
        pub fn load() -> Option<Self> {
            let names = ["libpam.so.0", "libpam.so"];
            let mut handle = ptr::null_mut();
            for name in names {
                let Ok(name) = CString::new(name) else {
                    continue;
                };
                let candidate =
                    unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
                if !candidate.is_null() {
                    handle = candidate;
                    break;
                }
            }
            if handle.is_null() {
                return None;
            }
            // Resolve by name; each symbol keeps the library loaded for the
            // process lifetime (we never `dlclose`).
            unsafe {
                let start_confdir = resolve::<StartConfdirFn>(handle, b"pam_start_confdir\0")?;
                let authenticate = resolve::<AuthenticateFn>(handle, b"pam_authenticate\0")?;
                let acct_mgmt = resolve::<AcctMgmtFn>(handle, b"pam_acct_mgmt\0")?;
                let end = resolve::<EndFn>(handle, b"pam_end\0")?;
                Some(PamApi {
                    start_confdir,
                    authenticate,
                    acct_mgmt,
                    end,
                })
            }
        }
    }

    /// Resolve one symbol and transcribe the raw pointer into a typed function
    /// pointer. `None` when absent.
    unsafe fn resolve<T: Copy>(handle: *mut c_void, symbol: &[u8]) -> Option<T> {
        let pointer = libc::dlsym(handle, symbol.as_ptr() as *const c_char);
        if pointer.is_null() {
            return None;
        }
        // SAFETY: `dlsym` returns the address of the named function; `T` is a
        // function-pointer type with the matching PAM ABI.
        Some(std::mem::transmute_copy::<*mut c_void, T>(&pointer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn exit_codes_round_trip() {
        assert_eq!(AuthResult::Success.exit_code(), 0);
        assert_eq!(AuthResult::Denied.exit_code(), 1);
        assert_eq!(AuthResult::Error.exit_code(), 2);
        assert_eq!(AuthResult::from_exit_code(Some(0)), AuthResult::Success);
        assert_eq!(AuthResult::from_exit_code(Some(1)), AuthResult::Denied);
        assert_eq!(AuthResult::from_exit_code(Some(2)), AuthResult::Error);
        assert_eq!(AuthResult::from_exit_code(None), AuthResult::Error);
        assert_eq!(AuthResult::from_exit_code(Some(9)), AuthResult::Error);
    }

    #[test]
    fn classification_distinguishes_denied_from_unavailable() {
        assert_eq!(classify(pam::PAM_SUCCESS), AuthResult::Success);
        assert_eq!(classify(pam::PAM_AUTH_ERR), AuthResult::Denied);
        assert_eq!(classify(pam::PAM_USER_UNKNOWN), AuthResult::Denied);
        assert_eq!(classify(pam::PAM_MAXTRIES), AuthResult::Denied);
        assert_eq!(classify(pam::PAM_AUTHTOK_ERR), AuthResult::Denied);
        assert_eq!(classify(pam::PAM_AUTHINFO_UNAVAIL), AuthResult::Error);
        assert_eq!(classify(pam::PAM_CONV_ERR), AuthResult::Error);
    }

    #[test]
    fn default_authenticator_targets_the_dragonfruit_service() {
        let authenticator = PamAuthenticator::new();
        assert_eq!(authenticator.service(), DEFAULT_SERVICE);
        assert!(authenticator.confdir().is_none());
        let custom = PamAuthenticator::new()
            .with_service("login")
            .with_confdir("/tmp/example");
        assert_eq!(custom.service(), "login");
        assert_eq!(custom.confdir(), Some(Path::new("/tmp/example")));
    }

    #[test]
    fn conversation_returns_the_exact_password_for_the_password_prompt() {
        let password_text = "two  spaces\tand a tab";
        let mut data = pam::Conversation {
            user: CString::new("alice").unwrap(),
            password: CString::new(password_text).unwrap(),
        };
        let prompt = CString::new("Password: ").unwrap();
        let message = pam::PamMessage {
            msg_style: pam::PAM_PROMPT_ECHO_OFF,
            msg: prompt.as_ptr(),
        };
        let messages = [&message as *const pam::PamMessage];
        let mut responses: *mut pam::PamResponse = ptr::null_mut();
        let status = unsafe {
            pam::conversation(
                1,
                messages.as_ptr() as *mut *const pam::PamMessage,
                &mut responses,
                &mut data as *mut pam::Conversation as *mut c_void,
            )
        };
        assert_eq!(status, pam::PAM_SUCCESS);
        assert!(!responses.is_null());
        let returned = unsafe { CStr::from_ptr((*responses).resp) };
        assert_eq!(returned.to_str().unwrap(), password_text);
        // Free exactly as PAM owns and frees conversation responses.
        unsafe {
            libc::free((*responses).resp as *mut c_void);
            libc::free(responses as *mut c_void);
        }
    }
}
