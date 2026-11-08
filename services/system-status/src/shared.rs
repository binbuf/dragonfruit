// SPDX-License-Identifier: MIT
//! One host-stack source shared by the adapters that project different
//! surfaces of it (T228).
//!
//! Privacy and Security are two views of one live host stack, and Users &
//! Groups, Biometrics, and Online Accounts are three views of the account
//! stack. The bridge host keeps one source behind an `Arc<Mutex<_>>` so the
//! adapters can each own a `SharedSource` handle without moving the source
//! twice. The mock keeps the same linked state, so a test that pushes a
//! fixture sees every view agree.

use std::sync::{Arc, Mutex};

use dragonfruit_account_adapter::{
    AccountOutcome, AccountSource, AccountType, AccountsData, BiometricsData, BiometricsOutcome,
    BiometricsSource, OnlineAccountsData, OnlineAccountsOutcome, OnlineAccountsSource,
};
use dragonfruit_privacy_adapter::{
    PrivacyData, PrivacyOutcome, PrivacySource, SecurityData, SecurityOutcome, SecuritySource,
};
use dragonfruit_system_adapters::AdapterError;

/// A cloneable handle to one host-stack source.
#[derive(Debug)]
pub struct SharedSource<S> {
    inner: Arc<Mutex<S>>,
}

impl<S> Clone for SharedSource<S> {
    fn clone(&self) -> Self {
        SharedSource {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<S> PartialEq for SharedSource<S> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<S> Eq for SharedSource<S> {}

impl<S> SharedSource<S> {
    /// Wrap `source` in a shared handle.
    pub fn new(source: S) -> Self {
        SharedSource {
            inner: Arc::new(Mutex::new(source)),
        }
    }

    /// Lock the shared source. Exposed so a host test can inspect the mock's
    /// counters through the shared handle.
    pub fn lock(&self) -> std::sync::MutexGuard<'_, S> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl<S: PrivacySource + Send> PrivacySource for SharedSource<S> {
    fn read(&mut self) -> Result<Option<PrivacyData>, AdapterError> {
        self.lock().read()
    }

    fn set_permission(
        &mut self,
        table: &str,
        id: &str,
        app: &str,
        permissions: &[String],
    ) -> PrivacyOutcome {
        self.lock().set_permission(table, id, app, permissions)
    }

    fn delete_permission(&mut self, table: &str, id: &str, app: &str) -> PrivacyOutcome {
        self.lock().delete_permission(table, id, app)
    }
}

impl<S: SecuritySource + Send> SecuritySource for SharedSource<S> {
    fn read_security(&mut self) -> Result<Option<SecurityData>, AdapterError> {
        self.lock().read_security()
    }

    fn set_source_enabled(&mut self, id: &str, enabled: bool) -> SecurityOutcome {
        self.lock().set_source_enabled(id, enabled)
    }
}

impl<S: AccountSource + Send> AccountSource for SharedSource<S> {
    fn read(&mut self) -> Result<Option<AccountsData>, AdapterError> {
        self.lock().read()
    }

    fn create_user(
        &mut self,
        user_name: &str,
        real_name: &str,
        account_type: AccountType,
    ) -> AccountOutcome {
        self.lock().create_user(user_name, real_name, account_type)
    }

    fn delete_user(&mut self, uid: u64) -> AccountOutcome {
        self.lock().delete_user(uid)
    }

    fn set_account_type(&mut self, uid: u64, account_type: AccountType) -> AccountOutcome {
        self.lock().set_account_type(uid, account_type)
    }

    fn set_locked(&mut self, uid: u64, locked: bool) -> AccountOutcome {
        self.lock().set_locked(uid, locked)
    }

    fn set_automatic_login(&mut self, uid: u64, automatic_login: bool) -> AccountOutcome {
        self.lock().set_automatic_login(uid, automatic_login)
    }

    fn create_group(&mut self, name: &str) -> AccountOutcome {
        self.lock().create_group(name)
    }

    fn delete_group(&mut self, name: &str) -> AccountOutcome {
        self.lock().delete_group(name)
    }

    fn set_group_members(&mut self, name: &str, members: &[String]) -> AccountOutcome {
        self.lock().set_group_members(name, members)
    }
}

impl<S: BiometricsSource + Send> BiometricsSource for SharedSource<S> {
    fn read_biometrics(&mut self) -> Result<Option<BiometricsData>, AdapterError> {
        self.lock().read_biometrics()
    }

    fn start_enrollment(&mut self, finger: &str) -> BiometricsOutcome {
        self.lock().start_enrollment(finger)
    }

    fn cancel_enrollment(&mut self) -> BiometricsOutcome {
        self.lock().cancel_enrollment()
    }

    fn delete_finger(&mut self, finger: &str) -> BiometricsOutcome {
        self.lock().delete_finger(finger)
    }

    fn change_password(&mut self, password: &str) -> BiometricsOutcome {
        self.lock().change_password(password)
    }
}

impl<S: OnlineAccountsSource + Send> OnlineAccountsSource for SharedSource<S> {
    fn read_online_accounts(&mut self) -> Result<Option<OnlineAccountsData>, AdapterError> {
        self.lock().read_online_accounts()
    }

    fn add_online_account(&mut self, provider: &str, identity: &str) -> OnlineAccountsOutcome {
        self.lock().add_online_account(provider, identity)
    }

    fn remove_online_account(&mut self, id: &str) -> OnlineAccountsOutcome {
        self.lock().remove_online_account(id)
    }

    fn set_online_account_sync(
        &mut self,
        id: &str,
        capability: &str,
        enabled: bool,
    ) -> OnlineAccountsOutcome {
        self.lock().set_online_account_sync(id, capability, enabled)
    }
}
