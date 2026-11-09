// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The first-party titlebar. It owns no windowing behavior: it reports intent
// (move / zoom / window-menu) so the host application can forward it to the
// compositor over the private protocol. The compositor's SSD renderer draws
// from the same tokens (FR-3), so the two cannot drift apart.
//
// T230b: the chrome height comes from the named window style
// (`WindowStyle`), never from an app constant. `titlebar` is a 32 pt
// title-only bar, `unified` is a 52 pt toolbar, `expanded`/`preference` add a
// title row above the toolbar row, and `unified` plus `labelledToolbar` is
// the 66 pt icon-and-label variant. An app can add its own toolbar row with
// `extraToolbarRow` (a Pages-style document is 48 + 22 = 70 pt).
Item {
    id: root

    property string title: ""
    property bool active: true
    property bool showTrafficLights: true
    // An app can set the style explicitly; otherwise a `TitleBar` inside an
    // `AppWindow` inherits the window's named style.
    property string style: {
        var item = root.parent;
        while (item) {
            if (item.windowStyle !== undefined)
                return item.windowStyle;
            item = item.parent;
        }
        return "titlebar";
    }
    // The 66 pt Disk Utility variant: `unified` plus the labelled toolbar row.
    property bool labelledToolbar: false
    // A document/app-specific toolbar row added to the style baseline.
    property real extraToolbarRow: 0
    // T206: first-party apps pass the resolved app-local material fill; the
    // default preserves the existing opaque chrome.
    property color fillColor: Theme.color.chrome
    property color borderColor: Theme.color.border
    property alias trafficLights: lights
    property alias hovered: dragArea.containsMouse
    property alias dragArea: dragArea
    property alias leadingData: leadingRow.data
    property alias trailingData: trailingRow.data
    property alias leading: leadingRow
    property alias trailing: trailingRow
    property alias titleRow: titleRow
    property alias toolbarRow: toolbarRow

    signal closeRequested()
    signal minimizeRequested()
    signal zoomRequested()
    signal menuRequested(real x, real y)
    signal moveRequested(real x, real y)

    readonly property real chromeHeight: WindowStyle.chromeHeight(style, {
        labelledToolbar: root.labelledToolbar,
        toolbarRowHeight: root.extraToolbarRow
    })
    // A style with no explicit title row (titlebar, unified, unifiedCompact)
    // puts everything in one row; expanded/preference split the title row and
    // the toolbar row.
    readonly property real titleRowHeight:
        WindowStyle.titleRowHeight(style) > 0
            ? WindowStyle.titleRowHeight(style) : root.chromeHeight
    readonly property real toolbarRowHeight:
        Math.max(0, root.chromeHeight - root.titleRowHeight)

    implicitWidth: 360
    implicitHeight: root.chromeHeight
    visible: root.chromeHeight > 0

    // A titlebar spans its window: when it is parented into a chrome slot
    // (e.g. `AppWindow.titleBarData`, whose slot is the window width) it fills
    // the parent width, so the drag area, centered title, and traffic-light
    // cluster cover the whole top strip. A standalone instance keeps its
    // implicit width; an explicit `width` still overrides this.
    width: parent ? parent.width : implicitWidth

    // Top corners follow the window radius; the bottom edge is square so the
    // titlebar reads as one sheet with the content below. The two rectangles
    // are composited inside one item so the shared opacity does not stack.
    Item {
        id: chrome
        anchors.fill: parent
        opacity: Theme.material.chromeOpacity

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.titlebar.cornerRadius
            color: root.fillColor
            antialiasing: true
        }
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: Theme.controls.titlebar.cornerRadius
            color: root.fillColor
        }
    }

    // The whole chrome strip is the drag surface. Toolbar controls declared
    // through `leadingData`/`trailingData` are later siblings and therefore
    // keep their own clicks; the titlebar itself never extends into content.
    MouseArea {
        id: dragArea
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        hoverEnabled: true
        onPressed: (mouse) => {
            if (mouse.button === Qt.LeftButton)
                root.moveRequested(mouse.x, mouse.y);
        }
        onDoubleClicked: root.zoomRequested()
        onClicked: (mouse) => {
            if (mouse.button === Qt.RightButton)
                root.menuRequested(mouse.x, mouse.y);
        }
    }

    Item {
        id: titleRow
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: root.titleRowHeight
    }

    Item {
        id: toolbarRow
        anchors.top: titleRow.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        height: root.toolbarRowHeight
        visible: root.toolbarRowHeight > 0
    }

    TrafficLights {
        id: lights
        visible: root.showTrafficLights && root.chromeHeight > 0
        active: root.active
        anchors.left: parent.left
        anchors.leftMargin: WindowStyle.trafficLightLeadingInset
        anchors.verticalCenter: titleRow.verticalCenter

        onCloseClicked: root.closeRequested()
        onMinimizeClicked: root.minimizeRequested()
        onZoomClicked: root.zoomRequested()
    }

    readonly property real titleSideReserve:
        WindowStyle.trafficLightLeadingInset
        + (lights.visible ? lights.width + Theme.controls.titlebar.spacing : 0)
        + (root.toolbarRowHeight > 0
           ? 0 : Math.max(leadingRow.implicitWidth, trailingRow.implicitWidth))
        + Theme.controls.titlebar.paddingH

    Text {
        anchors.centerIn: titleRow
        width: Math.max(0, titleRow.width - 2 * root.titleSideReserve)
        text: root.title
        horizontalAlignment: Text.AlignHCenter
        elide: Text.ElideMiddle
        color: root.active ? Theme.color.textPrimary : Theme.color.textTertiary
        font.pixelSize: Theme.controls.titlebar.fontSize
        font.weight: Theme.controls.titlebar.fontWeight

        Behavior on color {
            ColorAnimation {
                duration: Theme.motion.hover.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.motion.hover.curve
            }
        }
    }

    Row {
        id: leadingRow
        anchors.left: root.showTrafficLights ? lights.right : parent.left
        anchors.leftMargin: root.showTrafficLights
            ? Theme.controls.titlebar.spacing
            : WindowStyle.trafficLightLeadingInset
        anchors.verticalCenter: root.toolbarRowHeight > 0
            ? toolbarRow.verticalCenter : titleRow.verticalCenter
        spacing: Theme.controls.toolbar.spacing
        visible: children.length > 0
    }

    Row {
        id: trailingRow
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.titlebar.paddingH
        anchors.verticalCenter: root.toolbarRowHeight > 0
            ? toolbarRow.verticalCenter : titleRow.verticalCenter
        spacing: Theme.controls.toolbar.spacing
        visible: children.length > 0
    }

    Accessible.role: Accessible.TitleBar
    Accessible.name: root.title
}
