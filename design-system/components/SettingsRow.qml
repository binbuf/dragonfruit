// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// One label + control row inside a SettingsGroup (T-09.1a; row anatomy T223).
//
// The row owns one anatomy for every settings surface: the label column, the
// optional description, the control slot, an optional disclosure chevron, an
// optional trailing action, and an optional help affordance. It never draws the
// control itself. `settingsId`, `owner`, and `settingsKey` link the rendered row
// back to the executable map (`SettingsMap.qml`); T223's search and deep links
// use `settingsId` first and fall back to the label/synonyms.
//
// `highlighted` is the deep-link landing state: the row paints an accent wash
// that the shell clears after a short hold. AT-SPI sees a labelled grouping so
// a screen reader reads the control with its setting name.
Item {
    id: root

    property string label: ""
    property string description: ""
    // T279: an optional leading glyph. Privacy & Security categories and other
    // grouped lists carry a distinct mark; a row without one keeps the plain
    // anatomy and the same label column.
    property string leadingIcon: ""
    property color leadingIconColor: Theme.color.textSecondary
    property real leadingIconSize: 20
    // T286: an optional leading category tile (29 pt by default) or a custom
    // leading item. A pane supplies `leadingTile` (a resolved
    // `SettingsCategoryIcon` style) or fills `leadingData`; the label column
    // keeps one indent either way. `labelIndent` overrides the derived left
    // inset for a pane that owns a wider leading cell.
    property var leadingTile: null
    property real leadingTileSize: Theme.controls.settingsRow.leadingTile
    property real leadingWidth: root.leadingTile !== null
                                ? root.leadingTileSize
                                : (root.leadingIcon.length > 0
                                   ? root.leadingIconSize : 0)
    property real leadingHeight: root.leadingTile !== null
                                 ? root.leadingTileSize
                                 : (root.leadingIcon.length > 0
                                    ? root.leadingIconSize : 0)
    property real labelIndent: -1
    property alias leadingData: leadingSlot.data
    property alias leadingSlot: leadingSlot
    // T286: the row height variant. 0 derives it from the anatomy: 33 pt
    // plain, 52 pt with a description, 53 pt with a leading tile.
    property real rowHeight: 0
    // T287: a big-list row keeps its 52 pt token even when the optional second
    // line makes the label column a pixel taller. The row still grows for a
    // wrapped localized label unless this is set.
    property bool fixedRowHeight: false
    // T286: children declared inside a `SettingsRow` land in the control slot,
    // so a variant file can extend the anatomy without a second widget.
    default property alias contentData: controlSlot.data
    // T224: an inline failure/validation message. It sits under the label and
    // description so the row explains why a control reverted; empty hides it.
    property string errorText: ""
    property alias controlData: controlSlot.data
    property alias control: controlSlot
    property bool showSeparator: true

    // T223 map wiring. A row without an id is still rendered; it is simply not
    // an individually searchable/deep-linkable target until a later pane task
    // gives it the matching map id.
    property string settingsId: ""
    property string owner: ""
    property string settingsKey: ""
    property var synonyms: []
    // Help is indexed by SettingsSearch; a non-empty help string shows the row's
    // trailing `?` affordance. `helpUrl` is optional (the pane usually points
    // it at the map document anchor).
    property string help: ""
    property url helpUrl: ""

    // Disclosure rows (a right chevron) and trailing action rows (a ghost
    // button such as `Details...`) share the same trailing cell.
    property bool disclosure: false
    property string trailingAction: ""
    // T275: a delegate action may be present but unavailable (the external
    // program is not installed). The row keeps its anatomy and disables only
    // the action, with the reason in the description.
    property bool trailingActionEnabled: true
    signal disclosureClicked()
    signal trailingActionClicked()
    signal helpRequested(string topic)

    // The deep-link landing wash.
    property bool highlighted: false

    readonly property bool hasTrailing: root.disclosure
                                          || root.trailingAction.length > 0
                                          || root.help.length > 0
    // T266: the last SettingsRow in a SettingsGroup suppresses its separator.
    // The row computes this from the group's live children, so adding a row
    // restores the previous last row's separator without rewriting state.
    readonly property bool isLastGroupRow: {
        if (parent === null || parent === undefined
                || parent.settingsGroupRows !== true)
            return false;
        var siblings = parent.children;
        for (var i = siblings.length - 1; i >= 0; --i) {
            if (siblings[i].showSeparator !== undefined)
                return siblings[i] === root;
        }
        return false;
    }

    // Test/consumer aliases for the one anatomy.
    property alias labelText: labelText
    property alias leadingIconItem: leadingIconItem
    property alias descriptionText: descriptionText
    property alias errorLabel: errorTextItem
    property alias labelColumn: labelColumn
    property alias trailingRow: trailingRow
    property alias disclosureArea: disclosureArea
    property alias trailingButton: trailingButton
    property alias helpArea: helpArea
    property alias highlightRect: highlightRect
    property alias separatorRect: separatorRect
    property alias focusRing: rowFocusRing
    property alias disclosureFocusRing: disclosureFocusRing
    property alias helpFocusRing: helpFocusRing

    implicitWidth: 480
    readonly property real tokenHeight: root.rowHeight > 0 ? root.rowHeight
        : root.description.length > 0 ? Theme.controls.settingsRow.descriptionHeight
        : root.leadingWidth > 0 ? Theme.controls.settingsRow.iconHeight
        : Theme.controls.settingsRow.height
    implicitHeight: root.fixedRowHeight ? root.tokenHeight
                                        : Math.max(root.tokenHeight,
                             labelColumn.implicitHeight + 2 * Theme.primitive.spacing.sm)
    activeFocusOnTab: true

    // T223: a deep-linked row gets an accent wash that fades in and out. It
    // sits behind the row content so the control and label stay legible.
    Rectangle {
        id: highlightRect
        anchors.fill: parent
        radius: Theme.controls.settingsGroup.radius
        color: Theme.color.accentMuted
        opacity: root.highlighted ? 1.0 : 0.0
        visible: opacity > 0
        antialiasing: true

        Behavior on opacity {
            NumberAnimation {
                duration: Theme.motion.hover.duration
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.motion.hover.curve
            }
        }
    }

    // The trailing cell: trailing action, disclosure, then help, right to left
    // in that order so the chevron stays the closest mark to the control.
    Row {
        id: trailingRow
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.settingsRow.paddingH
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.primitive.spacing.xs
        visible: root.hasTrailing

        Button {
            id: trailingButton
            visible: root.trailingAction.length > 0
            enabled: root.trailingActionEnabled
            text: root.trailingAction
            variant: "ghost"
            accessibleName: root.trailingAction
            onClicked: root.trailingActionClicked()
        }

        Item {
            id: disclosureArea
            visible: root.disclosure
            width: Theme.controls.button.iconSize + 2 * Theme.primitive.spacing.xxs
            height: width
            activeFocusOnTab: true

            Icon {
                anchors.centerIn: parent
                name: root.LayoutMirroring.enabled ? "chevron-left"
                                                   : "chevron-right"
                size: Theme.controls.button.iconSize
                color: Theme.color.textTertiary
            }

            TapHandler {
                onTapped: root.disclosureClicked()
            }

            Keys.onPressed: (event) => {
                if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                        || event.key === Qt.Key_Space) {
                    root.disclosureClicked();
                    event.accepted = true;
                }
            }

            FocusRing {
                id: disclosureFocusRing
                target: disclosureArea
                cornerRadius: Theme.primitive.radius.xs
                shown: disclosureArea.activeFocus
            }

            Accessible.role: Accessible.Button
            Accessible.name: root.label
            Accessible.description: qsTr("Show details")
            Accessible.focusable: true
            Accessible.onPressAction: root.disclosureClicked()
        }

        Item {
            id: helpArea
            visible: root.help.length > 0
            width: Theme.controls.button.iconSize + 2 * Theme.primitive.spacing.xxs
            height: width
            activeFocusOnTab: true

            Rectangle {
                anchors.fill: parent
                radius: width / 2
                color: helpHover.hovered ? Theme.color.controlHover : "transparent"
                border.width: Theme.controls.window.borderWidth
                border.color: Theme.color.border
                antialiasing: true
            }

            Text {
                anchors.centerIn: parent
                text: "?" // df-allow-untranslated: help glyph
                color: Theme.color.textSecondary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightSemibold
            }

            HoverHandler { id: helpHover }

            TapHandler {
                onTapped: {
                    root.helpRequested(root.help);
                    if (root.helpUrl.toString().length > 0)
                        Qt.openUrlExternally(root.helpUrl);
                }
            }

            Keys.onPressed: (event) => {
                if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                        || event.key === Qt.Key_Space) {
                    root.helpRequested(root.help);
                    if (root.helpUrl.toString().length > 0)
                        Qt.openUrlExternally(root.helpUrl);
                    event.accepted = true;
                }
            }

            FocusRing {
                id: helpFocusRing
                target: helpArea
                cornerRadius: width / 2
                shown: helpArea.activeFocus
            }

            Accessible.role: Accessible.Button
            Accessible.name: qsTr("Help for %1").arg(root.label)
            Accessible.focusable: true
            Accessible.onPressAction: {
                root.helpRequested(root.help);
                if (root.helpUrl.toString().length > 0)
                    Qt.openUrlExternally(root.helpUrl);
            }
        }
    }

    // The control slot is sized from the child's implicit size (T266). The
    // previous `childrenRect.width/height` read made the slot depend on the
    // child's laid-out width while the child's width was bound through the
    // slot: a binding loop that also squeezed long labels to zero. The slot
    // never takes more than 60% of the row, so a long localized label always
    // keeps a readable column.
    Item {
        id: controlSlot
        anchors.right: root.hasTrailing ? trailingRow.left : parent.right
        anchors.rightMargin: root.hasTrailing
                ? Theme.controls.settingsRow.controlGap
                : Theme.controls.settingsRow.paddingH
        anchors.verticalCenter: parent.verticalCenter

        readonly property Item controlItem: children.length > 0 ? children[0] : null
        readonly property real naturalWidth: {
            var item = controlSlot.controlItem;
            if (item === null)
                return 0;
            var implicit = item.implicitWidth;
            return implicit > 0 ? implicit : item.width;
        }
        readonly property real naturalHeight: {
            var item = controlSlot.controlItem;
            if (item === null)
                return 0;
            var implicit = item.implicitHeight;
            return implicit > 0 ? implicit : item.height;
        }
        width: Math.min(root.width * 0.6, Math.max(0, controlSlot.naturalWidth))
        height: Math.max(0, controlSlot.naturalHeight)

        onChildrenChanged: root.syncControlLabel()
        Component.onCompleted: root.syncControlLabel()
    }

    // The leading cell: a category tile, a plain glyph, or a pane-supplied
    // item. It is one slot so the label column keeps a single indent.
    Item {
        id: leadingSlot
        objectName: "settingsRowLeadingSlot"
        visible: root.leadingWidth > 0 || children.length > 0
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.settingsRow.paddingH
        anchors.verticalCenter: parent.verticalCenter
        width: root.leadingWidth
        height: root.leadingHeight

        SettingsCategoryIcon {
            id: leadingTileItem
            anchors.fill: parent
            visible: root.leadingTile !== null
            source: root.leadingTile ? root.leadingTile.source : ""
            gradientStart: root.leadingTile ? root.leadingTile.gradientStart
                                            : Theme.color.accent
            gradientEnd: root.leadingTile ? root.leadingTile.gradientEnd
                                          : Theme.color.accent
        }

        Icon {
            id: leadingIconItem
            objectName: "settingsRowLeadingIcon"
            anchors.centerIn: parent
            visible: root.leadingTile === null && root.leadingIcon.length > 0
            name: root.leadingIcon
            size: root.leadingIconSize
            color: root.leadingIconColor
        }
    }

    Column {
        id: labelColumn
        anchors.left: parent.left
        anchors.leftMargin: root.labelIndent >= 0 ? root.labelIndent
                          : (leadingSlot.visible
                             ? Theme.controls.settingsRow.paddingH
                               + leadingSlot.width + Theme.primitive.spacing.md
                             : Theme.controls.settingsRow.paddingH)
        anchors.right: controlSlot.left
        anchors.rightMargin: Theme.controls.settingsRow.controlGap
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.primitive.spacing.xxs

        Text {
            id: labelText
            width: parent.width
            text: root.label
            color: Theme.color.textPrimary
            font.pixelSize: Theme.controls.button.fontSize
            elide: Text.ElideRight
            horizontalAlignment: root.LayoutMirroring.enabled
                                 ? Text.AlignRight : Text.AlignLeft
        }
        Text {
            id: descriptionText
            width: parent.width
            visible: root.description.length > 0
            text: root.description
            color: Glass.increaseContrast ? Theme.color.textPrimary
                                          : Theme.color.textTertiary
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
            horizontalAlignment: root.LayoutMirroring.enabled
                                 ? Text.AlignRight : Text.AlignLeft
        }
        Text {
            id: errorTextItem
            objectName: "settingsRowError"
            width: parent.width
            visible: root.errorText.length > 0
            text: root.errorText
            color: Theme.color.danger
            font.pixelSize: Theme.primitive.font.sizeSm
            wrapMode: Text.WordWrap
            horizontalAlignment: root.LayoutMirroring.enabled
                                 ? Text.AlignRight : Text.AlignLeft
            Accessible.role: Accessible.AlertMessage
            Accessible.name: root.errorText
        }
    }

    // A disclosure row activates from anywhere on the row, not only the
    // chevron (T266). The trailing sub-controls keep their own hit target, so
    // a chevron/help/trailing press is never counted twice.
    TapHandler {
        onTapped: (eventPoint) => {
            if (!root.disclosure)
                return;
            if (root.tapInside(disclosureArea, eventPoint.position)
                    || root.tapInside(helpArea, eventPoint.position)
                    || root.tapInside(trailingRow, eventPoint.position))
                return;
            root.disclosureClicked();
        }
    }

    Rectangle {
        id: separatorRect
        visible: root.showSeparator && !root.isLastGroupRow
        anchors.left: parent.left
        anchors.leftMargin: Theme.controls.settingsRow.paddingH
        anchors.right: parent.right
        anchors.rightMargin: Theme.controls.settingsRow.paddingH
        anchors.bottom: parent.bottom
        height: Theme.controls.window.borderWidth
        color: Glass.increaseContrast ? Theme.color.textPrimary
                                      : Theme.color.separator
    }

    // Every row is a keyboard stop with a visible focus ring (T266).
    FocusRing {
        id: rowFocusRing
        target: root
        cornerRadius: Theme.controls.settingsGroup.radius
        shown: root.activeFocus
    }

    onLabelChanged: root.syncControlLabel()

    // Give a stock control the row label as its accessible name when the pane
    // did not set one (T266). Controls opt in with a `rowLabel` property.
    function syncControlLabel() {
        var children = controlSlot.children;
        for (var i = 0; i < children.length; ++i) {
            var child = children[i];
            if (child && child.rowLabel !== undefined)
                child.rowLabel = root.label;
        }
    }

    // True when a root-coordinate point falls inside a sub-control.
    function tapInside(item, point) {
        if (!item || !item.visible)
            return false;
        var local = item.mapFromItem(root, point.x, point.y);
        return local.x >= 0 && local.y >= 0
                && local.x <= item.width && local.y <= item.height;
    }

    // Move keyboard focus into the row's control when a deep link lands on it.
    function focusFirstControl() {
        var found = root.findFocusable(controlSlot);
        if (found !== null) {
            found.forceActiveFocus();
            return;
        }
        if (root.hasTrailing)
            root.forceActiveFocus();
        else
            root.forceActiveFocus();
    }

    function findFocusable(item) {
        if (item === null || item === undefined)
            return null;
        if (item.activeFocusOnTab === true && item.visible)
            return item;
        var children = item.children;
        for (var i = 0; i < children.length; ++i) {
            var found = root.findFocusable(children[i]);
            if (found !== null)
                return found;
        }
        return null;
    }

    Keys.onPressed: (event) => {
        if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter
                || event.key === Qt.Key_Space) {
            if (root.disclosure) {
                root.disclosureClicked();
                event.accepted = true;
            } else if (root.trailingAction.length > 0) {
                root.trailingActionClicked();
                event.accepted = true;
            }
        }
    }

    Accessible.role: Accessible.Grouping
    Accessible.name: root.label
    Accessible.description: {
        var text = root.description;
        if (root.errorText.length > 0)
            text = (text.length > 0 ? text + " " : "") + root.errorText;
        if (root.help.length > 0)
            text = (text.length > 0 ? text + " " : "") + root.help;
        return text;
    }
    Accessible.focusable: true
    Accessible.onPressAction: {
        if (root.disclosure)
            root.disclosureClicked();
        else if (root.trailingAction.length > 0)
            root.trailingActionClicked();
    }

    // T267: a validation failure is announced when it appears, not only
    // exposed as an alert role that a screen reader may never reach.
    onErrorTextChanged: {
        if (root.errorText.length > 0)
            Accessible.announce(root.errorText);
    }
}
