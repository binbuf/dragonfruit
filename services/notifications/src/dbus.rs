// SPDX-License-Identifier: MIT
//! The session-bus surface (T-11.1a).
//!
//! The service owns one well-known name, `org.freedesktop.Notifications`, and
//! serves two interfaces at the standard object path
//! `/org/freedesktop/Notifications`:
//!
//! * `org.freedesktop.Notifications` — the standard app-facing interface
//!   (`Notify`, `CloseNotification`, `GetCapabilities`, `GetServerInformation`,
//!   `NotificationClosed`, `ActionInvoked`). Keeping the standard name and
//!   path is what makes every existing Linux app's notifications work.
//! * `org.dragonfruit.Notifications1` — the shell-facing interface
//!   (`Banners`, `History`, `Dismiss`, `Expire`, `DoNotDisturb`,
//!   `SetDoNotDisturb`, `Changed`). The Qt shell reads the flat JSON views
//!   from here; it never scrapes the freedesktop interface.
//!
//! Both interfaces share one [`Queue`] behind a mutex, so a `Notify` on the
//! standard interface is exactly the banner the shell reads. A background
//! expiry thread closes banners at their deadline, event-driven: it sleeps
//! until the earliest deadline and is woken when a new one arrives, so an
//! idle service does no work.
//!
//! Actions are advertised in the capabilities and round-tripped: the shell
//! calls `Invoke(id, action_key)` when a banner action is clicked, and the
//! service emits the freedesktop `ActionInvoked(id, action_key)` to the app
//! that sent the `Notify`, then dismisses the banner (T-11.1b).

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use zbus::blocking::connection;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;

use crate::model::{now_ms, Action, CloseReason, NotifyRequest, Queue, Urgency};
use crate::view;

/// The standard freedesktop well-known name on the user session bus.
pub const DBUS_NAME: &str = "org.freedesktop.Notifications";
/// The standard object path (the freedesktop spec's own path).
pub const DBUS_PATH: &str = "/org/freedesktop/Notifications";
/// The shell-facing interface name served at [`DBUS_PATH`].
pub const SHELL_INTERFACE: &str = "org.dragonfruit.Notifications1";
/// The freedesktop interface name.
pub const FREEDESKTOP_INTERFACE: &str = "org.freedesktop.Notifications";

/// What the server implements today. `actions` is advertised because the
/// shell round-trips action invocation back to the originating app
/// (T-11.1b).
const CAPABILITIES: &[&str] = &["body", "body-markup", "icon-static", "actions"];

/// The wake handle for the expiry thread: a flag plus a condvar so a new
/// notification with an earlier deadline re-arms the sleep immediately.
pub type Wake = Arc<(Mutex<bool>, Condvar)>;

/// A fresh wake handle.
pub fn new_wake() -> Wake {
    Arc::new((Mutex::new(false), Condvar::new()))
}

/// Wake the expiry thread so it recomputes its sleep.
pub fn wake_expiry(wake: &Wake) {
    let (flag, condvar) = &**wake;
    if let Ok(mut woken) = flag.lock() {
        *woken = true;
        condvar.notify_all();
    }
}

/// The app-facing `org.freedesktop.Notifications` object.
#[derive(Clone)]
pub struct FreedesktopNotifications {
    queue: Arc<Mutex<Queue>>,
    wake: Wake,
}

/// The shell-facing `org.dragonfruit.Notifications1` object.
#[derive(Clone)]
pub struct ShellNotifications {
    queue: Arc<Mutex<Queue>>,
    wake: Wake,
}

impl FreedesktopNotifications {
    /// A new object over a fresh queue and its own wake handle.
    pub fn new() -> Self {
        Self::from_shared(Arc::new(Mutex::new(Queue::new())), new_wake())
    }

    /// A new object sharing an existing queue and wake handle.
    pub fn from_shared(queue: Arc<Mutex<Queue>>, wake: Wake) -> Self {
        FreedesktopNotifications { queue, wake }
    }

    /// The shared queue behind this object.
    pub fn queue(&self) -> &Arc<Mutex<Queue>> {
        &self.queue
    }
}

