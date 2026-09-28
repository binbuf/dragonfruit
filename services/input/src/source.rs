// SPDX-License-Identifier: MIT
//! The transport seam: the raw libinput read and its mock.
//!
//! An [`InputSource`] is the only thing that talks to the host input stack.
//! The live source is [`crate::CommandLibinput`] (the `libinput` tool that
//! ships with libinput); tests and CI use [`MockInput`], which serves a
//! fixture with no hardware on the machine. The adapter
//! ([`crate::InputAdapter`]) turns one raw read into the typed
//! [`InputSnapshot`](crate::InputSnapshot) and drives the shared
//! `Subscription`.
//!
//! The adapter is **read-only**: libinput has no persisted configuration and
//! no setter CLI — the compositor applies keyboard/pointer settings live over
//! its own API (see docs/design/07-system-integration.md, principle 3), and
//! `settingsd` is the durable owner of the user's choices. This adapter is the
//! *inventory*: which keyboard, mouse, and trackpad the session has, what they
//! can do, and libinput's built-in defaults. Nothing above the adapter sees
//! libinput.

use dragonfruit_system_adapters::AdapterError;

/// The raw result of one libinput read: the devices the host stack recognizes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InputData {
    /// Every device `libinput` recognizes, in the order the tool reports them.
    pub devices: Vec<InputDeviceData>,
}

/// One libinput device, flattened from one `Device:` record.
///
/// The fields mirror what `libinput list-devices` prints: identity and
/// capabilities, plus the **built-in defaults** of the configurable features
/// (an option is `None` when the tool prints `n/a`, meaning the device does not
/// expose it). The values are libinput's defaults, not the desktop's applied
/// configuration; the user's own choices are `settingsd`'s.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InputDeviceData {
    /// `Device`: the human name (`AT Translated Set 2 keyboard`, …).
    pub name: String,
    /// `Kernel`: the event node (`/dev/input/event3`).
    pub kernel: String,
    /// `Id`: the tool's identity string (`usb:046d:c077`, `serial:…`).
    pub identity: String,
    /// `Group`: the physical-group id libinput assigns.
    pub group: u32,
    /// `Seat`: the seat line (`seat0, default`).
    pub seat: String,
    /// `Size` in millimeters, when the tool reports one.
    pub size_mm: Option<(f64, f64)>,
    /// `Capabilities`: the token list (`keyboard`, `pointer`, `touch`, …).
    pub capabilities: Vec<String>,
    /// `Tap-to-click` default; `None` when the device has no tapping.
    pub tap_to_click: Option<bool>,
    /// `Tap-and-drag` default (older tools call it only `Tap drag lock`).
    pub tap_and_drag: Option<bool>,
    /// `Tap drag lock` default.
    pub tap_drag_lock: Option<bool>,
    /// `Left-handed` default.
    pub left_handed: Option<bool>,
    /// `Nat.scrolling` default.
    pub natural_scroll: Option<bool>,
    /// `Middle emulation` default.
    pub middle_emulation: Option<bool>,
    /// `Disable-w-typing` default.
    pub disable_while_typing: Option<bool>,
    /// `Disable-w-trackpointing` default.
    pub disable_while_trackpointing: Option<bool>,
    /// `Accel profiles`: every available profile, default first preferred.
    pub accel_profiles: Vec<String>,
    /// The `*`-marked acceleration profile, when the device has any.
    pub default_accel_profile: Option<String>,
    /// `Scroll methods`: every available method.
    pub scroll_methods: Vec<String>,
    /// The `*`-marked scroll method, when the device has any.
    pub default_scroll_method: Option<String>,
    /// `Click methods`: every available method.
    pub click_methods: Vec<String>,
    /// The `*`-marked click method, when the device has any.
    pub default_click_method: Option<String>,
    /// `Rotation`: the supported rotation (`normal`, `n/a`, …).
    pub rotation: Option<String>,
}

/// Reads the host input stack.
///
/// The read result is a three-way answer, exactly as the adapter contract needs
/// it:
///
/// * `Ok(Some(data))` — libinput answered; `data` is the device inventory.
/// * `Ok(None)` — libinput is absent (not installed, or it cannot reach a
///   seat). A normal state; the item hides.
/// * `Err(error)` — libinput is present but the read failed; the item shows
///   visible and inert with the message.
///
/// A session that runs libinput but has **no devices** answers `Ok(Some)` with
/// an empty list; the snapshot's `present()` is then false, the second hide
/// rule beside the adapter-level `Unavailable`.
pub trait InputSource {
    /// One read of the host input stack.
    fn read(&mut self) -> Result<Option<InputData>, AdapterError>;
}

/// A fixture-backed source with a simulated host-stack lifecycle.
///
/// The mock is the CI path: it serves [`InputData`] with no hardware and no
/// `libinput` on the machine, and `kill`/`restart` exercise absence and
/// re-subscribe.
#[derive(Debug, Clone, PartialEq)]
pub struct MockInput {
    present: bool,
    data: Option<InputData>,
    failure: Option<AdapterError>,
    reads: u32,
}

impl MockInput {
    /// A host stack that is not available.
    pub fn absent() -> Self {
        MockInput {
            present: false,
            data: None,
            failure: None,
            reads: 0,
        }
    }

    /// A present host stack that answers with `data`.
    pub fn present(data: InputData) -> Self {
        MockInput {
            present: true,
            data: Some(data),
            failure: None,
            reads: 0,
        }
    }

    /// A present host stack that fails every read (e.g. it went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockInput {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            reads: 0,
        }
    }

    /// The host stack reports fresh devices.
    pub fn push(&mut self, data: InputData) {
        self.present = true;
        self.failure = None;
        self.data = Some(data);
    }

    /// The host stack becomes unavailable.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The host stack comes back (serving the last data, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated host stack is available.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }
}

impl InputSource for MockInput {
    fn read(&mut self) -> Result<Option<InputData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone().unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> InputData {
        InputData {
            devices: vec![InputDeviceData {
                name: "AT Translated Set 2 keyboard".to_owned(),
                kernel: "/dev/input/event3".to_owned(),
                capabilities: vec!["keyboard".to_owned()],
                ..InputDeviceData::default()
            }],
        }
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockInput::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockInput::present(data());
        assert_eq!(mock.read(), Ok(Some(data())));
        assert!(mock.is_present());
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockInput::failing("libinput: timeout");
        assert_eq!(mock.read().unwrap_err().message(), "libinput: timeout");
        assert!(mock.is_present());
    }

    #[test]
    fn kill_and_restart_flip_presence() {
        let mut mock = MockInput::present(data());
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(data()));
    }
}
