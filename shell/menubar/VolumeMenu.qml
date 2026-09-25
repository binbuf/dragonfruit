// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

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

    // Local slider state; it is re-seeded from the model whenever the host
    // pushes a new view, so a daemon change and a drag never disagree for
    // longer than one event.
    property real value: model.volume !== undefined ? model.volume : 0.0
    property bool muted: model.muted === true
    readonly property int percent: Math.round(root.value * 100)
    readonly property bool available: model.state === "available"
    readonly property var sinks: model.sinks !== undefined ? model.sinks : []
    readonly property var defaultSink: {
        for (var i = 0; i < root.sinks.length; ++i) {
            if (root.sinks[i] && root.sinks[i].default === true)
                return root.sinks[i];
        }
        return root.sinks.length > 0 ? root.sinks[0] : null;
    }
    readonly property string sinkLabel: root.defaultSink && root.defaultSink.description
                                        ? root.defaultSink.description : ""

    signal volumeSetRequested(double volume)
    signal muteToggleRequested()
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
        preferredWidth: 240
        accessibleRole: Accessible.PopupMenu
        accessibleName: qsTr("Sound")
        escapeCloses: true

        onOpened: {
            root.refreshRequested();
            if (root.parent && root.width > 0)
                popup.x = Math.min(popup.x,
                                   root.parent.width - popup.width
                                   - Theme.controls.menuBar.paddingH);
        }
        onClosed: root.closed()

        Keys.onUpPressed: (event) => {
            root.step(0.05);
            event.accepted = true;
        }
        Keys.onRightPressed: (event) => {
            root.step(0.05);
            event.accepted = true;
        }
        Keys.onDownPressed: (event) => {
            root.step(-0.05);
            event.accepted = true;
        }
        Keys.onLeftPressed: (event) => {
            root.step(-0.05);
            event.accepted = true;
        }
        Keys.onReturnPressed: (event) => {
            root.toggleMute();
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

            Button {
                objectName: "muteButton"
                width: parent.width
                text: root.muted ? qsTr("Unmute") : qsTr("Mute")
                variant: "secondary"
                onClicked: root.toggleMute()
            }
        }
    }
}