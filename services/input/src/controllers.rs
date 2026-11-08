// SPDX-License-Identifier: MIT
//! Game controllers: the udev inventory and the explicit test read (T229).
//!
//! The Settings Game Controllers pane is optional. It reads the kernel's
//! `/proc/bus/input/devices` inventory (the same data udev exposes) for
//! joystick/gamepad devices, and the session's SDL mapping table when one is
//! configured. Nothing above this module sees a device node.
//!
//! The test action is a **diagnostic read**, not a remapper: it opens the
//! controller's `js`/`event` node for one event with a short timeout and
//! reports what happened (an event, no event, a permission denial, or no
//! node). A remapper and a mapping editor are deliberately out of scope.
//!
//! Absence is layered like the input inventory: a host with no controller is
//! a live, empty list; a host with no `/proc/bus/input/devices` is still a
//! live empty list (the pane shows its empty state), never an error.

use std::path::Path;
use std::process::Command;

/// One controller the kernel recognizes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ControllerData {
    /// The kernel name (`Microsoft X-Box 360 pad`, …).
    pub name: String,
    /// The device node the test action opens (`/dev/input/js0` preferred).
    pub path: String,
    /// The bus id in upper-case hex (`0003`).
    pub bustype: String,
    /// The vendor id in upper-case hex (`045e`).
    pub vendor: String,
    /// The product id in upper-case hex (`028e`).
    pub product: String,
    /// The handler tokens (`js0`, `event12`, …).
    pub handlers: Vec<String>,
    /// The SDL mapping line when the session has one, else empty.
    pub mapping: String,
    /// The mapping source: `sdl` when a mapping matched, else `udev`.
    pub driver: String,
}

impl ControllerData {
    /// The stable id the pane and test action use: the device node.
    pub fn id(&self) -> &str {
        &self.path
    }

    /// A human identity line (`045e:028e`, empty when the kernel omitted it).
    pub fn identity(&self) -> String {
        if self.vendor.is_empty() && self.product.is_empty() {
            String::new()
        } else {
            format!("{}:{}", self.vendor, self.product)
        }
    }

    /// Whether an SDL mapping was found.
    pub fn has_mapping(&self) -> bool {
        !self.mapping.is_empty()
    }
}

/// The outcome of one controller test read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerTest {
    /// `event` (a byte arrived), `timeout` (no event), `denied` (permission),
    /// `absent` (no device node), or `failed`.
    pub outcome: String,
    /// The node the test opened.
    pub path: String,
    /// A human explanation.
    pub detail: String,
    /// The bytes read, in hex (`0000000000000000`), empty when none.
    pub bytes: String,
}

impl Default for ControllerTest {
    fn default() -> Self {
        ControllerTest {
            outcome: "absent".to_owned(),
            path: String::new(),
            detail: "No controller device node is available.".to_owned(),
            bytes: String::new(),
        }
    }
}

impl ControllerTest {
    /// A test that read an event.
    pub fn event(path: &str, bytes: &[u8]) -> Self {
        ControllerTest {
            outcome: "event".to_owned(),
            path: path.to_owned(),
            detail: format!("Read {} byte(s) from {path}.", bytes.len()),
            bytes: bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
        }
    }

    /// A test that timed out waiting for an event.
    pub fn timeout(path: &str) -> Self {
        ControllerTest {
            outcome: "timeout".to_owned(),
            path: path.to_owned(),
            detail: format!("No event arrived from {path} within 2 seconds."),
            bytes: String::new(),
        }
    }

    /// A permission denial.
    pub fn denied(path: &str) -> Self {
        ControllerTest {
            outcome: "denied".to_owned(),
            path: path.to_owned(),
            detail: format!("Permission denied opening {path}; add the user to the input group."),
            bytes: String::new(),
        }
    }

    /// A generic failure.
    pub fn failed(path: &str, detail: impl Into<String>) -> Self {
        ControllerTest {
            outcome: "failed".to_owned(),
            path: path.to_owned(),
            detail: detail.into(),
            bytes: String::new(),
        }
    }

    /// Whether the test read an event.
    pub fn is_event(&self) -> bool {
        self.outcome == "event"
    }
}

