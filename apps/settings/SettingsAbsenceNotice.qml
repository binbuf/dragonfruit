// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// One absence explanation rendered from a `SettingsAbsence` kind (T224).
//
// Panes bind their existing `absenceNote` alias to this component so the
// reason is one line, the policy is shared, and a delegated owner offers an
// explicit hand-off instead of an inert control. The component only draws the
// note/action; the pane keeps control visibility and enabling because a pane
// can mix a live settingsd half with an absent adapter half.
Item {
    id: root

    property string kind: "disabled-with-reason"
    property string reason: ""
    property string delegateLabel: qsTr("Open\u2026")
    // `live-on-defaults` is normally silent, but a pane that wants to explain
    // "running on defaults" can ask for the note explicitly.
    property bool showWhenLive: false
    // T267: the owner can gate the note without overriding `visible` (which
    // would break the note's own live visibility rule).
    property bool show: true

    readonly property bool controlsVisible: SettingsAbsence.controlsVisible(kind)
    readonly property bool controlsEnabled: SettingsAbsence.controlsEnabled(kind)
    readonly property bool writable: SettingsAbsence.writable(kind)
    readonly property bool delegated: SettingsAbsence.delegated(kind)
    readonly property bool noteVisible: root.show && SettingsAbsence.showsNote(kind)
        && reason.length > 0 && (kind !== "live-on-defaults" || showWhenLive)

    signal delegateRequested()

    property alias text: note.text

    implicitHeight: content.implicitHeight
    width: parent ? parent.width : 0
    visible: root.noteVisible
    // T267: an absence-only pane still needs a keyboard stop and a visible
    // focus ring, otherwise the whole pane is unreachable without a pointer.
    activeFocusOnTab: root.noteVisible

    FocusRing {
        target: root
        cornerRadius: Theme.primitive.radius.sm
        shown: root.activeFocus
    }

    // T267: the absence note is a named static-text/alert region, not an
    // unlabelled sentence, and a newly visible note is announced once.
    onNoteVisibleChanged: {
        if (root.noteVisible)
            Accessible.announce(root.reason);
    }
    onReasonChanged: {
        if (root.noteVisible)
            Accessible.announce(root.reason);
    }

    Column {
        id: content
        width: parent.width
        spacing: Theme.primitive.spacing.sm

        Text {
            id: note
            objectName: "settingsAbsenceNoteText"
            width: parent.width
            text: root.reason
            color: Glass.increaseContrast ? Theme.color.textPrimary
                                          : Theme.color.textSecondary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
            Accessible.role: Accessible.AlertMessage
            Accessible.name: root.reason
        }

        Button {
            objectName: "settingsAbsenceDelegate"
            visible: root.delegated
            text: root.delegateLabel
            accessibleName: root.delegateLabel
            onClicked: root.delegateRequested()
        }
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: root.reason
    Accessible.description: qsTr("This setting is unavailable: %1").arg(root.kind)
    Accessible.focusable: true
}
