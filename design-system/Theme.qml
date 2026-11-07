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
            readonly property int xxl: 28
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
            readonly property string family: "Inter"
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

    readonly property var glass: QtObject {
        readonly property string version: "1"
        readonly property var rollout: QtObject {
            readonly property string defaultMode: "legacy"
            readonly property string legacyMode: "legacy"
            readonly property string replacementMode: "replacement"
            readonly property string envVar: "DRAGONFRUIT_GLASS_ROLLOUT"
        }
        readonly property var slider: QtObject {
            readonly property real defaultPosition: 0.5255
            readonly property var lightAnchors: QtObject {
                readonly property var low: QtObject {
                    readonly property real position: 0.0
                    readonly property real alpha: 0.732
                    readonly property real tint: 0.953
                }
                readonly property var mid: QtObject {
                    readonly property real position: 0.5255
                    readonly property real alpha: 0.552
                    readonly property real tint: 0.993
                }
                readonly property var high: QtObject {
                    readonly property real position: 1.0
                    readonly property real alpha: 0.415
                    readonly property real tint: 0.997
                }
            }
        }
        readonly property var accessibility: QtObject {
            readonly property var reduceTransparency: QtObject {
                readonly property var light: QtObject {
                    readonly property real alpha: 0.98
                    readonly property real tint: 0.97
                    readonly property string colourSpace: "srgb"
                }
                readonly property var dark: QtObject {
                    readonly property real alpha: 0.96
                    readonly property real tint: 0.1
                    readonly property string colourSpace: "srgb"
                }
            }
            readonly property var increaseContrast: QtObject {
                readonly property real edgeBoost: 0.35
                readonly property real foregroundBoost: 0.2
                readonly property real edgeWidthBoost: 0.5
            }
        }
        readonly property var presets: QtObject {
            readonly property var dock: QtObject {
                readonly property string role: "dock"
                readonly property string render: "sampled"
                readonly property string slider: "none"
                readonly property var light: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md Dock plate: light neutral fit alpha 0.253 / tint 0.604; rim peak 0.397 on mid-grey; blur sigma 3.86pt first fit"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.253
                        readonly property real neutralTint: 0.604
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.253
                        readonly property real perceptualTint: 0.604
                    }
                    readonly property real blur: 3.86
                    readonly property real edge: 0.397
                    readonly property real edgeWidth: 1.0
                    readonly property real edgeTail: 6.0
                    readonly property real sharpFraction: 0.12
                    readonly property real cornerLensPeak: 9.6
                    readonly property real cornerLensDistance: 15.0
                    readonly property real cornerLensTail: 50.0
                    readonly property real foreground: 0.0
                }
                readonly property var dark: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md Dock plate: dark neutral fit alpha 0.149 / tint 0.207; blur and rim reuse the light fit because the dark rim was not separately fitted"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.149
                        readonly property real neutralTint: 0.207
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.149
                        readonly property real perceptualTint: 0.207
                    }
                    readonly property real blur: 3.86
                    readonly property real edge: 0.397
                    readonly property real edgeWidth: 1.0
                    readonly property real edgeTail: 6.0
                    readonly property real sharpFraction: 0.12
                    readonly property real cornerLensPeak: 9.6
                    readonly property real cornerLensDistance: 15.0
                    readonly property real cornerLensTail: 50.0
                    readonly property real foreground: 0.0
                }
            }
            readonly property var menuBarVeil: QtObject {
                readonly property string role: "chrome"
                readonly property string render: "veil"
                readonly property string slider: "none"
                readonly property int stripHeight: 39
                readonly property int extent: 95
                readonly property var light: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md menu-bar veil: alpha 0.451 at y=5pt, then 0.439/0.408/0.357/0.243/0.059/0.008 to y=90pt; white background shows no visible veil; the veil is not a blur"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.451
                        readonly property real neutralTint: 1.0
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.451
                        readonly property real perceptualTint: 1.0
                    }
                    readonly property var veil: QtObject {
                        readonly property real probe: 8.0
                        readonly property real readability: 0.35
                        readonly property real alpha5: 0.45098037
                        readonly property real alpha15: 0.43921566
                        readonly property real alpha25: 0.40784317
                        readonly property real alpha35: 0.35686272
                        readonly property real alpha50: 0.24313729
                        readonly property real alpha75: 0.05882353
                        readonly property real alpha90: 0.00784314
                        readonly property real alpha95: 0.0
                    }
                    readonly property real blur: 0.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
                readonly property var dark: QtObject {
                    readonly property string confidence: "baseline"
                    readonly property string provenance: "No standalone dark veil was measured; named Dragonfruit baseline: the measured light depth profile scaled to a 0.48 peak over black, with a 0.20 post-veil readability ceiling"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.48
                        readonly property real neutralTint: 0.0
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.48
                        readonly property real perceptualTint: 0.0
                    }
                    readonly property var veil: QtObject {
                        readonly property real probe: 8.0
                        readonly property real readability: 0.2
                        readonly property real alpha5: 0.48
                        readonly property real alpha15: 0.4674
                        readonly property real alpha25: 0.434
                        readonly property real alpha35: 0.3798
                        readonly property real alpha50: 0.2588
                        readonly property real alpha75: 0.0626
                        readonly property real alpha90: 0.00835
                        readonly property real alpha95: 0.0
                    }
                    readonly property real blur: 0.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
            }
            readonly property var menuAttached: QtObject {
                readonly property string role: "popup"
                readonly property string render: "sampled"
                readonly property string slider: "lightAttached"
                readonly property var light: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md System Settings light File anchors at 0/0.5255/1 -> alpha 0.732/0.552/0.415, tint 0.953/0.993/0.997; File blur sigma ~10pt"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.552
                        readonly property real neutralTint: 0.993
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.552
                        readonly property real perceptualTint: 0.993
                    }
                    readonly property real blur: 10.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
                readonly property var dark: QtObject {
                    readonly property string confidence: "adapted"
                    readonly property string provenance: "The published dark File fit is contaminated by a light crop (REFERENCE.md); the attached-menu baseline is anchored to the valid dark mid/black context/status family (alpha 0.66 / tint 0.26), mixed 25% toward a deeper adapted baseline. The white row stays open for a controlled recapture."
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.65
                        readonly property real neutralTint: 0.27
                        readonly property real perceptualWeight: 0.25
                        readonly property real perceptualAlpha: 0.62
                        readonly property real perceptualTint: 0.3
                    }
                    readonly property real blur: 10.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
            }
            readonly property var menuDetached: QtObject {
                readonly property string role: "popup"
                readonly property string render: "sampled"
                readonly property string slider: "lightRelative"
                readonly property var light: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md common light status/context menus cluster near alpha 0.55 and near-white tint; detached menus are rounded cards"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.55
                        readonly property real neutralTint: 0.96
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.55
                        readonly property real perceptualTint: 0.96
                    }
                    readonly property real blur: 10.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
                readonly property var dark: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md dark Desktop/Wi-Fi/Bluetooth/Battery fits near alpha 0.66 / tint 0.26 with substantial residuals; this is the detached-menu baseline"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.66
                        readonly property real neutralTint: 0.26
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.66
                        readonly property real perceptualTint: 0.26
                    }
                    readonly property real blur: 10.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
            }
            readonly property var statusMenu: QtObject {
                readonly property string role: "popup"
                readonly property string render: "sampled"
                readonly property string slider: "lightRelative"
                readonly property var light: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md light status menus join the common context-menu cluster; 8pt blur is a local adaptation"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.55
                        readonly property real neutralTint: 0.96
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.55
                        readonly property real perceptualTint: 0.96
                    }
                    readonly property real blur: 8.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
                readonly property var dark: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md dark Wi-Fi/Bluetooth/Battery status fits near alpha 0.66 / tint 0.26; 8pt blur is a local adaptation"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.66
                        readonly property real neutralTint: 0.26
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.66
                        readonly property real perceptualTint: 0.26
                    }
                    readonly property real blur: 8.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
            }
            readonly property var tooltip: QtObject {
                readonly property string role: "popup"
                readonly property string render: "sampled"
                readonly property string slider: "none"
                readonly property var light: QtObject {
                    readonly property string confidence: "adapted"
                    readonly property string provenance: "No direct tooltip optical fit; the reference tooltip is 33pt high and 8pt above the Dock plate. Named popup baseline 0.72/0.92"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.72
                        readonly property real neutralTint: 0.92
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.72
                        readonly property real perceptualTint: 0.92
                    }
                    readonly property real blur: 6.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
                readonly property var dark: QtObject {
                    readonly property string confidence: "adapted"
                    readonly property string provenance: "No dark tooltip fit; named popup baseline 0.76/0.24"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.76
                        readonly property real neutralTint: 0.24
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.76
                        readonly property real perceptualTint: 0.24
                    }
                    readonly property real blur: 6.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
            }
            readonly property var controlCenterOuter: QtObject {
                readonly property string role: "popup"
                readonly property string render: "sampled"
                readonly property string slider: "none"
                readonly property var light: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md later controlled Control Center light outer fit alpha 0.265 / tint 0.373; REPORT.md 3.5/Appendix A give the 8 pt blur, 0.30 rim and 6 pt/24 pt capsule-end lens"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.265
                        readonly property real neutralTint: 0.373
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.265
                        readonly property real perceptualTint: 0.373
                    }
                    readonly property real blur: 8.0
                    readonly property real edge: 0.3
                    readonly property real edgeWidth: 1.0
                    readonly property real edgeTail: 6.0
                    readonly property real sharpFraction: 0.1
                    readonly property real cornerLensPeak: 6.0
                    readonly property real cornerLensDistance: 24.0
                    readonly property real cornerLensTail: 50.0
                    readonly property real foreground: 0.0
                }
                readonly property var dark: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md later controlled Control Center dark outer fit alpha 0.489 / tint 0.677; blur/rim/lens reuse the light capsule fit because the dark end caustic was not separately measured"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.489
                        readonly property real neutralTint: 0.677
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.489
                        readonly property real perceptualTint: 0.677
                    }
                    readonly property real blur: 8.0
                    readonly property real edge: 0.3
                    readonly property real edgeWidth: 1.0
                    readonly property real edgeTail: 6.0
                    readonly property real sharpFraction: 0.1
                    readonly property real cornerLensPeak: 6.0
                    readonly property real cornerLensDistance: 24.0
                    readonly property real cornerLensTail: 50.0
                    readonly property real foreground: 0.0
                }
            }
            readonly property var controlCenterModule: QtObject {
                readonly property string role: "popup"
                readonly property string render: "sampled"
                readonly property string slider: "none"
                readonly property var light: QtObject {
                    readonly property string confidence: "measured-effective"
                    readonly property string provenance: "REFERENCE.md Wi-Fi module light fit 0.391/0.317 is an observed effective stacked response. The compositor flat-band output is opaque, so this preset is the module's final composite in its own region; it is not layered a second time over the outer plate"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.391
                        readonly property real neutralTint: 0.317
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.391
                        readonly property real perceptualTint: 0.317
                    }
                    readonly property real blur: 8.0
                    readonly property real edge: 0.3
                    readonly property real edgeWidth: 1.0
                    readonly property real edgeTail: 6.0
                    readonly property real sharpFraction: 0.1
                    readonly property real cornerLensPeak: 6.0
                    readonly property real cornerLensDistance: 24.0
                    readonly property real cornerLensTail: 50.0
                    readonly property real foreground: 0.0
                }
                readonly property var dark: QtObject {
                    readonly property string confidence: "adapted"
                    readonly property string provenance: "No reliable dark module decomposition; adapted effective 0.65/0.44 so the opaque module reads darker than the 0.489/0.677 dark outer plate"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.65
                        readonly property real neutralTint: 0.44
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.65
                        readonly property real perceptualTint: 0.44
                    }
                    readonly property real blur: 8.0
                    readonly property real edge: 0.3
                    readonly property real edgeWidth: 1.0
                    readonly property real edgeTail: 6.0
                    readonly property real sharpFraction: 0.1
                    readonly property real cornerLensPeak: 6.0
                    readonly property real cornerLensDistance: 24.0
                    readonly property real cornerLensTail: 50.0
                    readonly property real foreground: 0.0
                }
            }
            readonly property var controlCenterExpanded: QtObject {
                readonly property string role: "popup"
                readonly property string render: "sampled"
                readonly property string slider: "none"
                readonly property var light: QtObject {
                    readonly property string confidence: "adapted"
                    readonly property string provenance: "The expanded list is unmeasured in light; adapted readable near-opaque light plate (0.94/0.96) so dark list text stays legible"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.94
                        readonly property real neutralTint: 0.96
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.94
                        readonly property real perceptualTint: 0.96
                    }
                    readonly property real blur: 8.0
                    readonly property real edge: 0.22
                    readonly property real edgeWidth: 0.75
                    readonly property real edgeTail: 4.0
                    readonly property real sharpFraction: 0.05
                    readonly property real cornerLensPeak: 4.0
                    readonly property real cornerLensDistance: 16.0
                    readonly property real cornerLensTail: 32.0
                    readonly property real foreground: 0.0
                }
                readonly property var dark: QtObject {
                    readonly property string confidence: "observed-luma"
                    readonly property string provenance: "REFERENCE.md Bluetooth expanded list is dark (~0.13 luma) with no reliable alpha decomposition; near-opaque 0.96/0.13 keeps the composite within 0.04 luma over white/black"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.96
                        readonly property real neutralTint: 0.13
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.96
                        readonly property real perceptualTint: 0.13
                    }
                    readonly property real blur: 8.0
                    readonly property real edge: 0.22
                    readonly property real edgeWidth: 0.75
                    readonly property real edgeTail: 4.0
                    readonly property real sharpFraction: 0.05
                    readonly property real cornerLensPeak: 4.0
                    readonly property real cornerLensDistance: 16.0
                    readonly property real cornerLensTail: 32.0
                    readonly property real foreground: 0.0
                }
            }
            readonly property var notificationBanner: QtObject {
                readonly property string role: "popup"
                readonly property string render: "sampled"
                readonly property string slider: "none"
                readonly property var light: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md light notification banner 0.549/0.976"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.549
                        readonly property real neutralTint: 0.976
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.549
                        readonly property real perceptualTint: 0.976
                    }
                    readonly property real blur: 14.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
                readonly property var dark: QtObject {
                    readonly property string confidence: "adapted"
                    readonly property string provenance: "T205: the dark banner is uncovered as an independent fit (underconstrained); adapted readable 0.56/0.30 so it reads lighter than the dark stack and keeps light-on-glass text contrast"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.56
                        readonly property real neutralTint: 0.3
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.56
                        readonly property real perceptualTint: 0.3
                    }
                    readonly property real blur: 14.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
            }
            readonly property var notificationStack: QtObject {
                readonly property string role: "popup"
                readonly property string render: "sampled"
                readonly property string slider: "none"
                readonly property var light: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md light notification stack 0.551/0.706; the stack residual is materially larger than the banner's"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.551
                        readonly property real neutralTint: 0.706
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.551
                        readonly property real perceptualTint: 0.706
                    }
                    readonly property real blur: 14.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
                readonly property var dark: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md dark notification stack 0.507/0.550"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.507
                        readonly property real neutralTint: 0.55
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.507
                        readonly property real perceptualTint: 0.55
                    }
                    readonly property real blur: 14.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
            }
            readonly property var notificationLowerPanel: QtObject {
                readonly property string role: "popup"
                readonly property string render: "sampled"
                readonly property string slider: "none"
                readonly property var light: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md light notification lower panel 0.407/0.852; the residual is materially larger than the banner's"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.407
                        readonly property real neutralTint: 0.852
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.407
                        readonly property real perceptualTint: 0.852
                    }
                    readonly property real blur: 18.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
                readonly property var dark: QtObject {
                    readonly property string confidence: "adapted"
                    readonly property string provenance: "T205: the dark lower panel is uncovered as an independent fit (underconstrained); adapted readable 0.40/0.30 so it reads below the dark stack on black/mid/white and stays distinct from the dark banner"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.4
                        readonly property real neutralTint: 0.3
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.4
                        readonly property real perceptualTint: 0.3
                    }
                    readonly property real blur: 18.0
                    readonly property real edge: 0.0
                    readonly property real edgeWidth: 0.0
                    readonly property real foreground: 0.0
                }
            }
            readonly property var windowChrome: QtObject {
                readonly property string role: "chrome"
                readonly property string render: "solid"
                readonly property string slider: "none"
                readonly property var light: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md light System Settings sidebar/toolbar 0.986/0.857 and 0.961/0.965; Finder 0.985/0.858 and 0.994/0.922. Named near-opaque 0.975/0.91"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.975
                        readonly property real neutralTint: 0.91
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.975
                        readonly property real perceptualTint: 0.91
                    }
                    readonly property real blur: 0.0
                    readonly property real edge: 0.08
                    readonly property real edgeWidth: 1.0
                    readonly property real foreground: 0.0
                }
                readonly property var dark: QtObject {
                    readonly property string confidence: "measured"
                    readonly property string provenance: "REFERENCE.md dark Finder sidebar/toolbar 0.935/0.218 and 0.900/0.247. Named near-opaque 0.918/0.233"
                    readonly property string colourSpace: "srgb"
                    readonly property var calibration: QtObject {
                        readonly property real neutralAlpha: 0.918
                        readonly property real neutralTint: 0.233
                        readonly property real perceptualWeight: 0.0
                        readonly property real perceptualAlpha: 0.918
                        readonly property real perceptualTint: 0.233
                    }
                    readonly property real blur: 0.0
                    readonly property real edge: 0.08
                    readonly property real edgeWidth: 1.0
                    readonly property real foreground: 0.0
                }
            }
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
            readonly property color accent: tokens.hasAccentOverride ? tokens.accentOverrideColor : "#b32a66"
            readonly property color accentHover: tokens.hasAccentOverride ? Qt.lighter(tokens.accentOverrideColor, 1.15) : "#cf3a7c"
            readonly property color accentMuted: tokens.hasAccentOverride ? Qt.tint("#f8f6fa", Qt.rgba(tokens.accentOverrideColor.r, tokens.accentOverrideColor.g, tokens.accentOverrideColor.b, 0.14)) : "#fbe1ec"
            readonly property color accentContent: tokens.hasAccentOverride ? (tokens.accentOverrideColor.r * 0.299 + tokens.accentOverrideColor.g * 0.587 + tokens.accentOverrideColor.b * 0.114 > 0.5 ? "#130f17" : "#ffffff") : "#ffffff"
            readonly property color border: "#ded6e5"
            readonly property color separator: "#efeaf3"
            readonly property color controlFill: "#efeaf3"
            readonly property color controlHover: "#ded6e5"
            readonly property color controlActive: "#c3b8cc"
            readonly property color focusRing: "#7c5cff"
            readonly property color selection: "#b3a1ff"
            readonly property color marqueeFill: "#40ededed"
            readonly property color marqueeBorder: "#8cc4c4c4"
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
            readonly property color trashFillTop: "#ded6e5"
            readonly property color trashFillBottom: "#c3b8cc"
            readonly property color trashRim: "#786a84"
            readonly property color trashHighlight: "#ffffff"
            readonly property color trashPaper: "#f8f6fa"
            readonly property color trashPaperEdge: "#9c8fa8"
            readonly property color folderTab: "#3a66e0"
            readonly property color folderFillTop: "#5b8dff"
            readonly property color folderFillBottom: "#4a7dff"
            readonly property color folderRim: "#3a66e0"
            readonly property color folderHighlight: "#ffffff"
            readonly property color dockFill: "#ffffff"
            readonly property color dockFallbackFill: "#efeaf3"
            readonly property color chromeFallbackFill: "#f8f6fa"
            readonly property color dockRim: "#ffffff"
            readonly property color dockBorder: "#c3b8cc"
            readonly property color dockShadow: "#000000"
            readonly property color dockHoverFill: "#786a84"
            readonly property color dockDivider: "#786a84"
            readonly property color dockIndicator: "#1d1723"
            readonly property color skeletonBase: "#ded6e5"
            readonly property color skeletonHighlight: "#f8f6fa"
        }
        readonly property var material: QtObject {
            readonly property real chromeOpacity: 0.82
            readonly property int chromeBlur: 24
            readonly property real popupOpacity: 0.62
            readonly property int popupBlur: 30
            readonly property real shadowOpacity: 0.18
            readonly property real dockOpacity: 0.5
            readonly property int dockBlur: 30
            readonly property real chromeFallbackOpacity: 0.86
            readonly property real dockFallbackOpacity: 0.88
            readonly property real popupFallbackOpacity: 0.92
            readonly property int chromeRefraction: 6
            readonly property int popupRefraction: 8
            readonly property int dockRefraction: 8
            readonly property real chromeSpecular: 0.45
            readonly property real popupSpecular: 0.5
            readonly property real dockSpecular: 0.5
            readonly property int chromeSpecularWidth: 2
            readonly property real popupSpecularWidth: 2.5
            readonly property real dockSpecularWidth: 2.5
            readonly property real chromeTint: 0.16
            readonly property real popupTint: 0.2
            readonly property real dockTint: 0.22
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
            readonly property color accent: tokens.hasAccentOverride ? tokens.accentOverrideColor : "#e15c98"
            readonly property color accentHover: tokens.hasAccentOverride ? Qt.lighter(tokens.accentOverrideColor, 1.15) : "#ee93bc"
            readonly property color accentMuted: tokens.hasAccentOverride ? Qt.tint("#2d2534", Qt.rgba(tokens.accentOverrideColor.r, tokens.accentOverrideColor.g, tokens.accentOverrideColor.b, 0.28)) : "#591a39"
            readonly property color accentContent: tokens.hasAccentOverride ? (tokens.accentOverrideColor.r * 0.299 + tokens.accentOverrideColor.g * 0.587 + tokens.accentOverrideColor.b * 0.114 > 0.5 ? "#130f17" : "#ffffff") : "#130f17"
            readonly property color border: "#44394d"
            readonly property color separator: "#2d2534"
            readonly property color controlFill: "#2d2534"
            readonly property color controlHover: "#44394d"
            readonly property color controlActive: "#5b4e66"
            readonly property color focusRing: "#927dff"
            readonly property color selection: "#5538c2"
            readonly property color marqueeFill: "#33e6e6e6"
            readonly property color marqueeBorder: "#8cd8d8d8"
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
            readonly property color trashFillTop: "#5b4e66"
            readonly property color trashFillBottom: "#2d2534"
            readonly property color trashRim: "#9c8fa8"
            readonly property color trashHighlight: "#c3b8cc"
            readonly property color trashPaper: "#ded6e5"
            readonly property color trashPaperEdge: "#5b4e66"
            readonly property color folderTab: "#3a66e0"
            readonly property color folderFillTop: "#4a7dff"
            readonly property color folderFillBottom: "#3a66e0"
            readonly property color folderRim: "#5b8dff"
            readonly property color folderHighlight: "#ded6e5"
            readonly property color dockFill: "#2d2534"
            readonly property color dockFallbackFill: "#2d2534"
            readonly property color chromeFallbackFill: "#2d2534"
            readonly property color dockRim: "#ded6e5"
            readonly property color dockBorder: "#44394d"
            readonly property color dockShadow: "#000000"
            readonly property color dockHoverFill: "#c3b8cc"
            readonly property color dockDivider: "#9c8fa8"
            readonly property color dockIndicator: "#f8f6fa"
            readonly property color skeletonBase: "#2d2534"
            readonly property color skeletonHighlight: "#44394d"
        }
        readonly property var material: QtObject {
            readonly property real chromeOpacity: 0.72
            readonly property int chromeBlur: 28
            readonly property real popupOpacity: 0.7
            readonly property int popupBlur: 32
            readonly property real shadowOpacity: 0.45
            readonly property real dockOpacity: 0.42
            readonly property int dockBlur: 34
            readonly property real chromeFallbackOpacity: 0.82
            readonly property real dockFallbackOpacity: 0.84
            readonly property real popupFallbackOpacity: 0.9
            readonly property int chromeRefraction: 8
            readonly property int popupRefraction: 10
            readonly property int dockRefraction: 10
            readonly property real chromeSpecular: 0.3
            readonly property real popupSpecular: 0.34
            readonly property real dockSpecular: 0.34
            readonly property int chromeSpecularWidth: 2
            readonly property real popupSpecularWidth: 2.5
            readonly property real dockSpecularWidth: 2.5
            readonly property real chromeTint: 0.14
            readonly property real popupTint: 0.18
            readonly property real dockTint: 0.2
        }
    }

    // Active scheme. The shell binds this to the host appearance;
    // the gallery and tests set it directly. Assigning it breaks the
    // binding, which is exactly what a preview toggle wants.
    property bool dark: Application.styleHints.colorScheme === Qt.Dark
    // Reduced motion is a first-class token (FR-5): every animation has
    // a variant that removes translation/scale but keeps state legible.
    property bool reducedMotion: false
    // Accent override (T-09.2, `appearance.accent`): empty means the
    // active scheme's token accent. The shell's ThemeBinding and the
    // Settings app's local Theme bindings write it; the accent-family
    // scheme colors above consult it, so every accent consumer follows
    // without a per-component override.
    property string accentOverride: ""
    readonly property bool hasAccentOverride: tokens.accentOverride !== ""
    readonly property color accentOverrideColor: tokens.hasAccentOverride ? tokens.accentOverride : "transparent"
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
            readonly property int padding: 15
            readonly property int paddingAlong: 16
            readonly property int gap: 14
            readonly property int radius: 27
            readonly property int indicatorSize: 4
            readonly property int indicatorGap: 8
            readonly property real magnifyPeak: 1.217
            readonly property real magnifyPeakMax: 2.047
            readonly property real magnifyInfluence: 125.1
            readonly property int launchLift: 15
            readonly property int labelSize: 12
            readonly property int trashSize: 44
            readonly property int edgeMargin: 8
            readonly property int edgeTrigger: 4
            readonly property int revealDelay: 120
            readonly property int hideDelay: 350
            readonly property var plate: QtObject {
                readonly property real fillOpacity: 0.42
                readonly property real rimOpacity: 0.7
                readonly property int rimHeight: 1
                readonly property int glossHeight: 6
                readonly property real glossOpacity: 0.22
                readonly property int edgeHeight: 1
                readonly property real edgeOpacity: 0.5
                readonly property int borderWidth: 1
                readonly property real borderOpacity: 0.55
                readonly property int shadowBlur: 20
                readonly property real shadowOpacity: 0.3
                readonly property int shadowOffsetY: 6
            }
            readonly property var icon: QtObject {
                readonly property real radiusRatio: 0.24
                readonly property real inset: 0.0
            }
            readonly property var hover: QtObject {
                readonly property real fillOpacity: 0.18
                readonly property real radiusRatio: 0.28
                readonly property int shadowBlur: 10
                readonly property real shadowOpacity: 0.28
            }
            readonly property var indicator: QtObject {
                readonly property real opacity: 0.9
                readonly property real inactiveOpacity: 0.45
            }
            readonly property var divider: QtObject {
                readonly property real opacity: 0.7
                readonly property int width: 1
                readonly property real heightRatio: 0.78
                readonly property int gap: 22
            }
            readonly property var chooser: QtObject {
                readonly property int maxRows: 7
                readonly property int scrollbarWidth: 8
                readonly property int scrollbarMargin: 2
                readonly property int hoverDwell: 280
                readonly property int hoverCloseDelay: 260
            }
            readonly property var windowBadge: QtObject {
                readonly property real sizeRatio: 0.34
                readonly property int sizeMin: 14
                readonly property int sizeMax: 22
                readonly property real fontRatio: 0.56
                readonly property int fontMin: 10
                readonly property int paddingH: 5
                readonly property int inset: 2
                readonly property int borderWidth: 1
            }
            readonly property var appBadge: QtObject {
                readonly property real sizeRatio: 0.42
                readonly property int sizeMin: 16
                readonly property int sizeMax: 26
                readonly property real fontRatio: 0.56
                readonly property int fontMin: 10
                readonly property int paddingH: 5
                readonly property real outsideRatio: 0.18
                readonly property int borderWidth: 1
            }
            readonly property var overflow: QtObject {
                readonly property int maxRows: 7
                readonly property real gridInset: 0.18
                readonly property real gridGap: 0.08
                readonly property real gridCell: 0.16
            }
            readonly property var trashEmpty: QtObject {
                readonly property int busyDelay: 350
                readonly property int spinnerSize: 18
                readonly property int spinnerStroke: 2
                readonly property int spinnerSpeed: 900
            }
            readonly property var minimizeReaction: QtObject {
                readonly property real amplitudeRatio: 0.28
                readonly property int duration: 320
                readonly property int reducedDuration: 0
            }
        }
        readonly property var contextMenu: QtObject {
            readonly property int radius: 14
            readonly property int padding: 6
            readonly property int rowHeight: 26
            readonly property int minWidth: 180
            readonly property int shortcutGap: 24
            readonly property int submenuDelay: 150
            readonly property real attachedRadius: 12.5
            readonly property real closeScale: 0.95
        }
        readonly property var shortcutHint: QtObject {
            readonly property int gap: 2
            readonly property int fontSize: 12
            readonly property int modifierWeight: 400
            readonly property int keyWeight: 500
        }
        readonly property var focusRing: QtObject {
            readonly property int width: 2
            readonly property int offset: 2
            readonly property int radius: 6
        }
        readonly property var marquee: QtObject {
            readonly property int borderWidth: 2
            readonly property int radius: 0
        }
        readonly property var sidebar: QtObject {
            readonly property int width: 192
            readonly property int rowHeight: 28
            readonly property int rowRadius: 6
            readonly property int sectionGap: 16
            readonly property int iconSize: 16
            readonly property int categoryIconSize: 20
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
        readonly property var slider: QtObject {
            readonly property int height: 24
            readonly property int trackHeight: 4
            readonly property int knob: 16
            readonly property int minWidth: 160
            readonly property int captionGap: 2
            readonly property real step: 0.05
        }
        readonly property var select: QtObject {
            readonly property int height: 28
            readonly property int radius: 10
            readonly property int paddingH: 12
            readonly property int minWidth: 80
            readonly property int chevronSize: 12
            readonly property int chevronGap: 8
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
        readonly property var tooltip: QtObject {
            readonly property int dwell: 1400
            readonly property int offset: 8
            readonly property int radius: 999
            readonly property int paddingH: 12
            readonly property int paddingV: 8
            readonly property int maxWidth: 260
            readonly property int fontSize: 13
            readonly property int tailWidth: 16
            readonly property int tailHeight: 6
            readonly property color rimColor: "#ffffff"
            readonly property real rimOpacity: 0.5
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
        readonly property var osd: QtObject {
            readonly property int width: 180
            readonly property int height: 180
            readonly property int radius: 20
            readonly property int padding: 16
            readonly property int iconSize: 44
            readonly property int gap: 8
            readonly property int trackHeight: 6
            readonly property int trackRadius: 999
            readonly property int fontSize: 13
        }
        readonly property var skeleton: QtObject {
            readonly property int radius: 10
            readonly property int width: 120
            readonly property int height: 72
            readonly property real highlightRatio: 0.4
        }
        readonly property var settingsCategory: QtObject {
            readonly property int size: 32
            readonly property real radiusRatio: 0.24
            readonly property real iconRatio: 0.55
            readonly property real highlightRatio: 0.45
            readonly property real highlightOpacity: 0.28
            readonly property real glowOpacity: 0.42
        }
        readonly property var appsDrawer: QtObject {
            readonly property int contentPadding: 64
            readonly property int panelPadding: 50
            readonly property int panelPaddingVertical: 32
            readonly property int searchWidth: 360
            readonly property int searchMaxWidth: 640
            readonly property int pillGap: 8
            readonly property int tileSize: 64
            readonly property int tileRadius: 14
            readonly property int tileContentGap: 8
            readonly property int gridColumnGap: 24
            readonly property int gridRowGap: 16
            readonly property int labelSize: 12
            readonly property real scrimOpacity: 0.35
            readonly property real panelOpacity: 0.88
            readonly property real fallbackGlyphRatio: 0.6
            readonly property int emptyGlyphSize: 40
        }
    }

    readonly property var motion: QtObject {
        readonly property QtObject menuOpen: QtObject {
            readonly property int fullDuration: 160
            readonly property int duration: tokens.reducedMotion ? 0 : 160
            readonly property var curve: [0.2, 0.0, 0.0, 1.0, 1.0, 1.0]
        }
        readonly property QtObject menuClose: QtObject {
            readonly property int fullDuration: 183
            readonly property int duration: tokens.reducedMotion ? 0 : 183
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
            readonly property var curve: [0.22, 1.0, 0.36, 1.0, 1.0, 1.0]
        }
        readonly property QtObject dockHover: QtObject {
            readonly property int fullDuration: 135
            readonly property int duration: tokens.reducedMotion ? 0 : 135
            readonly property var curve: [0.3333333333333333, 0.0, 0.6666666666666666, 1.0, 1.0, 1.0]
        }
        readonly property QtObject dockMagnifyTrack: QtObject {
            readonly property int fullDuration: 100
            readonly property int duration: tokens.reducedMotion ? 0 : 100
            readonly property var curve: [0.2, 0.0, 0.0, 1.0, 1.0, 1.0]
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
        readonly property QtObject windowGenie: QtObject {
            readonly property int fullDuration: 450
            readonly property int duration: tokens.reducedMotion ? 0 : 450
            readonly property var curve: [0.3333333333333333, 0.0, 0.6666666666666666, 1.0, 1.0, 1.0]
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
        readonly property QtObject osd: QtObject {
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
        readonly property QtObject skeleton: QtObject {
            readonly property int fullDuration: 400
            readonly property int duration: tokens.reducedMotion ? 0 : 400
            readonly property var curve: [0.4, 0.0, 0.6, 1.0, 1.0, 1.0]
        }
    }
}