/// Parse `/proc/bus/input/devices` into the controller inventory.
///
/// A record is a controller when a handler token starts with `js`, or when its
/// name says gamepad/joystick/controller and it has an absolute axis and no
/// keyboard/mouse handler. Unknown lines are ignored, so a kernel that grows a
/// field keeps working.
pub fn controllers_from_proc(text: &str) -> Vec<ControllerData> {
    let mappings = sdl_mappings();
    let mut controllers = Vec::new();

    for block in text.split("\n\n") {
        let mut name = String::new();
        let mut bustype = String::new();
        let mut vendor = String::new();
        let mut product = String::new();
        let mut handlers: Vec<String> = Vec::new();
        let mut has_abs = false;
        for line in block.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("I:") {
                for token in rest.split_whitespace() {
                    if let Some(value) = token.strip_prefix("Bus=") {
                        bustype = value.to_ascii_uppercase();
                    } else if let Some(value) = token.strip_prefix("Vendor=") {
                        vendor = value.to_ascii_uppercase();
                    } else if let Some(value) = token.strip_prefix("Product=") {
                        product = value.to_ascii_uppercase();
                    }
                }
            } else if let Some(rest) = line.strip_prefix("N: Name=") {
                name = rest.trim_matches('"').to_owned();
            } else if let Some(rest) = line.strip_prefix("H: Handlers=") {
                handlers = rest.split_whitespace().map(str::to_owned).collect();
            } else if line.starts_with("B: ABS=") {
                has_abs = true;
            }
        }

        if name.is_empty() {
            continue;
        }
        let has_js = handlers.iter().any(|handler| handler.starts_with("js"));
        let lower = name.to_ascii_lowercase();
        let named = lower.contains("gamepad")
            || lower.contains("joystick")
            || lower.contains("controller")
            || lower.contains("x-box")
            || lower.contains("xbox");
        let has_kbd_or_mouse = handlers
            .iter()
            .any(|handler| handler == "kbd" || handler.starts_with("mouse"));
        if !has_js && !(named && has_abs && !has_kbd_or_mouse) {
            continue;
        }

        let path = handlers
            .iter()
            .find(|handler| handler.starts_with("js"))
            .map(|handler| format!("/dev/input/{handler}"))
            .or_else(|| {
                handlers
                    .iter()
                    .find(|handler| handler.starts_with("event"))
                    .map(|handler| format!("/dev/input/{handler}"))
            })
            .unwrap_or_default();
        let mapping = mappings
            .iter()
            .find(|line| {
                line.contains(&name)
                    || (!vendor.is_empty()
                        && !product.is_empty()
                        && line.contains(&format!("{vendor}{product}")))
            })
            .cloned()
            .unwrap_or_default();
        controllers.push(ControllerData {
            name,
            path,
            bustype,
            vendor,
            product,
            handlers,
            driver: if mapping.is_empty() {
                "udev".to_owned()
            } else {
                "sdl".to_owned()
            },
            mapping,
        });
    }
    controllers.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.path.cmp(&right.path))
    });
    controllers
}

/// The SDL controller mapping lines from `SDL_GAMECONTROLLERCONFIG` and the
/// common system mapping database, in that order. Empty when neither exists.
pub fn sdl_mappings() -> Vec<String> {
    let mut lines = Vec::new();
    if let Ok(value) = std::env::var("SDL_GAMECONTROLLERCONFIG") {
        lines.extend(
            value
                .split('\n')
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned),
        );
    }
    for path in [
        "/usr/share/gamecontrollerdb.txt",
        "/usr/local/share/gamecontrollerdb.txt",
    ] {
        if let Ok(text) = std::fs::read_to_string(path) {
            lines.extend(
                text.lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty() && !line.starts_with('#'))
                    .map(str::to_owned),
            );
        }
    }
    lines
}

/// Whether a speech-to-text command is installed. Dictation is delegated; this
/// is only the availability probe the pane uses to enable its row.
pub fn dictation_available() -> bool {
    [
        "nerd-dictation",
        "whisper",
        "faster-whisper",
        "vosk-transcriber",
    ]
    .iter()
    .any(|name| command_in_path(name))
}

/// Whether `name` is an executable on `PATH`.
pub fn command_in_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|directory| {
        let candidate = directory.join(name);
        candidate.is_file() && is_executable(&candidate)
    })
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    true
}

