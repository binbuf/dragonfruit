// SPDX-License-Identifier: MIT
//! GENERATED FILE — DO NOT EDIT. Edit design-system/tokens/tokens.json and run scripts/gen-tokens.py.
//!
//! Consumed by the compositor's server-side decoration renderer (T-13);
//! because it is generated from the same source as the QML singleton, an
//! app `TitleBar` and a compositor SSD titlebar cannot drift (FR-3).
#![allow(dead_code)]

pub mod primitive {
    pub mod color {
        pub const MAGENTA_50: [u8; 4] = [0xfd, 0xf2, 0xf7, 0xff];
        pub const MAGENTA_100: [u8; 4] = [0xfb, 0xe1, 0xec, 0xff];
        pub const MAGENTA_200: [u8; 4] = [0xf6, 0xc2, 0xd9, 0xff];
        pub const MAGENTA_300: [u8; 4] = [0xee, 0x93, 0xbc, 0xff];
        pub const MAGENTA_400: [u8; 4] = [0xe1, 0x5c, 0x98, 0xff];
        pub const MAGENTA_500: [u8; 4] = [0xcf, 0x3a, 0x7c, 0xff];
        pub const MAGENTA_600: [u8; 4] = [0xb3, 0x2a, 0x66, 0xff];
        pub const MAGENTA_700: [u8; 4] = [0x91, 0x21, 0x54, 0xff];
        pub const MAGENTA_800: [u8; 4] = [0x72, 0x1d, 0x45, 0xff];
        pub const MAGENTA_900: [u8; 4] = [0x59, 0x1a, 0x39, 0xff];
        pub const VIOLET_300: [u8; 4] = [0xb3, 0xa1, 0xff, 0xff];
        pub const VIOLET_400: [u8; 4] = [0x92, 0x7d, 0xff, 0xff];
        pub const VIOLET_500: [u8; 4] = [0x7c, 0x5c, 0xff, 0xff];
        pub const VIOLET_600: [u8; 4] = [0x68, 0x47, 0xe6, 0xff];
        pub const VIOLET_700: [u8; 4] = [0x55, 0x38, 0xc2, 0xff];
        pub const NEUTRAL_0: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
        pub const NEUTRAL_50: [u8; 4] = [0xf8, 0xf6, 0xfa, 0xff];
        pub const NEUTRAL_100: [u8; 4] = [0xef, 0xea, 0xf3, 0xff];
        pub const NEUTRAL_200: [u8; 4] = [0xde, 0xd6, 0xe5, 0xff];
        pub const NEUTRAL_300: [u8; 4] = [0xc3, 0xb8, 0xcc, 0xff];
        pub const NEUTRAL_400: [u8; 4] = [0x9c, 0x8f, 0xa8, 0xff];
        pub const NEUTRAL_500: [u8; 4] = [0x78, 0x6a, 0x84, 0xff];
        pub const NEUTRAL_600: [u8; 4] = [0x5b, 0x4e, 0x66, 0xff];
        pub const NEUTRAL_700: [u8; 4] = [0x44, 0x39, 0x4d, 0xff];
        pub const NEUTRAL_800: [u8; 4] = [0x2d, 0x25, 0x34, 0xff];
        pub const NEUTRAL_900: [u8; 4] = [0x1d, 0x17, 0x23, 0xff];
        pub const NEUTRAL_950: [u8; 4] = [0x13, 0x0f, 0x17, 0xff];
        pub const JADE_400: [u8; 4] = [0x3f, 0xc9, 0x8a, 0xff];
        pub const JADE_500: [u8; 4] = [0x2b, 0xb6, 0x73, 0xff];
        pub const JADE_600: [u8; 4] = [0x1f, 0x9a, 0x60, 0xff];
        pub const GOLD_400: [u8; 4] = [0xf0, 0xb6, 0x4b, 0xff];
        pub const GOLD_500: [u8; 4] = [0xe0, 0xa4, 0x3a, 0xff];
        pub const GOLD_600: [u8; 4] = [0xc7, 0x8a, 0x24, 0xff];
        pub const CORAL_400: [u8; 4] = [0xf2, 0x69, 0x7d, 0xff];
        pub const CORAL_500: [u8; 4] = [0xe8, 0x55, 0x6d, 0xff];
        pub const CORAL_600: [u8; 4] = [0xcf, 0x3f, 0x58, 0xff];
        pub const SKY_400: [u8; 4] = [0x5b, 0x8d, 0xff, 0xff];
        pub const SKY_500: [u8; 4] = [0x4a, 0x7d, 0xff, 0xff];
        pub const SKY_600: [u8; 4] = [0x3a, 0x66, 0xe0, 0xff];
    }
    pub mod radius {
        pub const NONE: f32 = 0.0_f32;
        pub const XS: f32 = 4.0_f32;
        pub const SM: f32 = 6.0_f32;
        pub const MD: f32 = 10.0_f32;
        pub const LG: f32 = 14.0_f32;
        pub const XL: f32 = 20.0_f32;
        pub const XXL: f32 = 28.0_f32;
        pub const PILL: f32 = 999.0_f32;
    }
    pub mod spacing {
        pub const NONE: f32 = 0.0_f32;
        pub const XXS: f32 = 2.0_f32;
        pub const XS: f32 = 4.0_f32;
        pub const SM: f32 = 8.0_f32;
        pub const MD: f32 = 12.0_f32;
        pub const LG: f32 = 16.0_f32;
        pub const XL: f32 = 24.0_f32;
        pub const XXL: f32 = 32.0_f32;
        pub const XXXL: f32 = 48.0_f32;
    }
    pub mod font {
        pub const FAMILY: &str = "Inter";
        pub const SIZE_XS: f32 = 10.0_f32;
        pub const SIZE_SM: f32 = 12.0_f32;
        pub const SIZE_MD: f32 = 13.0_f32;
        pub const SIZE_LG: f32 = 15.0_f32;
        pub const SIZE_XL: f32 = 18.0_f32;
        pub const SIZE_XXL: f32 = 22.0_f32;
        pub const SIZE_DISPLAY: f32 = 28.0_f32;
        pub const WEIGHT_REGULAR: f32 = 400.0_f32;
        pub const WEIGHT_MEDIUM: f32 = 500.0_f32;
        pub const WEIGHT_SEMIBOLD: f32 = 600.0_f32;
        pub const WEIGHT_BOLD: f32 = 700.0_f32;
    }
    pub mod duration {
        pub const INSTANT: f32 = 0.0_f32;
        pub const FAST: f32 = 100.0_f32;
        pub const NORMAL: f32 = 160.0_f32;
        pub const SLOW: f32 = 280.0_f32;
        pub const SLOWER: f32 = 400.0_f32;
    }
    pub mod elevation {
        pub const NONE: f32 = 0.0_f32;
        pub const LOW: f32 = 8.0_f32;
        pub const MED: f32 = 20.0_f32;
        pub const HIGH: f32 = 40.0_f32;
        pub const OVERLAY: f32 = 64.0_f32;
    }
}

