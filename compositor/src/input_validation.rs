// SPDX-License-Identifier: MIT
//! T-03.3 — the hardware input validation matrix.
//!
//! T-03.3 validates the input stack on **real devices**, but the compositor can
//! only drive libinput devices when it owns a logind seat, and this host has no
//! free seat (the KDE Wayland session owns DRM master; see
//! [`crate::drm_bringup`] and `docs/SLICING-REVIEW.md`). The track rule is
//! explicit: when hardware is unavailable the unit is **marked open, not
//! skipped**.
//!
//! This module is the pure, hardware-free half: it classifies each input class
//! the task names — `mouse`, `keyboard`, `touchpad-gestures`, `hot-corners`,
//! `tablet-pen`, and one non-US `layout` — into one of three coverages:
//!
//! * [`Coverage::Device`] — confirmed through the live seat on real hardware;
//! * [`Coverage::Headless`] — exercised through the *same* compositor input
//!   router by the libinput-equivalent synthetic harness
//!   (`crate::input::synthetic`), with real-device confirmation still due;
//! * [`Coverage::Gap`] — a recorded absence (no device, or no seat to drive
//!   one).
//!
//! `scripts/input-validation.sh` writes the human artifact. The six
//! [`InputClass::ALL`] ids defined here are the canonical list that script
//! must stay in sync with; `compositor/tests/input_validation.rs` (part of
//! `make e2e`) pins the decision without hardware.

use smithay::input::keyboard::xkb;

/// One input class the T-03.3 matrix covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InputClass {
    /// Relative pointing (a real mouse, not a touchpad).
    Mouse,
    /// Keyboard key entry.
    Keyboard,
    /// Touchpad swipe/pinch gesture recognition.
    TouchpadGestures,
    /// Compositor-side hot-corner dwell detection.
    HotCorners,
    /// Tablet/pen positioning and pressure.
    TabletPen,
    /// One non-US xkb layout.
    NonUsLayout,
}

impl InputClass {
    /// Every class, in the order the matrix prints them.
    pub const ALL: [InputClass; 6] = [
        InputClass::Mouse,
        InputClass::Keyboard,
        InputClass::TouchpadGestures,
        InputClass::HotCorners,
        InputClass::TabletPen,
        InputClass::NonUsLayout,
    ];

    /// The greppable, stable id the artifact and marker lines use.
    pub const fn id(self) -> &'static str {
        match self {
            InputClass::Mouse => "mouse",
            InputClass::Keyboard => "keyboard",
            InputClass::TouchpadGestures => "touchpad-gestures",
            InputClass::HotCorners => "hot-corners",
            InputClass::TabletPen => "tablet-pen",
            InputClass::NonUsLayout => "non-us-layout",
        }
    }

    /// A human label for the artifact.
    pub const fn label(self) -> &'static str {
        match self {
            InputClass::Mouse => "Mouse",
            InputClass::Keyboard => "Keyboard",
            InputClass::TouchpadGestures => "Touchpad gestures",
            InputClass::HotCorners => "Hot corners",
            InputClass::TabletPen => "Tablet / pen",
            InputClass::NonUsLayout => "Non-US layout",
        }
    }

    /// A pen is the only class with no headless (synthetic) route on this
    /// tree: the wire format has no pressure axis, so a tablet gap is never
    /// papered over by a synthetic pass.
    pub const fn headless_covered(self) -> bool {
        !matches!(self, InputClass::TabletPen)
    }

    /// Whether the class needs a pointer device on the seat.
    const fn needs_pointer(self) -> bool {
        matches!(
            self,
            InputClass::Mouse | InputClass::TouchpadGestures | InputClass::HotCorners
        )
    }

    /// Whether the class needs a keyboard device on the seat.
    const fn needs_keyboard(self) -> bool {
        matches!(self, InputClass::Keyboard | InputClass::NonUsLayout)
    }

    /// Whether the class needs a tablet/pen device on the seat.
    const fn needs_tablet(self) -> bool {
        matches!(self, InputClass::TabletPen)
    }
}

/// The seat devices the compositor would see, reduced to the flags the matrix
/// needs. It is derived by `scripts/input-validation.sh` from
/// `libinput list-devices` (or `/proc/bus/input/devices` when libinput cannot
/// read the event nodes), so the model carries no libinput type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SeatInventory {
    /// Any pointer device (a mouse, touchpad, or virtual pointer).
    pub pointer: bool,
    /// A keyboard device.
    pub keyboard: bool,
    /// A touchpad specifically (pointer plus gesture/tapping).
    pub touchpad: bool,
    /// A tablet/pen device.
    pub tablet: bool,
}

impl SeatInventory {
    /// Every class's device is present (a fully equipped seat).
    pub const fn full() -> Self {
        SeatInventory {
            pointer: true,
            keyboard: true,
            touchpad: true,
            tablet: true,
        }
    }

    /// Whether a real device for `class` is on this seat.
    pub const fn satisfies(&self, class: InputClass) -> bool {
        (class.needs_pointer() && self.pointer)
            || (class.needs_keyboard() && self.keyboard)
            || (class.needs_tablet() && self.tablet)
    }
}

