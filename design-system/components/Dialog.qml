// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// A modal dialog: a scrim over the host window plus a centred card. `show()`/
// `accept()`/`reject()` drive it; Escape rejects (when dismissible) and Return
// accepts. The card is AT-SPI Dialog and traps Tab focus while open.
//
// T266: a dialog declared inside a pane adopts the window overlay layer
// itself, so the detail `ScrollView` can never clip its card or its scrim. It
// fits the window, scrolls long content, sizes from a stable width, and always
// offers a visible Close action. Use Sheet for the window-attached variant.
FocusScope {
    id: root

    property string title: ""
    property string message: ""
    property alias contentData: contentColumn.data
    property alias content: contentColumn
    property alias buttonsData: buttonRow.data
    property alias buttons: buttonRow
    property bool open: false
    property bool dismissible: true
    // A visible way out of every dialog. A dialog that is not dismissible
    // hides it because closing is not allowed.
    property bool showCloseAction: true
    property alias card: card
    property alias closeAction: closeAction
    property alias scrim: scrim

    signal accepted()
    signal rejected()
    signal opened()
    signal closed()

    readonly property int padding: Theme.controls.dialog.padding
    // T266: the card is bounded by the window, not by its content. The width
    // reads implicit sizes only (never the card's own laid-out width), so it
    // cannot feed back into `cardWidth`.
    readonly property real windowMargin: Theme.primitive.spacing.lg
    readonly property real availableWidth:
        Math.max(Theme.controls.dialog.minWidth,
                 (root.parent ? root.parent.width : 720) - 2 * root.windowMargin)
    readonly property real availableHeight:
        Math.max(120, (root.parent ? root.parent.height : 480) - 2 * root.windowMargin)
    readonly property real preferredCardWidth:
        Math.max(Theme.controls.dialog.minWidth,
                 Math.max(titleText.implicitWidth,
                          Math.max(contentColumn.implicitWidth,
                                   buttonRow.implicitWidth)) + 2 * root.padding)
    readonly property real cardWidth:
        Math.min(root.availableWidth, root.preferredCardWidth)
    readonly property real maxBodyHeight:
        Math.max(0, root.availableHeight - 2 * root.padding)
    readonly property real footerHeight: buttonRow.implicitHeight
    readonly property real bodySpacing: Theme.primitive.spacing.md

    width: parent ? parent.width : 480
    height: parent ? parent.height : 320
    visible: opacity > 0
    z: 3000
    focus: root.open
    activeFocusOnTab: root.open

    function show() { root.open = true; }
    function hide() { root.open = false; }
    function accept() {
        root.hide();
        root.accepted();
    }
    function reject() {
        if (!root.dismissible)
            return;
        root.hide();
        root.rejected();
    }

    // Adopt the window overlay layer when one exists. Panes declare no
    // `popupParent`; the layer is the one place a dialog may live.
    function adoptOverlay() {
        var layer = OverlayLayer.layerFor(root);
        if (layer && root.parent !== layer)
            root.parent = layer;
    }

    Component.onCompleted: root.adoptOverlay()
    Connections {
        target: OverlayLayer
        function onLayersChanged() { root.adoptOverlay(); }
    }

    onOpenChanged: {
        if (root.open) {
            root.adoptOverlay();
            root.forceActiveFocus();
            root.opened();
        } else {
            root.closed();
        }
    }

    // The dialog's own focus trap. Tab never walks out of an open dialog;
    // Shift+Tab walks back. The window's normal traversal is restored when
    // the dialog closes.
    function focusableItems() {
        var out = [];
        root.collectFocusable(root.card, out);
        return out;
    }

    function collectFocusable(item, out) {
        if (item === null || item === undefined)
            return;
        if (item.activeFocusOnTab === true && item.visible && item.enabled !== false)
            out.push(item);
        var children = item.children;
        for (var i = 0; i < children.length; ++i)
            root.collectFocusable(children[i], out);
    }

    function cycleFocus(backwards) {
        var items = root.focusableItems();
        if (items.length === 0) {
            root.forceActiveFocus();
            return;
        }
        var hostWindow = root.Window.window;
        var current = hostWindow ? hostWindow.activeFocusItem : null;
        var at = items.indexOf(current);
        var next = 0;
        if (backwards)
            next = at <= 0 ? items.length - 1 : at - 1;
        else
            next = (at < 0 || at >= items.length - 1) ? 0 : at + 1;
        items[next].forceActiveFocus();
    }

    Rectangle {
        id: scrim
        objectName: "dialogScrim"
        anchors.fill: parent
        color: Theme.color.shadowColor
        opacity: root.open ? 0.4 : 0.0

        Behavior on opacity {
            NumberAnimation {
                duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve : Theme.motion.popupClose.curve
            }
        }
    }

    Item {
        id: card
        anchors.centerIn: parent
        width: root.cardWidth
        height: bodyScroll.height + root.footerHeight
                + 2 * root.padding + root.bodySpacing
        scale: root.open ? 1.0 : 0.97
        opacity: root.open ? 1.0 : 0.0
        transformOrigin: Item.Center

        Behavior on opacity {
            NumberAnimation {
                duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve : Theme.motion.popupClose.curve
            }
        }
        Behavior on scale {
            NumberAnimation {
                duration: root.open ? Theme.motion.popupOpen.duration : Theme.motion.popupClose.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: root.open ? Theme.motion.popupOpen.curve : Theme.motion.popupClose.curve
            }
        }

        Shadow {
            width: card.width
            height: card.height
            radius: Theme.controls.dialog.radius
        }

        Rectangle {
            anchors.fill: parent
            radius: Theme.controls.dialog.radius
            color: Theme.color.surfaceElevated
            border.width: Theme.controls.window.borderWidth
            border.color: Theme.color.border
            antialiasing: true
        }

        ScrollView {
            id: bodyScroll
            objectName: "dialogBodyScroll"
            x: root.padding
            y: root.padding
            width: Math.max(0, card.width - 2 * root.padding)
            height: Math.min(body.implicitHeight,
                             Math.max(0, root.maxBodyHeight - root.footerHeight
                                         - root.bodySpacing))
            interactive: body.implicitHeight > bodyScroll.height
            scrollbarVisible: body.implicitHeight > bodyScroll.height

            Column {
                id: body
                width: bodyScroll.width
                spacing: Theme.primitive.spacing.md

                Text {
                    id: titleText
                    width: parent.width
                    visible: root.title.length > 0
                    text: root.title
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.primitive.font.sizeLg
                    font.weight: Theme.primitive.font.weightSemibold
                    wrapMode: Text.WordWrap
                }

                Text {
                    id: messageText
                    width: parent.width
                    visible: root.message.length > 0
                    text: root.message
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                    wrapMode: Text.WordWrap
                }

                Column {
                    id: contentColumn
                    width: parent.width
                }
            }
        }

        // The actions stay pinned to the card bottom. A long body scrolls
        // above them, so Close/Done is always visible and always clickable.
        Item {
            id: footer
            x: root.padding
            y: card.height - root.padding - root.footerHeight
            width: Math.max(0, card.width - 2 * root.padding)
            height: root.footerHeight

            Row {
                id: buttonRow
                anchors.right: parent.right
                spacing: Theme.controls.dialog.buttonGap

                Button {
                    id: closeAction
                    objectName: "dialogCloseAction"
                    text: qsTr("Close")
                    variant: "secondary"
                    accessibleName: qsTr("Close dialog")
                    visible: root.showCloseAction && root.dismissible
                    onClicked: root.reject()
                }
            }
        }
    }

    Keys.onEscapePressed: (event) => {
        root.reject();
        event.accepted = true;
    }
    Keys.onReturnPressed: (event) => {
        root.accept();
        event.accepted = true;
    }
    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Tab) {
            root.cycleFocus((event.modifiers & Qt.ShiftModifier) !== 0);
            event.accepted = true;
        }
    }

    Accessible.role: Accessible.Dialog
    Accessible.name: root.title
}
