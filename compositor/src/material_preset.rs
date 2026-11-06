// SPDX-License-Identifier: MIT
//! The glass material-preset vocabulary and resolution policy (T195).
//!
//! `design-system/tokens/tokens.json` owns the optical data and
//! [`crate::design_tokens::glass`] is its generated Rust view. This module is
//! the compositor's reference resolver: it maps a preset plus scheme, slider,
//! accessibility, backend capability and degrade tier to explicit optics and
//! one rendering mode. It deliberately does not replace [`crate::window`]'s
//! `MaterialRole` wire values or `DegradeTier` ladder; those stay the render
//! transport, while these presets are the optical vocabulary layered above it.
//!
//! The QML twin is `design-system/Glass.qml`; both resolve the same generated
//! data and the shared cases in `tests/glass/test_material_presets.py`.
#![allow(dead_code)]

use crate::design_tokens::{glass, semantic};

/// Every optical preset, in the token file's order.
pub const PRESET_NAMES: [&str; 13] = [
    "dock",
    "menuBarVeil",
    "menuAttached",
    "menuDetached",
    "statusMenu",
    "tooltip",
    "controlCenterOuter",
    "controlCenterModule",
    "controlCenterExpanded",
    "notificationBanner",
    "notificationStack",
    "notificationLowerPanel",
    "windowChrome",
];

/// The light/dark scheme the preset is resolved for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scheme {
    #[default]
    Light,
    Dark,
}

impl Scheme {
    pub const fn name(self) -> &'static str {
        match self {
            Scheme::Light => "light",
            Scheme::Dark => "dark",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "light" => Some(Scheme::Light),
            "dark" => Some(Scheme::Dark),
            _ => None,
        }
    }
}

/// The existing wire material role. Presets map onto it; they do not replace
/// its stable values (ADR 0203/0204).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Chrome,
    Dock,
    Popup,
    Drawer,
}

impl Role {
    pub const fn name(self) -> &'static str {
        match self {
            Role::Chrome => "chrome",
            Role::Dock => "dock",
            Role::Popup => "popup",
            Role::Drawer => "drawer",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "chrome" => Some(Role::Chrome),
            "dock" => Some(Role::Dock),
            "popup" => Some(Role::Popup),
            "drawer" => Some(Role::Drawer),
            _ => None,
        }
    }

    /// Persistent chrome keeps its sampled material floor (ADR 0203/0211).
    pub const fn is_persistent(self) -> bool {
        matches!(self, Role::Chrome | Role::Dock)
    }
}

/// One optical surface preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Preset {
    Dock,
    MenuBarVeil,
    MenuAttached,
    MenuDetached,
    StatusMenu,
    Tooltip,
    ControlCenterOuter,
    ControlCenterModule,
    ControlCenterExpanded,
    NotificationBanner,
    NotificationStack,
    NotificationLowerPanel,
    WindowChrome,
}

impl Preset {
    pub const ALL: [Preset; 13] = [
        Preset::Dock,
        Preset::MenuBarVeil,
        Preset::MenuAttached,
        Preset::MenuDetached,
        Preset::StatusMenu,
        Preset::Tooltip,
        Preset::ControlCenterOuter,
        Preset::ControlCenterModule,
        Preset::ControlCenterExpanded,
        Preset::NotificationBanner,
        Preset::NotificationStack,
        Preset::NotificationLowerPanel,
        Preset::WindowChrome,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Preset::Dock => "dock",
            Preset::MenuBarVeil => "menuBarVeil",
            Preset::MenuAttached => "menuAttached",
            Preset::MenuDetached => "menuDetached",
            Preset::StatusMenu => "statusMenu",
            Preset::Tooltip => "tooltip",
            Preset::ControlCenterOuter => "controlCenterOuter",
            Preset::ControlCenterModule => "controlCenterModule",
            Preset::ControlCenterExpanded => "controlCenterExpanded",
            Preset::NotificationBanner => "notificationBanner",
            Preset::NotificationStack => "notificationStack",
            Preset::NotificationLowerPanel => "notificationLowerPanel",
            Preset::WindowChrome => "windowChrome",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|preset| preset.name() == name)
    }

    /// The existing wire role this preset is a visual variant of.
    pub const fn role(self) -> Role {
        match self {
            Preset::Dock => Role::Dock,
            Preset::MenuBarVeil | Preset::WindowChrome => Role::Chrome,
            _ => Role::Popup,
        }
    }

    /// The token file's intended optical mode, before backend/accessibility
    /// resolution. Read from the generated preset so the Rust and QML
    /// resolvers cannot disagree about which surfaces are veils.
    pub fn intended_render(self) -> IntendedRender {
        match token_render(self) {
            "veil" => IntendedRender::Veil,
            "solid" => IntendedRender::Solid,
            _ => IntendedRender::Sampled,
        }
    }

    /// Only the light attached-menu preset follows the measured File slider.
    pub fn follows_slider(self) -> bool {
        token_slider(self) == "lightAttached"
    }
}

