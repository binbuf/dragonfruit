// SPDX-License-Identifier: MIT
#![allow(dead_code)] // Forward-looking input API consumed by T-05/T-07/T-11/T-16/T-22/T-27.

//! The compositor-owned global shortcut engine (T-03).
//!
//! There is exactly one arbiter of key bindings. System shortcuts are
//! owned here; application accelerators are admitted only while their
//! owning window is focused (the menu-broker feeds them in T-22), and
//! sandboxed applications register through the portal's GlobalShortcuts
//! interface (T-27). **No client grabs keys directly** — the seat's key
//! filter intercepts every binding before it reaches a client, and the
//! protocols that would let a client install a raw grab are not advertised
//! ([02-compositor.md](../../docs/design/02-compositor.md)).
//!
//! Conflict resolution ([06-global-menu.md](../../docs/design/06-global-menu.md)):
//! system shortcuts take precedence over application accelerators, and the
//! focused window's menu wins among application accelerators.

use std::collections::HashSet;

use smithay::input::keyboard::xkb::keysyms;
use smithay::input::keyboard::ModifiersState;

use super::action::InputAction;
use super::keymap::{role_held, KeysymValue, ModifierRole, RoleMods};

/// A compositor-owned system shortcut binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
    pub mods: RoleMods,
    pub key: KeysymValue,
    pub action: InputAction,
}

impl Shortcut {
    pub const fn new(mods: RoleMods, key: KeysymValue, action: InputAction) -> Self {
        Shortcut { mods, key, action }
    }
}

/// An application accelerator admitted while `app_id` is focused.
///
/// The menu-broker (T-22) registers these for the focused window; the
/// portal (T-27) registers the same shape for sandboxed callers. The
/// compositor never lets a client install a grab — it only matches the
/// registered chord in its own filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppAccelerator {
    /// The owning application (`app_id` / `WM_CLASS` until T-23).
    pub app_id: String,
    pub mods: RoleMods,
    pub key: KeysymValue,
    /// Opaque accelerator id echoed back to the owner when triggered.
    pub accelerator_id: String,
}

/// What a keystroke resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortcutOutcome {
    /// A compositor-owned system action.
    System(InputAction),
    /// An accelerator owned by the focused application.
    Application {
        app_id: String,
        accelerator_id: String,
    },
}

/// A refused grab attempt, kept for the audit log (FR-5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrabRefusal<K = u32> {
    /// The client that tried to grab.
    pub client: K,
    pub kind: GrabKind,
    /// Why it was refused.
    pub reason: &'static str,
}

/// The kind of grab a client attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrabKind {
    /// A raw keyboard grab (shortcut-inhibit-style).
    RawKeyboard,
    /// A pointer grab.
    Pointer,
    /// A popup grab.
    Popup,
}

/// The single gate every grab request must pass through.
///
/// The compositor sanctions a small set of trusted session clients (the
/// shell and the Files desktop surface) with one-time launch tokens
/// ([02-compositor.md](../../docs/design/02-compositor.md)); everyone else
/// is logged and refused.
///
/// The client key is generic so the live compositor can key on Smithay's
/// `ClientId` (which cannot be constructed in a unit test) while the model
/// tests use plain integers.
#[derive(Debug)]
pub struct GrabArbiter<K = u32> {
    sanctioned: HashSet<K>,
    refusals: Vec<GrabRefusal<K>>,
}

impl<K> Default for GrabArbiter<K> {
    fn default() -> Self {
        GrabArbiter {
            sanctioned: HashSet::new(),
            refusals: Vec::new(),
        }
    }
}

impl<K: Eq + std::hash::Hash + Clone + std::fmt::Debug> GrabArbiter<K> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a client as a sanctioned session client (shell token, T-07).
    pub fn sanction(&mut self, client: K) {
        self.sanctioned.insert(client);
    }

    pub fn is_sanctioned(&self, client: &K) -> bool {
        self.sanctioned.contains(client)
    }

    /// Request a grab. Sanctioned clients are admitted; everyone else is
    /// refused and the refusal recorded. Never panics, never forwards.
    pub fn request(&mut self, client: K, kind: GrabKind) -> Result<(), GrabRefusal<K>> {
        if self.sanctioned.contains(&client) {
            return Ok(());
        }
        let refusal = GrabRefusal {
            client,
            kind,
            reason: "client is not a sanctioned session client",
        };
        eprintln!(
            "dragonfruit-compositor: refused {kind:?} grab from client {:?} (not sanctioned)",
            refusal.client
        );
        self.refusals.push(refusal.clone());
        Err(refusal)
    }

    /// All refusals so far, oldest first.
    pub fn refusals(&self) -> &[GrabRefusal<K>] {
        &self.refusals
    }
}