impl Default for FreedesktopNotifications {
    fn default() -> Self {
        Self::new()
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl FreedesktopNotifications {
    /// Show a notification. Returns its id. `replaces_id` reuses an existing
    /// notification's id; `expire_timeout` is milliseconds (`< 0` service
    /// default, `0` never). Hints carry the urgency and anything else the app
    /// supplied (unknown hints are tolerated and ignored).
    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> u32 {
        let urgency = hints
            .get("urgency")
            .and_then(|value| u8::try_from(value.clone()).ok())
            .map(Urgency::from_hint)
            .unwrap_or_default();
        let request = NotifyRequest {
            app_name: app_name.to_owned(),
            replaces_id,
            app_icon: app_icon.to_owned(),
            summary: summary.to_owned(),
            body: body.to_owned(),
            actions: pair_actions(actions),
            urgency,
            expire_timeout_ms: expire_timeout,
        };
        let id = lock(&self.queue).notify(request, now_ms());
        wake_expiry(&self.wake);
        emit_shell_changed(&emitter).await;
        id
    }

    /// Close a notification (reason 3, "closed by CloseNotification"). A
    /// no-op for an unknown id.
    async fn close_notification(
        &self,
        id: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        let closed = lock(&self.queue).close(id, CloseReason::Closed, now_ms());
        if closed.is_some() {
            wake_expiry(&self.wake);
            emit_closed(&emitter, id, CloseReason::Closed).await;
            emit_shell_changed(&emitter).await;
        }
    }

    /// The optional capabilities the server supports.
    fn get_capabilities(&self) -> Vec<String> {
        CAPABILITIES
            .iter()
            .map(|capability| (*capability).to_owned())
            .collect()
    }

    /// The server's name, vendor, version, and spec version.
    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "Dragonfruit".to_owned(),
            "Dragonfruit".to_owned(),
            env!("CARGO_PKG_VERSION").to_owned(),
            "1.2".to_owned(),
        )
    }

    /// A notification left the banner queue. The reason is the freedesktop
    /// code (`1` expired, `2` dismissed, `3` closed).
    #[zbus(signal)]
    async fn notification_closed(
        emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    /// An action was invoked by the user. The shell calls
    /// `org.dragonfruit.Notifications1.Invoke`, which emits this signal back
    /// to the app that sent the `Notify` (T-11.1b).
    #[zbus(signal)]
    async fn action_invoked(
        emitter: &SignalEmitter<'_>,
        id: u32,
        action_key: &str,
    ) -> zbus::Result<()>;
}

impl ShellNotifications {
    /// A new object over a fresh queue and its own wake handle.
    pub fn new() -> Self {
        Self::from_shared(Arc::new(Mutex::new(Queue::new())), new_wake())
    }

    /// A new object sharing an existing queue and wake handle.
    pub fn from_shared(queue: Arc<Mutex<Queue>>, wake: Wake) -> Self {
        ShellNotifications { queue, wake }
    }

    /// The shared queue behind this object.
    pub fn queue(&self) -> &Arc<Mutex<Queue>> {
        &self.queue
    }

    /// Close an active notification with `reason`, emit the freedesktop
    /// `NotificationClosed`, then the shell `Changed`. Returns whether it was
    /// active.
    async fn close_with(&self, id: u32, reason: CloseReason, emitter: &SignalEmitter<'_>) -> bool {
        let closed = lock(&self.queue).close(id, reason, now_ms());
        if closed.is_some() {
            wake_expiry(&self.wake);
            emit_closed(emitter, id, reason).await;
            emit_shell_changed(emitter).await;
            true
        } else {
            false
        }
    }
}

impl Default for ShellNotifications {
    fn default() -> Self {
        Self::new()
    }
}

#[interface(name = "org.dragonfruit.Notifications1")]
impl ShellNotifications {
    /// The active banners as a JSON array, oldest first.
    fn banners(&self) -> String {
        view::banners_json(&lock(&self.queue))
    }

    /// The recorded history as a JSON array, most recent first.
    fn history(&self) -> String {
        view::history_json(&lock(&self.queue))
    }