/// The generated `role` field for a preset.
fn token_role(preset: Preset) -> &'static str {
    use crate::design_tokens::glass::presets as p;
    match preset {
        Preset::Dock => p::dock::ROLE,
        Preset::MenuBarVeil => p::menu_bar_veil::ROLE,
        Preset::MenuAttached => p::menu_attached::ROLE,
        Preset::MenuDetached => p::menu_detached::ROLE,
        Preset::StatusMenu => p::status_menu::ROLE,
        Preset::Tooltip => p::tooltip::ROLE,
        Preset::ControlCenterOuter => p::control_center_outer::ROLE,
        Preset::ControlCenterModule => p::control_center_module::ROLE,
        Preset::ControlCenterExpanded => p::control_center_expanded::ROLE,
        Preset::NotificationBanner => p::notification_banner::ROLE,
        Preset::NotificationStack => p::notification_stack::ROLE,
        Preset::NotificationLowerPanel => p::notification_lower_panel::ROLE,
        Preset::WindowChrome => p::window_chrome::ROLE,
    }
}

/// The generated `render` field for a preset.
fn token_render(preset: Preset) -> &'static str {
    use crate::design_tokens::glass::presets as p;
    match preset {
        Preset::Dock => p::dock::RENDER,
        Preset::MenuBarVeil => p::menu_bar_veil::RENDER,
        Preset::MenuAttached => p::menu_attached::RENDER,
        Preset::MenuDetached => p::menu_detached::RENDER,
        Preset::StatusMenu => p::status_menu::RENDER,
        Preset::Tooltip => p::tooltip::RENDER,
        Preset::ControlCenterOuter => p::control_center_outer::RENDER,
        Preset::ControlCenterModule => p::control_center_module::RENDER,
        Preset::ControlCenterExpanded => p::control_center_expanded::RENDER,
        Preset::NotificationBanner => p::notification_banner::RENDER,
        Preset::NotificationStack => p::notification_stack::RENDER,
        Preset::NotificationLowerPanel => p::notification_lower_panel::RENDER,
        Preset::WindowChrome => p::window_chrome::RENDER,
    }
}

/// The generated `slider` field for a preset.
fn token_slider(preset: Preset) -> &'static str {
    use crate::design_tokens::glass::presets as p;
    match preset {
        Preset::Dock => p::dock::SLIDER,
        Preset::MenuBarVeil => p::menu_bar_veil::SLIDER,
        Preset::MenuAttached => p::menu_attached::SLIDER,
        Preset::MenuDetached => p::menu_detached::SLIDER,
        Preset::StatusMenu => p::status_menu::SLIDER,
        Preset::Tooltip => p::tooltip::SLIDER,
        Preset::ControlCenterOuter => p::control_center_outer::SLIDER,
        Preset::ControlCenterModule => p::control_center_module::SLIDER,
        Preset::ControlCenterExpanded => p::control_center_expanded::SLIDER,
        Preset::NotificationBanner => p::notification_banner::SLIDER,
        Preset::NotificationStack => p::notification_stack::SLIDER,
        Preset::NotificationLowerPanel => p::notification_lower_panel::SLIDER,
        Preset::WindowChrome => p::window_chrome::SLIDER,
    }
}

/// The static intent recorded in the token file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntendedRender {
    Sampled,
    Veil,
    Solid,
}