/// The global shortcut engine.
#[derive(Debug)]
pub struct ShortcutEngine {
    system: Vec<Shortcut>,
    app: Vec<AppAccelerator>,
    focused_app: Option<String>,
    /// Keys whose press was intercepted, so their release is swallowed too
    /// (a half-delivered shortcut would leave a stuck key in the client).
    active: HashSet<KeysymValue>,
}

impl Default for ShortcutEngine {
    fn default() -> Self {
        ShortcutEngine::with_bindings(default_system_bindings())
    }
}

impl ShortcutEngine {
    /// Build an engine with an explicit system binding table.
    pub fn with_bindings(system: Vec<Shortcut>) -> Self {
        ShortcutEngine {
            system,
            app: Vec::new(),
            focused_app: None,
            active: HashSet::new(),
        }
    }

    /// The system bindings, for tests and the Settings pane.
    pub fn system_bindings(&self) -> &[Shortcut] {
        &self.system
    }

    /// Replace the system binding table live (Settings Keyboard pane,
    /// T-16). Application accelerators and active keys are preserved.
    pub fn set_system_bindings(&mut self, bindings: Vec<Shortcut>) {
        self.system = bindings;
    }

    /// The focused application, as reported by focus tracking. Application
    /// accelerators only match this app.
    pub fn set_focused_app(&mut self, app: Option<String>) {
        self.focused_app = app;
    }

    pub fn focused_app(&self) -> Option<&str> {
        self.focused_app.as_deref()
    }

    /// Admit an application accelerator (menu-broker, T-22). Replaces any
    /// previous binding for the same app + chord.
    pub fn register_app_accelerator(&mut self, accelerator: AppAccelerator) {
        self.app.retain(|existing| {
            !(existing.app_id == accelerator.app_id
                && existing.mods == accelerator.mods
                && existing.key == accelerator.key)
        });
        self.app.push(accelerator);
    }

    /// Register a sandboxed application's shortcut (portal, T-27). Same
    /// admission rules as the menu-broker path.
    pub fn register_portal_shortcut(&mut self, accelerator: AppAccelerator) {
        self.register_app_accelerator(accelerator);
    }

    /// Drop all accelerators owned by an application (window closed / menu
    /// changed).
    pub fn clear_app_accelerators(&mut self, app_id: &str) {
        self.app.retain(|existing| existing.app_id != app_id);
    }

    /// Resolve a keystroke against system shortcuts first, then the
    /// focused application's accelerators.
    pub fn resolve(&self, mods: &ModifiersState, key: KeysymValue) -> Option<ShortcutOutcome> {
        let state = RoleMods::from_state(mods);
        let key = fold_ascii_letter(key);

        for shortcut in &self.system {
            if shortcut.mods == state && shortcut.key == key {
                return Some(ShortcutOutcome::System(shortcut.action));
            }
        }

        let focused = self.focused_app.as_deref()?;
        self.app
            .iter()
            .find(|accelerator| {
                accelerator.app_id == focused && accelerator.mods == state && accelerator.key == key
            })
            .map(|accelerator| ShortcutOutcome::Application {
                app_id: accelerator.app_id.clone(),
                accelerator_id: accelerator.accelerator_id.clone(),
            })
    }

    /// Record that a shortcut's key press was intercepted.
    pub fn note_press(&mut self, key: KeysymValue) {
        self.active.insert(key);
    }

    /// Whether a key press was intercepted and its release should be too.
    pub fn is_active(&self, key: KeysymValue) -> bool {
        self.active.contains(&key)
    }

    /// Release a previously intercepted key.
    pub fn note_release(&mut self, key: KeysymValue) {
        self.active.remove(&key);
    }