    /// The user dismissed a banner (reason 2).
    async fn dismiss(&self, id: u32, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> bool {
        self.close_with(id, CloseReason::Dismissed, &emitter).await
    }

    /// The banner's timeout elapsed (reason 1). The shell calls this when it
    /// visually removes an expired banner.
    async fn expire(&self, id: u32, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> bool {
        self.close_with(id, CloseReason::Expired, &emitter).await
    }

    /// The user invoked an action on an active banner: emit the freedesktop
    /// `ActionInvoked(id, action_key)` to the originating app, then dismiss
    /// the banner (reason 2), emitting `NotificationClosed` and `Changed`.
    /// Returns whether `id` named an active banner. The shell passes the
    /// literal `"default"` key when the user clicks the banner body and the
    /// app registered a default action.
    async fn invoke(
        &self,
        id: u32,
        action_key: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        if lock(&self.queue).banner(id).is_none() {
            return false;
        }
        emit_action_invoked(&emitter, id, action_key).await;
        self.close_with(id, CloseReason::Dismissed, &emitter).await;
        true
    }

    /// Whether Do Not Disturb suppresses banners.
    fn do_not_disturb(&self) -> bool {
        lock(&self.queue).do_not_disturb()
    }

    /// Set Do Not Disturb (history still records; T-11.2a owns the policy).
    async fn set_do_not_disturb(
        &self,
        enabled: bool,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        lock(&self.queue).set_do_not_disturb(enabled);
        emit_shell_changed(&emitter).await;
    }

    /// The banner queue or the history changed; re-read `Banners()` and
    /// `History()`. This is the only notification the shell gets; it never
    /// polls.
    #[zbus(signal)]
    async fn changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Split the freedesktop flattened `actions` array (`key, label, key, label`)
/// into typed actions, dropping an odd trailing key.
fn pair_actions(flat: Vec<String>) -> Vec<Action> {
    flat.chunks_exact(2)
        .map(|pair| Action::new(pair[0].clone(), pair[1].clone()))
        .collect()
}

/// Lock the shared queue, recovering from a poisoned mutex: a D-Bus method
/// may panic on a bad argument and the service must keep answering.
fn lock(queue: &Arc<Mutex<Queue>>) -> std::sync::MutexGuard<'_, Queue> {
    queue
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Emit the shell `Changed` signal at the served path.
async fn emit_shell_changed(emitter: &SignalEmitter<'_>) {
    let _ = emitter.emit(SHELL_INTERFACE, "Changed", &()).await;
}

/// Emit the freedesktop `NotificationClosed` signal at the served path.
async fn emit_closed(emitter: &SignalEmitter<'_>, id: u32, reason: CloseReason) {
    let _ = FreedesktopNotifications::notification_closed(emitter, id, reason.as_u32()).await;
}

/// Emit the freedesktop `ActionInvoked` signal at the served path.
async fn emit_action_invoked(emitter: &SignalEmitter<'_>, id: u32, action_key: &str) {
    let _ = FreedesktopNotifications::action_invoked(emitter, id, action_key).await;
}

/// Serve both interfaces on the session bus until the process is asked to
/// stop. Returns an error only when the bus or the name cannot be taken; a
/// session without a bus is reported and exited instead of blocking.
pub fn run() -> zbus::Result<()> {
    let queue = Arc::new(Mutex::new(Queue::new()));
    let wake = new_wake();
    let freedesktop = FreedesktopNotifications::from_shared(queue.clone(), wake.clone());
    let shell = ShellNotifications::from_shared(queue.clone(), wake.clone());

    let connection = connection::Builder::session()?
        .name(DBUS_NAME)?
        .serve_at(DBUS_PATH, freedesktop)?
        .build()?;
    // The second interface shares the freedesktop object path so both the
    // freedesktop `NotificationClosed` and the shell `Changed` signals land at
    // the path clients watch.
    connection.object_server().at(DBUS_PATH, shell)?;

    spawn_expiry(connection.clone(), queue.clone(), wake);

    // The blocking object server runs on its own executor; parking the main
    // thread keeps the process (and the name) alive without a poll loop.
    let _ = connection;
    loop {
        std::thread::park();
    }
}

/// Start the background expiry thread. It closes banners at their deadline
/// and emits the freedesktop and shell signals; it sleeps until the earliest
/// deadline (or a wake on a new notification), never polls.
pub fn spawn_expiry(connection: connection::Connection, queue: Arc<Mutex<Queue>>, wake: Wake) {
    let _ = std::thread::Builder::new()
        .name("dragonfruit-notifications-expiry".to_owned())
        .spawn(move || expiry_loop(connection, queue, wake));
}

fn expiry_loop(connection: connection::Connection, queue: Arc<Mutex<Queue>>, wake: Wake) {
    loop {
        let now = now_ms();
        let expired = lock(&queue).expire_due(now);
        for id in expired {
            let reason = CloseReason::Expired.as_u32();
            let _ = connection.emit_signal(
                None::<&str>,
                DBUS_PATH,
                FREEDESKTOP_INTERFACE,
                "NotificationClosed",
                &(id, reason),
            );
            let _ =
                connection.emit_signal(None::<&str>, DBUS_PATH, SHELL_INTERFACE, "Changed", &());
        }

        let wait = match lock(&queue).next_deadline_ms() {
            Some(deadline) => Duration::from_millis(deadline.saturating_sub(now_ms()).max(1)),
            None => Duration::from_secs(3600),
        };

        let (flag, condvar) = &*wake;
        let mut woken = flag.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if *woken {
            *woken = false;
            continue;
        }
        let (mut woken, _) = condvar
            .wait_timeout(woken, wait)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *woken {
            *woken = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_freedesktop_name_is_the_standard_one() {
        assert_eq!(DBUS_NAME, "org.freedesktop.Notifications");
        assert_eq!(DBUS_PATH, "/org/freedesktop/Notifications");
        assert!(df_ipc::is_valid_dbus_name(SHELL_INTERFACE));
        assert!(SHELL_INTERFACE.ends_with("Notifications1"));
    }

    #[test]
    fn actions_pair_up_and_an_odd_key_is_dropped() {
        let actions = pair_actions(vec![
            "open".to_owned(),
            "Open".to_owned(),
            "reply".to_owned(),
            "Reply".to_owned(),
            "dangling".to_owned(),
        ]);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], Action::new("open", "Open"));
        assert_eq!(actions[1], Action::new("reply", "Reply"));
    }
}