/// The one internal end-to-end rollout switch. T208 owns flipping the default;
/// until then an unset/unknown value keeps the existing appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rollout {
    #[default]
    Legacy,
    Replacement,
}

impl Rollout {
    pub const fn name(self) -> &'static str {
        match self {
            Rollout::Legacy => "legacy",
            Rollout::Replacement => "replacement",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "legacy" => Some(Rollout::Legacy),
            "replacement" => Some(Rollout::Replacement),
            _ => None,
        }
    }

    /// Read the shared fixture/native switch. A missing or unknown value is
    /// the safe existing appearance, never the replacement by accident.
    pub fn from_env() -> Self {
        std::env::var(glass::rollout::ENV_VAR)
            .ok()
            .and_then(|value| Self::from_name(value.trim()))
            .unwrap_or_default()
    }
}

/// How a resolved surface actually renders. A veil is an intentional no-blur
/// surface and is never reported as a missing sampled pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    SampledGlass,
    NoBlurVeil,
    Solid,
    SoftwareFallback,
}

impl RenderMode {
    pub const fn name(self) -> &'static str {
        match self {
            RenderMode::SampledGlass => "sampled-glass",
            RenderMode::NoBlurVeil => "no-blur-veil",
            RenderMode::Solid => "solid",
            RenderMode::SoftwareFallback => "software-fallback",
        }
    }
}

/// The backend's sampled-material capability. `GpuNoSampledBlur` is the
/// current DRM rail without the pass; `Software` is headless/no-readback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    #[default]
    GpuSampled,
    GpuNoSampledBlur,
    Software,
}

impl Backend {
    pub const fn name(self) -> &'static str {
        match self {
            Backend::GpuSampled => "gpu-sampled",
            Backend::GpuNoSampledBlur => "gpu-no-sampled-blur",
            Backend::Software => "software",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "gpu-sampled" => Some(Backend::GpuSampled),
            "gpu-no-sampled-blur" => Some(Backend::GpuNoSampledBlur),
            "software" => Some(Backend::Software),
            _ => None,
        }
    }
}

/// The degrade ladder's optical input. Kept separate from the wire tier enum so
/// this module stays renderer-free; T197 maps `DegradeTier` onto it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tier {
    #[default]
    Full,
    Reduced,
    Minimal,
}

impl Tier {
    pub const fn name(self) -> &'static str {
        match self {
            Tier::Full => "full",
            Tier::Reduced => "reduced",
            Tier::Minimal => "minimal",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "full" => Some(Tier::Full),
            "reduced" => Some(Tier::Reduced),
            "minimal" => Some(Tier::Minimal),
            _ => None,
        }
    }

    pub const fn blur_enabled(self) -> bool {
        !matches!(self, Tier::Minimal)
    }
}

/// The measured midpoint of the attached-menu slider. T196's settings schema
/// uses the same value as `appearance.transparency`'s default.
pub const DEFAULT_TRANSPARENCY: f32 = glass::slider::DEFAULT_POSITION;

/// The two accessibility overrides that participate in material resolution.
/// Reduce Motion is intentionally absent: it is orthogonal to optics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Accessibility {
    pub reduce_transparency: bool,
    pub increase_contrast: bool,
}

/// Everything the resolver needs. `slider` is the 0..1 Appearance slider; it
/// is clamped and non-finite values fall back to the token midpoint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolveInput {
    pub preset: Preset,
    pub scheme: Scheme,
    pub slider: f32,
    pub accessibility: Accessibility,
    pub backend: Backend,
    pub tier: Tier,
    pub rollout: Rollout,
    /// Accepted so callers cannot fold motion policy into optics; it has no
    /// effect on the result.
    pub reduce_motion: bool,
}

impl Default for ResolveInput {
    fn default() -> Self {
        Self {
            preset: Preset::Dock,
            scheme: Scheme::Light,
            slider: glass::slider::DEFAULT_POSITION,
            accessibility: Accessibility::default(),
            backend: Backend::GpuSampled,
            tier: Tier::Full,
            rollout: Rollout::default(),
            reduce_motion: false,
        }
    }
}

