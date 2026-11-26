// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Energy pane (id `battery`; T-15.6b, mockup order T289).
//
// macOS labels this pane Battery on a laptop and Energy on a desktop; the
// sidebar uses Energy on every machine so the category does not change with
// the hardware. The id stays `battery` because history, deep links, and the
// map persist it (ADR 0300, docs/design/26-settings-logic.md §2).
//
// The pane and the Quick Settings tile are one functional unit. `Low Power
// Mode` selects the active power profile through the `Settings` singleton to
// the power-profiles adapter (T-15.6a) via the bridge host; `Battery Health`
// and the charge status pill are read from UPower. The two daemons are
// independent: a desktop with no battery still selects a profile, and a laptop
// whose power-profiles-daemon is masked still reports its battery — each
// absence is a normal state with its own note, never an error.
//
// T289 follows the mockup's section order
// (docs/design/25-system-settings-visual-parity.md §5.3): Low Power Mode and
// Battery Health, the Last 24 Hours/Last 10 Days range, the live charge-status
// pill, the `SettingsChart` Battery Level chart, and the Options… footer. The
// Options sheet keeps the real `power.*` rows, the Linux-only Power Adapter
// values, and the Lock Screen link.
//
// Deviations from the macOS capture (ADR 0122), recorded so later tasks do not
// re-litigate them:
//   * the captured `Low Power Mode` value vocabulary (`Never`/`Always`) does
//     not exist in power-profiles-daemon; the picker maps onto the daemon's
//     `power-saver`/`balanced`/`performance` profiles, the same vocabulary the
//     adapter and Quick Settings tile use;
//   * the `Battery Health` detail covers reference-only hardware health and
//     optimized charging, so we keep the row and show UPower's
//     capacity/cycles instead;
//   * `Last charged to 100%` has no UPower provider; the row says so.
//     The battery-level chart is drawn from the system-status sampler
//     (`services/power`'s bounded history), not from fabricated data. The
//     sampler starts when the service starts, so the pane's first state is an
//     explicit "no samples yet" frame with a refresh path, never a permanently
//     empty chart;
//   * `Screen On Usage` has no session sampler in this build. It is one honest
//     row that names the missing half instead of a second empty chart;
//   * the macOS `Options...` sheet is the pane's Options… footer sheet.
//     `Show percentage` is a real menu-bar consumer, `Dim on battery` is a
//     real shell display consumer, `Prevent sleep` is a real session idle
//     consumer; wake-for-network and video optimization name the host/media
//     half that owns them;
//   * the Account/sidebar row is omitted project-wide.
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

    // The Battery Health detail (closed by default, like the capture's popups).
    property bool healthInfoOpen: false

    // The Usage History range: 0 = Last 24 Hours, 1 = Last 10 Days (T229: the
    // selection is the `power.historyRange` key so it survives a restart).
    readonly property int historyRange:
        Settings.values["power.historyRange"] !== undefined
            ? Number(Settings.values["power.historyRange"]) : 0
    readonly property bool showPercentage:
        Settings.values["power.showPercentage"] === true
    readonly property bool dimOnBattery:
        Settings.values["power.dimOnBattery"] === true
    readonly property bool preventSleep:
        Settings.values["power.preventSleep"] === true
    // T273: the real sampler's points, oldest first. Empty until the service
    // has read the battery at least once; the pane says so rather than drawing
    // an empty chart frame.
    readonly property var history:
        view.history !== undefined ? view.history : ({})
    readonly property var historySamples:
        history.samples !== undefined ? history.samples : []
    readonly property int historyCount:
        historySamples.length
    readonly property bool historyHasData:
        historyCount >= 2
    readonly property string historyEmptyReason: {
        if (root.history.state === "empty" || root.historyCount === 0)
            return qsTr("No battery history has been sampled yet. History "
                        + "starts when the system status service starts; "
                        + "refresh to record a reading now.");
        return qsTr("Collecting history: %1 reading so far.").arg(root.historyCount);
    }
    readonly property int idleBlank:
        Settings.values["idle.blank"] !== undefined
            ? Number(Settings.values["idle.blank"]) : 300
    readonly property int idleLock:
        Settings.values["idle.lock"] !== undefined
            ? Number(Settings.values["idle.lock"]) : 600
    signal navigateRequested(string paneId)
    readonly property var historyRangeOptions: [
        { label: qsTr("Last 24 Hours") },
        { label: qsTr("Last 10 Days") }
    ]
    readonly property string historyRangeLabel:
        historyRange === 0 ? qsTr("last 24 hours") : qsTr("last 10 days")
    readonly property int historyRangeSeconds:
        historyRange === 0 ? 24 * 60 * 60 : 10 * 24 * 60 * 60

    // The profile popup model, in the daemon's canonical order.
    readonly property var profileOptions: {
        var out = [];
        for (var i = 0; i < root.profiles.length; ++i)
            out.push({ value: root.profiles[i].id, label: root.profiles[i].label });
        return out;
    }

    // The `SettingsChart` series: 0..1 in display order (oldest first), the
    // sampler's own points. No series is invented when the sampler is empty.
    readonly property var chartValues: {
        var out = [];
        for (var i = 0; i < root.historySamples.length; ++i) {
            out.push(Math.max(0, Math.min(1,
                Number(root.historySamples[i].percent || 0) / 100)));
        }
        return out;
    }
    // The chart's x-axis ticks. The mockup labels the axis with clock times;
    // ours names the real range so a 24-hour and a 10-day chart never share a
    // false clock label.
    readonly property var chartAxisLabels: root.historyRange === 0
        ? [qsTr("24 h ago"), qsTr("18 h"), qsTr("12 h"), qsTr("6 h"), qsTr("Now")]
        : [qsTr("10 d ago"), qsTr("7 d"), qsTr("4 d"), qsTr("2 d"), qsTr("Now")]
    readonly property string chartCaption: root.historyHasData
        ? qsTr("%1 readings over the %2.")
            .arg(root.historyCount).arg(root.historyRangeLabel)
        : ""

    // Test surface (used by tst_settings_battery.qml).
    property alias profileSelect: profileSelect
    property alias healthInfoButton: healthInfoButton
    property alias healthDetail: healthDetail
    property alias statusRow: statusRow
    property alias statusPill: statusPill
    property alias statusPillText: statusPillText
    property alias statusDetail: statusDetail
    property alias rangeControl: rangeControl
    property alias batteryChart: batteryChart
    property alias batteryChartCaption: batteryChartCaption
    property alias chartAxis: chartAxis
    property alias historyEmpty: historyEmpty
    property alias historyRetryButton: historyRetryButton
    property alias screenOnRow: screenOnRow
    property alias absenceNote: absenceNote
    property alias batteryAbsenceNote: batteryAbsenceNote
    property alias profilesAbsenceNote: profilesAbsenceNote
    property alias optionsButton: optionsButton
    property alias optionsDialog: optionsDialog
    property alias optionsGroup: optionsGroup
    property alias showPercentageToggle: showPercentageToggle
    property alias dimOnBatteryToggle: dimOnBatteryToggle
    property alias preventSleepToggle: preventSleepToggle
    property alias wakeForNetworkRow: wakeForNetworkRow
    property alias wakeForNetworkToggle: wakeForNetworkToggle
    property alias optimizeVideoRow: optimizeVideoRow
    property alias optimizeVideoToggle: optimizeVideoToggle
    property alias adapterGroup: adapterGroup
    property alias adapterStateText: adapterStateText
    property alias powerSettingsGroup: powerSettingsGroup
    property alias openLockScreenButton: openLockScreenButton

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
            return qsTr("Fully Charged");
        case "discharging":
        case "pending-discharge":
            return qsTr("On Battery");
        case "empty":
            return qsTr("Empty");
        default:
            return qsTr("Unknown");
        }
    }

    // The pill's second line: the real estimate UPower reports. The mockup's
    // "Today, 4:18 PM" is example text and UPower exposes no last-update
    // timestamp, so the line names the estimate (or says it is not reported).
    readonly property string statusDetailText: {
        if (root.charging && root.timeToFull > 0)
            return qsTr("About %1 until fully charged.")
                .arg(root.duration(root.timeToFull));
        if (root.onBattery && root.timeToEmpty > 0)
            return qsTr("About %1 remaining.").arg(root.duration(root.timeToEmpty));
        if (root.chargeState === "fully-charged")
            return qsTr("The battery is fully charged.");
        return qsTr("The time remaining is not reported by this battery.");
    }

    readonly property string healthDetailText: {
        if (root.capacity < 0)
            return qsTr("This battery does not report its design capacity.");
        if (root.chargeCycles >= 0)
            return qsTr("Capacity %1% of design \u00b7 %2 cycles")
                .arg(root.capacity).arg(root.chargeCycles);
        return qsTr("Capacity %1% of design").arg(root.capacity);
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
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.sm
            }
        }

        // ── Low Power Mode ────────────────────────────────────────────────
        SettingsGroup {
            id: powerGroup
            width: parent.width
            // Power profiles are independent of UPower: a desktop with no
            // battery still selects a profile, and a masked profiles daemon
            // leaves the battery half live. The group keeps its title either
            // way; the row or the note inside explains the state.
            visible: true

            SettingsRow {
                width: parent.width
                label: qsTr("Low Power Mode")
                settingsId: "profile"
                owner: "power-profiles-daemon"
                settingsKey: "power.profile"
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

        // ── Battery health ────────────────────────────────────────────────
        SettingsGroup {
            id: batteryGroup
            width: parent.width
            title: qsTr("Battery")
            visible: true

            Text {
                id: batteryAbsenceNote
                objectName: "batteryAbsenceNote"
                width: parent.width
                visible: !root.ready
                text: root.available
                    ? qsTr("This system has no battery. Battery health and "
                           + "charge state appear when a battery is present.")
                    : qsTr("UPower is not running, so battery health and charge "
                           + "state are not available. The power options in "
                           + "Options… stay live.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.sm
            }

            SettingsRow {
                width: parent.width
                visible: root.ready
                label: qsTr("Battery Health")
                settingsId: "health"
                owner: "UPower"
                settingsKey: "Battery.Health"
                showSeparator: false
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
                        objectName: "batteryHealthDetailButton"
                        text: qsTr("Show Detail")
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
                visible: root.ready && root.healthInfoOpen
                text: root.healthDetailText
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                bottomPadding: Theme.primitive.spacing.sm
            }
        }

        // ── Usage history ─────────────────────────────────────────────────
        SettingsGroup {
            id: historyGroup
            width: parent.width
            title: qsTr("Usage History")
            visible: true

            Text {
                width: parent.width
                visible: !root.ready
                text: qsTr("Usage history needs a present battery. The sampler "
                           + "starts when the system status service starts.")
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
                leftPadding: Theme.controls.settingsRow.paddingH
                rightPadding: Theme.controls.settingsRow.paddingH
                topPadding: Theme.primitive.spacing.xs
                bottomPadding: Theme.primitive.spacing.sm
            }

            // The mockup's range control: the real `power.historyRange` key.
            SegmentedControl {
                id: rangeControl
                objectName: "batteryRangeControl"
                visible: root.ready
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                accessibleName: qsTr("Battery history range")
                model: root.historyRangeOptions
                onActivated: (index) => Settings.set("power.historyRange", index)
            }

            // The mockup's status line: a live UPower pill plus the real
            // estimate. It replaces the old separate Charging row.
            Row {
                id: statusRow
                objectName: "batteryStatusRow"
                visible: root.ready
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                spacing: Theme.primitive.spacing.sm

                Rectangle {
                    id: statusPill
                    objectName: "batteryStatusPill"
                    width: pillContent.implicitWidth + 2 * Theme.primitive.spacing.sm
                    height: pillContent.implicitHeight + 2 * Theme.primitive.spacing.xs
                    radius: height / 2
                    color: Theme.color.controlFill
                    border.width: Theme.controls.window.borderWidth
                    border.color: Theme.color.border
                    antialiasing: true

                    Row {
                        id: pillContent
                        anchors.centerIn: parent
                        spacing: Theme.primitive.spacing.xxs

                        Icon {
                            name: "battery"
                            size: Theme.primitive.font.sizeSm
                            color: root.charging ? Theme.color.success
                                                 : Theme.color.textSecondary
                        }

                        Text {
                            id: statusPillText
                            objectName: "batteryStatusPillText"
                            text: root.chargeLabel
                            color: Theme.color.textPrimary
                            font.pixelSize: Theme.primitive.font.sizeSm
                            font.weight: Theme.primitive.font.weightMedium
                        }
                    }

                    Accessible.role: Accessible.StaticText
                    Accessible.name: qsTr("Battery status: %1").arg(root.chargeLabel)
                }

                Item {
                    width: Math.max(0, statusRow.width - statusPill.width
                                       - statusRow.spacing)
                    height: statusPill.height

                    Text {
                        id: statusDetail
                        objectName: "batteryStatusDetail"
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width
                        text: root.statusDetailText
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
                    }
                }
            }

            SettingsRow {
                width: parent.width
                visible: root.ready
                label: qsTr("Last charged to 100%")
                settingsId: "lastCharged"
                owner: "UPower"
                settingsKey: "Battery.EnergyFull"
                description: qsTr("UPower does not expose a last-charged timestamp.")
                showSeparator: false
                controlData: Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Not available")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            // The real battery-level chart: the shared `SettingsChart` frame
            // over the sampler's own points. An empty series is an explicit
            // frame with a refresh action, not a chart with no data.
            Item {
                id: batteryChart
                objectName: "batteryLevelChart"
                width: parent.width
                height: visible ? chartColumn.implicitHeight
                                  + 2 * Theme.primitive.spacing.sm : 0
                visible: root.ready && root.historyHasData

                Column {
                    id: chartColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    anchors.top: parent.top
                    anchors.topMargin: Theme.primitive.spacing.sm
                    spacing: Theme.primitive.spacing.xxs

                    Text {
                        text: qsTr("Battery Level")
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        font.weight: Theme.primitive.font.weightMedium
                    }

                    Row {
                        width: parent.width
                        spacing: Theme.primitive.spacing.sm

                        Column {
                            width: 32
                            spacing: 0
                            topPadding: 0

                            Text {
                                text: qsTr("100%")
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }
                            Item {
                                width: 1
                                height: Math.max(0, Theme.controls.settingsChart.height
                                                    / 2 - 2 * Theme.primitive.font.sizeSm)
                            }
                            Text {
                                text: qsTr("50%")
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }
                            Item {
                                width: 1
                                height: Math.max(0, Theme.controls.settingsChart.height
                                                    / 2 - 2 * Theme.primitive.font.sizeSm)
                            }
                            Text {
                                text: qsTr("0%")
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeSm
                            }
                        }

                        SettingsChart {
                            id: levelChart
                            objectName: "batteryLevelSettingsChart"
                            width: Math.max(0, parent.width - 32
                                               - Theme.primitive.spacing.sm)
                            values: root.chartValues
                        }
                    }

                    Item {
                        width: parent.width
                        height: chartAxis.implicitHeight
                        visible: root.historyHasData

                        Row {
                            id: chartAxis
                            objectName: "batteryChartAxis"
                            anchors.left: parent.left
                            anchors.leftMargin: 32 + Theme.primitive.spacing.sm
                            width: parent.width - 32 - Theme.primitive.spacing.sm
                            spacing: 0

                            Repeater {
                                model: root.chartAxisLabels

                                delegate: Text {
                                    required property var modelData
                                    required property int index
                                    width: chartAxis.width
                                           / Math.max(1, root.chartAxisLabels.length)
                                    text: modelData
                                    color: Theme.color.textTertiary
                                    font.pixelSize: Theme.primitive.font.sizeSm
                                    elide: Text.ElideRight
                                    horizontalAlignment: index === 0
                                        ? Text.AlignLeft
                                        : (index === root.chartAxisLabels.length - 1
                                           ? Text.AlignRight : Text.AlignHCenter)
                                }
                            }
                        }
                    }

                    Text {
                        id: batteryChartCaption
                        objectName: "batteryChartCaption"
                        width: parent.width
                        text: root.chartCaption
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
                    }

                    Item { width: 1; height: Theme.primitive.spacing.sm }
                }
            }

            Item {
                id: historyEmpty
                objectName: "batteryHistoryEmpty"
                width: parent.width
                height: visible ? emptyColumn.implicitHeight
                                + 2 * Theme.primitive.spacing.sm : 0
                visible: root.ready && !root.historyHasData

                Column {
                    id: emptyColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.controls.settingsRow.paddingH
                    anchors.rightMargin: Theme.controls.settingsRow.paddingH
                    spacing: Theme.primitive.spacing.xs

                    Text {
                        width: parent.width
                        text: root.historyEmptyReason
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.primitive.font.sizeSm
                        wrapMode: Text.WordWrap
                    }

                    Button {
                        id: historyRetryButton
                        objectName: "batteryHistoryRetry"
                        text: qsTr("Refresh history")
                        onClicked: Settings.refreshBattery()
                    }
                }
            }

            // The second macOS chart has no session sampler. One honest row
            // names that half; a second empty frame would be a dead control.
            SettingsRow {
                id: screenOnRow
                objectName: "batteryScreenOnRow"
                width: parent.width
                visible: root.ready
                label: qsTr("Screen On Usage")
                settingsId: "screenOn"
                owner: "session sampler"
                settingsKey: "power.screenOn"
                description: qsTr("This session does not report screen-on time.")
                showSeparator: false
                controlData: Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Not available")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        // ── Options… footer (mockup) ──────────────────────────────────────
        SettingsFooter {
            id: optionsFooter
            objectName: "batteryOptionsFooter"
            width: parent.width

            Button {
                id: optionsButton
                objectName: "batteryOptionsButton"
                text: qsTr("Options…")
                onClicked: optionsDialog.show()
            }
        }
    }

    // The mockup's Options… sheet: the real `power.*` policy rows, the
    // Linux-only Power Adapter values, and the Lock Screen hand-off. All of
    // them stay live when UPower is absent; they are settingsd policy, not
    // host reads.
    Dialog {
        id: optionsDialog
        objectName: "batteryOptionsDialog"
        title: qsTr("Energy Options")
        message: qsTr("These preferences apply to this session. A dimmed row "
                      + "names the host or service that owns it.")

        contentData: Column {
            id: optionsContent
            width: parent.width
            spacing: Theme.primitive.spacing.lg

            SettingsGroup {
                id: optionsGroup
                width: parent.width
                visible: true

                SettingsRow {
                    width: parent.width
                    label: qsTr("Show battery percentage")
                    settingsId: "options.showPercentage"
                    owner: "settingsd -> shell/MenuBar"
                    settingsKey: "power.showPercentage"
                    description: qsTr("Draw the percentage beside the menu-bar glyph.")
                    controlData: Toggle {
                        id: showPercentageToggle
                        text: ""
                        onToggled: (checked) => Settings.set("power.showPercentage", checked)
                    }
                }

                SettingsRow {
                    width: parent.width
                    label: qsTr("Dim on battery")
                    settingsId: "options.dimOnBattery"
                    owner: "settingsd -> shell/display"
                    settingsKey: "power.dimOnBattery"
                    description: qsTr("Lower the display brightness while the "
                                      + "machine runs on battery.")
                    controlData: Toggle {
                        id: dimOnBatteryToggle
                        text: ""
                        onToggled: (checked) => Settings.set("power.dimOnBattery", checked)
                    }
                }

                SettingsRow {
                    width: parent.width
                    label: qsTr("Prevent sleep")
                    settingsId: "options.preventSleep"
                    owner: "settingsd -> compositor idle"
                    settingsKey: "power.preventSleep"
                    description: qsTr("Disable the automatic suspend stage. Dim, "
                                      + "blank, and lock still apply.")
                    controlData: Toggle {
                        id: preventSleepToggle
                        text: ""
                        onToggled: (checked) => Settings.set("power.preventSleep", checked)
                    }
                }

                SettingsRow {
                    id: wakeForNetworkRow
                    width: parent.width
                    label: qsTr("Wake for network access")
                    settingsId: "options.wakeForNetwork"
                    owner: "host firmware"
                    settingsKey: "power.wakeForNetwork"
                    description: qsTr("Wake-on-LAN is a host firmware setting; the "
                                      + "session bus cannot change it.")
                    showSeparator: true
                    controlData: Toggle {
                        id: wakeForNetworkToggle
                        text: ""
                        checked: false
                        enabled: false
                    }
                }

                SettingsRow {
                    id: optimizeVideoRow
                    width: parent.width
                    label: qsTr("Optimize video streaming")
                    settingsId: "options.optimizeVideo"
                    owner: "media pipeline"
                    settingsKey: "power.optimizeVideo"
                    description: qsTr("No media pipeline in this session exposes a "
                                      + "battery optimization switch.")
                    showSeparator: false
                    controlData: Toggle {
                        id: optimizeVideoToggle
                        text: ""
                        checked: false
                        enabled: false
                    }
                }
            }

            // ── Power adapter (Linux-only, T273) ──────────────────────────
            SettingsGroup {
                id: adapterGroup
                width: parent.width
                title: qsTr("Power Adapter")
                visible: root.ready

                SettingsRow {
                    width: parent.width
                    label: qsTr("Power adapter")
                    settingsId: "adapter.state"
                    owner: "UPower"
                    settingsKey: "Battery.State"
                    description: qsTr("The live power source UPower reports.")
                    controlData: Text {
                        id: adapterStateText
                        objectName: "batteryAdapterState"
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.charging || root.view.plugged === true
                            ? qsTr("Connected") : qsTr("On battery")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }

                SettingsRow {
                    width: parent.width
                    label: qsTr("Charge limit")
                    settingsId: "adapter.chargeLimit"
                    owner: "UPower"
                    settingsKey: "Battery.ChargeThreshold"
                    description: qsTr("This battery does not report a charge threshold.")
                    showSeparator: false
                    controlData: Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Not available")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }
            }

            // ── Power settings link (T229) ────────────────────────────────
            SettingsGroup {
                id: powerSettingsGroup
                width: parent.width
                title: qsTr("Power Settings")
                // The idle policy is settingsd's and applies with no battery.
                visible: true

                SettingsRow {
                    width: parent.width
                    label: qsTr("Display off")
                    description: qsTr("Blank the display after inactivity.")
                    controlData: Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.idleBlank > 0
                            ? qsTr("%1 min").arg(Math.round(root.idleBlank / 60))
                            : qsTr("Never")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }

                SettingsRow {
                    width: parent.width
                    label: qsTr("Require password")
                    description: qsTr("Lock the session after inactivity.")
                    controlData: Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.idleLock > 0
                            ? qsTr("%1 min").arg(Math.round(root.idleLock / 60))
                            : qsTr("Never")
                        color: Theme.color.textSecondary
                        font.pixelSize: Theme.controls.button.fontSize
                    }
                }

                SettingsRow {
                    width: parent.width
                    label: qsTr("More power settings")
                    description: qsTr("Display and lock timers live in Lock Screen.")
                    showSeparator: false
                    controlData: Button {
                        id: openLockScreenButton
                        objectName: "batteryOpenLockScreen"
                        text: qsTr("Open Lock Screen")
                        onClicked: {
                            optionsDialog.hide();
                            root.navigateRequested("lock-screen");
                        }
                    }
                }
            }
        }
    }

    // Keep the controls in step with an external daemon change.
    Binding {
        target: profileSelect
        property: "currentIndex"
        value: root.profileIndex(root.activeProfile)
    }
    Binding {
        target: rangeControl
        property: "currentIndex"
        value: root.historyRange
    }
    Binding {
        target: showPercentageToggle
        property: "checked"
        value: root.showPercentage
    }
    Binding {
        target: dimOnBatteryToggle
        property: "checked"
        value: root.dimOnBattery
    }
    Binding {
        target: preventSleepToggle
        property: "checked"
        value: root.preventSleep
    }
}