/// Why a class has no real-device confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapReason {
    /// No device for the class on the seat.
    NoDevice,
    /// A device is present but the compositor cannot own the seat to drive it.
    NoSeat,
}

impl GapReason {
    /// The greppable reason token.
    pub const fn id(self) -> &'static str {
        match self {
            GapReason::NoDevice => "no-device",
            GapReason::NoSeat => "no-seat",
        }
    }
}

/// How one class was validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    /// Confirmed on a real device through the live seat.
    Device,
    /// Exercised through the compositor input router headlessly; real-device
    /// confirmation is still pending.
    Headless,
    /// Recorded absence.
    Gap(GapReason),
}

impl Coverage {
    /// The greppable status token.
    pub const fn id(self) -> &'static str {
        match self {
            Coverage::Device => "device",
            Coverage::Headless => "headless",
            Coverage::Gap(_) => "gap",
        }
    }

    /// Whether the class is a recorded gap (neither device nor headless).
    pub const fn is_gap(self) -> bool {
        matches!(self, Coverage::Gap(_))
    }
}

/// One row of the matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassRow {
    /// The class.
    pub class: InputClass,
    /// How it was validated.
    pub coverage: Coverage,
}

impl ClassRow {
    /// The single greppable marker line for this row.
    pub fn marker(&self) -> String {
        match self.coverage {
            Coverage::Gap(reason) => format!(
                "Input validation: {} = gap ({})",
                self.class.id(),
                reason.id()
            ),
            other => format!("Input validation: {} = {}", self.class.id(), other.id()),
        }
    }
}

/// The whole matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputValidation {
    /// One row per [`InputClass::ALL`], in that order.
    pub rows: Vec<ClassRow>,
}

impl InputValidation {
    /// Classify every class from the seat ownership and device inventory.
    ///
    /// A class is `Device` only when the compositor owns the seat *and* a
    /// matching device is present. Otherwise a headless-covered class is
    /// `Headless`; the rest are a gap, naming the specific blocker.
    pub fn classify(seat_owned: bool, inventory: &SeatInventory) -> Self {
        let rows = InputClass::ALL
            .into_iter()
            .map(|class| {
                let present = inventory.satisfies(class);
                let coverage = if seat_owned && present {
                    Coverage::Device
                } else if class.headless_covered() {
                    Coverage::Headless
                } else if present {
                    Coverage::Gap(GapReason::NoSeat)
                } else {
                    Coverage::Gap(GapReason::NoDevice)
                };
                ClassRow { class, coverage }
            })
            .collect();
        InputValidation { rows }
    }

    /// The row for one class.
    pub fn row(&self, class: InputClass) -> &ClassRow {
        self.rows
            .iter()
            .find(|row| row.class == class)
            .expect("InputValidation always has a row per class")
    }

    /// Every gap row, in class order.
    pub fn gaps(&self) -> impl Iterator<Item = &ClassRow> {
        self.rows.iter().filter(|row| row.coverage.is_gap())
    }

    /// Whether the matrix has any recorded gap.
    pub fn is_open(&self) -> bool {
        self.gaps().next().is_some()
    }

    /// The one-line summary marker
    /// (`Input validation: READY (6/6 classes)` or
    /// `Input validation: OPEN (1 gap: tablet-pen)`).
    pub fn marker(&self) -> String {
        let total = self.rows.len();
        let gaps: Vec<&'static str> = self.gaps().map(|row| row.class.id()).collect();
        if gaps.is_empty() {
            format!("Input validation: READY ({total}/{total} classes)")
        } else {
            format!(
                "Input validation: OPEN ({} gap{}: {})",
                gaps.len(),
                if gaps.len() == 1 { "" } else { "s" },
                gaps.join(", ")
            )
        }
    }

    /// Every row's marker line, one per class.
    pub fn lines(&self) -> Vec<String> {
        self.rows.iter().map(ClassRow::marker).collect()
    }
}

/// Compile the pinned RMLVO path with `layout` and return the keysym the given
/// **evdev** key code produces, or `None` if the layout does not compile.
///
/// xkb keycodes are evdev + 8 — the same offset libinput and winit add — so a
/// caller can pass the raw evdev code. This is the headless half of the
/// non-US-layout validation: it exercises exactly the xkb configuration the
/// compositor's keyboard uses (`df_ipc::keymap` rules/model/options), only with
/// an explicit layout instead of the user's `XKB_DEFAULT_LAYOUT`.
pub fn layout_keysym_for_evdev(layout: &str, evdev_code: u32) -> Option<u32> {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let keymap = xkb::Keymap::new_from_names(
        &context,
        df_ipc::keymap::RULES,
        df_ipc::keymap::MODEL,
        layout,
        df_ipc::keymap::VARIANT,
        Some(df_ipc::keymap::OPTIONS.to_owned()),
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )?;
    let state = xkb::State::new(&keymap);
    Some(
        state
            .key_get_one_sym(xkb::Keycode::new(evdev_code + 8))
            .raw(),
    )
}
