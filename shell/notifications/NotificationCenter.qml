// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The notification-center history (T-11.1a; mapped by T204, materials T205,
// measured layout/widgets/actions T234).
//
// The service owns the history, expiry, Focus/DND and banner policy. The shell
// decodes `History()` once (`NotificationCenter` in shell/src) into stable-keyed
// rows: `key`, `active`, `canAct`, `status`, the filtered `actions` and the
// contiguous `group`/`groupStart`/`groupCount` metadata. This component is only
// the view. It renders the measured 344 pt right-edge ramp: a header, the
// banner stack, the widget stack and the lower panel.
//
// History-only rows (expired, closed, dismissed or DND-suppressed) keep their
// text and stable identity but never show actions: `actions` is empty and
// `canAct` is false in the model.
//
// T205/T234: the Center is still two independently keyed materials. The
// notification stack (header + banner stack) uses `notificationStack`; the
// lower panel (widget stack + actions) uses `notificationLowerPanel`. Both
// regions are declared to the compositor and commit-latched with the surface
// image. On the sampled path the compositor paints the measured flat bands and
// the QML plates are transparent; on solid/software/Reduce Transparency the QML
// paints the same resolved tint/alpha. The regions never overlap, so a panel is
// never frosted twice. The banner itself remains its own `notificationBanner`
// material on the separate banner surface.
Item {
    id: root

    // The measured right-edge panel width (structural review §5.1).
    readonly property int measuredWidth: 344

    // Standalone/test default; the shell sets the live surface size through
    // `prepareChromeScene`.
    width: measuredWidth
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

    // T234: the widget stack below the notifications. Each entry is a map
    // (`id`, `kind`, `size`, `title`, `available`, optional live fields). The
    // live panel size is the measured 164 pt calendar, never the gallery
    // thumbnail. Widgets are optional: an empty list shows the widget empty
    // state and the lower panel keeps its action row.
    property var widgets: []
    readonly property bool hasWidgets: widgets.length > 0

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
    // The measured banner stack row height (structural review §5.1).
    readonly property int activeRowHeight: 75
    readonly property int historyRowHeight: 64

    // The two panel bands. The stack clears an active banner and the lower
    // panel is a footer; the gap keeps both rounded edges visible.
    readonly property int panelGap: Theme.primitive.spacing.sm
    readonly property int lowerPanelBaseHeight: 64
    // T234: the widget stack lives inside the lower panel's material band, so
    // the measured lower-panel ramp covers the widgets too and no third
    // material is invented. The stack grows by one 164 pt calendar card plus
    // the section's padding.
    readonly property int widgetSectionHeight: hasWidgets ? 180 : 0
    readonly property int lowerPanelHeight: lowerPanelBaseHeight + widgetSectionHeight
    readonly property int stackTop: Math.min(bannerInset,
        Math.max(0, height - lowerPanelHeight - panelGap))
    readonly property int lowerTop: Math.max(stackTop + panelGap,
                                             height - lowerPanelHeight)
    readonly property int stackHeight: Math.max(0, lowerTop - panelGap - stackTop)
    readonly property int panelRadius: Theme.controls.window.radius

    // T205/T234: the shell declares these regions on the `notification-center`
    // layer surface. `stack` and `lower` are stable keys with disjoint rects;
    // the material list is commit-latched with the image. A banner uses its
    // own `notification` surface and `banner` region, so no region samples a
    // panel's own content. The widget stack is part of the lower region, not a
    // third band.
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
    // T234: header and lower-panel actions. The shell forwards these to the
    // notification service (clear) or opens the separate widget gallery.
    signal clearAllRequested()
    signal clearHistoryRequested()
    signal editWidgetsRequested()
    signal widgetRemoveRequested(string id)

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

    // The notification stack: header plus the banner-stack viewport. Its
    // material region starts at `stackTop`, clearing a co-visible banner.
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

            // Measured header: "Notifications" plus Clear All. The
            // overflow menu clears only the recorded history; Clear All also
            // closes the active banner stack.
            Item {
                id: header
                objectName: "centerHeader"
                width: parent.width
                height: Math.max(titleText.implicitHeight,
                                 headerActions.implicitHeight)

                Text {
                    id: titleText
                    width: parent.width - headerActions.width
                           - Theme.primitive.spacing.sm
                    objectName: "centerTitle"
                    text: qsTr("Notifications")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.primitive.font.sizeLg
                    font.weight: Theme.primitive.font.weightSemibold
                    elide: Text.ElideRight
                    anchors.verticalCenter: parent.verticalCenter
                }

                Row {
                    id: headerActions
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xs

                    Rectangle {
                        id: clearAllButton
                        objectName: "clearAllButton"
                        width: clearAllLabel.implicitWidth
                               + 2 * Theme.primitive.spacing.sm
                        height: 24
                        radius: Theme.controls.sidebar.rowRadius
                        color: clearAllArea.containsMouse
                               ? Theme.color.surfaceElevated : "transparent"
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border
                        visible: root.hasHistory

                        Accessible.role: Accessible.Button
                        Accessible.name: qsTr("Clear All")

                        Text {
                            id: clearAllLabel
                            anchors.centerIn: parent
                            text: qsTr("Clear All")
                            color: Theme.color.accent
                            font.pixelSize: Theme.primitive.font.sizeSm
                            font.weight: Theme.primitive.font.weightMedium
                        }

                        MouseArea {
                            id: clearAllArea
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: root.clearAllRequested()
                        }
                    }

                    Rectangle {
                        id: clearHistoryButton
                        objectName: "clearHistoryButton"
                        width: 24
                        height: 24
                        radius: Theme.controls.sidebar.rowRadius
                        color: clearHistoryArea.containsMouse
                               ? Theme.color.surfaceElevated : "transparent"
                        visible: root.hasHistory

                        Accessible.role: Accessible.Button
                        Accessible.name: qsTr("Clear Notifications")

                        Icon {
                            anchors.centerIn: parent
                            name: "list-view"
                            size: Theme.primitive.font.sizeMd
                            color: Theme.color.textSecondary
                        }

                        MouseArea {
                            id: clearHistoryArea
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: root.clearHistoryRequested()
                        }
                    }
                }
            }

            Item {
                id: listArea
                width: parent.width
                height: parent.height - header.height - layout.spacing

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
                        readonly property bool groupStart: modelData.groupStart === true
                        readonly property int groupCount:
                            modelData.groupCount !== undefined
                                ? Number(modelData.groupCount) : 1
                        // The active banner-stack row keeps its own app label
                        // so the 75 pt row has a stable anatomy; grouping is a
                        // history affordance.
                        readonly property bool showGroupHeader: groupStart
                            && groupCount > 1 && !active
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
                        // The measured banner-stack row is 75 pt for an active
                        // notification; a compact history row keeps more of
                        // the list visible.
                        height: active ? root.activeRowHeight
                                       : Math.max(root.historyRowHeight,
                                                  rowContent.implicitHeight
                                                  + 2 * Theme.primitive.spacing.sm)
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
                            // Leave room for the active row's dismiss target.
                            anchors.rightMargin: row.active
                                ? 26 : Theme.primitive.spacing.sm
                            anchors.topMargin: row.active ? 4
                                                          : Theme.primitive.spacing.sm
                            anchors.bottomMargin: row.active ? 4
                                                               : Theme.primitive.spacing.sm
                            spacing: Theme.primitive.spacing.xxs

                            // T234: one group header per contiguous app run.
                            // It carries the app name and the run length; the
                            // per-row app label is suppressed below it.
                            Text {
                                id: groupHeader
                                objectName: "historyGroupHeader"
                                visible: row.showGroupHeader
                                width: parent.width
                                height: visible ? implicitHeight : 0
                                text: qsTr("%1 · %2").arg(modelData.appName)
                                    .arg(row.groupCount)
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeXs
                                font.weight: Theme.primitive.font.weightMedium
                                elide: Text.ElideRight
                            }

                            Row {
                                width: parent.width
                                visible: !row.showGroupHeader
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
                            // The status stays visible even when the group
                            // header replaces the app label.
                            Text {
                                objectName: "historyStatusGrouped"
                                visible: row.showGroupHeader && row.statusText.length > 0
                                height: visible ? implicitHeight : 0
                                text: row.statusText
                                color: Theme.color.textTertiary
                                font.pixelSize: Theme.primitive.font.sizeXs
                                elide: Text.ElideRight
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
                                // The 75 pt active row gives its second line
                                // to the action row; the body stays on a row
                                // without actions and on history rows.
                                visible: modelData.body.length > 0
                                    && (!row.active || row.actions.length === 0)
                                text: modelData.body
                                color: Theme.color.textSecondary
                                font.pixelSize: Theme.primitive.font.sizeSm
                                wrapMode: Text.WordWrap
                                maximumLineCount: row.active ? 1 : 2
                                elide: Text.ElideRight
                            }

                            Row {
                                id: actionRow
                                objectName: "historyActions"
                                visible: row.active && row.actions.length > 0
                                width: parent.width
                                height: visible ? 22 : 0
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

                        }

                        // The dismiss target sits in the active row's top-right
                        // corner, outside the text column, so the measured
                        // 75 pt row keeps its text and action anatomy.
                        Rectangle {
                            objectName: "historyDismiss"
                            visible: row.active
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.margins: 4
                            width: 20
                            height: 20
                            radius: 10
                            color: dismissArea.containsMouse
                                   ? Theme.color.surfaceElevated : "transparent"

                            Accessible.role: Accessible.Button
                            Accessible.name: qsTr("Dismiss notification")

                            Icon {
                                anchors.centerIn: parent
                                name: "close"
                                size: 12
                                color: Theme.color.textTertiary
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

    // The lower panel: the widget stack plus a separately toned action footer.
    // Its region is disjoint from the stack and covers both the widgets and
    // the actions, so the measured lower-panel ramp is never split.
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

        Column {
            anchors.fill: parent
            anchors.margins: Theme.primitive.spacing.sm
            spacing: Theme.primitive.spacing.xs

            WidgetStack {
                id: widgetStack
                objectName: "widgetStack"
                width: parent.width
                height: root.hasWidgets
                    ? root.lowerPanelHeight - 2 * Theme.primitive.spacing.sm
                      - lowerFooter.height - Theme.primitive.spacing.xs
                    : 0
                visible: root.hasWidgets
                widgets: root.widgets
                available: root.available
                onRemoveRequested: (id) => root.widgetRemoveRequested(id)
            }

            Item {
                id: lowerFooter
                width: parent.width
                height: root.lowerPanelBaseHeight - 2 * Theme.primitive.spacing.sm

                Text {
                    id: lowerHint
                    objectName: "lowerPanelHint"
                    anchors.right: lowerActions.left
                    anchors.rightMargin: Theme.primitive.spacing.md
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.scrollable ? qsTr("Scroll for more") : qsTr("Esc to close")
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeXs
                }

                Row {
                    id: lowerActions
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.primitive.spacing.xs

                    Rectangle {
                        id: editWidgetsButton
                        objectName: "editWidgetsButton"
                        width: editWidgetsLabel.implicitWidth
                               + 2 * Theme.primitive.spacing.sm
                        height: 24
                        radius: Theme.controls.sidebar.rowRadius
                        color: editWidgetsArea.containsMouse
                               ? Theme.color.surfaceElevated : "transparent"
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border

                        Accessible.role: Accessible.Button
                        Accessible.name: qsTr("Edit Widgets")

                        Text {
                            id: editWidgetsLabel
                            anchors.centerIn: parent
                            text: qsTr("Edit Widgets")
                            color: Theme.color.accent
                            font.pixelSize: Theme.primitive.font.sizeXs
                            font.weight: Theme.primitive.font.weightMedium
                        }

                        MouseArea {
                            id: editWidgetsArea
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: root.editWidgetsRequested()
                        }
                    }
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