pub mod semantic {
    pub mod light {
        pub mod color {
            pub const SURFACE: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
            pub const SURFACE_ELEVATED: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
            pub const SURFACE_SUNKEN: [u8; 4] = [0xef, 0xea, 0xf3, 0xff];
            pub const SURFACE_MUTED: [u8; 4] = [0xf8, 0xf6, 0xfa, 0xff];
            pub const CHROME: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
            pub const TEXT_PRIMARY: [u8; 4] = [0x1d, 0x17, 0x23, 0xff];
            pub const TEXT_SECONDARY: [u8; 4] = [0x5b, 0x4e, 0x66, 0xff];
            pub const TEXT_TERTIARY: [u8; 4] = [0x78, 0x6a, 0x84, 0xff];
            pub const ACCENT: [u8; 4] = [0xb3, 0x2a, 0x66, 0xff];
            pub const ACCENT_HOVER: [u8; 4] = [0xcf, 0x3a, 0x7c, 0xff];
            pub const ACCENT_MUTED: [u8; 4] = [0xfb, 0xe1, 0xec, 0xff];
            pub const ACCENT_CONTENT: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
            pub const BORDER: [u8; 4] = [0xde, 0xd6, 0xe5, 0xff];
            pub const SEPARATOR: [u8; 4] = [0xef, 0xea, 0xf3, 0xff];
            pub const CONTROL_FILL: [u8; 4] = [0xef, 0xea, 0xf3, 0xff];
            pub const CONTROL_HOVER: [u8; 4] = [0xde, 0xd6, 0xe5, 0xff];
            pub const CONTROL_ACTIVE: [u8; 4] = [0xc3, 0xb8, 0xcc, 0xff];
            pub const FOCUS_RING: [u8; 4] = [0x7c, 0x5c, 0xff, 0xff];
            pub const SELECTION: [u8; 4] = [0xb3, 0xa1, 0xff, 0xff];
            pub const MARQUEE_FILL: [u8; 4] = [0xed, 0xed, 0xed, 0x40];
            pub const MARQUEE_BORDER: [u8; 4] = [0xc4, 0xc4, 0xc4, 0x8c];
            pub const DANGER: [u8; 4] = [0xe8, 0x55, 0x6d, 0xff];
            pub const SUCCESS: [u8; 4] = [0x2b, 0xb6, 0x73, 0xff];
            pub const WARNING: [u8; 4] = [0xe0, 0xa4, 0x3a, 0xff];
            pub const INFO: [u8; 4] = [0x4a, 0x7d, 0xff, 0xff];
            pub const CLOSE: [u8; 4] = [0xe8, 0x55, 0x6d, 0xff];
            pub const MINIMIZE: [u8; 4] = [0xe0, 0xa4, 0x3a, 0xff];
            pub const ZOOM: [u8; 4] = [0x2b, 0xb6, 0x73, 0xff];
            pub const CONTROL_KNOB: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
            pub const TRAFFIC_GLYPH: [u8; 4] = [0x00, 0x00, 0x00, 0x8f];
            pub const SHADOW_COLOR: [u8; 4] = [0x00, 0x00, 0x00, 0xff];
            pub const TRASH_FILL_TOP: [u8; 4] = [0xde, 0xd6, 0xe5, 0xff];
            pub const TRASH_FILL_BOTTOM: [u8; 4] = [0xc3, 0xb8, 0xcc, 0xff];
            pub const TRASH_RIM: [u8; 4] = [0x78, 0x6a, 0x84, 0xff];
            pub const TRASH_HIGHLIGHT: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
            pub const TRASH_PAPER: [u8; 4] = [0xf8, 0xf6, 0xfa, 0xff];
            pub const TRASH_PAPER_EDGE: [u8; 4] = [0x9c, 0x8f, 0xa8, 0xff];
            pub const FOLDER_TAB: [u8; 4] = [0x3a, 0x66, 0xe0, 0xff];
            pub const FOLDER_FILL_TOP: [u8; 4] = [0x5b, 0x8d, 0xff, 0xff];
            pub const FOLDER_FILL_BOTTOM: [u8; 4] = [0x4a, 0x7d, 0xff, 0xff];
            pub const FOLDER_RIM: [u8; 4] = [0x3a, 0x66, 0xe0, 0xff];
            pub const FOLDER_HIGHLIGHT: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
            pub const DOCK_FILL: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
            pub const DOCK_FALLBACK_FILL: [u8; 4] = [0xef, 0xea, 0xf3, 0xff];
            pub const CHROME_FALLBACK_FILL: [u8; 4] = [0xf8, 0xf6, 0xfa, 0xff];
            pub const DOCK_RIM: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
            pub const DOCK_BORDER: [u8; 4] = [0xc3, 0xb8, 0xcc, 0xff];
            pub const DOCK_SHADOW: [u8; 4] = [0x00, 0x00, 0x00, 0xff];
            pub const DOCK_HOVER_FILL: [u8; 4] = [0x78, 0x6a, 0x84, 0xff];
            pub const DOCK_DIVIDER: [u8; 4] = [0x78, 0x6a, 0x84, 0xff];
            pub const DOCK_INDICATOR: [u8; 4] = [0x1d, 0x17, 0x23, 0xff];
            pub const SKELETON_BASE: [u8; 4] = [0xde, 0xd6, 0xe5, 0xff];
            pub const SKELETON_HIGHLIGHT: [u8; 4] = [0xf8, 0xf6, 0xfa, 0xff];
        }
        pub mod material {
            pub const CHROME_OPACITY: f32 = 0.82_f32;
            pub const CHROME_BLUR: f32 = 24.0_f32;
            pub const POPUP_OPACITY: f32 = 0.62_f32;
            pub const POPUP_BLUR: f32 = 30.0_f32;
            pub const SHADOW_OPACITY: f32 = 0.18_f32;
            pub const DOCK_OPACITY: f32 = 0.5_f32;
            pub const DOCK_BLUR: f32 = 30.0_f32;
            pub const CHROME_FALLBACK_OPACITY: f32 = 0.86_f32;
            pub const DOCK_FALLBACK_OPACITY: f32 = 0.88_f32;
            pub const POPUP_FALLBACK_OPACITY: f32 = 0.92_f32;
            pub const CHROME_REFRACTION: f32 = 6.0_f32;
            pub const POPUP_REFRACTION: f32 = 8.0_f32;
            pub const DOCK_REFRACTION: f32 = 8.0_f32;
            pub const CHROME_SPECULAR: f32 = 0.45_f32;
            pub const POPUP_SPECULAR: f32 = 0.5_f32;
            pub const DOCK_SPECULAR: f32 = 0.5_f32;
            pub const CHROME_SPECULAR_WIDTH: f32 = 2.0_f32;
            pub const POPUP_SPECULAR_WIDTH: f32 = 2.5_f32;
            pub const DOCK_SPECULAR_WIDTH: f32 = 2.5_f32;
            pub const CHROME_TINT: f32 = 0.16_f32;
            pub const POPUP_TINT: f32 = 0.2_f32;
            pub const DOCK_TINT: f32 = 0.22_f32;
        }
    }
    pub mod dark {
        pub mod color {
            pub const SURFACE: [u8; 4] = [0x1d, 0x17, 0x23, 0xff];
            pub const SURFACE_ELEVATED: [u8; 4] = [0x2d, 0x25, 0x34, 0xff];
            pub const SURFACE_SUNKEN: [u8; 4] = [0x13, 0x0f, 0x17, 0xff];
            pub const SURFACE_MUTED: [u8; 4] = [0x2d, 0x25, 0x34, 0xff];
            pub const CHROME: [u8; 4] = [0x2d, 0x25, 0x34, 0xff];
            pub const TEXT_PRIMARY: [u8; 4] = [0xf8, 0xf6, 0xfa, 0xff];
            pub const TEXT_SECONDARY: [u8; 4] = [0xc3, 0xb8, 0xcc, 0xff];
            pub const TEXT_TERTIARY: [u8; 4] = [0x9c, 0x8f, 0xa8, 0xff];
            pub const ACCENT: [u8; 4] = [0xe1, 0x5c, 0x98, 0xff];
            pub const ACCENT_HOVER: [u8; 4] = [0xee, 0x93, 0xbc, 0xff];
            pub const ACCENT_MUTED: [u8; 4] = [0x59, 0x1a, 0x39, 0xff];
            pub const ACCENT_CONTENT: [u8; 4] = [0x13, 0x0f, 0x17, 0xff];
            pub const BORDER: [u8; 4] = [0x44, 0x39, 0x4d, 0xff];
            pub const SEPARATOR: [u8; 4] = [0x2d, 0x25, 0x34, 0xff];
            pub const CONTROL_FILL: [u8; 4] = [0x2d, 0x25, 0x34, 0xff];
            pub const CONTROL_HOVER: [u8; 4] = [0x44, 0x39, 0x4d, 0xff];
            pub const CONTROL_ACTIVE: [u8; 4] = [0x5b, 0x4e, 0x66, 0xff];
            pub const FOCUS_RING: [u8; 4] = [0x92, 0x7d, 0xff, 0xff];
            pub const SELECTION: [u8; 4] = [0x55, 0x38, 0xc2, 0xff];
            pub const MARQUEE_FILL: [u8; 4] = [0xe6, 0xe6, 0xe6, 0x33];
            pub const MARQUEE_BORDER: [u8; 4] = [0xd8, 0xd8, 0xd8, 0x8c];
            pub const DANGER: [u8; 4] = [0xf2, 0x69, 0x7d, 0xff];
            pub const SUCCESS: [u8; 4] = [0x3f, 0xc9, 0x8a, 0xff];
            pub const WARNING: [u8; 4] = [0xf0, 0xb6, 0x4b, 0xff];
            pub const INFO: [u8; 4] = [0x5b, 0x8d, 0xff, 0xff];
            pub const CLOSE: [u8; 4] = [0xf2, 0x69, 0x7d, 0xff];
            pub const MINIMIZE: [u8; 4] = [0xf0, 0xb6, 0x4b, 0xff];
            pub const ZOOM: [u8; 4] = [0x3f, 0xc9, 0x8a, 0xff];
            pub const CONTROL_KNOB: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
            pub const TRAFFIC_GLYPH: [u8; 4] = [0x00, 0x00, 0x00, 0x99];
            pub const SHADOW_COLOR: [u8; 4] = [0x00, 0x00, 0x00, 0xff];
            pub const TRASH_FILL_TOP: [u8; 4] = [0x5b, 0x4e, 0x66, 0xff];
            pub const TRASH_FILL_BOTTOM: [u8; 4] = [0x2d, 0x25, 0x34, 0xff];
            pub const TRASH_RIM: [u8; 4] = [0x9c, 0x8f, 0xa8, 0xff];
            pub const TRASH_HIGHLIGHT: [u8; 4] = [0xc3, 0xb8, 0xcc, 0xff];
            pub const TRASH_PAPER: [u8; 4] = [0xde, 0xd6, 0xe5, 0xff];
            pub const TRASH_PAPER_EDGE: [u8; 4] = [0x5b, 0x4e, 0x66, 0xff];
            pub const FOLDER_TAB: [u8; 4] = [0x3a, 0x66, 0xe0, 0xff];
            pub const FOLDER_FILL_TOP: [u8; 4] = [0x4a, 0x7d, 0xff, 0xff];
            pub const FOLDER_FILL_BOTTOM: [u8; 4] = [0x3a, 0x66, 0xe0, 0xff];
            pub const FOLDER_RIM: [u8; 4] = [0x5b, 0x8d, 0xff, 0xff];
            pub const FOLDER_HIGHLIGHT: [u8; 4] = [0xde, 0xd6, 0xe5, 0xff];
            pub const DOCK_FILL: [u8; 4] = [0x2d, 0x25, 0x34, 0xff];
            pub const DOCK_FALLBACK_FILL: [u8; 4] = [0x2d, 0x25, 0x34, 0xff];
            pub const CHROME_FALLBACK_FILL: [u8; 4] = [0x2d, 0x25, 0x34, 0xff];
            pub const DOCK_RIM: [u8; 4] = [0xde, 0xd6, 0xe5, 0xff];
            pub const DOCK_BORDER: [u8; 4] = [0x44, 0x39, 0x4d, 0xff];
            pub const DOCK_SHADOW: [u8; 4] = [0x00, 0x00, 0x00, 0xff];
            pub const DOCK_HOVER_FILL: [u8; 4] = [0xc3, 0xb8, 0xcc, 0xff];
            pub const DOCK_DIVIDER: [u8; 4] = [0x9c, 0x8f, 0xa8, 0xff];
            pub const DOCK_INDICATOR: [u8; 4] = [0xf8, 0xf6, 0xfa, 0xff];
            pub const SKELETON_BASE: [u8; 4] = [0x2d, 0x25, 0x34, 0xff];
            pub const SKELETON_HIGHLIGHT: [u8; 4] = [0x44, 0x39, 0x4d, 0xff];
        }
        pub mod material {
            pub const CHROME_OPACITY: f32 = 0.72_f32;
            pub const CHROME_BLUR: f32 = 28.0_f32;
            pub const POPUP_OPACITY: f32 = 0.7_f32;
            pub const POPUP_BLUR: f32 = 32.0_f32;
            pub const SHADOW_OPACITY: f32 = 0.45_f32;
            pub const DOCK_OPACITY: f32 = 0.42_f32;
            pub const DOCK_BLUR: f32 = 34.0_f32;
            pub const CHROME_FALLBACK_OPACITY: f32 = 0.82_f32;
            pub const DOCK_FALLBACK_OPACITY: f32 = 0.84_f32;
            pub const POPUP_FALLBACK_OPACITY: f32 = 0.9_f32;
            pub const CHROME_REFRACTION: f32 = 8.0_f32;
            pub const POPUP_REFRACTION: f32 = 10.0_f32;
            pub const DOCK_REFRACTION: f32 = 10.0_f32;
            pub const CHROME_SPECULAR: f32 = 0.3_f32;
            pub const POPUP_SPECULAR: f32 = 0.34_f32;
            pub const DOCK_SPECULAR: f32 = 0.34_f32;
            pub const CHROME_SPECULAR_WIDTH: f32 = 2.0_f32;
            pub const POPUP_SPECULAR_WIDTH: f32 = 2.5_f32;
            pub const DOCK_SPECULAR_WIDTH: f32 = 2.5_f32;
            pub const CHROME_TINT: f32 = 0.14_f32;
            pub const POPUP_TINT: f32 = 0.18_f32;
            pub const DOCK_TINT: f32 = 0.2_f32;
        }
    }
}

