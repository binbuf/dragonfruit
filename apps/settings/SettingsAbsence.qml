// SPDX-License-Identifier: MIT
import QtQuick

// The five absence kinds the Settings map declares (T224).
//
// One pure policy table so every pane answers the same questions the same way:
//
//   live-on-defaults      controls stay visible and enabled; the owner's
//                         defaults are the contract, and an optional note says
//                         the daemon is away.
//   disabled-with-reason  controls stay visible but inert; the note carries the
//                         reason, so a disabled control is never a dead end.
//   hidden                the affected group is not shown at all.
//   read-only             reads stay visible, writes are disabled; the note
//                         explains the owner is not accepting changes.
//   delegated             the surface lives in another app/service; the note
//                         offers the delegate action instead of a fake control.
//
// `SettingsAbsenceNotice` renders the note and exposes the policy; panes keep
// their own control enabling because a pane can mix kinds.
pragma Singleton
QtObject {
    id: root

    readonly property var kinds: [
        "live-on-defaults", "disabled-with-reason", "hidden",
        "read-only", "delegated"
    ]

    function valid(kind) {
        return root.kinds.indexOf(kind) >= 0;
    }

    // Whether the affected controls are drawn at all.
    function controlsVisible(kind) {
        return kind !== "hidden" && kind !== "delegated";
    }

    // Whether the controls accept a write. `read-only` and
    // `disabled-with-reason` stay visible but inert; `delegated` hands off.
    function writable(kind) {
        return kind === "live-on-defaults";
    }

    function controlsEnabled(kind) {
        return root.writable(kind);
    }

    // Whether the pane shows a one-line explanation. `hidden` is the only kind
    // that is silent by definition; `live-on-defaults` may still explain that
    // the session is running on schema defaults.
    function showsNote(kind) {
        return kind !== "hidden";
    }

    function delegated(kind) {
        return kind === "delegated";
    }

    // The map's absence kind for a pane, with `live-on-defaults` as the safe
    // fallback for a custom surface that has not been mapped yet.
    function forPane(pane) {
        if (pane && pane.absence !== undefined && root.valid(String(pane.absence)))
            return String(pane.absence);
        return "live-on-defaults";
    }
}