/// One resolved surface. `alpha`/`tint` are the neutral plate fit after the
/// perceptual mix, slider (where supported) and accessibility overrides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedPreset {
    pub preset: Preset,
    pub scheme: Scheme,
    pub look: Rollout,
    pub render_mode: RenderMode,
    pub alpha: f32,
    pub tint: f32,
    pub blur: f32,
    pub edge: f32,
    pub edge_width: f32,
    pub foreground: f32,
    pub slider_applied: bool,
    pub confidence: &'static str,
    pub fallback_reason: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct Calibration {
    neutral_alpha: f32,
    neutral_tint: f32,
    perceptual_weight: f32,
    perceptual_alpha: f32,
    perceptual_tint: f32,
    blur: f32,
    edge: f32,
    edge_width: f32,
    foreground: f32,
    confidence: &'static str,
}

/// The generated optical values for one preset/scheme. The match is explicit
/// so the compiler catches a preset that is added without a token entry.
fn calibration(preset: Preset, scheme: Scheme) -> Calibration {
    use crate::design_tokens::glass::presets as p;
    macro_rules! cal {
        ($preset:ident, $scheme:ident) => {
            Calibration {
                neutral_alpha: p::$preset::$scheme::calibration::NEUTRAL_ALPHA,
                neutral_tint: p::$preset::$scheme::calibration::NEUTRAL_TINT,
                perceptual_weight: p::$preset::$scheme::calibration::PERCEPTUAL_WEIGHT,
                perceptual_alpha: p::$preset::$scheme::calibration::PERCEPTUAL_ALPHA,
                perceptual_tint: p::$preset::$scheme::calibration::PERCEPTUAL_TINT,
                blur: p::$preset::$scheme::BLUR,
                edge: p::$preset::$scheme::EDGE,
                edge_width: p::$preset::$scheme::EDGE_WIDTH,
                foreground: p::$preset::$scheme::FOREGROUND,
                confidence: p::$preset::$scheme::CONFIDENCE,
            }
        };
    }
    match (preset, scheme) {
        (Preset::Dock, Scheme::Light) => cal!(dock, light),
        (Preset::Dock, Scheme::Dark) => cal!(dock, dark),
        (Preset::MenuBarVeil, Scheme::Light) => cal!(menu_bar_veil, light),
        (Preset::MenuBarVeil, Scheme::Dark) => cal!(menu_bar_veil, dark),
        (Preset::MenuAttached, Scheme::Light) => cal!(menu_attached, light),
        (Preset::MenuAttached, Scheme::Dark) => cal!(menu_attached, dark),
        (Preset::MenuDetached, Scheme::Light) => cal!(menu_detached, light),
        (Preset::MenuDetached, Scheme::Dark) => cal!(menu_detached, dark),
        (Preset::StatusMenu, Scheme::Light) => cal!(status_menu, light),
        (Preset::StatusMenu, Scheme::Dark) => cal!(status_menu, dark),
        (Preset::Tooltip, Scheme::Light) => cal!(tooltip, light),
        (Preset::Tooltip, Scheme::Dark) => cal!(tooltip, dark),
        (Preset::ControlCenterOuter, Scheme::Light) => cal!(control_center_outer, light),
        (Preset::ControlCenterOuter, Scheme::Dark) => cal!(control_center_outer, dark),
        (Preset::ControlCenterModule, Scheme::Light) => cal!(control_center_module, light),
        (Preset::ControlCenterModule, Scheme::Dark) => cal!(control_center_module, dark),
        (Preset::ControlCenterExpanded, Scheme::Light) => cal!(control_center_expanded, light),
        (Preset::ControlCenterExpanded, Scheme::Dark) => cal!(control_center_expanded, dark),
        (Preset::NotificationBanner, Scheme::Light) => cal!(notification_banner, light),
        (Preset::NotificationBanner, Scheme::Dark) => cal!(notification_banner, dark),
        (Preset::NotificationStack, Scheme::Light) => cal!(notification_stack, light),
        (Preset::NotificationStack, Scheme::Dark) => cal!(notification_stack, dark),
        (Preset::NotificationLowerPanel, Scheme::Light) => cal!(notification_lower_panel, light),
        (Preset::NotificationLowerPanel, Scheme::Dark) => cal!(notification_lower_panel, dark),
        (Preset::WindowChrome, Scheme::Light) => cal!(window_chrome, light),
        (Preset::WindowChrome, Scheme::Dark) => cal!(window_chrome, dark),
    }
}