pub mod component {
    pub mod window {
        pub const RADIUS: f32 = 14.0_f32;
        pub const BORDER_WIDTH: f32 = 1.0_f32;
        pub const RESIZE_BORDER: f32 = 6.0_f32;
        pub const RESIZE_CORNER: f32 = 12.0_f32;
        pub const SHADOW_BLUR: f32 = 40.0_f32;
    }
    pub mod window_style {
        pub const TRAFFIC_LIGHT_SIZE: f32 = 16.0_f32;
        pub const TRAFFIC_LIGHT_DIAMETER: f32 = 14.0_f32;
        pub const TRAFFIC_LIGHT_SPACING: f32 = 23.0_f32;
        pub const LABELLED_TOOLBAR_ROW: f32 = 14.0_f32;
        pub mod borderless {
            pub const HEIGHT: f32 = 0.0_f32;
            pub const TITLE_ROW: f32 = 0.0_f32;
            pub const TOOLBAR_ROW: f32 = 0.0_f32;
        }
        pub mod titlebar {
            pub const HEIGHT: f32 = 32.0_f32;
            pub const TITLE_ROW: f32 = 32.0_f32;
            pub const TOOLBAR_ROW: f32 = 0.0_f32;
        }
        pub mod unified_compact {
            pub const HEIGHT: f32 = 40.0_f32;
            pub const TITLE_ROW: f32 = 0.0_f32;
            pub const TOOLBAR_ROW: f32 = 40.0_f32;
        }
        pub mod expanded {
            pub const HEIGHT: f32 = 48.0_f32;
            pub const TITLE_ROW: f32 = 28.0_f32;
            pub const TOOLBAR_ROW: f32 = 20.0_f32;
        }
        pub mod unified {
            pub const HEIGHT: f32 = 52.0_f32;
            pub const TITLE_ROW: f32 = 0.0_f32;
            pub const TOOLBAR_ROW: f32 = 52.0_f32;
        }
        pub mod preference {
            pub const HEIGHT: f32 = 88.0_f32;
            pub const TITLE_ROW: f32 = 32.0_f32;
            pub const TOOLBAR_ROW: f32 = 56.0_f32;
        }
    }
    pub mod titlebar {
        pub const PADDING_H: f32 = 8.0_f32;
        pub const PADDING_V: f32 = 4.0_f32;
        pub const SPACING: f32 = 8.0_f32;
        pub const FONT_SIZE: f32 = 13.0_f32;
        pub const FONT_WEIGHT: f32 = 600.0_f32;
        pub const CORNER_RADIUS: f32 = 14.0_f32;
    }
    pub mod traffic_lights {
        pub const BUTTON_SIZE: f32 = 16.0_f32;
        pub const DIAMETER: f32 = 14.0_f32;
        pub const SPACING: f32 = 23.0_f32;
        pub const GLYPH_SIZE: f32 = 7.0_f32;
        pub const HOVER_REVEAL: bool = true;
    }
    pub mod toggle {
        pub const WIDTH: f32 = 40.0_f32;
        pub const HEIGHT: f32 = 24.0_f32;
        pub const KNOB: f32 = 18.0_f32;
        pub const INSET: f32 = 3.0_f32;
        pub const LABEL_GAP: f32 = 8.0_f32;
    }
    pub mod button {
        pub const HEIGHT: f32 = 28.0_f32;
        pub const PADDING_H: f32 = 12.0_f32;
        pub const RADIUS: f32 = 10.0_f32;
        pub const FONT_SIZE: f32 = 13.0_f32;
        pub const FONT_WEIGHT: f32 = 500.0_f32;
        pub const ICON_SIZE: f32 = 16.0_f32;
    }
    pub mod popup {
        pub const RADIUS: f32 = 14.0_f32;
        pub const PADDING: f32 = 8.0_f32;
        pub const MIN_WIDTH: f32 = 180.0_f32;
        pub const ROW_HEIGHT: f32 = 28.0_f32;
        pub const OFFSET: f32 = 6.0_f32;
        pub const SHADOW_BLUR: f32 = 20.0_f32;
    }
    pub mod menu_bar_menu {
        pub const BAR_HEIGHT: f32 = 24.0_f32;
        pub const BAR_PADDING_H: f32 = 10.0_f32;
        pub const ITEM_HEIGHT: f32 = 24.0_f32;
        pub const ROW_HEIGHT: f32 = 26.0_f32;
        pub const MIN_WIDTH: f32 = 200.0_f32;
        pub const PADDING: f32 = 6.0_f32;
        pub const SHORTCUT_GAP: f32 = 24.0_f32;
        pub const RADIUS: f32 = 14.0_f32;
    }
    pub mod menu_bar {
        pub const HEIGHT: f32 = 28.0_f32;
        pub const PADDING_H: f32 = 8.0_f32;
        pub const STATUS_ITEM_PADDING_H: f32 = 6.0_f32;
        pub const STATUS_ITEM_GAP: f32 = 2.0_f32;
        pub const ICON_SIZE: f32 = 15.0_f32;
        pub const FONT_SIZE: f32 = 13.0_f32;
        pub const LABEL_GAP: f32 = 5.0_f32;
        pub const CLOCK_GAP: f32 = 8.0_f32;
        pub const REVEAL_DELAY: f32 = 0.0_f32;
        pub const HIDE_DELAY: f32 = 350.0_f32;
        pub const RADIUS: f32 = 10.0_f32;
        pub const HOVER_RADIUS: f32 = 6.0_f32;
        pub const FOREGROUND_ON_LIGHT: [u8; 4] = [0x00, 0x00, 0x00, 0xff];
        pub const FOREGROUND_ON_DARK: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
        pub const FOREGROUND_SECONDARY_ON_LIGHT: [u8; 4] = [0x00, 0x00, 0x00, 0xff];
        pub const FOREGROUND_SECONDARY_ON_DARK: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
        pub const FOREGROUND_FLIP_THRESHOLD_LIGHT: f32 = 0.78_f32;
        pub const FOREGROUND_FLIP_THRESHOLD_DARK: f32 = 0.78_f32;
    }
    pub mod dock {
        pub const TOOLTIP_DWELL: f32 = 0.0_f32;
        pub const ICON_SIZE: f32 = 48.0_f32;
        pub const ICON_SIZE_MIN: f32 = 32.0_f32;
        pub const ICON_SIZE_MAX: f32 = 64.0_f32;
        pub const PADDING: f32 = 15.0_f32;
        pub const PADDING_ALONG: f32 = 16.0_f32;
        pub const GAP: f32 = 14.0_f32;
        pub const RADIUS: f32 = 27.0_f32;
        pub const INDICATOR_SIZE: f32 = 4.0_f32;
        pub const INDICATOR_GAP: f32 = 8.0_f32;
        pub const MAGNIFY_PEAK: f32 = 1.217_f32;
        pub const MAGNIFY_PEAK_MAX: f32 = 2.047_f32;
        pub const MAGNIFY_INFLUENCE: f32 = 125.1_f32;
        pub const LAUNCH_LIFT: f32 = 15.0_f32;
        pub const LABEL_SIZE: f32 = 12.0_f32;
        pub const TRASH_SIZE: f32 = 44.0_f32;
        pub const EDGE_MARGIN: f32 = 8.0_f32;
        pub const EDGE_TRIGGER: f32 = 4.0_f32;
        pub const REVEAL_DELAY: f32 = 120.0_f32;
        pub const HIDE_DELAY: f32 = 350.0_f32;
        pub mod plate {
            pub const FILL_OPACITY: f32 = 0.42_f32;
            pub const RIM_OPACITY: f32 = 0.84_f32;
            pub const RIM_HEIGHT: f32 = 1.0_f32;
            pub const GLOW_OUTER_HEIGHT: f32 = 6.0_f32;
            pub const GLOW_OUTER_OPACITY: f32 = 0.03_f32;
            pub const GLOW_MID_HEIGHT: f32 = 3.0_f32;
            pub const GLOW_MID_OPACITY: f32 = 0.06_f32;
            pub const GLOW_INNER_HEIGHT: f32 = 1.0_f32;
            pub const GLOW_INNER_OPACITY: f32 = 0.1_f32;
            pub const BORDER_WIDTH: f32 = 1.0_f32;
            pub const BORDER_OPACITY: f32 = 0.18_f32;
        }
        pub mod icon {
            pub const RADIUS_RATIO: f32 = 0.24_f32;
            pub const INSET: f32 = 0.0_f32;
        }
        pub mod hover {
            pub const FILL_OPACITY: f32 = 0.18_f32;
            pub const RADIUS_RATIO: f32 = 0.28_f32;
            pub const SHADOW_BLUR: f32 = 10.0_f32;
            pub const SHADOW_OPACITY: f32 = 0.28_f32;
        }
        pub mod indicator {
            pub const OPACITY: f32 = 0.9_f32;
            pub const INACTIVE_OPACITY: f32 = 0.45_f32;
        }
        pub mod divider {
            pub const OPACITY: f32 = 0.7_f32;
            pub const WIDTH: f32 = 1.0_f32;
            pub const HEIGHT_RATIO: f32 = 0.78_f32;
            pub const GAP: f32 = 22.0_f32;
        }
        pub mod chooser {
            pub const MAX_ROWS: f32 = 7.0_f32;
            pub const SCROLLBAR_WIDTH: f32 = 8.0_f32;
            pub const SCROLLBAR_MARGIN: f32 = 2.0_f32;
            pub const HOVER_DWELL: f32 = 280.0_f32;
            pub const HOVER_CLOSE_DELAY: f32 = 260.0_f32;
        }
        pub mod window_badge {
            pub const SIZE_RATIO: f32 = 0.34_f32;
            pub const SIZE_MIN: f32 = 14.0_f32;
            pub const SIZE_MAX: f32 = 22.0_f32;
            pub const FONT_RATIO: f32 = 0.56_f32;
            pub const FONT_MIN: f32 = 10.0_f32;
            pub const PADDING_H: f32 = 5.0_f32;
            pub const INSET: f32 = 2.0_f32;
            pub const BORDER_WIDTH: f32 = 1.0_f32;
        }
        pub mod app_badge {
            pub const SIZE_RATIO: f32 = 0.42_f32;
            pub const SIZE_MIN: f32 = 16.0_f32;
            pub const SIZE_MAX: f32 = 26.0_f32;
            pub const FONT_RATIO: f32 = 0.56_f32;
            pub const FONT_MIN: f32 = 10.0_f32;
            pub const PADDING_H: f32 = 5.0_f32;
            pub const OUTSIDE_RATIO: f32 = 0.18_f32;
            pub const BORDER_WIDTH: f32 = 1.0_f32;
        }
        pub mod overflow {
            pub const MAX_ROWS: f32 = 7.0_f32;
            pub const GRID_INSET: f32 = 0.18_f32;
            pub const GRID_GAP: f32 = 0.08_f32;
            pub const GRID_CELL: f32 = 0.16_f32;
        }
        pub mod trash_empty {
            pub const BUSY_DELAY: f32 = 350.0_f32;
            pub const SPINNER_SIZE: f32 = 18.0_f32;
            pub const SPINNER_STROKE: f32 = 2.0_f32;
            pub const SPINNER_SPEED: f32 = 900.0_f32;
        }
        pub mod minimize_reaction {
            pub const AMPLITUDE_RATIO: f32 = 0.28_f32;
            pub const DURATION: f32 = 320.0_f32;
            pub const REDUCED_DURATION: f32 = 0.0_f32;
        }
    }
    pub mod context_menu {
        pub const RADIUS: f32 = 14.0_f32;
        pub const PADDING: f32 = 6.0_f32;
        pub const ROW_HEIGHT: f32 = 26.0_f32;
        pub const MIN_WIDTH: f32 = 180.0_f32;
        pub const SHORTCUT_GAP: f32 = 24.0_f32;
        pub const SUBMENU_DELAY: f32 = 150.0_f32;
        pub const ATTACHED_RADIUS: f32 = 12.5_f32;
        pub const CLOSE_SCALE: f32 = 0.95_f32;
    }
    pub mod shortcut_hint {
        pub const GAP: f32 = 2.0_f32;
        pub const FONT_SIZE: f32 = 12.0_f32;
        pub const MODIFIER_WEIGHT: f32 = 400.0_f32;
        pub const KEY_WEIGHT: f32 = 500.0_f32;
    }
    pub mod focus_ring {
        pub const WIDTH: f32 = 2.0_f32;
        pub const OFFSET: f32 = 2.0_f32;
        pub const RADIUS: f32 = 6.0_f32;
    }
    pub mod marquee {
        pub const BORDER_WIDTH: f32 = 2.0_f32;
        pub const RADIUS: f32 = 0.0_f32;
    }
    pub mod sidebar {
        pub const WIDTH: f32 = 192.0_f32;
        pub const ROW_HEIGHT: f32 = 28.0_f32;
        pub const ROW_RADIUS: f32 = 6.0_f32;
        pub const SECTION_GAP: f32 = 16.0_f32;
        pub const ICON_SIZE: f32 = 16.0_f32;
        pub const CATEGORY_ICON_SIZE: f32 = 20.0_f32;
        pub const PADDING: f32 = 8.0_f32;
    }
    pub mod toolbar {
        pub const HEIGHT: f32 = 52.0_f32;
        pub const PADDING_H: f32 = 12.0_f32;
        pub const SPACING: f32 = 8.0_f32;
        pub const BUTTON_SIZE: f32 = 28.0_f32;
        pub const RADIUS: f32 = 10.0_f32;
    }
    pub mod files {
        pub const SIDEBAR_WIDTH: f32 = 198.0_f32;
        pub const VIEW_GROUP_WIDTH: f32 = 164.0_f32;
        pub const VIEW_GROUP_HEIGHT: f32 = 52.0_f32;
        pub const VIEW_SEGMENT_WIDTH: f32 = 40.0_f32;
        pub const VIEW_SEGMENT_HEIGHT: f32 = 36.0_f32;
        pub const VIEW_GROUP_PADDING_X: f32 = 2.0_f32;
        pub const VIEW_GROUP_PADDING_Y: f32 = 8.0_f32;
        pub const TOOLBAR_HIDDEN_INSET: f32 = 64.0_f32;
        pub const GET_INFO_WIDTH: f32 = 328.0_f32;
        pub const GET_INFO_HEIGHT: f32 = 1034.0_f32;
        pub const GET_INFO_CHROME_HEIGHT: f32 = 32.0_f32;
    }
    pub mod accessory_bar {
        pub const TAB: f32 = 28.0_f32;
        pub const PATH: f32 = 30.0_f32;
        pub const STATUS: f32 = 28.0_f32;
        pub const FORMAT: f32 = 20.0_f32;
    }
    pub mod split_view {
        pub const DIVIDER_WIDTH: f32 = 1.0_f32;
        pub const MIN_PANE_WIDTH: f32 = 180.0_f32;
    }
    pub mod settings_row {
        pub const HEIGHT: f32 = 44.0_f32;
        pub const PADDING_H: f32 = 12.0_f32;
        pub const LABEL_WIDTH: f32 = 200.0_f32;
        pub const CONTROL_GAP: f32 = 16.0_f32;
    }
    pub mod settings_group {
        pub const RADIUS: f32 = 10.0_f32;
        pub const PADDING: f32 = 8.0_f32;
        pub const ROW_GAP: f32 = 1.0_f32;
        pub const MARGIN_BOTTOM: f32 = 16.0_f32;
    }
    pub mod segmented_control {
        pub const HEIGHT: f32 = 28.0_f32;
        pub const RADIUS: f32 = 10.0_f32;
        pub const PADDING: f32 = 2.0_f32;
        pub const SEGMENT_MIN_WIDTH: f32 = 64.0_f32;
        pub const FONT_SIZE: f32 = 13.0_f32;
    }
    pub mod slider {
        pub const HEIGHT: f32 = 24.0_f32;
        pub const TRACK_HEIGHT: f32 = 4.0_f32;
        pub const KNOB: f32 = 16.0_f32;
        pub const MIN_WIDTH: f32 = 160.0_f32;
        pub const CAPTION_GAP: f32 = 2.0_f32;
        pub const STEP: f32 = 0.05_f32;
    }
    pub mod select {
        pub const HEIGHT: f32 = 28.0_f32;
        pub const RADIUS: f32 = 10.0_f32;
        pub const PADDING_H: f32 = 12.0_f32;
        pub const MIN_WIDTH: f32 = 80.0_f32;
        pub const CHEVRON_SIZE: f32 = 12.0_f32;
        pub const CHEVRON_GAP: f32 = 8.0_f32;
    }
    pub mod search_field {
        pub const HEIGHT: f32 = 28.0_f32;
        pub const RADIUS: f32 = 999.0_f32;
        pub const PADDING_H: f32 = 8.0_f32;
        pub const ICON_SIZE: f32 = 14.0_f32;
        pub const MIN_WIDTH: f32 = 160.0_f32;
    }
    pub mod source_list {
        pub const ROW_HEIGHT: f32 = 26.0_f32;
        pub const ROW_RADIUS: f32 = 6.0_f32;
        pub const INDENT: f32 = 12.0_f32;
    }
    pub mod dialog {
        pub const RADIUS: f32 = 14.0_f32;
        pub const PADDING: f32 = 16.0_f32;
        pub const MIN_WIDTH: f32 = 320.0_f32;
        pub const BUTTON_GAP: f32 = 8.0_f32;
    }
    pub mod sheet {
        pub const RADIUS: f32 = 14.0_f32;
        pub const PADDING: f32 = 16.0_f32;
        pub const WIDTH: f32 = 420.0_f32;
    }
    pub mod popover {
        pub const RADIUS: f32 = 14.0_f32;
        pub const PADDING: f32 = 12.0_f32;
        pub const ARROW_SIZE: f32 = 8.0_f32;
        pub const SHADOW_BLUR: f32 = 40.0_f32;
    }
    pub mod tooltip {
        pub const DWELL: f32 = 1400.0_f32;
        pub const OFFSET: f32 = 8.0_f32;
        pub const RADIUS: f32 = 999.0_f32;
        pub const PADDING_H: f32 = 12.0_f32;
        pub const PADDING_V: f32 = 8.0_f32;
        pub const MAX_WIDTH: f32 = 260.0_f32;
        pub const FONT_SIZE: f32 = 13.0_f32;
        pub const TAIL_WIDTH: f32 = 16.0_f32;
        pub const TAIL_HEIGHT: f32 = 6.0_f32;
        pub const RIM_COLOR: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
        pub const RIM_OPACITY: f32 = 0.5_f32;
    }
    pub mod shadow {
        pub const LAYERS: f32 = 24.0_f32;
        pub const OFFSET_Y: f32 = 5.0_f32;
    }
    pub mod elevation {
        pub mod low {
            pub const BLUR: f32 = 8.0_f32;
            pub const OFFSET_Y: f32 = 5.0_f32;
            pub const LAYERS: f32 = 24.0_f32;
        }
        pub mod med {
            pub const BLUR: f32 = 20.0_f32;
            pub const OFFSET_Y: f32 = 5.0_f32;
            pub const LAYERS: f32 = 24.0_f32;
        }
        pub mod high {
            pub const BLUR: f32 = 40.0_f32;
            pub const OFFSET_Y: f32 = 5.0_f32;
            pub const LAYERS: f32 = 24.0_f32;
        }
        pub mod overlay {
            pub const BLUR: f32 = 64.0_f32;
            pub const OFFSET_Y: f32 = 5.0_f32;
            pub const LAYERS: f32 = 24.0_f32;
        }
    }
    pub mod scroll_view {
        pub const SCROLLBAR_WIDTH: f32 = 8.0_f32;
        pub const SCROLLBAR_MARGIN: f32 = 2.0_f32;
        pub const SCROLLBAR_RADIUS: f32 = 999.0_f32;
        pub const MIN_THUMB: f32 = 24.0_f32;
    }
    pub mod overview {
        pub const STRIP_GAP: f32 = 12.0_f32;
        pub const STRIP_MARGIN: f32 = 16.0_f32;
        pub const CARD_WIDTH: f32 = 132.0_f32;
        pub const CARD_HEIGHT: f32 = 84.0_f32;
        pub const CARD_PADDING: f32 = 8.0_f32;
        pub const CARD_RADIUS: f32 = 14.0_f32;
        pub const CHIP_HEIGHT: f32 = 36.0_f32;
        pub const CHIP_RADIUS: f32 = 10.0_f32;
        pub const CHIP_GAP: f32 = 8.0_f32;
        pub const CHIP_PADDING: f32 = 10.0_f32;
        pub const FONT_SIZE: f32 = 13.0_f32;
        pub const TITLE_SIZE: f32 = 12.0_f32;
        pub const SCRIM_OPACITY: f32 = 0.18_f32;
    }
    pub mod osd {
        pub const WIDTH: f32 = 180.0_f32;
        pub const HEIGHT: f32 = 180.0_f32;
        pub const RADIUS: f32 = 20.0_f32;
        pub const PADDING: f32 = 16.0_f32;
        pub const ICON_SIZE: f32 = 44.0_f32;
        pub const GAP: f32 = 8.0_f32;
        pub const TRACK_HEIGHT: f32 = 6.0_f32;
        pub const TRACK_RADIUS: f32 = 999.0_f32;
        pub const FONT_SIZE: f32 = 13.0_f32;
    }
    pub mod skeleton {
        pub const RADIUS: f32 = 10.0_f32;
        pub const WIDTH: f32 = 120.0_f32;
        pub const HEIGHT: f32 = 72.0_f32;
        pub const HIGHLIGHT_RATIO: f32 = 0.4_f32;
    }
    pub mod spinner {
        pub const SIZE: f32 = 16.0_f32;
        pub const STROKE_WIDTH: f32 = 2.0_f32;
        pub const SPEED: f32 = 900.0_f32;
        pub const TRACK_OPACITY: f32 = 0.35_f32;
    }
    pub mod wallpaper {
        pub const COLUMNS: f32 = 4.0_f32;
        pub const TILE_GAP: f32 = 10.0_f32;
        pub const TILE_ASPECT: f32 = 1.5_f32;
        pub const APPLY_MIN_MS: f32 = 450.0_f32;
        pub const HERO_WIDTH: f32 = 160.0_f32;
        pub const HERO_HEIGHT: f32 = 100.0_f32;
    }
    pub mod settings_category {
        pub const SIZE: f32 = 32.0_f32;
        pub const RADIUS_RATIO: f32 = 0.24_f32;
        pub const ICON_RATIO: f32 = 0.55_f32;
        pub const HIGHLIGHT_RATIO: f32 = 0.45_f32;
        pub const HIGHLIGHT_OPACITY: f32 = 0.28_f32;
        pub const GLOW_OPACITY: f32 = 0.42_f32;
    }
    pub mod apps_drawer {
        pub const CONTENT_PADDING: f32 = 64.0_f32;
        pub const PANEL_PADDING: f32 = 50.0_f32;
        pub const PANEL_PADDING_VERTICAL: f32 = 32.0_f32;
        pub const SEARCH_WIDTH: f32 = 360.0_f32;
        pub const SEARCH_MAX_WIDTH: f32 = 640.0_f32;
        pub const PILL_GAP: f32 = 8.0_f32;
        pub const TILE_SIZE: f32 = 64.0_f32;
        pub const TILE_RADIUS: f32 = 14.0_f32;
        pub const TILE_CONTENT_GAP: f32 = 8.0_f32;
        pub const GRID_COLUMN_GAP: f32 = 24.0_f32;
        pub const GRID_ROW_GAP: f32 = 16.0_f32;
        pub const LABEL_SIZE: f32 = 12.0_f32;
        pub const SCRIM_OPACITY: f32 = 0.35_f32;
        pub const PANEL_OPACITY: f32 = 0.78_f32;
        pub const FALLBACK_GLYPH_RATIO: f32 = 0.6_f32;
        pub const EMPTY_GLYPH_SIZE: f32 = 40.0_f32;
    }
}

