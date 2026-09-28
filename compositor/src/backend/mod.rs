// SPDX-License-Identifier: MIT
//! Backend plumbing shared by all three backends (T-02).

pub mod drm;
pub mod headless;
pub mod nested;
pub mod synthetic_output;

use smithay::output::{Mode, Output, PhysicalProperties, Subpixel};
use smithay::utils::Transform;

use crate::state::DfState;

/// Create a `wl_output` global for `output` and map it into the scene at
/// `position` with the given mode/scale/transform. Shared by every backend
/// so output state changes behave identically (T-16 displays pane builds
/// on this).
pub fn add_output(
    state: &mut DfState,
    name: &str,
    physical: PhysicalProperties,
    mode: Mode,
    position: (i32, i32),
    scale: f64,
) -> Output {
    let output = Output::new(name.to_string(), physical);
    output.set_preferred(mode);
    output.change_current_state(
        Some(mode),
        Some(Transform::Normal),
        Some(smithay::output::Scale::Fractional(scale)),
        Some(position.into()),
    );
    let _global = output.create_global::<DfState>(&state.display_handle);
    state.space.map_output(&output, position);
    // A fresh output gets a fresh Space list (T-05 FR-7).
    state.on_output_added(&output);
    output
}

/// Create the seat's input capabilities once, for every backend (FR-1:
/// the same session code runs everywhere; headless just has no devices
/// feeding the handles).
pub fn add_seat_capabilities(state: &mut DfState) {
    let _pointer = state.seat.add_pointer();
    let _touch = state.seat.add_touch();
    // Keymap conventions (Cmd→Super, Option→Alt) are fixed in df-ipc and
    // consumed by the shortcut engine and this keymap (T-03 FR-2).
    let keyboard = state.input_settings.keyboard;
    match state.seat.add_keyboard(
        crate::input::keymap::xkb_config(),
        keyboard.repeat_delay_ms,
        keyboard.repeat_rate_hz,
    ) {
        Ok(_keyboard) => {}
        Err(err) => eprintln!("dragonfruit-compositor: failed to add keyboard: {err}"),
    }
}

/// The headless default mode.
pub const HEADLESS_MODE_SIZE: (i32, i32) = (1280, 720);

/// Environment variable overriding the headless output's scale (T-16.3a).
/// The default is `1.0`; a scaling conformance test sets e.g. `2.0` so
/// Xwayland boots against an integer-scaled primary display.
pub const ENV_HEADLESS_SCALE: &str = "DRAGONFRUIT_HEADLESS_SCALE";

/// Environment variable overriding the nested output's scale (T-16.3b).
/// The default is `1.0`; a fractional value (e.g. `1.5`) lets the nested
/// session exercise fractional-scale chrome and the per-surface downscale
/// without a physical display.
pub const ENV_NESTED_SCALE: &str = "DRAGONFRUIT_NESTED_SCALE";

/// Parse a positive finite output scale from `value`, or the `fallback`.
pub fn parse_output_scale(value: Option<&str>, fallback: f64) -> f64 {
    value
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|scale| scale.is_finite() && *scale > 0.0)
        .unwrap_or(fallback)
}

pub fn headless_physical_properties() -> PhysicalProperties {
    PhysicalProperties {
        size: (0, 0).into(),
        subpixel: Subpixel::Unknown,
        make: "Dragonfruit".into(),
        model: "Headless".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_output_scale;

    #[test]
    fn parses_a_positive_output_scale_and_rejects_bad_values() {
        assert_eq!(parse_output_scale(Some("2"), 1.0), 2.0);
        assert_eq!(parse_output_scale(Some("1.5"), 1.0), 1.5);
        // Missing, unparseable, non-finite, and non-positive fall back.
        assert_eq!(parse_output_scale(None, 1.0), 1.0);
        assert_eq!(parse_output_scale(Some("wide"), 1.0), 1.0);
        assert_eq!(parse_output_scale(Some("NaN"), 1.0), 1.0);
        assert_eq!(parse_output_scale(Some("0"), 1.0), 1.0);
        assert_eq!(parse_output_scale(Some("-2"), 1.0), 1.0);
    }
}
