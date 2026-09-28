// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Battery pane (T-15.6b).
//
// The pane and the Control Center tile are one functional unit. `Low Power
// Mode` selects the active power profile through the `Settings` singleton to
// the power-profiles adapter (T-15.6a) via the bridge host; `Battery Health`
// and `Charging` are read from UPower. The two daemons are independent: a
// desktop with no battery still selects a profile, and a laptop whose
// power-profiles-daemon is masked still reports its battery — each absence is a
// normal state with its own note, never an error.
//
// Deviations from the macOS capture (ADR 0122), recorded so later tasks do not
// re-litigate them:
//   * the captured `Low Power Mode` value vocabulary (`Never`/`Always`) does
//     not exist in power-profiles-daemon; the picker maps onto the daemon's
//     `power-saver`/`balanced`/`performance` profiles, the same vocabulary the
//     adapter and Control Center tile use;
//   * the `i` detail panels behind `Battery Health` and `Charging` cover
//     Apple-only hardware health and optimized charging, so we keep the rows
//     and show UPower's capacity/cycles and the charge estimate instead;
//   * `Last 24 Hours`/`Last 10 Days`, the battery-level chart, the
//     `Last charged to 100%` timestamp, `Screen On Usage`, and `Options...`
//     have no provider in this build — UPower no longer exposes charge
//     history — so the range switch is live and the history group renders an
//     honest absent state (provider T-15.x). No dead controls ship;
//   * the Apple Account/sidebar row is omitted project-wide.
Item {
    id: root

    // The bridge host's battery view (empty when absent).
    readonly property var view: Settings.battery
    readonly property bool available: Settings.batteryAvailable
    // UPower is present and this machine has a battery.
    readonly property bool present: view.state === "available"
                                    && view.present === true
    // power-profiles-daemon is present and exposed at least one profile.
    readonly property bool profilesAvailable: view.state === "available"
                                              && view.profilesAvailable === true
    readonly property var profiles: view.profiles !== undefined ? view.profiles : []
    readonly property bool ready: root.available && root.present
    readonly property bool profileReady: root.available && root.profilesAvailable
                                       && root.profiles.length > 0

    readonly property int percent: view.percent !== undefined ? Number(view.percent) : 0
    readonly property bool charging: view.charging === true
    readonly property bool onBattery: view.onBattery === true
    readonly property string chargeState:
        view.chargeState !== undefined ? view.chargeState : "unknown"
    readonly property string healthLabel:
        view.healthLabel !== undefined ? view.healthLabel : qsTr("Unknown")
    readonly property int capacity:
        view.capacity !== undefined && view.capacity !== null
            ? Number(view.capacity) : -1
    readonly property int chargeCycles:
        view.chargeCycles !== undefined && view.chargeCycles !== null
            ? Number(view.chargeCycles) : -1
    readonly property int timeToEmpty:
        view.timeToEmpty !== undefined && view.timeToEmpty !== null
            ? Number(view.timeToEmpty) : 0
    readonly property int timeToFull:
        view.timeToFull !== undefined && view.timeToFull !== null
            ? Number(view.timeToFull) : 0
    readonly property string activeProfile:
        view.activeProfile !== undefined ? String(view.activeProfile) : ""
    readonly property string profileLabel:
        view.profileLabel !== undefined ? String(view.profileLabel) : qsTr("Unavailable")
    readonly property bool performanceDegraded:
        view.performanceDegraded !== undefined
            && String(view.performanceDegraded).length > 0

    // The two `i` detail rows (closed by default, like the capture's popups).
    property bool healthInfoOpen: false
    property bool chargingInfoOpen: false

    // The Usage History range: 0 = Last 24 Hours, 1 = Last 10 Days. No charge
    // history provider exists in this build (T-15.x), so the switch drives the
    // empty-state caption rather than a chart series.
    property int historyRange: 0
    readonly property var historyRangeOptions: [
        { label: qsTr("Last 24 Hours") },
        { label: qsTr("Last 10 Days") }
    ]
    readonly property string historyRangeLabel:
        historyRange === 0 ? qsTr("last 24 hours") : qsTr("last 10 days")

    // The profile popup model, in the daemon's canonical order.
    readonly property var profileOptions: {
        var out = [];
        for (var i = 0; i < root.profiles.length; ++i)
            out.push({ value: root.profiles[i].id, label: root.profiles[i].label });
        return out;
    }

    // Test surface (used by tst_settings_battery.qml).
    property alias profileSelect: profileSelect
    property alias healthInfoButton: healthInfoButton
    property alias chargingInfoButton: chargingInfoButton
    property alias healthDetail: healthDetail
    property alias chargingDetail: chargingDetail
    property alias rangeControl: rangeControl
    property alias batteryChart: batteryChart
    property alias screenChart: screenChart
    property alias absenceNote: absenceNote
    property alias profilesAbsenceNote: profilesAbsenceNote

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    width: parent ? parent.width : implicitWidth

    function profileIndex(id) {
        for (var i = 0; i < root.profileOptions.length; ++i) {
            if (root.profileOptions[i].value === id)
                return i;
        }
        return 0;
    }

    // A short duration string for the charge estimate ("2 h 5 min", "45 min").
    function duration(seconds) {
        var minutes = Math.round(seconds / 60);
        if (minutes < 60)
            return qsTr("%1 min").arg(minutes);
        var hours = Math.floor(minutes / 60);
        var rest = minutes % 60;
        return rest === 0 ? qsTr("%1 h").arg(hours)
                          : qsTr("%1 h %2 min").arg(hours).arg(rest);
    }

    readonly property string chargeLabel: {
        switch (root.chargeState) {
        case "charging":
        case "pending-charge":
            return qsTr("Charging");
        case "fully-charged":
            return qsTr("Charged");
        case "discharging":
        case "pending-discharge":
            return qsTr("On Battery");
        case "empty":
            return qsTr("Empty");
        default:
            return qsTr("Unknown");
        }
    }

    readonly property string healthDetailText: {
        if (root.capacity < 0)
            return qsTr("This battery does not report its design capacity.");
        if (root.chargeCycles >= 0)
            return qsTr("Capacity %1% of design \u00b7 %2 cycles")
                .arg(root.capacity).arg(root.chargeCycles);
        return qsTr("Capacity %1% of design").arg(root.capacity);
    }

    readonly property string chargingDetailText: {
        if (root.charging && root.timeToFull > 0)
            return qsTr("About %1 until fully charged.").arg(root.duration(root.timeToFull));
        if (root.onBattery && root.timeToEmpty > 0)
            return qsTr("About %1 remaining.").arg(root.duration(root.timeToEmpty));
        if (root.chargeState === "fully-charged")
            return qsTr("The battery is fully charged.");
        return qsTr("The time remaining is not reported by this battery.");
    }

    // Opening the pane asks the host for a re-read (a no-op when absent).
    Component.onCompleted: Settings.refreshBattery()

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        // ── Absence ───────────────────────────────────────────────────────
        SettingsGroup {
            width: parent.width
            visible: !root.available

            Text {
                id: absenceNote
                objectName: "batteryAbsenceNote"
                width: parent.width
                text: qsTr("The system status service or UPower is not "
                           + "running, so battery information and power "
                           + "profiles are not available.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }
        }

        // ── Power Mode ────────────────────────────────────────────────────
        SettingsGroup {
            id: powerGroup
            width: parent.width
            title: qsTr("Power Mode")
            visible: root.available

            SettingsRow {
                width: parent.width
                label: qsTr("Low Power Mode")
                description: qsTr("Choose the power profile the system "
                                  + "applies. Balanced is the default.")
                visible: root.profileReady
                showSeparator: false
                controlData: Select {
                    id: profileSelect
                    accessibleName: qsTr("Low Power Mode")
                    model: root.profileOptions
                    onActivated: (index) => Settings.setPowerProfile(
                                    root.profileOptions[index].value)
                }
            }

            Text {
                id: profilesAbsenceNote
                objectName: "batteryProfilesAbsenceNote"
                width: parent.width
                visible: !root.profileReady
                text: qsTr("No power profiles are available on this system.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }

            Text {
                objectName: "batteryPerformanceNote"
                width: parent.width
                visible: root.performanceDegraded
                text: qsTr("Performance is currently degraded by the system.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.xs
            }
        }

        // ── Battery ───────────────────────────────────────────────────────
        SettingsGroup {
            id: batteryGroup
            width: parent.width
            title: qsTr("Battery")
            visible: root.ready

            SettingsRow {
                width: parent.width
                label: qsTr("Battery Health")
                controlData: Row {
                    spacing: Theme.primitive.spacing.sm
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.healthLabel
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                    Button {
                        id: healthInfoButton
                        icon: "info"
                        variant: "ghost"
                        accessibleName: qsTr("Battery Health details")
                        onClicked: root.healthInfoOpen = !root.healthInfoOpen
                    }
                }
            }

            Text {
                id: healthDetail
                objectName: "batteryHealthDetail"
                width: parent.width
                visible: root.healthInfoOpen
                text: root.healthDetailText
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                bottomPadding: Theme.primitive.spacing.sm
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Charging")
                showSeparator: false
                controlData: Row {
                    spacing: Theme.primitive.spacing.sm
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.chargeLabel
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                    Button {
                        id: chargingInfoButton
                        icon: "info"
                        variant: "ghost"
                        accessibleName: qsTr("Charging details")
                        onClicked: root.chargingInfoOpen = !root.chargingInfoOpen
                    }
                }
            }

            Text {
                id: chargingDetail
                objectName: "batteryChargingDetail"
                width: parent.width
                visible: root.chargingInfoOpen
                text: root.chargingDetailText
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                bottomPadding: Theme.primitive.spacing.sm
            }
        }

        // ── Usage History ─────────────────────────────────────────────────
        SettingsGroup {
            id: historyGroup
            width: parent.width
            title: qsTr("Usage History")
            visible: root.ready

            SegmentedControl {
                id: rangeControl
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                model: root.historyRangeOptions
                onActivated: (index) => root.historyRange = index
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Last charged to 100%")
                description: qsTr("Charge history is not provided by this system.")
                showSeparator: false
                controlData: Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Not available")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            ChartFrame {
                id: batteryChart
                width: parent.width
                title: qsTr("Battery Level")
                maxLabel: qsTr("100%")
                midLabel: qsTr("50%")
                zeroLabel: qsTr("0%")
                caption: qsTr("No battery history for the %1.").arg(root.historyRangeLabel)
            }

            ChartFrame {
                id: screenChart
                width: parent.width
                title: qsTr("Screen On Usage")
                maxLabel: qsTr("60m")
                midLabel: qsTr("30m")
                zeroLabel: qsTr("0m")
                caption: qsTr("No screen-on history for the %1.").arg(root.historyRangeLabel)
            }
        }
    }

    // Keep the popup in step with an external daemon change.
    Binding {
        target: profileSelect
        property: "currentIndex"
        value: root.profileIndex(root.activeProfile)
    }

    // A chart frame with the capture's y-axis labels and an honest empty state
    // (the host exposes no series yet). Original geometry, tokens only.
    component ChartFrame: Item {
        id: frame
        property string title: ""
        property string maxLabel: "100%"
        property string midLabel: "50%"
        property string zeroLabel: "0%"
        property string caption: ""

        implicitHeight: frameColumn.implicitHeight + 2 * Theme.primitive.spacing.sm

        Column {
            id: frameColumn
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: Theme.controls.settingsRow.paddingH
            anchors.rightMargin: Theme.controls.settingsRow.paddingH
            spacing: Theme.primitive.spacing.xxs

            Text {
                text: frame.title
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                font.weight: Theme.primitive.font.weightMedium
            }

            Row {
                spacing: Theme.primitive.spacing.sm

                Column {
                    spacing: 0
                    Text {
                        text: frame.maxLabel
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                    Item { width: 1; height: Theme.primitive.spacing.md }
                    Text {
                        text: frame.midLabel
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                    Item { width: 1; height: Theme.primitive.spacing.md }
                    Text {
                        text: frame.zeroLabel
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }

                Rectangle {
                    width: Math.max(0, frame.width - 2 * Theme.controls.settingsRow.paddingH
                                       - 36 - Theme.primitive.spacing.sm)
                    height: 96
                    radius: Theme.primitive.radius.sm
                    color: Theme.color.surfaceSunken
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.separator

                    Text {
                        anchors.centerIn: parent
                        width: parent.width - 2 * Theme.primitive.spacing.md
                        text: frame.caption
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.WordWrap
                    }
                }
            }

            Item { width: 1; height: Theme.primitive.spacing.sm }
        }
    }
}