    /// Number of admitted application accelerators (diagnostics/tests).
    pub fn app_accelerator_count(&self) -> usize {
        self.app.len()
    }
}

/// Fold an ASCII lowercase letter keysym to uppercase.
///
/// The input filter resolves a key's base (level-0) symbol, which xkb reports
/// lowercase for letter keys, while the binding table uses the `KEY_A`-style
/// uppercase constants. Shortcuts never distinguish case (Shift is a modifier
/// role), so fold the queried key. Non-letters are unchanged.
fn fold_ascii_letter(key: KeysymValue) -> KeysymValue {
    if (0x61..=0x7a).contains(&key) {
        key - 0x20
    } else {
        key
    }
}

/// The default system shortcut table.
///
/// The exact chords are defaults; the Settings Keyboard pane (T-16)
/// rebinds them. What matters for T-03 is that they all flow through this
/// one engine and the same dispatch path.
pub fn default_system_bindings() -> Vec<Shortcut> {
    let (command, control, shift) = (RoleMods::COMMAND, RoleMods::CONTROL, RoleMods::SHIFT);
    let mut bindings = vec![
        Shortcut::new(control, keysyms::KEY_Left, InputAction::WorkspacePrev),
        Shortcut::new(control, keysyms::KEY_Right, InputAction::WorkspaceNext),
        Shortcut::new(control, keysyms::KEY_Up, InputAction::MissionControl),
        Shortcut::new(control, keysyms::KEY_Down, InputAction::DesktopReveal),
        Shortcut::new(command, keysyms::KEY_Tab, InputAction::AppSwitcher),
        // Cmd+Shift+Tab is the same switcher action in the reverse direction;
        // the input path derives the direction from the held Shift role.
        Shortcut::new(
            command.union(shift),
            keysyms::KEY_Tab,
            InputAction::AppSwitcher,
        ),
        Shortcut::new(
            command.union(shift),
            keysyms::KEY_3,
            InputAction::Screenshot,
        ),
        Shortcut::new(
            command.union(shift),
            keysyms::KEY_4,
            InputAction::Screenshot,
        ),
        Shortcut::new(
            command.union(control),
            keysyms::KEY_Q,
            InputAction::LockScreen,
        ),
        Shortcut::new(
            control.union(shift),
            keysyms::KEY_N,
            InputAction::NotificationCenter,
        ),
        // T-10 section 20: move keyboard focus into the Dock and toggle its
        // auto-hide. The design's "Fn-Control-F3" resolves to Control-F3 on
        // Linux (the Fn layer is a hardware key); Super+Option+D matches the
        // macOS auto-hide toggle.
        Shortcut::new(control, keysyms::KEY_F3, InputAction::FocusDock),
        Shortcut::new(
            command.union(RoleMods::OPTION),
            keysyms::KEY_D,
            InputAction::ToggleDock,
        ),
    ];
    for (i, key) in [
        keysyms::KEY_1,
        keysyms::KEY_2,
        keysyms::KEY_3,
        keysyms::KEY_4,
        keysyms::KEY_5,
        keysyms::KEY_6,
        keysyms::KEY_7,
        keysyms::KEY_8,
        keysyms::KEY_9,
    ]
    .into_iter()
    .enumerate()
    {
        bindings.push(Shortcut::new(
            control,
            key,
            InputAction::WorkspaceActivate(i),
        ));
    }
    bindings
}

