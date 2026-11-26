// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit

// A keyboard chord recorder (T275). It is deliberately a small control, not a
// second shortcut engine: it builds the canonical `Modifier+Key` spelling the
// compositor's `parse_binding` accepts and reports it once. The owner writes
// `accessibility.shortcut`; the compositor installs the binding.
//
// Interaction: click/Space/Return starts recording, Escape cancels, Backspace
// (with no modifiers) clears, and the first non-modifier key completes the
// chord. A lone modifier never records.
Item {
    id: root

    property string chord: ""
    property bool recording: false
    property string placeholder: qsTr("Record Shortcut")
    property string accessibleName: qsTr("Accessibility shortcut")

    signal recorded(string chord)
    signal cleared()

    implicitWidth: 176
    implicitHeight: Theme.controls.button.height
    activeFocusOnTab: true
    opacity: enabled ? 1.0 : 0.45

    function startRecording() {
        if (!root.enabled)
            return;
        root.recording = true;
        root.forceActiveFocus();
    }

    function stopRecording() {
        root.recording = false;
    }

    function isModifierKey(key) {
        return key === Qt.Key_Shift || key === Qt.Key_Control
                || key === Qt.Key_Alt || key === Qt.Key_Meta
                || key === Qt.Key_AltGr || key === Qt.Key_CapsLock;
    }

    function keyName(key) {
        switch (key) {
        case Qt.Key_Tab: return "Tab";
        case Qt.Key_Backtab: return "Tab";
        case Qt.Key_Left: return "Left";
        case Qt.Key_Right: return "Right";
        case Qt.Key_Up: return "Up";
        case Qt.Key_Down: return "Down";
        case Qt.Key_Home: return "Home";
        case Qt.Key_End: return "End";
        case Qt.Key_PageUp: return "Page_Up";
        case Qt.Key_PageDown: return "Page_Down";
        case Qt.Key_Insert: return "Insert";
        case Qt.Key_Delete: return "Delete";
        case Qt.Key_Backspace: return "BackSpace";
        case Qt.Key_Return: return "Return";
        case Qt.Key_Enter: return "Return";
        case Qt.Key_Space: return "space";
        case Qt.Key_Escape: return "Escape";
        case Qt.Key_Print: return "Print";
        case Qt.Key_Menu: return "Menu";
        case Qt.Key_Comma: return "comma";
        case Qt.Key_Period: return "period";
        case Qt.Key_Slash: return "slash";
        case Qt.Key_Semicolon: return "semicolon";
        case Qt.Key_Apostrophe: return "apostrophe";
        case Qt.Key_BracketLeft: return "bracketleft";
        case Qt.Key_BracketRight: return "bracketright";
        case Qt.Key_Backslash: return "backslash";
        case Qt.Key_Minus: return "minus";
        case Qt.Key_Equal: return "equal";
        case Qt.Key_QuoteLeft: return "grave";
        default:
            break;
        }
        if (key >= Qt.Key_F1 && key <= Qt.Key_F12)
            return "F" + (key - Qt.Key_F1 + 1);
        if (key >= Qt.Key_A && key <= Qt.Key_Z)
            return String.fromCharCode(key);
        if (key >= Qt.Key_0 && key <= Qt.Key_9)
            return String.fromCharCode(key);
        return "";
    }

    function modifierParts(modifiers) {
        var parts = [];
        if (modifiers & Qt.MetaModifier)
            parts.push("Cmd");
        if (modifiers & Qt.ControlModifier)
            parts.push("Ctrl");
        if (modifiers & Qt.AltModifier)
            parts.push("Option");
        if (modifiers & Qt.ShiftModifier)
            parts.push("Shift");
        return parts;
    }

    // Display-only symbol form; the stored chord stays canonical.
    function displayChord(value) {
        if (value.length === 0)
            return "";
        var parts = value.split("+");
        var out = "";
        for (var i = 0; i < parts.length; ++i) {
            var part = parts[i];
            if (part === "Cmd")
                out += "⌘";
            else if (part === "Ctrl")
                out += "⌃";
            else if (part === "Option")
                out += "⌥";
            else if (part === "Shift")
                out += "⇧";
            else
                out += part;
        }
        return out;
    }

    Rectangle {
        anchors.fill: parent
        radius: Theme.controls.button.radius
        color: root.recording ? Theme.color.controlHover : Theme.color.controlFill
        border.width: root.activeFocus || root.recording
                      ? 2 : Theme.controls.window.borderWidth
        border.color: root.recording ? Theme.color.accent
                    : root.activeFocus ? Theme.color.focusRing : Theme.color.border
        antialiasing: true
    }

    Text {
        anchors.fill: parent
        anchors.leftMargin: Theme.controls.button.paddingH
        anchors.rightMargin: Theme.controls.button.paddingH
        verticalAlignment: Text.AlignVCenter
        horizontalAlignment: Text.AlignHCenter
        text: root.recording ? qsTr("Press keys…")
              : (root.chord.length > 0 ? root.displayChord(root.chord)
                                       : root.placeholder)
        color: root.recording ? Theme.color.accent : Theme.color.textPrimary
        font.pixelSize: Theme.controls.button.fontSize
        elide: Text.ElideRight
    }

    TapHandler {
        onTapped: root.startRecording()
    }

    Keys.onPressed: (event) => {
        if (!root.recording) {
            if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                    || event.key === Qt.Key_Space) {
                root.startRecording();
                event.accepted = true;
            }
            return;
        }
        if (event.key === Qt.Key_Escape) {
            root.stopRecording();
            event.accepted = true;
            return;
        }
        if ((event.key === Qt.Key_Backspace || event.key === Qt.Key_Delete)
                && event.modifiers === Qt.NoModifier) {
            root.chord = "";
            root.cleared();
            root.stopRecording();
            event.accepted = true;
            return;
        }
        if (root.isModifierKey(event.key)) {
            event.accepted = true;
            return;
        }
        var name = root.keyName(event.key);
        if (name.length === 0)
            return;
        var parts = root.modifierParts(event.modifiers);
        parts.push(name);
        root.chord = parts.join("+");
        root.recording = false;
        root.recorded(root.chord);
        event.accepted = true;
    }

    Accessible.role: Accessible.Button
    Accessible.name: root.accessibleName
    Accessible.description: root.recording
        ? qsTr("Recording. Press Escape to cancel.")
        : (root.chord.length > 0 ? root.displayChord(root.chord)
                                 : qsTr("Activate to record a shortcut."))
    Accessible.focusable: true
    Accessible.onPressAction: root.startRecording()
}
