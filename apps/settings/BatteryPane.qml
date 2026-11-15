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
//   * `Last charged to 100%` has no UPower provider; the row says so.
//     The battery-level chart is drawn from the system-status sampler
//     (`services/power`'s bounded history), not from fabricated data. The
//     sampler starts when the service starts, so the pane's first state is an
//     explicit "no samples yet" frame with a refresh path, never two
//     permanently empty charts;
//   * `Screen On Usage` has no session sampler in this build. It is one honest
//     row that names the missing half instead of a second empty chart;
//   * the macOS `Options...` sheet is an inline group. `Show percentage` is a
//     real menu-bar consumer, `Dim on battery` is a real shell display
//     consumer, `Prevent sleep` is a real session idle consumer; wake-for-
//     network and video optimization name the host/media half that owns them;
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

    // Test surface (used by tst_settings_battery.qml).
    property alias profileSelect: profileSelect
    property alias healthInfoButton: healthInfoButton
    property alias chargingInfoButton: chargingInfoButton
    property alias healthDetail: healthDetail
    property alias chargingDetail: chargingDetail
    property alias rangeControl: rangeControl
    property alias batteryChart: batteryChart
    property alias historyEmpty: historyEmpty
    property alias historyRetryButton: historyRetryButton
    property alias screenOnRow: screenOnRow
    property alias absenceNote: absenceNote
    property alias batteryAbsenceNote: batteryAbsenceNote
    property alias profilesAbsenceNote: profilesAbsenceNote
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
            // Power profiles are independent of UPower: a desktop with no
            // battery still selects a profile, and a masked profiles daemon
            // leaves the battery half live. The group keeps its title either
            // way; the row or the note inside explains the state.
            visible: true

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
                           + "state are not available. The power options below "
                           + "stay live.")
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
                visible: root.ready && root.healthInfoOpen
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
                visible: root.ready
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
                visible: root.ready && root.chargingInfoOpen
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

            SegmentedControl {
                id: rangeControl
                visible: root.ready
                width: parent.width - 2 * Theme.controls.settingsRow.paddingH
                x: Theme.controls.settingsRow.paddingH
                model: root.historyRangeOptions
                onActivated: (index) => Settings.set("power.historyRange", index)
            }

            SettingsRow {
                width: parent.width
                visible: root.ready
                label: qsTr("Last charged to 100%")
                description: qsTr("UPower does not expose a last-charged timestamp.")
                showSeparator: false
                controlData: Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Not available")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            // The real battery-level chart: a polyline over the sampler's
            // points. An empty series is an explicit frame with a refresh
            // action, not a chart with no data.
            HistoryChart {
                id: batteryChart
                width: parent.width
                visible: root.ready && root.historyHasData
                title: qsTr("Battery Level")
                samples: root.historySamples
                rangeSeconds: root.historyRangeSeconds
                rangeLabel: root.historyRangeLabel
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

        // ── Battery options (T229, completed T273) ────────────────────────
        SettingsGroup {
            id: optionsGroup
            width: parent.width
            title: qsTr("Options")
            // settingsd options stay live when UPower is absent: they are
            // policy, not host reads.
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
                owner: "settingsd -> session/idle"
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
                label: qsTr("Optimize video streaming while on battery")
                settingsId: "options.optimizeVideo"
                owner: "media pipeline"
                settingsKey: "power.optimizeVideo"
                description: qsTr("No media pipeline in this session exposes a "
                                  + "battery optimization switch.")
                controlData: Toggle {
                    id: optimizeVideoToggle
                    text: ""
                    checked: false
                    enabled: false
                }
            }

            SettingsRow {
                width: parent.width
                label: qsTr("Usage range")
                settingsId: "usage"
                owner: "settingsd + sampler"
                settingsKey: "power.historyRange"
                description: qsTr("The range the history chart covers.")
                showSeparator: false
                controlData: Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.historyRange === 0 ? qsTr("Last 24 Hours")
                                                  : qsTr("Last 10 Days")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }
        }

        // ── Power adapter (Linux-only, T273) ──────────────────────────────
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

        // ── Power settings link (T229) ────────────────────────────────────
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
                    onClicked: root.navigateRequested("lock-screen")
                }
            }
        }
    }

    // Keep the popup in step with an external daemon change.
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

    // The battery-level chart: a polyline over the sampler's real points, with
    // the capture's y-axis labels. The caller only instantiates it when at
    // least two points exist; the empty state is a separate row with a refresh
    // action, so this frame never draws a permanently blank chart.
    component HistoryChart: Item {
        id: chart
        property string title: ""
        property var samples: []
        property int rangeSeconds: 24 * 60 * 60
        property string rangeLabel: ""

        readonly property bool hasData: chart.samples.length >= 2
        readonly property int sampleCount: chart.samples.length
        readonly property string caption: chart.hasData
            ? qsTr("%1 samples over the %2.").arg(chart.sampleCount)
                .arg(chart.rangeLabel)
            : ""

        implicitHeight: chartColumn.implicitHeight
                         + 2 * Theme.primitive.spacing.sm

        onSamplesChanged: canvas.requestPaint()
        onRangeSecondsChanged: canvas.requestPaint()
        Component.onCompleted: canvas.requestPaint()

        Column {
            id: chartColumn
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: Theme.controls.settingsRow.paddingH
            anchors.rightMargin: Theme.controls.settingsRow.paddingH
            spacing: Theme.primitive.spacing.xxs

            Text {
                text: chart.title
                color: Theme.color.textPrimary
                font.pixelSize: Theme.controls.button.fontSize
                font.weight: Theme.primitive.font.weightMedium
            }

            Row {
                spacing: Theme.primitive.spacing.sm

                Column {
                    spacing: 0
                    Text {
                        text: qsTr("100%")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                    Item { width: 1; height: Theme.primitive.spacing.lg }
                    Text {
                        text: qsTr("50%")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                    Item { width: 1; height: Theme.primitive.spacing.lg }
                    Text {
                        text: qsTr("0%")
                        color: Theme.color.textTertiary
                        font.pixelSize: Theme.primitive.font.sizeSm
                    }
                }

                Canvas {
                    id: canvas
                    objectName: "batteryHistoryCanvas"
                    width: Math.max(0, chart.width - 2 * Theme.controls.settingsRow.paddingH
                                       - 36 - Theme.primitive.spacing.sm)
                    height: 112
                    antialiasing: true

                    onWidthChanged: requestPaint()

                    onPaint: {
                        var ctx = getContext("2d");
                        var w = width;
                        var h = height;
                        if (w <= 0 || h <= 0)
                            return;
                        ctx.clearRect(0, 0, w, h);
                        ctx.fillStyle = Theme.color.surfaceSunken;
                        ctx.fillRect(0, 0, w, h);

                        // Grid at 0, 50, and 100 percent.
                        ctx.strokeStyle = Theme.color.separator;
                        ctx.lineWidth = 1;
                        for (var grid = 0; grid <= 2; ++grid) {
                            var gy = Math.round(h * grid / 2) + 0.5;
                            ctx.beginPath();
                            ctx.moveTo(0, gy);
                            ctx.lineTo(w, gy);
                            ctx.stroke();
                        }

                        var points = chart.samples;
                        var range = Math.max(1, chart.rangeSeconds);
                        var path = [];
                        for (var i = 0; i < points.length; ++i) {
                            var age = Number(points[i].secondsAgo || 0);
                            var percent = Math.max(0, Math.min(100,
                                Number(points[i].percent || 0)));
                            var x = w * (1 - Math.min(1, age / range));
                            var y = h * (1 - percent / 100);
                            path.push({ x: x, y: y });
                        }
                        if (path.length < 2)
                            return;

                        // A light wash under the line, then the line itself.
                        ctx.beginPath();
                        ctx.moveTo(path[0].x, h);
                        for (var p = 0; p < path.length; ++p)
                            ctx.lineTo(path[p].x, path[p].y);
                        ctx.lineTo(path[path.length - 1].x, h);
                        ctx.closePath();
                        ctx.fillStyle = Theme.color.accentMuted;
                        ctx.fill();

                        ctx.beginPath();
                        ctx.moveTo(path[0].x, path[0].y);
                        for (var q = 1; q < path.length; ++q)
                            ctx.lineTo(path[q].x, path[q].y);
                        ctx.strokeStyle = Theme.color.accent;
                        ctx.lineWidth = 2;
                        ctx.lineJoin = "round";
                        ctx.stroke();

                        // The newest point gets a dot so a short series is
                        // legible even when the line is nearly flat.
                        ctx.beginPath();
                        ctx.arc(path[path.length - 1].x,
                                path[path.length - 1].y, 3, 0, Math.PI * 2);
                        ctx.fillStyle = Theme.color.accent;
                        ctx.fill();
                    }
                }
            }

            Text {
                width: parent.width
                text: chart.caption
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            Item { width: 1; height: Theme.primitive.spacing.sm }
        }
    }
}