/// The existing per-role material pair, used by the legacy rollout look.
fn legacy_values(role: Role, scheme: Scheme) -> (f32, f32, f32, f32) {
    // (opacity, blur, tint, fallback opacity)
    match (role, scheme) {
        (Role::Chrome, Scheme::Light) => (
            semantic::light::material::CHROME_OPACITY,
            semantic::light::material::CHROME_BLUR,
            1.0,
            semantic::light::material::CHROME_FALLBACK_OPACITY,
        ),
        (Role::Chrome, Scheme::Dark) => (
            semantic::dark::material::CHROME_OPACITY,
            semantic::dark::material::CHROME_BLUR,
            1.0,
            semantic::dark::material::CHROME_FALLBACK_OPACITY,
        ),
        (Role::Dock, Scheme::Light) => (
            semantic::light::material::DOCK_OPACITY,
            semantic::light::material::DOCK_BLUR,
            1.0,
            semantic::light::material::DOCK_FALLBACK_OPACITY,
        ),
        (Role::Dock, Scheme::Dark) => (
            semantic::dark::material::DOCK_OPACITY,
            semantic::dark::material::DOCK_BLUR,
            1.0,
            semantic::dark::material::DOCK_FALLBACK_OPACITY,
        ),
        (Role::Popup | Role::Drawer, Scheme::Light) => (
            semantic::light::material::POPUP_OPACITY,
            semantic::light::material::POPUP_BLUR,
            1.0,
            semantic::light::material::POPUP_FALLBACK_OPACITY,
        ),
        (Role::Popup | Role::Drawer, Scheme::Dark) => (
            semantic::dark::material::POPUP_OPACITY,
            semantic::dark::material::POPUP_BLUR,
            1.0,
            semantic::dark::material::POPUP_FALLBACK_OPACITY,
        ),
    }
}

fn opaque_values(scheme: Scheme) -> (f32, f32) {
    match scheme {
        Scheme::Light => (
            glass::accessibility::reduce_transparency::light::ALPHA,
            glass::accessibility::reduce_transparency::light::TINT,
        ),
        Scheme::Dark => (
            glass::accessibility::reduce_transparency::dark::ALPHA,
            glass::accessibility::reduce_transparency::dark::TINT,
        ),
    }
}