pub mod glass {
    pub const VERSION: &str = "1";
    pub mod rollout {
        pub const DEFAULT_MODE: &str = "replacement";
        pub const LEGACY_MODE: &str = "legacy";
        pub const REPLACEMENT_MODE: &str = "replacement";
        pub const ENV_VAR: &str = "DRAGONFRUIT_GLASS_ROLLOUT";
    }
    pub mod slider {
        pub const DEFAULT_POSITION: f32 = 0.5255_f32;
        pub mod light_anchors {
            pub mod low {
                pub const POSITION: f32 = 0.0_f32;
                pub const ALPHA: f32 = 0.732_f32;
                pub const TINT: f32 = 0.953_f32;
            }
            pub mod mid {
                pub const POSITION: f32 = 0.5255_f32;
                pub const ALPHA: f32 = 0.552_f32;
                pub const TINT: f32 = 0.993_f32;
            }
            pub mod high {
                pub const POSITION: f32 = 1.0_f32;
                pub const ALPHA: f32 = 0.415_f32;
                pub const TINT: f32 = 0.997_f32;
            }
        }
    }
    pub mod accessibility {
        pub mod reduce_transparency {
            pub mod light {
                pub const ALPHA: f32 = 0.98_f32;
                pub const TINT: f32 = 0.97_f32;
                pub const COLOUR_SPACE: &str = "srgb";
            }
            pub mod dark {
                pub const ALPHA: f32 = 0.96_f32;
                pub const TINT: f32 = 0.1_f32;
                pub const COLOUR_SPACE: &str = "srgb";
            }
        }
        pub mod increase_contrast {
            pub const EDGE_BOOST: f32 = 0.35_f32;
            pub const FOREGROUND_BOOST: f32 = 0.2_f32;
            pub const EDGE_WIDTH_BOOST: f32 = 0.5_f32;
        }
    }
    pub mod presets {
        pub mod dock {
            pub const ROLE: &str = "dock";
            pub const RENDER: &str = "sampled";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md Dock plate: light neutral fit alpha 0.253 / tint 0.604; one uniform rim peak 0.397 on mid-grey (first covered pixel), half-amplitude 1pt, 10% at 2.5pt, 6pt tail; corner-lens peak is the 45-degree SDF-normal displacement whose horizontal component is 9.6pt at 15pt inside; blur sigma 3.86pt first fit";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.253_f32;
                    pub const NEUTRAL_TINT: f32 = 0.604_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.253_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.604_f32;
                }
                pub const BLUR: f32 = 3.86_f32;
                pub const EDGE: f32 = 0.397_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const EDGE_TAIL: f32 = 6.0_f32;
                pub const SHARP_FRACTION: f32 = 0.12_f32;
                pub const CORNER_LENS_PEAK: f32 = 13.6_f32;
                pub const CORNER_LENS_DISTANCE: f32 = 15.0_f32;
                pub const CORNER_LENS_TAIL: f32 = 50.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md Dock plate: dark neutral fit alpha 0.149 / tint 0.207; one uniform rim peak 0.397 on mid-grey (first covered pixel), corner-lens peak is the 45-degree SDF-normal displacement whose horizontal component is 9.6pt at 15pt inside; blur and rim reuse the light fit because the dark rim was not separately fitted";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.149_f32;
                    pub const NEUTRAL_TINT: f32 = 0.207_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.149_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.207_f32;
                }
                pub const BLUR: f32 = 3.86_f32;
                pub const EDGE: f32 = 0.397_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const EDGE_TAIL: f32 = 6.0_f32;
                pub const SHARP_FRACTION: f32 = 0.12_f32;
                pub const CORNER_LENS_PEAK: f32 = 13.6_f32;
                pub const CORNER_LENS_DISTANCE: f32 = 15.0_f32;
                pub const CORNER_LENS_TAIL: f32 = 50.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod menu_bar_veil {
            pub const ROLE: &str = "chrome";
            pub const RENDER: &str = "veil";
            pub const SLIDER: &str = "none";
            pub const STRIP_HEIGHT: f32 = 39.0_f32;
            pub const EXTENT: f32 = 95.0_f32;
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md menu-bar veil: alpha 0.451 at y=5pt, then 0.439/0.408/0.357/0.243/0.059/0.008 to y=90pt; white background shows no visible veil; the veil is not a blur";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.451_f32;
                    pub const NEUTRAL_TINT: f32 = 1.0_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.451_f32;
                    pub const PERCEPTUAL_TINT: f32 = 1.0_f32;
                }
                pub mod veil {
                    pub const PROBE: f32 = 8.0_f32;
                    pub const ALPHA_5: f32 = 0.45098037_f32;
                    pub const ALPHA_15: f32 = 0.43921566_f32;
                    pub const ALPHA_25: f32 = 0.40784317_f32;
                    pub const ALPHA_35: f32 = 0.35686272_f32;
                    pub const ALPHA_50: f32 = 0.24313729_f32;
                    pub const ALPHA_75: f32 = 0.05882353_f32;
                    pub const ALPHA_90: f32 = 0.00784314_f32;
                    pub const ALPHA_95: f32 = 0.0_f32;
                    pub const ADAPT_LUMA: [f32; 5] = [0.0_f32, 0.25_f32, 0.5_f32, 0.78_f32, 1.0_f32];
                    pub const ADAPT_SCALE: [f32; 5] = [1.0_f32, 1.06_f32, 0.91_f32, 0.48_f32, 0.0_f32];
                }
                pub const BLUR: f32 = 0.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "baseline";
                pub const PROVENANCE: &str = "No standalone dark veil was measured in the first campaign; the coverage dark captures over neutral black/mid/white give a top alpha of ~0.125 over mid-grey and zero over white. Dragonfruit uses the measured light depth shape at a 0.48 peak, scaled by 0.286 of the light luminance response to match the mid-grey capture.";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.48_f32;
                    pub const NEUTRAL_TINT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.48_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.0_f32;
                }
                pub mod veil {
                    pub const PROBE: f32 = 8.0_f32;
                    pub const ALPHA_5: f32 = 0.48_f32;
                    pub const ALPHA_15: f32 = 0.4674_f32;
                    pub const ALPHA_25: f32 = 0.434_f32;
                    pub const ALPHA_35: f32 = 0.3798_f32;
                    pub const ALPHA_50: f32 = 0.2588_f32;
                    pub const ALPHA_75: f32 = 0.0626_f32;
                    pub const ALPHA_90: f32 = 0.00835_f32;
                    pub const ALPHA_95: f32 = 0.0_f32;
                    pub const ADAPT_LUMA: [f32; 5] = [0.0_f32, 0.25_f32, 0.5_f32, 0.78_f32, 1.0_f32];
                    pub const ADAPT_SCALE: [f32; 5] = [0.286_f32, 0.30316_f32, 0.26026_f32, 0.13728_f32, 0.0_f32];
                }
                pub const BLUR: f32 = 0.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod menu_attached {
            pub const ROLE: &str = "popup";
            pub const RENDER: &str = "sampled";
            pub const SLIDER: &str = "lightAttached";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md System Settings light File anchors at 0/0.5255/1 -> alpha 0.732/0.552/0.415, tint 0.953/0.993/0.997; File blur sigma ~10pt";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.552_f32;
                    pub const NEUTRAL_TINT: f32 = 0.993_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.552_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.993_f32;
                }
                pub const BLUR: f32 = 10.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "adapted";
                pub const PROVENANCE: &str = "The published dark File fit is contaminated by a light crop (REFERENCE.md); the attached-menu baseline is anchored to the valid dark mid/black context/status family (alpha 0.66 / tint 0.26), mixed 25% toward a deeper adapted baseline. The white row stays open for a controlled recapture.";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.65_f32;
                    pub const NEUTRAL_TINT: f32 = 0.27_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.25_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.62_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.3_f32;
                }
                pub const BLUR: f32 = 10.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod menu_detached {
            pub const ROLE: &str = "popup";
            pub const RENDER: &str = "sampled";
            pub const SLIDER: &str = "lightRelative";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md common light status/context menus cluster near alpha 0.55 and near-white tint; detached menus are rounded cards";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.55_f32;
                    pub const NEUTRAL_TINT: f32 = 0.96_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.55_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.96_f32;
                }
                pub const BLUR: f32 = 10.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md dark Desktop/Wi-Fi/Bluetooth/Battery fits near alpha 0.66 / tint 0.26 with substantial residuals; this is the detached-menu baseline";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.66_f32;
                    pub const NEUTRAL_TINT: f32 = 0.26_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.66_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.26_f32;
                }
                pub const BLUR: f32 = 10.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod status_menu {
            pub const ROLE: &str = "popup";
            pub const RENDER: &str = "sampled";
            pub const SLIDER: &str = "lightRelative";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md light status menus join the common context-menu cluster; 8pt blur is a local adaptation";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.55_f32;
                    pub const NEUTRAL_TINT: f32 = 0.96_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.55_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.96_f32;
                }
                pub const BLUR: f32 = 8.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md dark Wi-Fi/Bluetooth/Battery status fits near alpha 0.66 / tint 0.26; 8pt blur is a local adaptation";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.66_f32;
                    pub const NEUTRAL_TINT: f32 = 0.26_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.66_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.26_f32;
                }
                pub const BLUR: f32 = 8.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod tooltip {
            pub const ROLE: &str = "popup";
            pub const RENDER: &str = "sampled";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "adapted";
                pub const PROVENANCE: &str = "No direct tooltip optical fit; the reference tooltip is 33pt high and 8pt above the Dock plate. Named popup baseline 0.72/0.92";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.72_f32;
                    pub const NEUTRAL_TINT: f32 = 0.92_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.72_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.92_f32;
                }
                pub const BLUR: f32 = 6.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "adapted";
                pub const PROVENANCE: &str = "No dark tooltip fit; named popup baseline 0.76/0.24";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.76_f32;
                    pub const NEUTRAL_TINT: f32 = 0.24_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.76_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.24_f32;
                }
                pub const BLUR: f32 = 6.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod control_center_outer {
            pub const ROLE: &str = "popup";
            pub const RENDER: &str = "sampled";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md later controlled Control Center light outer fit alpha 0.265 / tint 0.373; REPORT.md 3.5/Appendix A give the 8 pt blur, 0.30 rim and 6 pt/24 pt capsule-end lens";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.265_f32;
                    pub const NEUTRAL_TINT: f32 = 0.373_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.265_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.373_f32;
                }
                pub const BLUR: f32 = 8.0_f32;
                pub const EDGE: f32 = 0.3_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const EDGE_TAIL: f32 = 6.0_f32;
                pub const SHARP_FRACTION: f32 = 0.1_f32;
                pub const CORNER_LENS_PEAK: f32 = 6.0_f32;
                pub const CORNER_LENS_DISTANCE: f32 = 24.0_f32;
                pub const CORNER_LENS_TAIL: f32 = 50.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md later controlled Control Center dark outer fit alpha 0.489 / tint 0.677; blur/rim/lens reuse the light capsule fit because the dark end caustic was not separately measured";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.489_f32;
                    pub const NEUTRAL_TINT: f32 = 0.677_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.489_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.677_f32;
                }
                pub const BLUR: f32 = 8.0_f32;
                pub const EDGE: f32 = 0.3_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const EDGE_TAIL: f32 = 6.0_f32;
                pub const SHARP_FRACTION: f32 = 0.1_f32;
                pub const CORNER_LENS_PEAK: f32 = 6.0_f32;
                pub const CORNER_LENS_DISTANCE: f32 = 24.0_f32;
                pub const CORNER_LENS_TAIL: f32 = 50.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod control_center_expanded_outer {
            pub const ROLE: &str = "popup";
            pub const RENDER: &str = "sampled";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md controlled Control Center expanded-outer fit alpha 0.286 / tint 0.257 (SURFACE-REVIEW 6.2/6.4); the sheet grows the outer panel behind the module";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.286_f32;
                    pub const NEUTRAL_TINT: f32 = 0.257_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.286_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.257_f32;
                }
                pub const BLUR: f32 = 8.0_f32;
                pub const EDGE: f32 = 0.3_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const EDGE_TAIL: f32 = 6.0_f32;
                pub const SHARP_FRACTION: f32 = 0.1_f32;
                pub const CORNER_LENS_PEAK: f32 = 6.0_f32;
                pub const CORNER_LENS_DISTANCE: f32 = 24.0_f32;
                pub const CORNER_LENS_TAIL: f32 = 50.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "adapted";
                pub const PROVENANCE: &str = "No independent dark expanded-outer fit; adapted 0.50/0.60 so the expanded plate stays darker than the module capsules and lighter than the collapsed dark outer plate";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.5_f32;
                    pub const NEUTRAL_TINT: f32 = 0.6_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.5_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.6_f32;
                }
                pub const BLUR: f32 = 8.0_f32;
                pub const EDGE: f32 = 0.3_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const EDGE_TAIL: f32 = 6.0_f32;
                pub const SHARP_FRACTION: f32 = 0.1_f32;
                pub const CORNER_LENS_PEAK: f32 = 6.0_f32;
                pub const CORNER_LENS_DISTANCE: f32 = 24.0_f32;
                pub const CORNER_LENS_TAIL: f32 = 50.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod control_center_module {
            pub const ROLE: &str = "popup";
            pub const RENDER: &str = "sampled";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "measured-effective";
                pub const PROVENANCE: &str = "REFERENCE.md Wi-Fi module light fit 0.391/0.317 is an observed effective stacked response. The compositor flat-band output is opaque, so this preset is the module's final composite in its own region; it is not layered a second time over the outer plate";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.391_f32;
                    pub const NEUTRAL_TINT: f32 = 0.317_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.391_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.317_f32;
                }
                pub const BLUR: f32 = 8.0_f32;
                pub const EDGE: f32 = 0.3_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const EDGE_TAIL: f32 = 6.0_f32;
                pub const SHARP_FRACTION: f32 = 0.1_f32;
                pub const CORNER_LENS_PEAK: f32 = 6.0_f32;
                pub const CORNER_LENS_DISTANCE: f32 = 24.0_f32;
                pub const CORNER_LENS_TAIL: f32 = 50.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "adapted";
                pub const PROVENANCE: &str = "No reliable dark module decomposition; adapted effective 0.65/0.44 so the opaque module reads darker than the 0.489/0.677 dark outer plate";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.65_f32;
                    pub const NEUTRAL_TINT: f32 = 0.44_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.65_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.44_f32;
                }
                pub const BLUR: f32 = 8.0_f32;
                pub const EDGE: f32 = 0.3_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const EDGE_TAIL: f32 = 6.0_f32;
                pub const SHARP_FRACTION: f32 = 0.1_f32;
                pub const CORNER_LENS_PEAK: f32 = 6.0_f32;
                pub const CORNER_LENS_DISTANCE: f32 = 24.0_f32;
                pub const CORNER_LENS_TAIL: f32 = 50.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod control_center_expanded {
            pub const ROLE: &str = "popup";
            pub const RENDER: &str = "sampled";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "adapted";
                pub const PROVENANCE: &str = "The expanded list is unmeasured in light; adapted readable near-opaque light plate (0.94/0.96) so dark list text stays legible";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.94_f32;
                    pub const NEUTRAL_TINT: f32 = 0.96_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.94_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.96_f32;
                }
                pub const BLUR: f32 = 8.0_f32;
                pub const EDGE: f32 = 0.22_f32;
                pub const EDGE_WIDTH: f32 = 0.75_f32;
                pub const EDGE_TAIL: f32 = 4.0_f32;
                pub const SHARP_FRACTION: f32 = 0.05_f32;
                pub const CORNER_LENS_PEAK: f32 = 4.0_f32;
                pub const CORNER_LENS_DISTANCE: f32 = 16.0_f32;
                pub const CORNER_LENS_TAIL: f32 = 32.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "observed-luma";
                pub const PROVENANCE: &str = "REFERENCE.md Bluetooth expanded list is dark (~0.13 luma) with no reliable alpha decomposition; near-opaque 0.96/0.13 keeps the composite within 0.04 luma over white/black";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.96_f32;
                    pub const NEUTRAL_TINT: f32 = 0.13_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.96_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.13_f32;
                }
                pub const BLUR: f32 = 8.0_f32;
                pub const EDGE: f32 = 0.22_f32;
                pub const EDGE_WIDTH: f32 = 0.75_f32;
                pub const EDGE_TAIL: f32 = 4.0_f32;
                pub const SHARP_FRACTION: f32 = 0.05_f32;
                pub const CORNER_LENS_PEAK: f32 = 4.0_f32;
                pub const CORNER_LENS_DISTANCE: f32 = 16.0_f32;
                pub const CORNER_LENS_TAIL: f32 = 32.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod notification_banner {
            pub const ROLE: &str = "popup";
            pub const RENDER: &str = "sampled";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md light notification banner 0.549/0.976";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.549_f32;
                    pub const NEUTRAL_TINT: f32 = 0.976_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.549_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.976_f32;
                }
                pub const BLUR: f32 = 14.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "adapted";
                pub const PROVENANCE: &str = "T205: the dark banner is uncovered as an independent fit (underconstrained); adapted readable 0.56/0.30 so it reads lighter than the dark stack and keeps light-on-glass text contrast";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.56_f32;
                    pub const NEUTRAL_TINT: f32 = 0.3_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.56_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.3_f32;
                }
                pub const BLUR: f32 = 14.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod notification_stack {
            pub const ROLE: &str = "popup";
            pub const RENDER: &str = "sampled";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md light notification stack 0.551/0.706; the stack residual is materially larger than the banner's";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.551_f32;
                    pub const NEUTRAL_TINT: f32 = 0.706_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.551_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.706_f32;
                }
                pub const BLUR: f32 = 14.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md dark notification stack 0.507/0.550";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.507_f32;
                    pub const NEUTRAL_TINT: f32 = 0.55_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.507_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.55_f32;
                }
                pub const BLUR: f32 = 14.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod notification_lower_panel {
            pub const ROLE: &str = "popup";
            pub const RENDER: &str = "sampled";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md light notification lower panel 0.407/0.852; the residual is materially larger than the banner's";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.407_f32;
                    pub const NEUTRAL_TINT: f32 = 0.852_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.407_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.852_f32;
                }
                pub const BLUR: f32 = 18.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "adapted";
                pub const PROVENANCE: &str = "T205: the dark lower panel is uncovered as an independent fit (underconstrained); adapted readable 0.40/0.30 so it reads below the dark stack on black/mid/white and stays distinct from the dark banner";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.4_f32;
                    pub const NEUTRAL_TINT: f32 = 0.3_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.4_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.3_f32;
                }
                pub const BLUR: f32 = 18.0_f32;
                pub const EDGE: f32 = 0.0_f32;
                pub const EDGE_WIDTH: f32 = 0.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod window_chrome {
            pub const ROLE: &str = "chrome";
            pub const RENDER: &str = "solid";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md light System Settings sidebar/toolbar 0.986/0.857 and 0.961/0.965; Finder 0.985/0.858 and 0.994/0.922. Named near-opaque 0.975/0.91";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.975_f32;
                    pub const NEUTRAL_TINT: f32 = 0.91_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.975_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.91_f32;
                }
                pub const BLUR: f32 = 0.0_f32;
                pub const EDGE: f32 = 0.08_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "REFERENCE.md dark Finder sidebar/toolbar 0.935/0.218 and 0.900/0.247. Named near-opaque 0.918/0.233";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.918_f32;
                    pub const NEUTRAL_TINT: f32 = 0.233_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.918_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.233_f32;
                }
                pub const BLUR: f32 = 0.0_f32;
                pub const EDGE: f32 = 0.08_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod settings_sidebar {
            pub const ROLE: &str = "chrome";
            pub const RENDER: &str = "solid";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "T206 REFERENCE.md light System Settings sidebar 0.986/0.857. The muted sidebar shows a small chroma pickup; the preset stores only a scalar neutral tint, so the chroma component stays an explicit transport gap (ADR 0225).";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.986_f32;
                    pub const NEUTRAL_TINT: f32 = 0.857_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.986_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.857_f32;
                }
                pub const BLUR: f32 = 0.0_f32;
                pub const EDGE: f32 = 0.08_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "adapted";
                pub const PROVENANCE: &str = "T206 Settings Dark is not independently measured; adapted from the measured Finder dark sidebar 0.935/0.218 (REFERENCE.md) because System Settings and Finder share the near-opaque source-list material.";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.935_f32;
                    pub const NEUTRAL_TINT: f32 = 0.218_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.935_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.218_f32;
                }
                pub const BLUR: f32 = 0.0_f32;
                pub const EDGE: f32 = 0.08_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod settings_toolbar {
            pub const ROLE: &str = "chrome";
            pub const RENDER: &str = "solid";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "T206 REFERENCE.md light System Settings toolbar 0.961/0.965. Near-opaque, so the scalar tint carries the visible result; no chroma channel is stored.";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.961_f32;
                    pub const NEUTRAL_TINT: f32 = 0.965_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.961_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.965_f32;
                }
                pub const BLUR: f32 = 0.0_f32;
                pub const EDGE: f32 = 0.08_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "adapted";
                pub const PROVENANCE: &str = "T206 Settings Dark is not independently measured; adapted from the measured Finder dark toolbar 0.900/0.247 (REFERENCE.md).";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.9_f32;
                    pub const NEUTRAL_TINT: f32 = 0.247_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.9_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.247_f32;
                }
                pub const BLUR: f32 = 0.0_f32;
                pub const EDGE: f32 = 0.08_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod files_sidebar {
            pub const ROLE: &str = "chrome";
            pub const RENDER: &str = "solid";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "T206 REFERENCE.md light Finder sidebar 0.985/0.858. The muted sidebar shows the measured small chroma pickup; the scalar preset cannot carry it (ADR 0225).";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.985_f32;
                    pub const NEUTRAL_TINT: f32 = 0.858_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.985_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.858_f32;
                }
                pub const BLUR: f32 = 0.0_f32;
                pub const EDGE: f32 = 0.08_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "T206 REFERENCE.md dark Finder sidebar 0.935/0.218.";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.935_f32;
                    pub const NEUTRAL_TINT: f32 = 0.218_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.935_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.218_f32;
                }
                pub const BLUR: f32 = 0.0_f32;
                pub const EDGE: f32 = 0.08_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
        pub mod files_toolbar {
            pub const ROLE: &str = "chrome";
            pub const RENDER: &str = "solid";
            pub const SLIDER: &str = "none";
            pub mod light {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "T206 REFERENCE.md light Finder toolbar 0.994/0.922.";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.994_f32;
                    pub const NEUTRAL_TINT: f32 = 0.922_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.994_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.922_f32;
                }
                pub const BLUR: f32 = 0.0_f32;
                pub const EDGE: f32 = 0.08_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
            pub mod dark {
                pub const CONFIDENCE: &str = "measured";
                pub const PROVENANCE: &str = "T206 REFERENCE.md dark Finder toolbar 0.900/0.247.";
                pub const COLOUR_SPACE: &str = "srgb";
                pub mod calibration {
                    pub const NEUTRAL_ALPHA: f32 = 0.9_f32;
                    pub const NEUTRAL_TINT: f32 = 0.247_f32;
                    pub const PERCEPTUAL_WEIGHT: f32 = 0.0_f32;
                    pub const PERCEPTUAL_ALPHA: f32 = 0.9_f32;
                    pub const PERCEPTUAL_TINT: f32 = 0.247_f32;
                }
                pub const BLUR: f32 = 0.0_f32;
                pub const EDGE: f32 = 0.08_f32;
                pub const EDGE_WIDTH: f32 = 1.0_f32;
                pub const FOREGROUND: f32 = 0.0_f32;
            }
        }
    }
}

