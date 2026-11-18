// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The System Settings back/forward control (T286): one 78 x 36 pill with two
// 38 pt buttons and a 1 x 18 pt divider between them. The pill is the
// left-most element of the detail toolbar; the pane title follows it.
//
// Both buttons are Tab stops with their own focus ring, a disabled state, and
// an AT-SPI Button role. The control reports intent only; the shell owns the
// history it walks.
Item {
    id: root

    property bool canGoBack: true
    property bool canGoForward: true
    property bool rtl: false

    signal backRequested()
    signal forwardRequested()

    property alias pill: pill
    property alias backButton: backButton
    property alias forwardButton: forwardButton
    property alias divider: divider

    implicitWidth: Theme.controls.settingsWindow.navPillWidth
    implicitHeight: Theme.controls.settingsWindow.navPillHeight
    activeFocusOnTab: false

    Rectangle {
        id: pill
        anchors.fill: parent
        radius: Theme.controls.settingsWindow.navPillRadius
        color: Theme.color.controlFill
        border.width: Theme.controls.window.borderWidth
        border.color: Glass.increaseContrast ? Theme.color.textPrimary
                                             : Theme.color.border
        antialiasing: true
    }

    Row {
        anchors.fill: parent
        layoutDirection: root.rtl ? Qt.RightToLeft : Qt.LeftToRight

        SettingsNavButton {
            id: backButton
            objectName: "settingsNavBack"
            enabled: root.canGoBack
            glyph: root.rtl ? "chevron-right" : "chevron-left"
            label: qsTr("Back")
            onActivated: root.backRequested()
        }

        Rectangle {
            id: divider
            anchors.verticalCenter: parent.verticalCenter
            width: Theme.controls.window.borderWidth
            height: Theme.controls.settingsWindow.navDividerHeight
            color: Glass.increaseContrast ? Theme.color.textPrimary
                                          : Theme.color.separator
        }

        SettingsNavButton {
            id: forwardButton
            objectName: "settingsNavForward"
            enabled: root.canGoForward
            glyph: root.rtl ? "chevron-left" : "chevron-right"
            label: qsTr("Forward")
            onActivated: root.forwardRequested()
        }
    }

    // One half of the pill. The hit target is the full 38 x 36 slot; the
    // glyph is centred and dims with the disabled state.
    component SettingsNavButton: Item {
        id: button

        property string glyph: "chevron-left"
        property string label: ""
        signal activated()

        width: Theme.controls.settingsWindow.navButtonWidth
        height: Theme.controls.settingsWindow.navPillHeight
        activeFocusOnTab: true

        readonly property bool hovered: buttonHover.hovered

        Rectangle {
            anchors.fill: parent
            anchors.margins: Theme.controls.window.borderWidth
            radius: Theme.controls.settingsWindow.navPillRadius
                   - Theme.controls.window.borderWidth
            color: button.enabled && button.hovered && !button.activeFocus
                   ? Theme.color.controlHover : "transparent"
            antialiasing: true
        }

        Icon {
            anchors.centerIn: parent
            name: button.glyph
            size: Theme.controls.button.iconSize
            color: !button.enabled
                   ? Theme.color.textTertiary
                   : (Glass.increaseContrast ? Theme.color.textPrimary
                                             : Theme.color.textSecondary)
        }

        HoverHandler { id: buttonHover }

        TapHandler {
            onTapped: {
                if (button.enabled)
                    button.activated();
            }
        }

        FocusRing {
            target: button
            cornerRadius: Theme.controls.settingsWindow.navPillRadius
            shown: button.activeFocus
        }

        Keys.onPressed: (event) => {
            if (!button.enabled)
                return;
            if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                    || event.key === Qt.Key_Space) {
                button.activated();
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.Button
        Accessible.name: button.label
        Accessible.focusable: true
        Accessible.onPressAction: {
            if (button.enabled)
                button.activated();
        }
    }
}
