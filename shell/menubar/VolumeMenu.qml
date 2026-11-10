// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
pragma ComponentBehavior: Bound

// The volume status menu (T-07.5a): the default sink's volume slider and mute,
// drawn in the design-system popup. The model is the bridge host's `audio`
// view (services/system-status); the popover raises `volumeSetRequested` /
// `muteToggleRequested` and the shell forwards them to the host.
Item {
    id: root

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

    // Local slider state; it is re-seeded from the model whenever the host
    // pushes a new view, so a daemon change and a drag never disagree for
    // longer than one event.
    property real value: model.volume !== undefined ? model.volume : 0.0
    property bool muted: model.muted === true
    readonly property int percent: Math.round(root.value * 100)
    readonly property bool available: model.state === "available"
    readonly property var sinks: model.sinks !== undefined ? model.sinks : []
    readonly property string errorMessage: model.error !== undefined
        ? String(model.error) : ""
    readonly property var defaultSink: {
        for (var i = 0; i < root.sinks.length; ++i) {
            if (root.sinks[i] && root.sinks[i].default === true)
                return root.sinks[i];
        }
        return root.sinks.length > 0 ? root.sinks[0] : null;
    }
    readonly property string sinkLabel: root.defaultSink && root.defaultSink.description
                                        ? root.defaultSink.description : ""
    property int sinkHighlightIndex: -1

    signal volumeSetRequested(double volume)
    signal muteToggleRequested()
    signal sinkSelected(int id)
    signal settingsRequested()
    signal refreshRequested()
    // Raised when the popup closes for any reason, for the bar's state
    // (T-07.5b keyboard a11y).
    signal closed()

    onModelChanged: {
        if (model.volume !== undefined)
            root.value = model.volume;
        root.muted = model.muted === true;
    }

    // Set the linear fraction (0..=1) and apply it. Exposed so a test can
    // drive the slider without a pointer.
    function setFraction(fraction) {
        root.value = Math.max(0.0, Math.min(1.0, fraction));
        root.commit();
    }

    function toggleMute() {
        root.muteToggleRequested();
    }

    function selectSink(id) {
        if (!root.available)
            return;
        for (var i = 0; i < root.sinks.length; ++i) {
            if (Number(root.sinks[i].id) === Number(id)
                    && root.sinks[i].default === true)
                return;
        }
        root.sinkSelected(Number(id));
    }

    function moveSinkHighlight(delta) {
        var n = root.sinks.length;
        if (n <= 0) {
            root.sinkHighlightIndex = -1;
            return;
        }
        var i = root.sinkHighlightIndex < 0
            ? (delta > 0 ? -1 : 0) : root.sinkHighlightIndex;
        root.sinkHighlightIndex = ((i + delta) % n + n) % n;
    }

    function activateSinkHighlight() {
        if (root.sinkHighlightIndex >= 0
                && root.sinkHighlightIndex < root.sinks.length)
            root.selectSink(Number(root.sinks[root.sinkHighlightIndex].id));
        else
            root.toggleMute();
    }

    // Adjust by a keyboard step and apply it (T-07.5b).
    function step(delta) {
        root.value = Math.max(0.0, Math.min(1.0, root.value + delta));
        root.commit();
    }

    function commit() {
        if (!root.available)
            return;
        root.volumeSetRequested(root.value);
    }

    Popup {
        id: popup
        objectName: "volumePopup"
        anchorItem: root.anchorItem
        preferredWidth: 280
        accessibleRole: Accessible.PopupMenu
        accessibleName: qsTr("Sound")
        escapeCloses: true
        motionPreset: "menu"
        materialPreset: root.materialPreset
        materialBackdrop: root.materialBackdrop
        materialBlurActive: root.materialBlurActive
        materialTier: root.materialTier
        materialFallback: root.materialFallback

        onOpened: {
            root.sinkHighlightIndex = -1;
            root.refreshRequested();
            if (root.parent && root.width > 0)
                popup.x = Math.min(popup.x,
                                   root.parent.width - popup.width
                                   - Theme.controls.menuBar.paddingH);
        }
        onClosed: {
            root.sinkHighlightIndex = -1;
            root.closed();
        }

        Keys.onUpPressed: (event) => {
            root.moveSinkHighlight(-1);
            event.accepted = true;
        }
        Keys.onDownPressed: (event) => {
            root.moveSinkHighlight(1);
            event.accepted = true;
        }
        Keys.onRightPressed: (event) => {
            root.step(0.05);
            event.accepted = true;
        }
        Keys.onLeftPressed: (event) => {
            root.step(-0.05);
            event.accepted = true;
        }
        Keys.onReturnPressed: (event) => {
            root.activateSinkHighlight();
            event.accepted = true;
        }

        Column {
            width: parent.width
            spacing: Theme.primitive.spacing.sm

            // Header: "Sound" on the left, the output device on the right.
            Item {
                width: parent.width
                height: Theme.controls.button.fontSize + 4

                Text {
                    text: qsTr("Sound")
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    font.weight: Theme.primitive.font.weightMedium
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    text: root.sinkLabel
                    color: Theme.color.textTertiary
                    font.pixelSize: Theme.primitive.font.sizeSm
                    elide: Text.ElideRight
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.min(implicitWidth, parent.width / 2)
                }
            }

            // The host's error message is visible; an absent host hides the
            // whole item in the bar, so the popover only reaches this on a
            // present-but-unreadable view.
            Text {
                objectName: "volumeStateNote"
                visible: !root.available || root.model.state === "error"
                width: parent.width
                text: root.model.state === "error"
                      ? (root.errorMessage.length > 0 ? root.errorMessage
                                                      : qsTr("Sound is unavailable"))
                      : qsTr("Sound unavailable")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                wrapMode: Text.WordWrap
            }

            // Slider row: speaker glyph, the draggable slider, the percent.
            Item {
                width: parent.width
                height: 24

                StatusGlyph {
                    id: volumeGlyph
                    name: root.muted ? "volume-muted"
                                     : (root.model.glyph !== undefined
                                        ? root.model.glyph : "volume")
                    size: Theme.controls.menuBar.iconSize
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    id: percentText
                    objectName: "volumePercent"
                    text: root.muted ? qsTr("Muted")
                                     : qsTr("%1%").arg(root.percent)
                    color: Theme.color.textPrimary
                    font.pixelSize: Theme.controls.button.fontSize
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                }

                Item {
                    id: slider
                    objectName: "volumeSlider"
                    anchors.left: volumeGlyph.right
                    anchors.leftMargin: Theme.primitive.spacing.sm
                    anchors.right: percentText.left
                    anchors.rightMargin: Theme.primitive.spacing.sm
                    anchors.verticalCenter: parent.verticalCenter
                    height: 24

                    Rectangle {
                        id: track
                        width: parent.width
                        height: 4
                        radius: 2
                        color: Theme.color.controlFill
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Rectangle {
                        width: track.width * root.value
                        height: track.height
                        radius: track.radius
                        color: root.muted ? Theme.color.textTertiary
                                          : Theme.color.accent
                        anchors.left: track.left
                        anchors.verticalCenter: track.verticalCenter
                    }

                    Rectangle {
                        id: knob
                        width: 14
                        height: 14
                        radius: 7
                        color: Theme.color.controlKnob
                        border.width: Theme.controls.window.borderWidth
                        border.color: Theme.color.border
                        x: track.width * root.value - width / 2
                        anchors.verticalCenter: track.verticalCenter
                        antialiasing: true
                    }

                    TapHandler {
                        onTapped: (eventPoint) => {
                            root.value = Math.max(0.0, Math.min(1.0,
                                eventPoint.position.x / slider.width));
                            root.commit();
                        }
                    }
                    DragHandler {
                        target: null
                        onActiveChanged: if (!active) root.commit()
                        onTranslationChanged: {
                            var delta = activeTranslation.x / slider.width;
                            root.value = Math.max(0.0, Math.min(1.0, root.value + delta));
                        }
                    }
                }
            }

            Rectangle {
                visible: root.available
                width: parent.width
                height: Theme.controls.window.borderWidth
                color: Theme.color.separator
            }

            Text {
                visible: root.available
                width: parent.width
                text: qsTr("Output")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
                font.weight: Theme.primitive.font.weightMedium
            }

            Repeater {
                objectName: "volumeSinkRows"
                model: root.available ? root.sinks : []

                delegate: Item {
                    id: sinkRow
                    required property var modelData
                    required property int index
                    objectName: "volumeSink-" + sinkRow.modelData.id
                    readonly property bool isDefault: sinkRow.modelData.default === true
                    width: parent.width
                    height: Theme.controls.popup.rowHeight
                    enabled: !sinkRow.isDefault

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.controls.focusRing.radius
                        color: (sinkHover.hovered
                                || root.sinkHighlightIndex === sinkRow.index)
                               ? Theme.color.controlHover : "transparent"
                        antialiasing: true
                    }

                    Icon {
                        visible: sinkRow.isDefault
                        name: "check"
                        size: Theme.controls.button.iconSize
                        color: Theme.color.accent
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.primitive.spacing.xs
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Text {
                        text: sinkRow.modelData.description !== undefined
                              && String(sinkRow.modelData.description).length > 0
                              ? sinkRow.modelData.description : sinkRow.modelData.name
                        color: Theme.color.textPrimary
                        font.pixelSize: Theme.controls.button.fontSize
                        elide: Text.ElideRight
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.controls.button.iconSize
                                         + Theme.primitive.spacing.sm
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.primitive.spacing.xs
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    HoverHandler { id: sinkHover }

                    TapHandler {
                        onTapped: root.selectSink(Number(sinkRow.modelData.id))
                    }

                    Accessible.role: Accessible.MenuItem
                    Accessible.name: (sinkRow.modelData.description !== undefined
                                      && String(sinkRow.modelData.description).length > 0
                                      ? sinkRow.modelData.description : sinkRow.modelData.name)
                                     + (sinkRow.isDefault ? qsTr(", selected") : "")
                    Accessible.description: sinkRow.isDefault
                        ? qsTr("Already the output device") : ""
                    Accessible.onPressAction: root.selectSink(Number(sinkRow.modelData.id))
                }
            }

            Text {
                objectName: "volumeNoOutputs"
                visible: root.available && root.sinks.length === 0
                width: parent.width
                text: qsTr("No output devices")
                color: Theme.color.textTertiary
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Button {
                objectName: "muteButton"
                width: parent.width
                text: root.muted ? qsTr("Unmute") : qsTr("Mute")
                variant: "secondary"
                onClicked: root.toggleMute()
            }

            Button {
                objectName: "volumeSettingsButton"
                width: parent.width
                text: qsTr("Sound Settings…")
                variant: "secondary"
                accessibleName: qsTr("Open Sound Settings")
                onClicked: root.settingsRequested()
            }
        }
    }
}