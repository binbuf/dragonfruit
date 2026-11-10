// SPDX-License-Identifier: MIT
import QtQuick

// T233: the per-display menu-bar policy.
//
// The compositor composites the one all-output menu-bar surface on every
// output, filtered by its per-output auto-hide state (ADR 0220). This object
// is the pure model for the part that must differ by display: each display
// gets a bar, only the active display owns the application menu, and status
// items follow the configured policy (`all` displays or the active one).
//
// Keeping the rule here means a future per-display surface implementation and
// the scene tests share one answer; the shell does not scatter the "which
// display owns the app menu" question through QML.
QtObject {
    id: policy

    // [{ name, active, online, fullscreen, notchWidth }]
    property var displays: []
    // "all" (default) or "active".
    property string statusPolicy: "all"

    readonly property var _online: {
        var out = [];
        for (var i = 0; i < displays.length; ++i) {
            var display = displays[i] || {};
            if (display.online === false)
                continue;
            out.push(display);
        }
        return out;
    }

    // The active display is the one the focused window is on; before any
    // focus the first online display owns the menu (the primary fallback).
    readonly property string activeDisplay: {
        for (var i = 0; i < _online.length; ++i) {
            if (_online[i].active === true)
                return String(_online[i].name);
        }
        return _online.length > 0 ? String(_online[0].name) : "";
    }

    function barFor(name) {
        var found = null;
        for (var i = 0; i < displays.length; ++i) {
            if (String(displays[i].name) === String(name)) {
                found = displays[i];
                break;
            }
        }
        if (!found || found.online === false) {
            return { name: String(name), present: false, active: false,
                     ownsAppMenu: false, showStatusItems: false,
                     notchWidth: 0, fullscreen: false };
        }
        var active = String(name) === activeDisplay;
        return {
            name: String(name),
            present: true,
            active: active,
            ownsAppMenu: active,
            showStatusItems: statusPolicy !== "active" || active,
            notchWidth: found.notchWidth !== undefined ? Number(found.notchWidth) : 0,
            fullscreen: found.fullscreen === true
        };
    }

    // One bar state per display, in input order.
    readonly property var bars: {
        var out = [];
        for (var i = 0; i < displays.length; ++i) {
            var display = displays[i] || {};
            if (display.online === false)
                continue;
            out.push(barFor(display.name));
        }
        return out;
    }

    readonly property int barCount: bars.length

    function ownsAppMenu(name) {
        return barFor(name).ownsAppMenu;
    }

    function showsStatusItems(name) {
        return barFor(name).showStatusItems;
    }
}
