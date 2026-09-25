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
        }
        pub mod material {
            pub const CHROME_OPACITY: f32 = 0.82_f32;
            pub const CHROME_BLUR: f32 = 24.0_f32;
            pub const POPUP_OPACITY: f32 = 0.96_f32;
            pub const POPUP_BLUR: f32 = 30.0_f32;
            pub const SHADOW_OPACITY: f32 = 0.18_f32;
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
        }
        pub mod material {
            pub const CHROME_OPACITY: f32 = 0.72_f32;
            pub const CHROME_BLUR: f32 = 28.0_f32;
            pub const POPUP_OPACITY: f32 = 0.92_f32;
            pub const POPUP_BLUR: f32 = 32.0_f32;
            pub const SHADOW_OPACITY: f32 = 0.45_f32;
        }
    }
}

pub mod component {
    pub mod window {
        pub const RADIUS: f32 = 14.0_f32;
        pub const BORDER_WIDTH: f32 = 1.0_f32;
        pub const SHADOW_BLUR: f32 = 40.0_f32;
    }
    pub mod titlebar {
        pub const HEIGHT: f32 = 40.0_f32;
        pub const PADDING_H: f32 = 8.0_f32;
        pub const PADDING_V: f32 = 4.0_f32;
        pub const SPACING: f32 = 8.0_f32;
        pub const FONT_SIZE: f32 = 13.0_f32;
        pub const FONT_WEIGHT: f32 = 600.0_f32;
        pub const CORNER_RADIUS: f32 = 14.0_f32;
    }
    pub mod traffic_lights {
        pub const DIAMETER: f32 = 12.0_f32;
        pub const GAP: f32 = 8.0_f32;
        pub const INSET: f32 = 12.0_f32;
        pub const GLYPH_SIZE: f32 = 6.0_f32;
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
        pub const RADIUS: f32 = 10.0_f32;
        pub const HOVER_RADIUS: f32 = 6.0_f32;
    }
    pub mod dock {
        pub const ICON_SIZE: f32 = 48.0_f32;
        pub const ICON_SIZE_MIN: f32 = 32.0_f32;
        pub const ICON_SIZE_MAX: f32 = 64.0_f32;
        pub const PADDING: f32 = 6.0_f32;
        pub const GAP: f32 = 6.0_f32;
        pub const RADIUS: f32 = 14.0_f32;
        pub const INDICATOR_SIZE: f32 = 4.0_f32;
        pub const INDICATOR_GAP: f32 = 3.0_f32;
        pub const MAGNIFY_PEAK: f32 = 1.6_f32;
        pub const MAGNIFY_PEAK_MAX: f32 = 2.2_f32;
        pub const MAGNIFY_FALLOFF: f32 = 3.0_f32;
        pub const LABEL_SIZE: f32 = 12.0_f32;
        pub const TRASH_SIZE: f32 = 44.0_f32;
        pub const EDGE_MARGIN: f32 = 4.0_f32;
        pub const EDGE_TRIGGER: f32 = 4.0_f32;
        pub const REVEAL_DELAY: f32 = 120.0_f32;
        pub const HIDE_DELAY: f32 = 350.0_f32;
    }
    pub mod context_menu {
        pub const RADIUS: f32 = 14.0_f32;
        pub const PADDING: f32 = 6.0_f32;
        pub const ROW_HEIGHT: f32 = 26.0_f32;
        pub const MIN_WIDTH: f32 = 180.0_f32;
        pub const SHORTCUT_GAP: f32 = 24.0_f32;
        pub const SUBMENU_DELAY: f32 = 150.0_f32;
    }
    pub mod focus_ring {
        pub const WIDTH: f32 = 2.0_f32;
        pub const OFFSET: f32 = 2.0_f32;
        pub const RADIUS: f32 = 6.0_f32;
    }
    pub mod sidebar {
        pub const WIDTH: f32 = 220.0_f32;
        pub const ROW_HEIGHT: f32 = 28.0_f32;
        pub const ROW_RADIUS: f32 = 6.0_f32;
        pub const SECTION_GAP: f32 = 16.0_f32;
        pub const ICON_SIZE: f32 = 16.0_f32;
        pub const PADDING: f32 = 8.0_f32;
    }
    pub mod toolbar {
        pub const HEIGHT: f32 = 52.0_f32;
        pub const PADDING_H: f32 = 12.0_f32;
        pub const SPACING: f32 = 8.0_f32;
        pub const BUTTON_SIZE: f32 = 28.0_f32;
        pub const RADIUS: f32 = 10.0_f32;
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
        pub const MIN_WIDTH: f32 = 140.0_f32;
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
    pub mod shadow {
        pub const LAYERS: f32 = 8.0_f32;
        pub const OFFSET_Y: f32 = 5.0_f32;
    }
    pub mod elevation {
        pub mod low {
            pub const BLUR: f32 = 8.0_f32;
            pub const OFFSET_Y: f32 = 5.0_f32;
            pub const LAYERS: f32 = 8.0_f32;
        }
        pub mod med {
            pub const BLUR: f32 = 20.0_f32;
            pub const OFFSET_Y: f32 = 5.0_f32;
            pub const LAYERS: f32 = 8.0_f32;
        }
        pub mod high {
            pub const BLUR: f32 = 40.0_f32;
            pub const OFFSET_Y: f32 = 5.0_f32;
            pub const LAYERS: f32 = 8.0_f32;
        }
        pub mod overlay {
            pub const BLUR: f32 = 64.0_f32;
            pub const OFFSET_Y: f32 = 5.0_f32;
            pub const LAYERS: f32 = 8.0_f32;
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
        duration_ms: 160_u32,
        curve: [0.2_f32, 0.0_f32, 0.0_f32, 1.0_f32],
        reduced_duration_ms: 0_u32,
    };
    pub const MENU_CLOSE: Motion = Motion {
        duration_ms: 100_u32,
        curve: [0.4_f32, 0.0_f32, 1.0_f32, 1.0_f32],
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
        curve: [0.34_f32, 1.56_f32, 0.64_f32, 1.0_f32],
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
}
