// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
pragma ComponentBehavior: Bound

// The battery status menu (T-07.5b, T232): the charge state, level, time
// remaining, health, the active power mode, and the Settings link, drawn in
// the design-system popup. The model is the bridge host's `battery` view
// (services/system-status); the popover raises `refreshRequested` and the one
// power-profile write, and the shell forwards them to the host.
Item {
    id: root

    // The host's decoded battery view ({state, present, glyph, label, percent,
    // level, charging, onBattery, chargeState, profilesAvailable, profiles...}).
    property var model: ({})
    property Item anchorItem: null
    property alias popup: popup
    property alias open: popup.open
    // T-21.2c: the menu bar pushes the compositor's Popup role state here.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0
    property string materialPreset: "statusMenu"
    property string materialBackdrop: "trusted"
    readonly property string materialFallbackReason:
        Material.fallbackReason(materialFallback, materialBlurActive, materialTier)

    readonly property bool available: model.state === "available"
    readonly property bool present: model.present === true
    readonly property bool charging: model.charging === true
    readonly property bool onBattery: model.onBattery === true
    readonly property int percent: model.percent !== undefined ? model.percent : 0
    readonly property real level: model.level !== undefined ? model.level : 0.0
    readonly property string label: model.label !== undefined ? model.label : ""
    readonly property string chargeState: model.chargeState !== undefined
        ? String(model.chargeState) : ""
    readonly property string errorMessage: model.error !== undefined
        ? String(model.error) : ""
    readonly property string healthLabel: model.healthLabel !== undefined
        ? String(model.healthLabel) : ""
    readonly property bool profilesAvailable: model.profilesAvailable === true
    readonly property var profiles: model.profiles !== undefined ? model.profiles : []
    readonly property string activeProfile: model.activeProfile !== undefined
        ? String(model.activeProfile) : ""
    readonly property string profileLabel: model.profileLabel !== undefined
        ? String(model.profileLabel) : ""
    readonly property bool low: root.present && root.onBattery
        && root.percent > 0 && root.percent <= 20
    readonly property bool full: root.present
        && (root.chargeState === "fully-charged" || root.percent >= 99)
    readonly property string stateLabel: {
        if (!root.present)
            return qsTr("No battery");
        if (root.low)
            return qsTr("Low Battery");
        if (root.full)
            return qsTr("Fully Charged");
        if (root.charging)
            return qsTr("Charging");
        if (root.onBattery)
            return qsTr("On Battery");
        return qsTr("Plugged In");
    }
    property int profileHighlightIndex: -1

    signal refreshRequested()
    signal powerProfileSelected(string profile)
    signal settingsRequested()
    // Raised when the popup closes for any reason, for the bar's state
    // (T-07.5b keyboard a11y).
    signal closed()

    // A compact "H:MM" duration for a non-zero second count.
    function formatDuration(seconds) {
        if (seconds === undefined || seconds === null || seconds <= 0)
            return "";
        var totalMinutes = Math.floor(seconds / 60);
        var hours = Math.floor(totalMinutes / 60);
        var minutes = totalMinutes % 60;
        if (hours <= 0)
            return qsTr("%1 min").arg(minutes);
        return qsTr("%1:%2").arg(hours).arg(minutes < 10 ? "0" + minutes : minutes);
    }

    readonly property string remaining: {
        if (root.charging)
            return root.formatDuration(root.model.timeToFull);
        if (root.onBattery)
            return root.formatDuration(root.model.timeToEmpty);
        return "";
    }

    function selectProfile(id) {
        if (!root.profilesAvailable || id.length === 0)
            return;
        for (var i = 0; i < root.profiles.length; ++i) {
            if (String(root.profiles[i].id) === id
                    && root.profiles[i].active === true)
                return;
        }
        root.powerProfileSelected(id);
    }

    function moveProfileHighlight(delta) {
        var n = root.profiles.length;
        if (n <= 0 || !root.profilesAvailable) {
            root.profileHighlightIndex = -1;
            return;
        }
        var i = root.profileHighlightIndex < 0
            ? (delta > 0 ? -1 : 0) : root.profileHighlightIndex;
        root.profileHighlightIndex = ((i + delta) % n + n) % n;
    }

    function activateProfileHighlight() {
        if (root.profileHighlightIndex < 0
                || root.profileHighlightIndex >= root.profiles.length)
            return;
        root.selectProfile(String(root.profiles[root.profileHighlightIndex].id));
    }

    Popup {
        id: popup
        objectName: "batteryPopup"
        anchorItem: root.anchorItem
        preferredWidth: 280
        accessibleRole: Accessible.PopupMenu
        accessibleName: qsTr("Battery")
        escapeCloses: true
        motionPreset: "menu"
        materialPreset: root.materialPreset
        materialBackdrop: root.materialBackdrop
        materialBlurActive: root.materialBlurActive
        materialTier: root.materialTier
        materialFallback: root.materialFallback

        onOpened: {
            root.profileHighlightIndex = -1;
            root.refreshRequested();
            if (root.parent && root.width > 0)
                popup.x = Math.min(popup.x,
                                   root.parent.width - popup.width
                                   - Theme.controls.menuBar.paddingH);
        }
        onClosed: {
            root.profileHighlightIndex = -1;
            root.closed();
        }

        Keys.onUpPressed: (event) => {
            root.moveProfileHighlight(-1);
            event.accepted = true;
        }
        Keys.onDownPressed: (event) => {
            root.moveProfileHighlight(1);
            event.accepted = true;
        }
        Keys.onReturnPressed: (event) => {
            root.activateProfileHighlight();
            event.accepted = true;
        }

        Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            Item {
                width: parent.width
                height: Theme.controls.button.fontSize + 4

                Text {
                    text: qsTr("Battery")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    font.weight: Theme.primitive.font.weightMedium
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    objectName: "batteryRemaining"
                    visible: root.remaining.length > 0
                    text: root.charging
                          ? qsTr("%1 until full").arg(root.remaining)
                          : qsTr("%1 left").arg(root.remaining)
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                }
            }

            Text {
                objectName: "batteryStateNote"
                visible: !root.available || root.model.state === "error"
                width: parent.width
                text: root.model.state === "error"
                      ? (root.errorMessage.length > 0 ? root.errorMessage
                                                      : qsTr("Battery is unavailable"))
                      : qsTr("Battery unavailable")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            // The glyph carries the fill; the percentage is the label.
            Row {
                width: parent.width
                spacing: Theme.primitive.spacing.sm
                visible: root.available && root.present

                StatusGlyph {
                    name: root.charging ? "battery-charging" : "battery"
                    size: Theme.controls.menuBar.iconSize + 8
                    level: root.level
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    objectName: "batteryPercent"
                    text: root.label.length > 0 ? root.label
                                                : qsTr("%1%").arg(root.percent)
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    anchors.verticalCenter: parent.verticalCenter
                }
            }

            Text {
                visible: root.available && root.present
                width: parent.width
                text: root.stateLabel
                color: root.low ? Theme.color.danger : Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Text {
                objectName: "batteryHealth"
                visible: root.available && root.present
                         && root.healthLabel.length > 0
                width: parent.width
                text: qsTr("Battery Health: %1").arg(root.healthLabel)
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Text {
                visible: root.available && !root.present
                width: parent.width
                text: qsTr("This computer has no battery.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Rectangle {
                visible: root.available && (root.present || root.profilesAvailable)
                width: parent.width
                height: Theme.controls.window.borderWidth
                color: Theme.color.separator
            }

            Text {
                visible: root.available && root.profilesAvailable
                width: parent.width
                text: qsTr("Power Mode")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightMedium
            }

            Repeater {
                objectName: "batteryProfileRows"
                model: root.profilesAvailable ? root.profiles : []

                delegate: Item {
                    id: profileRow
                    required property var modelData
                    required property int index
                    objectName: "batteryProfile-" + profileRow.modelData.id
                    readonly property bool active: profileRow.modelData.active === true
                    width: parent.width
                    height: Theme.controls.popup.rowHeight
                    enabled: !profileRow.active
                    opacity: 1.0

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.controls.focusRing.radius
                        color: (profileHover.hovered
                                || root.profileHighlightIndex === profileRow.index)
                               ? Theme.color.controlHover : "transparent"
                        antialiasing: true
                    }

                    Icon {
                        visible: profileRow.active
                        name: "check"
                        size: Theme.controls.button.iconSize
                        color: Theme.color.accent
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.primitive.spacing.xs
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Text {
                        text: profileRow.modelData.label !== undefined ? profileRow.modelData.label
                                                            : profileRow.modelData.id
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.button.iconSize
                                         + Theme.primitive.spacing.sm
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Text {
                        visible: profileRow.active
                        text: qsTr("Active")
                        color: Theme.color.accent
                        font.pixelSize: Theme.primitive.font.sizeSm
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.primitive.spacing.xs
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    HoverHandler { id: profileHover }

                    TapHandler {
                        onTapped: root.selectProfile(String(profileRow.modelData.id))
                    }

                    Accessible.role: Accessible.MenuItem
                    Accessible.name: (profileRow.modelData.label !== undefined
                                      ? profileRow.modelData.label : profileRow.modelData.id)
                                     + (profileRow.active ? qsTr(", active") : "")
                    Accessible.description: profileRow.active
                        ? qsTr("Already active") : ""
                    Accessible.onPressAction: root.selectProfile(String(profileRow.modelData.id))
                }
            }

            Text {
                objectName: "batteryProfilesAbsent"
                visible: root.available && !root.profilesAvailable
                width: parent.width
                text: qsTr("Power modes are unavailable.")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Button {
                objectName: "batterySettingsButton"
                width: parent.width
                text: qsTr("Battery Settings…")
                variant: "secondary"
                accessibleName: qsTr("Open Battery Settings")
                onClicked: root.settingsRequested()
            }
        }
    }

    Accessible.role: Accessible.PopupMenu
    Accessible.name: qsTr("Battery")
}
