// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Accessibility pane (T-15.14b).
//
// The pane and the Control Center tile are one functional unit: both read the
// same host-stack adapter (the AT-SPI accessibility bus, `org.a11y.Status`)
// through the bridge host. The pane mirrors the macOS pane
// (SystemSettings_Accessibility.md / System_Preferences.md) as a grouped inset
// card with the honest Linux rows (ADR 0122, ADR 0144):
//
//   * `Screen Reader` is the live AT-SPI `ScreenReaderEnabled` flag; the
//     reference `VoiceOver` row maps here and drops the Apple brand name. It is
//     a read-only status, because the host stack publishes it and offers no
//     setter.
//   * `Accessibility` is the live toolkit bridge flag (`IsEnabled`), the
//     prerequisite the pane reports.
//   * `Reduce Motion` is the reference `Motion` row collapsed to an inline
//     toggle. It is the one durable preference, the settingsd key
//     `accessibility.reduceMotion`, applied live by the shell theme, the Dock,
//     and the compositor.
//
// Deviations from the macOS capture reference (recorded so later tasks do not
// re-litigate them):
//   * `Zoom`, `Hover Text`, and `Display` have no Linux host owner yet; a
//     compositor magnifier and a durable display-contrast preference are a
//     follow-up, so no dead disclosure row is invented.
//   * The whole `Hearing` card (`Hearing Devices`, `Audio`, `Captions`, `Live
//     Captions`, `Name Recognition`) is Apple-only or host-ownerless (ADR
//     0122) and is omitted.
//   * `Motor`, `Speech`, and `Cognition` are named only in the reference header
//     copy and were not captured; there is no host owner to bind them to.
//   * `Learn more...` and the trailing `?` help are omitted project-wide.
Item {
    id: root

    // The bridge host's Accessibility view (empty when absent).
    readonly property var view: Settings.accessibility
    readonly property bool available: Settings.accessibilityAvailable
    readonly property bool ready: root.available && root.view.state === "available"
    readonly property bool enabledOn: root.ready && root.view.enabled === true
    readonly property bool screenReaderOn: root.ready && root.view.screenReader === true
    readonly property string bridgeLabel: root.ready && root.view.enabledLabel !== undefined
        ? String(root.view.enabledLabel) : qsTr("Off")
    readonly property string screenReaderLabel:
        root.ready && root.view.screenReaderLabel !== undefined
            ? String(root.view.screenReaderLabel) : qsTr("Off")

    // The one durable preference (settingsd), applied live by the shell.
    readonly property bool reduceMotion:
        Settings.values["accessibility.reduceMotion"] === true

    // Test surface (used by tst_settings_accessibility.qml).
    property alias statusGroup: statusGroup
    property alias motionGroup: motionGroup
    property alias absenceNote: absenceNote
    property alias bridgeRow: bridgeRow
    property alias screenReaderRow: screenReaderRow
    property alias reduceMotionToggle: reduceMotionToggle

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    // Opening the pane asks the host for a re-read (a no-op when absent).
    Component.onCompleted: Settings.refreshAccessibility()

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── The live AT-SPI status ────────────────────────────────────────
        SettingsGroup {
            id: statusGroup
            width: parent.width
            visible: root.ready
            title: qsTr("Vision")

            SettingsRow {
                id: screenReaderRow
                objectName: "accessibilityScreenReaderRow"
                width: parent.width
                label: qsTr("Screen Reader")
                description: qsTr("Reported by the assistive technology over AT-SPI.")
                controlData: Text {
                    objectName: "accessibilityScreenReaderValue"
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.screenReaderLabel
                    color: root.screenReaderOn ? Theme.color.accent
                                               : Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            SettingsRow {
                id: bridgeRow
                objectName: "accessibilityBridgeRow"
                width: parent.width
                label: qsTr("Accessibility")
                description: qsTr("The toolkit accessibility bridge.")
                showSeparator: false
                controlData: Text {
                    objectName: "accessibilityBridgeValue"
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.bridgeLabel
                    color: root.enabledOn ? Theme.color.accent
                                          : Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        // ── Host absent ───────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !root.ready

            Text {
                id: absenceNote
                objectName: "accessibilityAbsenceNote"
                width: parent.width
                text: qsTr("The accessibility service (AT-SPI) is not running, so "
                           + "the live screen reader status is unavailable. Your "
                           + "preference below still applies.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── The durable motion preference ─────────────────────────────────
        SettingsGroup {
            id: motionGroup
            width: parent.width
            title: qsTr("Motion")

            SettingsRow {
                width: parent.width
                label: qsTr("Reduce Motion")
                description: qsTr("Collapse interface animations to reduce motion.")
                showSeparator: false
                controlData: Toggle {
                    id: reduceMotionToggle
                    objectName: "accessibilityReduceMotionToggle"
                    text: ""
                    onToggled: (checked) =>
                        Settings.set("accessibility.reduceMotion", checked)
                }
            }
        }
    }

    // Keep the toggle in step with external changes.
    Binding {
        target: reduceMotionToggle
        property: "checked"
        value: root.reduceMotion
    }
}