// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The notification-center history (T-11.1a; mapped by T204, materials T205).
//
// The service owns the history, expiry, Focus/DND and banner policy. The shell
// decodes `History()` once (`NotificationCenter` in shell/src) into stable-keyed
// rows: `key`, `active`, `canAct`, `status` and the filtered `actions`. This
// component is only the view. It renders history rows, exposes keyboard
// traversal (Up/Down/Home/End/Return/Delete) and raises the action/dismissal
// requests the shell forwards through the existing notification service
// contract.
//
// History-only rows (expired, closed, dismissed or DND-suppressed) keep their
// text and stable identity but never show actions: `actions` is empty and
// `canAct` is false in the model.
//
// T205: the Center is two independently keyed materials. The notification
// stack (title + history list) uses `notificationStack`; the lower panel uses
// `notificationLowerPanel`. Both regions are declared to the compositor and
// commit-latched with the surface image. On the sampled path the compositor
// paints the measured flat bands and the QML plates are transparent; on
// solid/software/Reduce Transparency the QML paints the same resolved
// tint/alpha. The regions never overlap, so a panel is never frosted twice.
Item {
    id: root

    // Standalone/test default; the shell sets the live surface size through
    // `prepareChromeScene`.
    width: 360
    height: 620

    // The normalized `History()` view, most recent first.
    property var history: []
    // The model's selected row. Kept in C++ so it follows the stable key when
    // the service replaces or reorders history.
    property int selectedIndex: -1
    // False while the notification service is absent. The Center stays open
    // with an accessible unavailable state and repopulates on reconnect.
    property bool available: true
    // T205: when a banner is on screen the shell insets the stack below it so
    // the two surfaces stay co-visible without covering each other. The
    // transparent strip above the stack declares no material region.
    property int bannerInset: 0
    // T-21.2c: the shell applies the compositor's Popup role state. Without
    // the sampled pass the plates take the shared milky fallback.
    property bool materialBlurActive: true
    property int materialTier: 0
    property int materialFallback: 0
    readonly property string materialFallbackReason:
        Material.fallbackReason(materialFallback, materialBlurActive, materialTier)

    // T205: resolve the two measured presets through the one shared policy.
    // The stack is the measured Light 0.551/0.706 and Dark 0.507/0.550 fit;
    // the lower panel is the measured Light 0.407/0.852 fit and an adapted,
    // underconstrained Dark variant. Accessibility overrides win inside
    // `Glass.resolve`, so the QML complement never invents a second plate.
    readonly property var stackGlass: Glass.resolve("notificationStack", {
        scheme: Theme.dark ? "dark" : "light",
        rollout: Glass.rollout,
        reduceTransparency: Glass.reduceTransparency,
        increaseContrast: Glass.increaseContrast,
        backend: materialFallback === 2 ? "software"
               : materialBlurActive ? "gpu-sampled" : "gpu-no-sampled-blur",
        tier: materialTier === 2 ? "minimal" : materialTier === 1 ? "reduced" : "full"
    })
    readonly property var lowerGlass: Glass.resolve("notificationLowerPanel", {
        scheme: Theme.dark ? "dark" : "light",
        rollout: Glass.rollout,
        reduceTransparency: Glass.reduceTransparency,
        increaseContrast: Glass.increaseContrast,
        backend: materialFallback === 2 ? "software"
               : materialBlurActive ? "gpu-sampled" : "gpu-no-sampled-blur",
        tier: materialTier === 2 ? "minimal" : materialTier === 1 ? "reduced" : "full"
    })
    readonly property bool replacementLook: Glass.rollout === "replacement"
    readonly property bool sampledGlass: replacementLook && materialBlurActive
        && stackGlass.renderMode === "sampled-glass"
    readonly property color stackFill: sampledGlass ? "transparent"
        : replacementLook
          ? Qt.rgba(stackGlass.tint, stackGlass.tint, stackGlass.tint,
                    stackGlass.alpha)
          : Material.popupFill(materialBlurActive)
    readonly property color lowerFill: sampledGlass ? "transparent"
        : replacementLook
          ? Qt.rgba(lowerGlass.tint, lowerGlass.tint, lowerGlass.tint,
                    lowerGlass.alpha)
          : Material.popupFill(materialBlurActive)

    readonly property bool hasHistory: history.length > 0
    readonly property int activeCount: {
        var count = 0;
        for (var i = 0; i < history.length; ++i) {
            if (history[i] && history[i].active === true)
                ++count;
        }
        return count;
    }
    readonly property string emptyText: available
        ? qsTr("No notifications")
        : qsTr("Notifications unavailable")
    readonly property string lowerSummary: {
        if (!available)
            return qsTr("Notification service unavailable");
        if (!hasHistory)
            return qsTr("0 notifications");
        if (activeCount > 0)
            return qsTr("%1 notifications · %2 active").arg(history.length).arg(activeCount);
        return qsTr("%1 notifications").arg(history.length);
    }

    // The two panel bands. The stack clears an active banner and the lower
    // panel is a fixed footer; the gap keeps both rounded edges visible.
    readonly property int panelGap: Theme.primitive.spacing.sm
    readonly property int lowerPanelHeight: 64
    readonly property int stackTop: Math.min(bannerInset,
        Math.max(0, height - lowerPanelHeight - panelGap))
    readonly property int lowerTop: Math.max(stackTop + panelGap,
                                             height - lowerPanelHeight)
    readonly property int stackHeight: Math.max(0, lowerTop - panelGap - stackTop)
    readonly property int panelRadius: Theme.controls.window.radius

    // T205: the shell declares these regions on the `notification-center`
    // layer surface. `stack` and `lower` are stable keys with disjoint rects;
    // the material list is commit-latched with the image. A banner uses its
    // own `notification` surface and `banner` region, so no region samples a
    // panel's own content.
    readonly property var materialRegions: {
        var out = [];
        if (stackHeight > 0) {
            out.push({
                key: "stack", preset: "notificationStack",
                x: 0, y: stackTop, w: width, h: stackHeight,
                tl: panelRadius, tr: panelRadius,
                br: panelRadius, bl: panelRadius,
                overscan: 8
            });
        }
        if (lowerPanelHeight > 0) {
            out.push({
                key: "lower", preset: "notificationLowerPanel",
                x: 0, y: lowerTop, w: width, h: lowerPanelHeight,
                tl: panelRadius, tr: panelRadius,
                br: panelRadius, bl: panelRadius,
                overscan: 8
            });
        }
        return out;
    }

    signal closed()
    signal moveRequested(int delta)
    signal selectRequested(int index)
    signal activateRequested()
    signal dismissSelectedRequested()
    signal actionInvoked(int id, string key)
    signal dismissEntryRequested(int id)

    focus: true
    Accessible.role: Accessible.List
    Accessible.name: qsTr("Notifications")
    Accessible.focusable: true

    Keys.onEscapePressed: (event) => {
        root.closed();
        event.accepted = true;
    }
    Keys.onUpPressed: (event) => {
        root.moveRequested(-1);
        event.accepted = true;
    }
    Keys.onDownPressed: (event) => {
        root.moveRequested(1);
        event.accepted = true;
    }
    Keys.onReturnPressed: (event) => {
        root.activateRequested();
        event.accepted = true;
    }
    Keys.onEnterPressed: (event) => {
        root.activateRequested();
        event.accepted = true;
    }
    Keys.onDeletePressed: (event) => {
        root.dismissSelectedRequested();
        event.accepted = true;
    }
    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Home) {
            root.selectRequested(0);
            event.accepted = true;
        } else if (event.key === Qt.Key_End) {
            root.selectRequested(Math.max(0, root.history.length - 1));
            event.accepted = true;
        } else if (event.key === Qt.Key_Backspace) {
            root.dismissSelectedRequested();
            event.accepted = true;
        }
    }

    // The notification stack: title plus the history viewport. Its material
    // region starts at `stackTop`, clearing a co-visible banner.
    Shadow {
        anchors.fill: stackPlate
        radius: stackPlate.radius
        level: "overlay"
        // T205: the compositor's opaque flat band is the sampled material; a
        // QML shadow under a transparent fill would show through and darken
        // it. The shadow is only for the solid/software plates.
        visible: !root.sampledGlass
    }

    Rectangle {
        id: stackPlate
        objectName: "notificationCenterPlate"
        x: 0
        y: root.stackTop
        width: root.width
        height: root.stackHeight
        radius: root.panelRadius
        color: root.stackFill
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border

        Column {
            id: layout
            anchors.fill: parent
            anchors.margins: Theme.controls.popup.padding
            spacing: Theme.controls.popover.padding

            Text {
                id: titleText
                width: parent.width
                objectName: "centerTitle"
                text: qsTr("Notifications")
                color: Theme.color.textPrimary
                font.pixelSize: Theme.primitive.font.sizeLg
                font.weight: Theme.primitive.font.weightSemibold
                elide: Text.ElideRight
            }

            Item {
                id: listArea
                width: parent.width
                height: parent.height - titleText.height - layout.spacing

                ListView {
                    id: list
                    objectName: "historyList"
                    anchors.fill: parent
                    clip: true
                    model: root.history
                    spacing: Theme.primitive.spacing.xs
                    boundsBehavior: Flickable.StopAtBounds

                    onCountChanged: {
                        if (root.selectedIndex >= 0)
                            positionViewAtIndex(root.selectedIndex, ListView.Contain)
                    }

                    delegate: Rectangle {
                        id: row
                        required property var modelData
                        required property int index
                        readonly property bool selected: index === root.selectedIndex
                        readonly property bool active: modelData.active === true
                        readonly property bool canAct: modelData.canAct === true
                        readonly property var actions: modelData.actions || []
                        readonly property string statusText: {
                            if (modelData.status === "suppressed")
                                return qsTr("Suppressed by Focus");
                            if (modelData.status === "expired")
                                return qsTr("Expired");
                            if (modelData.status === "dismissed")
                                return qsTr("Dismissed");
                            if (modelData.status === "closed" || modelData.status === "undefined")
                                return qsTr("Closed");
                            return "";
                        }

                        objectName: "history_" + modelData.key
                        width: list.width
                        height: rowContent.implicitHeight + 2 * Theme.primitive.spacing.sm
                        radius: Theme.controls.sidebar.rowRadius
                        color: selected ? Theme.color.surfaceElevated : Theme.color.surfaceMuted
                        border.width: selected ? Theme.controls.window.borderWidth : 0
                        border.color: Theme.color.accent
                        opacity: active ? 1.0 : 0.78

                        Accessible.role: Accessible.ListItem
                        Accessible.name: modelData.summary
                        Accessible.description: {
                            var parts = [];
                            if (modelData.body)
                                parts.push(modelData.body);
                            if (row.statusText.length > 0)
                                parts.push(row.statusText);
                            return parts.join(". ");
                        }

                        MouseArea {
                            anchors.fill: parent
                            // Keep the row selector below the action/dismiss
                            // targets; otherwise the full-row click area would
                            // swallow their presses.
                            z: -1
                            onClicked: root.selectRequested(row.index)
                        }

                        Column {
                            id: rowContent
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.leftMargin: Theme.primitive.spacing.sm
                            anchors.rightMargin: Theme.primitive.spacing.sm
                            spacing: Theme.primitive.spacing.xxs

                            Row {
                                width: parent.width
                                spacing: Theme.primitive.spacing.sm

                                Text {
                                    width: parent.width - (statusLabel.visible
                                        ? statusLabel.width + parent.spacing : 0)
                                    text: modelData.appName
                                    color: Theme.color.textTertiary
                                    font.pixelSize: Theme.primitive.font.sizeXs
                                    elide: Text.ElideRight
                                }
                                Text {
                                    id: statusLabel
                                    objectName: "historyStatus"
                                    visible: row.statusText.length > 0
                                    text: row.statusText
                                    color: Theme.color.textTertiary
                                    font.pixelSize: Theme.primitive.font.sizeXs
                                    elide: Text.ElideRight
                                }
                            }
                            Text {
                                width: parent.width
                                objectName: "historySummary"
                                text: modelData.summary
                                color: Theme.color.textPrimary
                                font.pixelSize: Theme.primitive.font.sizeMd
                                font.weight: Theme.primitive.font.weightSemibold
                                elide: Text.ElideRight
                            }
                            Text {
                                width: parent.width
                                visible: modelData.body.length > 0
                                text: modelData.body
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                wrapMode: Text.WordWrap
                                maximumLineCount: 2
                                elide: Text.ElideRight
                            }

                            Row {
                                id: actionRow
                                objectName: "historyActions"
                                visible: row.active && row.actions.length > 0
                                width: parent.width
                                height: visible ? 26 : 0
                                spacing: Theme.primitive.spacing.sm

                                Repeater {
                                    model: row.actions
                                    delegate: Rectangle {
                                        required property var modelData
                                        objectName: "historyAction_" + modelData.key
                                        height: actionRow.height
                                        width: actionLabel.implicitWidth
                                               + 2 * Theme.primitive.spacing.md
                                        radius: Theme.controls.sidebar.rowRadius
                                        color: actionArea.containsMouse
                                               ? Theme.color.surfaceElevated : "transparent"
                                        border.width: Theme.controls.window.borderWidth
                                        border.color: Theme.color.border

                                        Accessible.role: Accessible.Button
                                        Accessible.name: modelData.label

                                        Text {
                                            id: actionLabel
                                            anchors.centerIn: parent
                                            text: modelData.label
                                            color: Theme.color.accent
                                            font.pixelSize: Theme.primitive.font.sizeSm
                                            font.weight: Theme.primitive.font.weightMedium
                                        }

                                        MouseArea {
                                            id: actionArea
                                            anchors.fill: parent
                                            hoverEnabled: true
                                            onClicked: root.actionInvoked(row.modelData.id,
                                                                          modelData.key)
                                        }
                                    }
                                }
                            }

                            Rectangle {
                                objectName: "historyDismiss"
                                visible: row.active
                                height: visible ? 24 : 0
                                width: dismissLabel.implicitWidth
                                       + 2 * Theme.primitive.spacing.sm
                                radius: Theme.controls.sidebar.rowRadius
                                color: dismissArea.containsMouse
                                       ? Theme.color.surfaceElevated : "transparent"

                                Accessible.role: Accessible.Button
                                Accessible.name: qsTr("Dismiss notification")

                                Text {
                                    id: dismissLabel
                                    anchors.centerIn: parent
                                    text: qsTr("Dismiss")
                                    color: Theme.color.textTertiary
                                    font.pixelSize: Theme.primitive.font.sizeXs
                                }

                                MouseArea {
                                    id: dismissArea
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    onClicked: root.dismissEntryRequested(row.modelData.id)
                                }
                            }
                        }
                    }
                }

                // Deliberate empty/unavailable state: the stack keeps its
                // material and the lower panel carries the count summary.
                Text {
                    id: emptyTextItem
                    objectName: "emptyHistory"
                    visible: !root.hasHistory
                    anchors.centerIn: parent
                    width: Math.min(parent.width - 2 * Theme.primitive.spacing.lg,
                                    implicitWidth)
                    text: root.emptyText
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeMd
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.WordWrap
                    Accessible.role: Accessible.StaticText
                    Accessible.name: root.emptyText
                }

                // Deliberate scrollable state: a visible thumb only while the
                // history overflows the stack viewport.
                Rectangle {
                    id: scrollTrack
                    objectName: "historyScrollBar"
                    visible: root.scrollable
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    anchors.rightMargin: 2
                    anchors.topMargin: Theme.primitive.spacing.xs
                    anchors.bottomMargin: Theme.primitive.spacing.xs
                    width: 4
                    radius: width / 2
                    color: Theme.color.controlFill

                    Rectangle {
                        width: parent.width
                        height: root.scrollThumbHeight
                        radius: width / 2
                        color: Theme.color.textTertiary
                        y: root.scrollMax > 0
                           ? (list.contentY / root.scrollMax)
                             * Math.max(0, scrollTrack.height - height) : 0
                    }
                }
            }
        }
    }

    // The lower panel: a separately toned footer with the history summary and
    // the keyboard/scroll hint. Its region is disjoint from the stack.
    Shadow {
        anchors.fill: lowerPanel
        radius: lowerPanel.radius
        level: "overlay"
        visible: !root.sampledGlass
    }

    Rectangle {
        id: lowerPanel
        objectName: "notificationLowerPanel"
        x: 0
        y: root.lowerTop
        width: root.width
        height: root.lowerPanelHeight
        radius: root.panelRadius
        color: root.lowerFill
        border.width: Theme.controls.window.borderWidth
        border.color: Theme.color.border

        Item {
            anchors.fill: parent
            anchors.leftMargin: Theme.primitive.spacing.lg
            anchors.rightMargin: Theme.primitive.spacing.lg

            Text {
                id: lowerHint
                objectName: "lowerPanelHint"
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                text: root.scrollable ? qsTr("Scroll for more") : qsTr("Esc to close")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeXs
            }

            Text {
                id: lowerSummaryText
                objectName: "lowerPanelSummary"
                anchors.left: parent.left
                anchors.right: lowerHint.left
                anchors.rightMargin: Theme.primitive.spacing.md
                anchors.verticalCenter: parent.verticalCenter
                text: root.lowerSummary
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                elide: Text.ElideRight
                Accessible.role: Accessible.StaticText
                Accessible.name: root.lowerSummary
            }
        }
    }

    readonly property bool scrollable: list.contentHeight > list.height + 0.5
    readonly property real scrollMax: Math.max(0, list.contentHeight - list.height)
    readonly property real scrollThumbHeight: list.contentHeight > 0
        ? Math.max(24, list.height * list.height / list.contentHeight) : 0

    onSelectedIndexChanged: {
        if (selectedIndex >= 0 && list.count > 0)
            list.positionViewAtIndex(selectedIndex, ListView.Contain)
    }
}