/// A named motion: full duration, cubic-bezier control points, and
/// the reduced-motion duration (0 = state change is instant).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    pub duration_ms: u32,
    pub curve: [f32; 4],
    pub reduced_duration_ms: u32,
}

pub mod motion {
    use super::Motion;
    pub const MENU_OPEN: Motion = Motion {
        duration_ms: 0_u32,
        curve: [0.2_f32, 0.0_f32, 0.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const MENU_CLOSE: Motion = Motion {
        duration_ms: 183_u32,
        curve: [0.4_f32, 0.0_f32, 1.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const MENU_BAR_REVEAL: Motion = Motion {
        duration_ms: 160_u32,
        curve: [0.2_f32, 0.0_f32, 0.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const POPUP_OPEN: Motion = Motion {
        duration_ms: 160_u32,
        curve: [0.16_f32, 1.0_f32, 0.3_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const POPUP_CLOSE: Motion = Motion {
        duration_ms: 100_u32,
        curve: [0.4_f32, 0.0_f32, 1.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const TOGGLE: Motion = Motion {
        duration_ms: 160_u32,
        curve: [0.2_f32, 0.0_f32, 0.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const HOVER: Motion = Motion {
        duration_ms: 100_u32,
        curve: [0.4_f32, 0.0_f32, 0.2_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const FOCUS: Motion = Motion {
        duration_ms: 100_u32,
        curve: [0.4_f32, 0.0_f32, 0.2_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const SPACES_SWITCH: Motion = Motion {
        duration_ms: 280_u32,
        curve: [0.2_f32, 0.0_f32, 0.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const DOCK_MAGNIFY: Motion = Motion {
        duration_ms: 160_u32,
        curve: [0.22_f32, 1.0_f32, 0.36_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const DOCK_HOVER: Motion = Motion {
        duration_ms: 135_u32,
        curve: [0.3333333333333333_f32, 0.0_f32, 0.6666666666666666_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const DOCK_MAGNIFY_TRACK: Motion = Motion {
        duration_ms: 100_u32,
        curve: [0.2_f32, 0.0_f32, 0.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const DOCK_REVEAL: Motion = Motion {
        duration_ms: 160_u32,
        curve: [0.2_f32, 0.0_f32, 0.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const WINDOW_OPEN: Motion = Motion {
        duration_ms: 160_u32,
        curve: [0.2_f32, 0.0_f32, 0.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const WINDOW_CLOSE: Motion = Motion {
        duration_ms: 100_u32,
        curve: [0.4_f32, 0.0_f32, 1.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const WINDOW_GENIE: Motion = Motion {
        duration_ms: 450_u32,
        curve: [0.3333333333333333_f32, 0.0_f32, 0.6666666666666666_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const MISSION_CONTROL: Motion = Motion {
        duration_ms: 280_u32,
        curve: [0.2_f32, 0.0_f32, 0.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const NOTIFICATION: Motion = Motion {
        duration_ms: 160_u32,
        curve: [0.2_f32, 0.0_f32, 0.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const OSD: Motion = Motion {
        duration_ms: 160_u32,
        curve: [0.2_f32, 0.0_f32, 0.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const SEGMENTED: Motion = Motion {
        duration_ms: 100_u32,
        curve: [0.4_f32, 0.0_f32, 0.2_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const SIDEBAR_REVEAL: Motion = Motion {
        duration_ms: 160_u32,
        curve: [0.2_f32, 0.0_f32, 0.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const SKELETON: Motion = Motion {
        duration_ms: 400_u32,
        curve: [0.4_f32, 0.0_f32, 0.6_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
}
