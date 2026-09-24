// SPDX-License-Identifier: MIT
// GENERATED FILE — DO NOT EDIT. Edit design-system/tokens/tokens.json and run scripts/gen-tokens.py.
pragma Singleton
import QtQuick

QtObject {
    id: tokens

    readonly property var primitive: QtObject {
        readonly property var color: QtObject {
            readonly property color magenta50: "#fdf2f7"
            readonly property color magenta100: "#fbe1ec"
            readonly property color magenta200: "#f6c2d9"
            readonly property color magenta300: "#ee93bc"
            readonly property color magenta400: "#e15c98"
            readonly property color magenta500: "#cf3a7c"
            readonly property color magenta600: "#b32a66"
            readonly property color magenta700: "#912154"
            readonly property color magenta800: "#721d45"
            readonly property color magenta900: "#591a39"
            readonly property color violet300: "#b3a1ff"
            readonly property color violet400: "#927dff"
            readonly property color violet500: "#7c5cff"
            readonly property color violet600: "#6847e6"
            readonly property color violet700: "#5538c2"
            readonly property color neutral0: "#ffffff"
            readonly property color neutral50: "#f8f6fa"
            readonly property color neutral100: "#efeaf3"
            readonly property color neutral200: "#ded6e5"
            readonly property color neutral300: "#c3b8cc"
            readonly property color neutral400: "#9c8fa8"
            readonly property color neutral500: "#786a84"
            readonly property color neutral600: "#5b4e66"
            readonly property color neutral700: "#44394d"
            readonly property color neutral800: "#2d2534"
            readonly property color neutral900: "#1d1723"
            readonly property color neutral950: "#130f17"
            readonly property color jade400: "#3fc98a"
            readonly property color jade500: "#2bb673"
            readonly property color jade600: "#1f9a60"
            readonly property color gold400: "#f0b64b"
            readonly property color gold500: "#e0a43a"
            readonly property color gold600: "#c78a24"
            readonly property color coral400: "#f2697d"
            readonly property color coral500: "#e8556d"
            readonly property color coral600: "#cf3f58"
            readonly property color sky400: "#5b8dff"
            readonly property color sky500: "#4a7dff"
            readonly property color sky600: "#3a66e0"
        }
        readonly property var radius: QtObject {
            readonly property int none: 0
            readonly property int xs: 4
            readonly property int sm: 6
            readonly property int md: 10
            readonly property int lg: 14
            readonly property int xl: 20
            readonly property int pill: 999
        }
        readonly property var spacing: QtObject {
            readonly property int none: 0
            readonly property int xxs: 2
            readonly property int xs: 4
            readonly property int sm: 8
            readonly property int md: 12
            readonly property int lg: 16
            readonly property int xl: 24
            readonly property int xxl: 32
            readonly property int xxxl: 48
        }
        readonly property var font: QtObject {
            readonly property int sizeXs: 10
            readonly property int sizeSm: 12
            readonly property int sizeMd: 13
            readonly property int sizeLg: 15
            readonly property int sizeXl: 18
            readonly property int sizeXxl: 22
            readonly property int sizeDisplay: 28
            readonly property int weightRegular: 400
            readonly property int weightMedium: 500
            readonly property int weightSemibold: 600
            readonly property int weightBold: 700
        }
        readonly property var duration: QtObject {
            readonly property int instant: 0
            readonly property int fast: 100
            readonly property int normal: 160
            readonly property int slow: 280
            readonly property int slower: 400
        }
        readonly property var elevation: QtObject {
            readonly property int none: 0
            readonly property int low: 8
            readonly property int med: 20
            readonly property int high: 40
            readonly property int overlay: 64
        }
    }

    readonly property var lightScheme: QtObject {
        readonly property var color: QtObject {
            readonly property color surface: "#ffffff"
            readonly property color surfaceElevated: "#ffffff"
            readonly property color surfaceSunken: "#efeaf3"
            readonly property color surfaceMuted: "#f8f6fa"
            readonly property color chrome: "#ffffff"
            readonly property color textPrimary: "#1d1723"
            readonly property color textSecondary: "#5b4e66"
            readonly property color textTertiary: "#786a84"
            readonly property color accent: "#b32a66"
            readonly property color accentHover: "#cf3a7c"
            readonly property color accentMuted: "#fbe1ec"
            readonly property color accentContent: "#ffffff"
            readonly property color border: "#ded6e5"
            readonly property color separator: "#efeaf3"
            readonly property color controlFill: "#efeaf3"
            readonly property color controlHover: "#ded6e5"
            readonly property color controlActive: "#c3b8cc"
            readonly property color focusRing: "#7c5cff"
            readonly property color selection: "#b3a1ff"
            readonly property color danger: "#e8556d"
            readonly property color success: "#2bb673"
            readonly property color warning: "#e0a43a"
            readonly property color info: "#4a7dff"
            readonly property color close: "#e8556d"
            readonly property color minimize: "#e0a43a"
            readonly property color zoom: "#2bb673"
            readonly property color controlKnob: "#ffffff"
            readonly property color trafficGlyph: "#8f000000"
            readonly property color shadowColor: "#000000"
        }
        readonly property var material: QtObject {
            readonly property real chromeOpacity: 0.82
            readonly property int chromeBlur: 24
            readonly property real popupOpacity: 0.96
            readonly property int popupBlur: 30
            readonly property real shadowOpacity: 0.18
        }
    }
    readonly property var darkScheme: QtObject {
        readonly property var color: QtObject {
            readonly property color surface: "#1d1723"
            readonly property color surfaceElevated: "#2d2534"
            readonly property color surfaceSunken: "#130f17"
            readonly property color surfaceMuted: "#2d2534"
            readonly property color chrome: "#2d2534"
            readonly property color textPrimary: "#f8f6fa"
            readonly property color textSecondary: "#c3b8cc"
            readonly property color textTertiary: "#9c8fa8"
            readonly property color accent: "#e15c98"
            readonly property color accentHover: "#ee93bc"
            readonly property color accentMuted: "#591a39"
            readonly property color accentContent: "#130f17"
            readonly property color border: "#44394d"
            readonly property color separator: "#2d2534"
            readonly property color controlFill: "#2d2534"
            readonly property color controlHover: "#44394d"
            readonly property color controlActive: "#5b4e66"
            readonly property color focusRing: "#927dff"
            readonly property color selection: "#5538c2"
            readonly property color danger: "#f2697d"
            readonly property color success: "#3fc98a"
            readonly property color warning: "#f0b64b"
            readonly property color info: "#5b8dff"
            readonly property color close: "#f2697d"
            readonly property color minimize: "#f0b64b"
            readonly property color zoom: "#3fc98a"
            readonly property color controlKnob: "#ffffff"
            readonly property color trafficGlyph: "#99000000"
            readonly property color shadowColor: "#000000"
        }
        readonly property var material: QtObject {
            readonly property real chromeOpacity: 0.72
            readonly property int chromeBlur: 28
            readonly property real popupOpacity: 0.92
            readonly property int popupBlur: 32
            readonly property real shadowOpacity: 0.45
        }
    }

    // Active scheme. The shell binds this to the host appearance;
    // the gallery and tests set it directly. Assigning it breaks the
    // binding, which is exactly what a preview toggle wants.
    property bool dark: Application.styleHints.colorScheme === Qt.Dark
    // Reduced motion is a first-class token (FR-5): every animation has
    // a variant that removes translation/scale but keeps state legible.
    property bool reducedMotion: false
    readonly property var color: tokens.dark ? tokens.darkScheme.color : tokens.lightScheme.color
    readonly property var material: tokens.dark ? tokens.darkScheme.material : tokens.lightScheme.material

    readonly property var controls: QtObject {
        readonly property var window: QtObject {
            readonly property int radius: 14
            readonly property int borderWidth: 1
            readonly property int shadowBlur: 40
        }
        readonly property var titlebar: QtObject {
            readonly property int height: 40
            readonly property int paddingH: 8
            readonly property int paddingV: 4
            readonly property int spacing: 8
            readonly property int fontSize: 13
            readonly property int fontWeight: 600
            readonly property int cornerRadius: 14
        }
        readonly property var trafficLights: QtObject {
            readonly property int diameter: 12
            readonly property int gap: 8
            readonly property int inset: 12
            readonly property int glyphSize: 6
            readonly property bool hoverReveal: true
        }
        readonly property var toggle: QtObject {
            readonly property int width: 40
            readonly property int height: 24
            readonly property int knob: 18
            readonly property int inset: 3
            readonly property int labelGap: 8
        }
        readonly property var button: QtObject {
            readonly property int height: 28
            readonly property int paddingH: 12
            readonly property int radius: 10
            readonly property int fontSize: 13
            readonly property int fontWeight: 500
            readonly property int iconSize: 16
        }
        readonly property var popup: QtObject {
            readonly property int radius: 14
            readonly property int padding: 8
            readonly property int minWidth: 180
            readonly property int rowHeight: 28
            readonly property int offset: 6
            readonly property int shadowBlur: 20
        }
        readonly property var menuBarMenu: QtObject {
            readonly property int barHeight: 24
            readonly property int barPaddingH: 10
            readonly property int itemHeight: 24
            readonly property int rowHeight: 26
            readonly property int minWidth: 200
            readonly property int padding: 6
            readonly property int shortcutGap: 24
            readonly property int radius: 14
        }
        readonly property var menuBar: QtObject {
            readonly property int height: 28
            readonly property int paddingH: 8
            readonly property int statusItemPaddingH: 6
            readonly property int statusItemGap: 2
            readonly property int iconSize: 15
            readonly property int fontSize: 13
            readonly property int labelGap: 5
            readonly property int clockGap: 8
            readonly property int radius: 10
            readonly property int hoverRadius: 6
        }
        readonly property var dock: QtObject {
            readonly property int iconSize: 48
            readonly property int iconSizeMin: 32
            readonly property int iconSizeMax: 64
            readonly property int padding: 6
            readonly property int gap: 6
            readonly property int radius: 14
            readonly property int indicatorSize: 4
            readonly property int indicatorGap: 3
            readonly property real magnifyPeak: 1.6
            readonly property real magnifyPeakMax: 2.2
            readonly property real magnifyFalloff: 3.0
            readonly property int labelSize: 12
            readonly property int trashSize: 44
            readonly property int edgeMargin: 4
            readonly property int edgeTrigger: 4
            readonly property int revealDelay: 120
            readonly property int hideDelay: 350
        }
        readonly property var contextMenu: QtObject {
            readonly property int radius: 14
            readonly property int padding: 6
            readonly property int rowHeight: 26
            readonly property int minWidth: 180
            readonly property int shortcutGap: 24
            readonly property int submenuDelay: 150
        }
        readonly property var focusRing: QtObject {
            readonly property int width: 2
            readonly property int offset: 2
            readonly property int radius: 6
        }
        readonly property var sidebar: QtObject {
            readonly property int width: 220
            readonly property int rowHeight: 28
            readonly property int rowRadius: 6
            readonly property int sectionGap: 16
            readonly property int iconSize: 16
            readonly property int padding: 8
        }
        readonly property var toolbar: QtObject {
            readonly property int height: 52
            readonly property int paddingH: 12
            readonly property int spacing: 8
            readonly property int buttonSize: 28
            readonly property int radius: 10
        }
        readonly property var splitView: QtObject {
            readonly property int dividerWidth: 1
            readonly property int minPaneWidth: 180
        }
        readonly property var settingsRow: QtObject {
            readonly property int height: 44
            readonly property int paddingH: 12
            readonly property int labelWidth: 200
            readonly property int controlGap: 16
        }
        readonly property var settingsGroup: QtObject {
            readonly property int radius: 10
            readonly property int padding: 8
            readonly property int rowGap: 1
            readonly property int marginBottom: 16
        }
        readonly property var segmentedControl: QtObject {
            readonly property int height: 28
            readonly property int radius: 10
            readonly property int padding: 2
            readonly property int segmentMinWidth: 64
            readonly property int fontSize: 13
        }
        readonly property var searchField: QtObject {
            readonly property int height: 28
            readonly property int radius: 999
            readonly property int paddingH: 8
            readonly property int iconSize: 14
            readonly property int minWidth: 160
        }
        readonly property var sourceList: QtObject {
            readonly property int rowHeight: 26
            readonly property int rowRadius: 6
            readonly property int indent: 12
        }
        readonly property var dialog: QtObject {
            readonly property int radius: 14
            readonly property int padding: 16
            readonly property int minWidth: 320
            readonly property int buttonGap: 8
        }
        readonly property var sheet: QtObject {
            readonly property int radius: 14
            readonly property int padding: 16
            readonly property int width: 420
        }
        readonly property var popover: QtObject {
            readonly property int radius: 14
            readonly property int padding: 12
            readonly property int arrowSize: 8
            readonly property int shadowBlur: 40
        }
        readonly property var shadow: QtObject {
            readonly property int layers: 8
            readonly property int offsetY: 5
        }
        readonly property var elevation: QtObject {
            readonly property var low: QtObject {
                readonly property int blur: 8
                readonly property int offsetY: 5
                readonly property int layers: 8
            }
            readonly property var med: QtObject {
                readonly property int blur: 20
                readonly property int offsetY: 5
                readonly property int layers: 8
            }
            readonly property var high: QtObject {
                readonly property int blur: 40
                readonly property int offsetY: 5
                readonly property int layers: 8
            }
            readonly property var overlay: QtObject {
                readonly property int blur: 64
                readonly property int offsetY: 5
                readonly property int layers: 8
            }
        }
        readonly property var scrollView: QtObject {
            readonly property int scrollbarWidth: 8
            readonly property int scrollbarMargin: 2
            readonly property int scrollbarRadius: 999
            readonly property int minThumb: 24
        }
        readonly property var overview: QtObject {
            readonly property int stripGap: 12
            readonly property int stripMargin: 16
            readonly property int cardWidth: 132
            readonly property int cardHeight: 84
            readonly property int cardPadding: 8
            readonly property int cardRadius: 14
            readonly property int chipHeight: 36
            readonly property int chipRadius: 10
            readonly property int chipGap: 8
            readonly property int chipPadding: 10
            readonly property int fontSize: 13
            readonly property int titleSize: 12
            readonly property real scrimOpacity: 0.18
        }
    }

    readonly property var motion: QtObject {
        readonly property QtObject menuOpen: QtObject {
            readonly property int fullDuration: 160
            readonly property int duration: tokens.reducedMotion ? 0 : 160
            readonly property var curve: [0.2, 0.0, 0.0, 1.0, 1.0, 1.0]
        }
        readonly property QtObject menuClose: QtObject {
            readonly property int fullDuration: 100
            readonly property int duration: tokens.reducedMotion ? 0 : 100
            readonly property var curve: [0.4, 0.0, 1.0, 1.0, 1.0, 1.0]
        }
        readonly property QtObject popupOpen: QtObject {
            readonly property int fullDuration: 160
            readonly property int duration: tokens.reducedMotion ? 0 : 160
            readonly property var curve: [0.16, 1.0, 0.3, 1.0, 1.0, 1.0]
        }
        readonly property QtObject popupClose: QtObject {
            readonly property int fullDuration: 100
            readonly property int duration: tokens.reducedMotion ? 0 : 100
            readonly property var curve: [0.4, 0.0, 1.0, 1.0, 1.0, 1.0]
        }
        readonly property QtObject toggle: QtObject {
            readonly property int fullDuration: 160
            readonly property int duration: tokens.reducedMotion ? 0 : 160
            readonly property var curve: [0.2, 0.0, 0.0, 1.0, 1.0, 1.0]
        }
        readonly property QtObject hover: QtObject {
            readonly property int fullDuration: 100
            readonly property int duration: tokens.reducedMotion ? 0 : 100
            readonly property var curve: [0.4, 0.0, 0.2, 1.0, 1.0, 1.0]
        }
        readonly property QtObject focus: QtObject {
            readonly property int fullDuration: 100
            readonly property int duration: tokens.reducedMotion ? 0 : 100
            readonly property var curve: [0.4, 0.0, 0.2, 1.0, 1.0, 1.0]
        }
        readonly property QtObject spacesSwitch: QtObject {
            readonly property int fullDuration: 280
            readonly property int duration: tokens.reducedMotion ? 0 : 280
            readonly property var curve: [0.2, 0.0, 0.0, 1.0, 1.0, 1.0]
        }
        readonly property QtObject dockMagnify: QtObject {
            readonly property int fullDuration: 160
            readonly property int duration: tokens.reducedMotion ? 0 : 160
            readonly property var curve: [0.34, 1.56, 0.64, 1.0, 1.0, 1.0]
        }
        readonly property QtObject dockReveal: QtObject {
            readonly property int fullDuration: 160
            readonly property int duration: tokens.reducedMotion ? 0 : 160
            readonly property var curve: [0.2, 0.0, 0.0, 1.0, 1.0, 1.0]
        }
        readonly property QtObject windowOpen: QtObject {
            readonly property int fullDuration: 160
            readonly property int duration: tokens.reducedMotion ? 0 : 160
            readonly property var curve: [0.2, 0.0, 0.0, 1.0, 1.0, 1.0]
        }
        readonly property QtObject windowClose: QtObject {
            readonly property int fullDuration: 100
            readonly property int duration: tokens.reducedMotion ? 0 : 100
            readonly property var curve: [0.4, 0.0, 1.0, 1.0, 1.0, 1.0]
        }
        readonly property QtObject missionControl: QtObject {
            readonly property int fullDuration: 280
            readonly property int duration: tokens.reducedMotion ? 0 : 280
            readonly property var curve: [0.2, 0.0, 0.0, 1.0, 1.0, 1.0]
        }
        readonly property QtObject notification: QtObject {
            readonly property int fullDuration: 160
            readonly property int duration: tokens.reducedMotion ? 0 : 160
            readonly property var curve: [0.2, 0.0, 0.0, 1.0, 1.0, 1.0]
        }
        readonly property QtObject segmented: QtObject {
            readonly property int fullDuration: 100
            readonly property int duration: tokens.reducedMotion ? 0 : 100
            readonly property var curve: [0.4, 0.0, 0.2, 1.0, 1.0, 1.0]
        }
        readonly property QtObject sidebarReveal: QtObject {
            readonly property int fullDuration: 160
            readonly property int duration: tokens.reducedMotion ? 0 : 160
            readonly property var curve: [0.2, 0.0, 0.0, 1.0, 1.0, 1.0]
        }
    }
}
