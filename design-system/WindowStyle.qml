// SPDX-License-Identifier: MIT
pragma Singleton
import QtQuick
import Dragonfruit

// The window-chrome style model (T230b). Chrome height is a property of the
// window style, not one universal height: borderless 0, titlebar-only 32,
// unified-compact 40, expanded 48, unified 52, and preference 88. The 66 pt
// icon-and-label toolbar is `unified` plus the labelled toolbar row; a
// document window can add its own toolbar row (e.g. 48 + 22 = 70).
//
// The table lives in `design-system/tokens/tokens.json` so the compositor's
// SSD renderer reads the same values. This singleton is the one QML resolver:
// first-party apps name a style and never hard-code a height.
QtObject {
    id: model

    readonly property var styleIds: [
        "borderless", "titlebar", "unifiedCompact", "expanded", "unified",
        "preference"
    ]

    readonly property var definitions: ({
        borderless: Theme.controls.windowStyle.borderless,
        titlebar: Theme.controls.windowStyle.titlebar,
        unifiedCompact: Theme.controls.windowStyle.unifiedCompact,
        expanded: Theme.controls.windowStyle.expanded,
        unified: Theme.controls.windowStyle.unified,
        preference: Theme.controls.windowStyle.preference
    })

    readonly property int trafficLightSize:
        Theme.controls.windowStyle.trafficLightSize
    readonly property int trafficLightSpacing:
        Theme.controls.windowStyle.trafficLightSpacing
    readonly property int trafficLightLeadingInset:
        Theme.controls.windowStyle.trafficLightLeadingInset
    readonly property int labelledToolbarRow:
        Theme.controls.windowStyle.labelledToolbarRow

    function hasStyle(style) {
        return model.definitions[style] !== undefined;
    }

    function definition(style) {
        return model.definitions[style] || model.definitions.titlebar;
    }

    function height(style) {
        return model.definition(style).height;
    }

    function titleRowHeight(style) {
        return model.definition(style).titleRow;
    }

    function toolbarRowHeight(style) {
        return model.definition(style).toolbarRow;
    }

    // The final chrome height: the style baseline plus an optional toolbar
    // row. `options` is either a number (an extra toolbar row in points) or
    // `{ toolbarRowHeight, labelledToolbar }`.
    function chromeHeight(style, options) {
        var extra = 0;
        if (typeof options === "number") {
            extra = options;
        } else if (options) {
            extra = options.toolbarRowHeight || 0;
            if (options.labelledToolbar)
                extra += model.labelledToolbarRow;
        }
        return model.height(style) + Math.max(0, extra);
    }

    // The measured top inset centres a 16 pt light in the bar: 8 pt in a
    // 32 pt bar, 18 pt in a 52 pt bar. Derived from the final height so the
    // 66/70 pt variants stay centred too.
    function trafficTopInset(chromeHeight) {
        return Math.max(0, (chromeHeight - model.trafficLightSize) / 2);
    }

    // Row spacing is centre-to-centre (23 pt); Qt's Row uses the edge gap.
    function trafficLightGap() {
        return model.trafficLightSpacing - model.trafficLightSize;
    }

    function isBorderless(style) {
        return model.height(style) === 0;
    }
}