fn sanitize01(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

fn mix(neutral: f32, perceptual: f32, weight: f32) -> f32 {
    let weight = sanitize01(weight, 0.0);
    neutral * (1.0 - weight) + perceptual * weight
}

/// Piecewise-linear interpolation through the three measured light anchors.
fn interpolate(position: f32) -> (f32, f32) {
    use glass::slider::light_anchors as a;
    let position = sanitize01(position, a::mid::POSITION);
    let points = [
        (a::low::POSITION, a::low::ALPHA, a::low::TINT),
        (a::mid::POSITION, a::mid::ALPHA, a::mid::TINT),
        (a::high::POSITION, a::high::ALPHA, a::high::TINT),
    ];
    for pair in points.windows(2) {
        let (left, right) = (pair[0], pair[1]);
        if position <= right.0 {
            let span = right.0 - left.0;
            let ratio = if span <= 0.0 { 0.0 } else { (position - left.0) / span };
            return (
                left.1 + (right.1 - left.1) * ratio,
                left.2 + (right.2 - left.2) * ratio,
            );
        }
    }
    (points[2].1, points[2].2)
}

/// The safe unsupported-name path: an opaque solid plate with a named reason.
pub fn safe_solid(scheme: Scheme, reason: &'static str) -> ResolvedPreset {
    let (alpha, tint) = opaque_values(scheme);
    ResolvedPreset {
        preset: Preset::Dock,
        scheme,
        look: Rollout::Replacement,
        render_mode: RenderMode::Solid,
        alpha,
        tint,
        blur: 0.0,
        edge: 0.0,
        edge_width: 0.0,
        foreground: 0.0,
        slider_applied: false,
        confidence: "baseline",
        fallback_reason: reason,
    }
}

/// Resolve one preset. See the module docs for the precedence order.
pub fn resolve(input: ResolveInput) -> ResolvedPreset {
    let ResolveInput {
        preset,
        scheme,
        slider,
        accessibility,
        backend,
        tier,
        rollout,
        reduce_motion: _,
    } = input;

    let role = preset.role();
    let persistent = role.is_persistent();
    let mut slider_applied = false;
    let mut fallback_reason = "none";
    let mut confidence = "baseline";
    let mut alpha;
    let mut tint;
    let mut blur;
    let mut edge;
    let mut edge_width;
    let mut foreground;

    if rollout == Rollout::Legacy {
        let (opacity, legacy_blur, legacy_tint, _fallback_opacity) =
            legacy_values(role, scheme);
        alpha = opacity;
        tint = legacy_tint;
        blur = legacy_blur;
        edge = 0.0;
        edge_width = 0.0;
        foreground = 0.0;
        fallback_reason = "legacy";
    } else {
        let calibration = calibration(preset, scheme);
        alpha = mix(
            calibration.neutral_alpha,
            calibration.perceptual_alpha,
            calibration.perceptual_weight,
        );
        tint = mix(
            calibration.neutral_tint,
            calibration.perceptual_tint,
            calibration.perceptual_weight,
        );
        blur = calibration.blur;
        edge = calibration.edge;
        edge_width = calibration.edge_width;
        foreground = calibration.foreground;
        confidence = calibration.confidence;

        if preset.follows_slider() && scheme == Scheme::Light {
            let position = sanitize01(slider, glass::slider::DEFAULT_POSITION);
            (alpha, tint) = interpolate(position);
            slider_applied = true;
        }
    }

    // Reduce Transparency is the explicit override: it wins over the slider and
    // any sampled path, in both rollout looks, with the scheme's opaque plate.
    if accessibility.reduce_transparency {
        (alpha, tint) = opaque_values(scheme);
        blur = 0.0;
        slider_applied = false;
        fallback_reason = "reduce-transparency";
    }

    // Increase Contrast is applied after the fill decision so it strengthens
    // the edge/foreground even when the plate is already opaque.
    if accessibility.increase_contrast {
        edge = sanitize01(edge + glass::accessibility::increase_contrast::EDGE_BOOST, edge);
        foreground = sanitize01(
            foreground + glass::accessibility::increase_contrast::FOREGROUND_BOOST,
            foreground,
        );
        edge_width =
            (edge_width + glass::accessibility::increase_contrast::EDGE_WIDTH_BOOST).max(0.0);
    }

    let render_mode = if accessibility.reduce_transparency {
        RenderMode::Solid
    } else if rollout == Rollout::Legacy {
        if backend == Backend::Software {
            let (_, fallback_opacity) = legacy_fallback(role, scheme);
            alpha = fallback_opacity;
            blur = 0.0;
            fallback_reason = "software";
            RenderMode::SoftwareFallback
        } else if backend == Backend::GpuNoSampledBlur {
            let (_, fallback_opacity) = legacy_fallback(role, scheme);
            alpha = fallback_opacity;
            blur = 0.0;
            fallback_reason = "no-sampled-blur";
            RenderMode::Solid
        } else if tier == Tier::Minimal && !persistent {
            let (_, fallback_opacity) = legacy_fallback(role, scheme);
            alpha = fallback_opacity;
            blur = 0.0;
            fallback_reason = "minimal";
            RenderMode::Solid
        } else {
            // The persistent floor keeps Chrome/Dock sampled even when the
            // controller visited Minimal (ADR 0203/0211).
            fallback_reason = "none";
            RenderMode::SampledGlass
        }
    } else {
        match preset.intended_render() {
            IntendedRender::Veil => {
                // A veil is an intentional no-blur surface. It must never be
                // reported as a missing sampled pass.
                blur = 0.0;
                fallback_reason = "none";
                RenderMode::NoBlurVeil
            }
            IntendedRender::Solid => {
                blur = 0.0;
                fallback_reason = "none";
                RenderMode::Solid
            }
            IntendedRender::Sampled => {
                if backend == Backend::Software {
                    (alpha, tint) = opaque_values(scheme);
                    blur = 0.0;
                    fallback_reason = "software";
                    RenderMode::SoftwareFallback
                } else if backend == Backend::GpuNoSampledBlur {
                    (alpha, tint) = opaque_values(scheme);
                    blur = 0.0;
                    fallback_reason = "no-sampled-blur";
                    RenderMode::Solid
                } else if tier == Tier::Minimal && !persistent {
                    (alpha, tint) = opaque_values(scheme);
                    blur = 0.0;
                    fallback_reason = "minimal";
                    RenderMode::Solid
                } else {
                    fallback_reason = "none";
                    RenderMode::SampledGlass
                }
            }
        }
    };

    ResolvedPreset {
        preset,
        scheme,
        look: rollout,
        render_mode,
        alpha,
        tint,
        blur,
        edge,
        edge_width,
        foreground,
        slider_applied,
        confidence,
        fallback_reason,
    }
}

fn legacy_fallback(role: Role, scheme: Scheme) -> (f32, f32) {
    let (_, _, _, fallback) = legacy_values(role, scheme);
    (0.0, fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(preset: Preset, scheme: Scheme) -> ResolveInput {
        ResolveInput {
            preset,
            scheme,
            rollout: Rollout::Replacement,
            ..ResolveInput::default()
        }
    }

    #[test]
    fn preset_names_round_trip() {
        assert_eq!(Preset::ALL.len(), PRESET_NAMES.len());
        for (preset, name) in Preset::ALL.into_iter().zip(PRESET_NAMES) {
            assert_eq!(preset.name(), name);
            assert_eq!(Preset::from_name(name), Some(preset));
        }
        assert_eq!(Preset::from_name("not-a-preset"), None);
    }

    #[test]
    fn preset_metadata_matches_the_generated_tokens() {
        for preset in Preset::ALL {
            assert_eq!(preset.role().name(), token_role(preset));
            assert_eq!(preset.follows_slider(), token_slider(preset) == "lightAttached");
            let expected_render = match token_render(preset) {
                "veil" => IntendedRender::Veil,
                "solid" => IntendedRender::Solid,
                _ => IntendedRender::Sampled,
            };
            assert_eq!(preset.intended_render(), expected_render);
        }
    }

    #[test]
    fn light_slider_uses_all_three_anchors() {
        for (position, expected_alpha, expected_tint) in [
            (0.0, 0.732, 0.953),
            (0.5255, 0.552, 0.993),
            (1.0, 0.415, 0.997),
        ] {
            let resolved = resolve(ResolveInput {
                slider: position,
                ..input(Preset::MenuAttached, Scheme::Light)
            });
            assert!((resolved.alpha - expected_alpha).abs() < 1e-6);
            assert!((resolved.tint - expected_tint).abs() < 1e-6);
            assert!(resolved.slider_applied);
        }
    }

    #[test]
    fn only_the_light_attached_menu_follows_the_slider() {
        for preset in Preset::ALL {
            for scheme in [Scheme::Light, Scheme::Dark] {
                let low = resolve(ResolveInput {
                    slider: 0.0,
                    ..input(preset, scheme)
                });
                let high = resolve(ResolveInput {
                    slider: 1.0,
                    ..input(preset, scheme)
                });
                let expected = preset == Preset::MenuAttached && scheme == Scheme::Light;
                assert_eq!(low.slider_applied, expected, "{} {scheme:?}", preset.name());
                if !expected {
                    assert_eq!(low.alpha, high.alpha, "{} {scheme:?}", preset.name());
                    assert_eq!(low.tint, high.tint, "{} {scheme:?}", preset.name());
                }
            }
        }
    }

    #[test]
    fn dark_attached_and_detached_are_distinct() {
        let attached = resolve(input(Preset::MenuAttached, Scheme::Dark));
        let detached = resolve(input(Preset::MenuDetached, Scheme::Dark));
        assert_ne!((attached.alpha, attached.tint), (detached.alpha, detached.tint));
    }

    #[test]
    fn reduce_transparency_wins_over_the_slider_and_sampling() {
        let resolved = resolve(ResolveInput {
            slider: 0.0,
            accessibility: Accessibility {
                reduce_transparency: true,
                increase_contrast: false,
            },
            ..input(Preset::MenuAttached, Scheme::Light)
        });
        assert_eq!(resolved.render_mode, RenderMode::Solid);
        assert!(!resolved.slider_applied);
        assert!(resolved.alpha >= 0.9);
        assert_eq!(resolved.blur, 0.0);
    }

    #[test]
    fn increase_contrast_applies_with_reduce_transparency() {
        let plain = resolve(ResolveInput {
            accessibility: Accessibility {
                reduce_transparency: true,
                increase_contrast: false,
            },
            ..input(Preset::MenuAttached, Scheme::Light)
        });
        let contrasted = resolve(ResolveInput {
            accessibility: Accessibility {
                reduce_transparency: true,
                increase_contrast: true,
            },
            ..input(Preset::MenuAttached, Scheme::Light)
        });
        assert!(contrasted.edge > plain.edge);
        assert!(contrasted.foreground > plain.foreground);
    }

    #[test]
    fn veil_is_not_an_unsupported_blur() {
        for backend in [Backend::GpuSampled, Backend::GpuNoSampledBlur, Backend::Software] {
            let resolved = resolve(ResolveInput {
                backend,
                ..input(Preset::MenuBarVeil, Scheme::Light)
            });
            assert_eq!(resolved.render_mode, RenderMode::NoBlurVeil);
            assert_eq!(resolved.fallback_reason, "none");
            assert_eq!(resolved.blur, 0.0);
        }
    }

    #[test]
    fn unsupported_sampled_paths_are_safe() {
        for (backend, expected) in [
            (Backend::GpuNoSampledBlur, RenderMode::Solid),
            (Backend::Software, RenderMode::SoftwareFallback),
        ] {
            let resolved = resolve(ResolveInput {
                backend,
                ..input(Preset::MenuDetached, Scheme::Dark)
            });
            assert_eq!(resolved.render_mode, expected);
            assert!(resolved.alpha >= 0.9);
            assert_eq!(resolved.blur, 0.0);
        }
        assert_eq!(safe_solid(Scheme::Light, "unknown-preset").render_mode, RenderMode::Solid);
    }

    #[test]
    fn persistent_dock_keeps_its_sampled_floor() {
        let dock = resolve(ResolveInput {
            tier: Tier::Minimal,
            ..input(Preset::Dock, Scheme::Light)
        });
        assert_eq!(dock.render_mode, RenderMode::SampledGlass);
        let popup = resolve(ResolveInput {
            tier: Tier::Minimal,
            ..input(Preset::MenuDetached, Scheme::Light)
        });
        assert_eq!(popup.render_mode, RenderMode::Solid);
    }

    #[test]
    fn default_rollout_is_the_existing_appearance() {
        assert_eq!(Rollout::default(), Rollout::Legacy);
        let legacy = resolve(ResolveInput {
            preset: Preset::Dock,
            ..ResolveInput::default()
        });
        assert_eq!(legacy.look, Rollout::Legacy);
        assert_eq!(legacy.alpha, semantic::light::material::DOCK_OPACITY);
        assert_eq!(legacy.render_mode, RenderMode::SampledGlass);
        assert_eq!(Rollout::from_name("bogus"), None);
    }

    #[test]
    fn reduce_motion_does_not_change_optics() {
        let moving = resolve(ResolveInput {
            reduce_motion: false,
            ..input(Preset::Dock, Scheme::Light)
        });
        let still = resolve(ResolveInput {
            reduce_motion: true,
            ..input(Preset::Dock, Scheme::Light)
        });
        assert_eq!((moving.alpha, moving.render_mode), (still.alpha, still.render_mode));
    }

    #[test]
    fn non_finite_slider_falls_back_to_the_measured_midpoint() {
        let resolved = resolve(ResolveInput {
            slider: f32::NAN,
            ..input(Preset::MenuAttached, Scheme::Light)
        });
        assert!((resolved.alpha - 0.552).abs() < 1e-6);
        assert!((resolved.tint - 0.993).abs() < 1e-6);
    }
}