/// Whether a role is held — a thin alias so callers can stay in the
/// shortcut vocabulary.
pub fn mod_held(role: ModifierRole, mods: &ModifiersState) -> bool {
    role_held(role, mods)
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::input::keyboard::xkb::keysyms;

    fn mods(command: bool, option: bool, control: bool, shift: bool) -> ModifiersState {
        ModifiersState {
            logo: command,
            alt: option,
            ctrl: control,
            shift,
            ..Default::default()
        }
    }

    #[test]
    fn system_shortcuts_resolve_with_the_canonical_mapping() {
        let engine = ShortcutEngine::default();
        assert_eq!(
            engine.resolve(&mods(false, false, true, false), keysyms::KEY_Right),
            Some(ShortcutOutcome::System(InputAction::WorkspaceNext))
        );
        assert_eq!(
            engine.resolve(&mods(true, false, false, false), keysyms::KEY_Tab),
            Some(ShortcutOutcome::System(InputAction::AppSwitcher))
        );
        assert_eq!(
            engine.resolve(&mods(false, false, true, false), keysyms::KEY_3),
            Some(ShortcutOutcome::System(InputAction::WorkspaceActivate(2)))
        );
    }

    #[test]
    fn caps_lock_does_not_change_resolution() {
        let engine = ShortcutEngine::default();
        let mut m = mods(false, false, true, false);
        m.caps_lock = true;
        m.num_lock = true;
        assert_eq!(
            engine.resolve(&m, keysyms::KEY_Right),
            Some(ShortcutOutcome::System(InputAction::WorkspaceNext))
        );
    }

    #[test]
    fn letter_bindings_match_the_base_lowercase_symbol() {
        // The input filter resolves a key's base (level-0) symbol, which is
        // lowercase for letter keys; the binding table uses `KEY_Q`-style
        // uppercase constants. Letter chords must still resolve.
        let engine = ShortcutEngine::default();
        assert_eq!(
            engine.resolve(&mods(true, false, true, false), keysyms::KEY_q),
            Some(ShortcutOutcome::System(InputAction::LockScreen))
        );
        assert_eq!(
            engine.resolve(&mods(true, true, false, false), keysyms::KEY_d),
            Some(ShortcutOutcome::System(InputAction::ToggleDock))
        );
        assert_eq!(
            engine.resolve(&mods(false, false, true, false), keysyms::KEY_F3),
            Some(ShortcutOutcome::System(InputAction::FocusDock))
        );
    }

    #[test]
    fn system_wins_over_application_accelerators() {
        let mut engine = ShortcutEngine::default();
        engine.set_focused_app(Some("org.example.App".into()));
        engine.register_app_accelerator(AppAccelerator {
            app_id: "org.example.App".into(),
            mods: RoleMods::CONTROL,
            key: keysyms::KEY_Right,
            accelerator_id: "app.next".into(),
        });
        // The focused app binds Ctrl+Right, but the system shortcut wins.
        assert_eq!(
            engine.resolve(&mods(false, false, true, false), keysyms::KEY_Right),
            Some(ShortcutOutcome::System(InputAction::WorkspaceNext))
        );
    }

    #[test]
    fn only_the_focused_app_accelerator_matches() {
        let mut engine = ShortcutEngine::default();
        engine.set_focused_app(Some("app.one".into()));
        engine.register_app_accelerator(AppAccelerator {
            app_id: "app.one".into(),
            mods: RoleMods::COMMAND,
            key: keysyms::KEY_Q,
            accelerator_id: "one.quit".into(),
        });
        engine.register_app_accelerator(AppAccelerator {
            app_id: "app.two".into(),
            mods: RoleMods::COMMAND,
            key: keysyms::KEY_Q,
            accelerator_id: "two.quit".into(),
        });
        assert_eq!(
            engine.resolve(&mods(true, false, false, false), keysyms::KEY_Q),
            Some(ShortcutOutcome::Application {
                app_id: "app.one".into(),
                accelerator_id: "one.quit".into(),
            })
        );
        engine.set_focused_app(Some("app.two".into()));
        assert_eq!(
            engine.resolve(&mods(true, false, false, false), keysyms::KEY_Q),
            Some(ShortcutOutcome::Application {
                app_id: "app.two".into(),
                accelerator_id: "two.quit".into(),
            })
        );
        engine.set_focused_app(None);
        assert_eq!(
            engine.resolve(&mods(true, false, false, false), keysyms::KEY_Q),
            None
        );
    }

    #[test]
    fn unsanctioned_grabs_are_refused_and_logged() {
        let mut arbiter = GrabArbiter::new();
        let err = arbiter
            .request(42, GrabKind::RawKeyboard)
            .expect_err("unsanctioned client must be refused");
        assert_eq!(err.client, 42);
        assert_eq!(arbiter.refusals().len(), 1);

        arbiter.sanction(7);
        assert!(arbiter.request(7, GrabKind::Popup).is_ok());
        assert_eq!(arbiter.refusals().len(), 1);
    }
}
