// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A passive width probe for one menu-bar status slot.
//
// The status row's budget is measured from the live widths so a localized
// clock, a tray label, or a themed icon moves the right number of items into
// overflow. The old measurement row instantiated a complete `StatusItem` per
// entry — including a second `MenuBarClock` with its own boundary timer and
// the full hover/tap/tooltip tree. This object keeps only the parts that
// affect width: the same `TextMetrics` typography and the same token spacing
// the visible `StatusItem` uses.
//
// It is an `Item` so a `Repeater` can own it, but it is invisible, has no
// input handlers, and paints nothing. `measuredWidth` is the contract:
// `tst_menubar` compares it with the visible slot for icon, label, combined
// and clock rows.
Item {
    id: root

    property var entry: null
    property var clockSource: null
    property bool showDate: false

    readonly property string itemId:
        entry && entry.id !== undefined ? String(entry.id) : ""
    readonly property string icon:
        entry && entry.icon !== undefined ? String(entry.icon) : ""
    readonly property url iconSource:
        entry && entry.iconSource !== undefined ? entry.iconSource : ""
    readonly property string label:
        entry && entry.label !== undefined ? String(entry.label) : ""
    readonly property string kind:
        entry && entry.kind !== undefined ? String(entry.kind) : "status"
    readonly property bool available: entry ? entry.available !== false : true
    readonly property bool isClock: root.kind === "clock"
    readonly property bool hasImageIcon: root.iconSource.toString().length > 0
    readonly property bool hasGlyphIcon: root.icon.length > 0
                                        && !root.hasImageIcon

    readonly property string clockTimeText:
        root.clockSource ? root.clockSource.timeText : ""
    readonly property string clockDateText:
        root.clockSource ? root.clockSource.dateText : ""

    TextMetrics {
        id: labelMetrics
        font.pixelSize: Theme.controls.menuBar.fontSize
        text: root.label
    }
    TextMetrics {
        id: clockDateMetrics
        font.pixelSize: Theme.controls.menuBar.fontSize
        font.features: ({ "tnum": 1 })
        text: root.clockDateText
    }
    TextMetrics {
        id: clockTimeMetrics
        font.pixelSize: Theme.controls.menuBar.fontSize
        font.features: ({ "tnum": 1 })
        text: root.clockTimeText
    }

    // Mirrors `StatusItem.implicitWidth`:
    //   * a hidden slot is zero width;
    //   * a non-clock row is its visible icon plus its label, separated by
    //     `labelGap` only when both are present;
    //   * a clock row is the date label (when shown) plus the time label,
    //     separated by `clockGap`;
    //   * every row adds the status slot's horizontal padding.
    readonly property real measuredWidth: {
        if (!root.available)
            return 0;
        var padding = 2 * Theme.controls.menuBar.statusItemPaddingH;
        if (root.isClock) {
            var clockWidth = 0;
            var clockParts = 0;
            if (root.showDate) {
                clockWidth += clockDateMetrics.advanceWidth;
                clockParts++;
            }
            clockWidth += clockTimeMetrics.advanceWidth;
            clockParts++;
            if (clockParts > 1)
                clockWidth += Theme.controls.menuBar.clockGap;
            return clockWidth + padding;
        }
        var width = 0;
        if (root.hasGlyphIcon || root.hasImageIcon)
            width += Theme.controls.menuBar.iconSize;
        if (root.label.length > 0) {
            if (width > 0)
                width += Theme.controls.menuBar.labelGap;
            width += labelMetrics.advanceWidth;
        }
        return width + padding;
    }

    visible: false
    width: 0
    height: 0
}