/// Read one event from a controller node with a two-second timeout.
///
/// `js` nodes deliver fixed 8-byte events; `event` nodes deliver 24-byte
/// `input_event` records. The `timeout` tool bounds the read so a quiet
/// controller cannot wedge the bridge host.
pub fn test_controller(path: &str) -> ControllerTest {
    if path.is_empty() || !Path::new(path).exists() {
        return ControllerTest {
            outcome: "absent".to_owned(),
            path: path.to_owned(),
            detail: "No controller device node is available.".to_owned(),
            bytes: String::new(),
        };
    }
    let block = if path.contains("/event") { "24" } else { "8" };
    let output = Command::new("timeout")
        .args([
            "2",
            "dd",
            &format!("if={path}"),
            "bs=1",
            &format!("count={block}"),
            "status=none",
        ])
        .output();
    let output = match output {
        Ok(output) => output,
        Err(error) => return ControllerTest::failed(path, format!("cannot run dd: {error}")),
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let lower = stderr.to_ascii_lowercase();
        if lower.contains("permission denied") {
            return ControllerTest::denied(path);
        }
        // `timeout` exits 124 when the read timed out with no bytes.
        if output.status.code() == Some(124) || output.stdout.is_empty() {
            return ControllerTest::timeout(path);
        }
        return ControllerTest::failed(path, stderr.trim().to_owned());
    }
    if output.stdout.is_empty() {
        return ControllerTest::timeout(path);
    }
    ControllerTest::event(path, &output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROC: &str = r#"I: Bus=0003 Vendor=045e Product=028e Version=0114
N: Name="Microsoft X-Box 360 pad"
P: Phys=usb-0000:00:14.0-1/input0
S: Sysfs=/devices/pci0000:00/0000:00:14.0/usb1/1-1/input/input20
U: Uniq=
H: Handlers=js0 event20
B: PROP=0
B: EV=20000b
B: KEY=7cdb000000000000 0 0 0 0
B: ABS=3003f

I: Bus=0003 Vendor=1038 Product=1610 Version=0111
N: Name="SteelSeries Apex Pro"
P: Phys=usb-0000:04:00.4-1.4/input0
S: Sysfs=/devices/pci0000:00/0000:00:08.1/usb3/3-1/3-1.4/input/input7
U: Uniq=
H: Handlers=sysrq kbd leds event5
B: PROP=0
B: EV=120013
B: KEY=1000000000007 ff9f207ac14057ff
B: ABS=100000000
"#;

    #[test]
    fn the_proc_inventory_finds_the_controller_and_skips_the_keyboard() {
        let controllers = controllers_from_proc(PROC);
        assert_eq!(controllers.len(), 1);
        let controller = &controllers[0];
        assert_eq!(controller.name, "Microsoft X-Box 360 pad");
        assert_eq!(controller.path, "/dev/input/js0");
        assert_eq!(controller.identity(), "045E:028E");
        assert_eq!(controller.driver, "udev");
        assert!(!controller.has_mapping());
    }

    #[test]
    fn an_empty_inventory_is_empty_not_an_error() {
        assert!(controllers_from_proc("").is_empty());
    }

    #[test]
    fn a_named_controller_without_js_still_lists_its_event_node() {
        let proc = r#"I: Bus=0003 Vendor=054c Product=05c4 Version=0110
N: Name="Sony Computer Entertainment Wireless Controller"
H: Handlers=event22
B: ABS=3003f
"#;
        let controllers = controllers_from_proc(proc);
        assert_eq!(controllers.len(), 1);
        assert_eq!(controllers[0].path, "/dev/input/event22");
    }

    #[test]
    fn a_missing_node_is_an_absent_test() {
        let test = test_controller("/dev/input/does-not-exist");
        assert_eq!(test.outcome, "absent");
        assert!(!test.is_event());
    }

    #[test]
    fn the_test_outcomes_are_typed() {
        assert!(ControllerTest::event("/dev/input/js0", &[1, 2, 3]).is_event());
        assert_eq!(ControllerTest::timeout("/dev/input/js0").outcome, "timeout");
        assert_eq!(ControllerTest::denied("/dev/input/js0").outcome, "denied");
    }
}
