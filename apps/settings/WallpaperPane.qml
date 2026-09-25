// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Wallpaper pane (T-09.3). Every control is a stock design-system
// component bound to the `Settings` singleton (never D-Bus directly) with the
// T-09.1b write-on-interaction / bind-to-`Settings.values` pattern, so a user
// pick and an external settingsd change converge without a restart.
//
// The selection is per Space: `wallpaper.showOnAllSpaces` applies it to every
// Space, otherwise the shell forwards it only to the active Space (the T-05
// per-Space wallpaper model; the compositor renders it). The built-in artwork
// is our own gradient set generated to stable files by `SettingsBridge`; the
// preview reads the exact file the compositor decodes. "Add Photo…" uses the
// xdg-desktop-portal FileChooser and disables cleanly when the portal is
// absent.
Item {
    id: root

    readonly property string currentSource: Settings.values["wallpaper.source"] || ""
    readonly property string currentFit: Settings.values["wallpaper.fit"] || "fill"
    readonly property bool showOnAllSpaces:
        Settings.values["wallpaper.showOnAllSpaces"] !== false
    readonly property var presets: Settings.wallpaperPresets

    readonly property var fitOptions: [
        { value: "fill", label: qsTr("Fill") },
        { value: "fit", label: qsTr("Fit") },
        { value: "stretch", label: qsTr("Stretch") },
        { value: "center", label: qsTr("Center") }
    ]

    // The built-in collections, in declaration order.
    readonly property var collections: {
        var out = [];
        for (var i = 0; i < root.presets.length; ++i) {
            var collection = root.presets[i].collection;
            if (out.indexOf(collection) < 0)
                out.push(collection);
        }
        return out;
    }

    readonly property string currentName: {
        for (var i = 0; i < root.presets.length; ++i) {
            if (root.presets[i].source === root.currentSource)
                return root.presets[i].name;
        }
        if (root.currentSource.length === 0)
            return qsTr("Default");
        var parts = root.currentSource.split("/");
        return parts[parts.length - 1];
    }

    readonly property string currentUrl: {
        for (var i = 0; i < root.presets.length; ++i) {
            if (root.presets[i].source === root.currentSource)
                return root.presets[i].url;
        }
        return root.currentSource.length > 0 ? "file://" + root.currentSource : "";
    }

    // Every instantiated tile, in declaration order, for the headless test.
    property var tileItems: []

    // Test surface (used by tst_settings_wallpaper.qml).
    property alias showOnAllToggle: showOnAllToggle
    property alias showOnAllRow: showOnAllRow
    property alias fitControl: fitControl
    property alias photoRow: photoRow
    property alias photoButton: photoButton
    property alias photoNotice: photoNotice

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    // Fill the detail pane slot (see AppearancePane).
    width: parent ? parent.width : implicitWidth

    function presetsIn(collection) {
        return root.presets.filter(function(preset) {
            return preset.collection === collection;
        });
    }

    function fitIndex() {
        for (var i = 0; i < root.fitOptions.length; ++i) {
            if (root.fitOptions[i].value === root.currentFit)
                return i;
        }
        return 0;
    }

    function chooseSource(source) {
        Settings.set("wallpaper.source", source);
    }

    function applyPhoto(path) {
        if (path && path.length > 0) {
            Settings.set("wallpaper.source", path);
            photoNotice.text = qsTr("Using %1").arg(path.split("/").pop());
        }
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.lg

        SettingsGroup {
            width: parent.width
            title: qsTr("Current wallpaper")

            // The hero preview: the exact image the compositor decodes, or the
            // solid-color placeholder when the Space keeps its default.
            Rectangle {
                id: preview
                width: parent.width
                height: 180
                radius: Theme.controls.settingsGroup.radius
                color: Theme.color.surfaceMuted
                clip: true

                Image {
                    anchors.fill: parent
                    source: root.currentUrl
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    Accessible.role: Accessible.Graphic
                    Accessible.name: root.currentName
                }

                Text {
                    anchors.centerIn: parent
                    visible: root.currentUrl.length === 0
                    text: qsTr("Solid color")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: namePlate.implicitHeight + 2 * Theme.primitive.spacing.sm
                    color: Theme.color.surfaceSunken
                    opacity: 0.85
                    visible: root.currentUrl.length > 0

                    Text {
                        id: namePlate
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.leftMargin: Theme.primitive.spacing.md
                        anchors.rightMargin: Theme.primitive.spacing.md
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.currentName
                        color: Theme.color.textPrimary
                        elide: Text.ElideRight
                        font.pixelSize: Theme.controls.button.fontSize
                        font.weight: Theme.primitive.font.weightMedium
                    }
                }
            }

            SettingsRow {
                id: showOnAllRow
                width: parent.width
                label: qsTr("Show on all Spaces")
                description: qsTr("Apply this wallpaper to every Space.")

                controlData: Toggle {
                    id: showOnAllToggle
                    text: ""
                    onToggled: (checked) => Settings.set("wallpaper.showOnAllSpaces", checked)
                }
            }

            SettingsRow {
                id: fitRow
                width: parent.width
                label: qsTr("Fit")
                showSeparator: false

                controlData: SegmentedControl {
                    id: fitControl
                    model: root.fitOptions.map(function(option) { return option.label; })
                    onActivated: (index) => Settings.set("wallpaper.fit",
                                                        root.fitOptions[index].value)
                }
            }
        }

        Repeater {
            model: root.collections

            SettingsGroup {
                required property string modelData

                width: content.width
                title: modelData

                Grid {
                    width: parent.width
                    columns: 3
                    columnSpacing: Theme.primitive.spacing.md
                    rowSpacing: Theme.primitive.spacing.md

                    Repeater {
                        model: root.presetsIn(modelData)
                        delegate: WallpaperTile { }
                    }
                }
            }
        }

        SettingsGroup {
            width: parent.width
            title: qsTr("Your Photos")

            SettingsRow {
                id: photoRow
                width: parent.width
                label: qsTr("Add Photo…")
                description: Settings.wallpaperChooserAvailable
                             ? qsTr("Choose an image file for this Space.")
                             : qsTr("No file chooser is available.")
                showSeparator: false

                controlData: Button {
                    id: photoButton
                    text: qsTr("Add Photo…")
                    accessibleName: qsTr("Add wallpaper photo")
                    enabled: Settings.wallpaperChooserAvailable
                    onClicked: Settings.chooseWallpaperPhoto()
                }
            }
        }

        Text {
            id: photoNotice
            width: parent.width
            text: ""
            visible: text.length > 0
            color: Theme.color.textSecondary
            elide: Text.ElideMiddle
            font.pixelSize: Theme.primitive.font.sizeSm
        }
    }

    // Two-way bindings: the controls write on interaction; these keep them in
    // step with `Settings.values` (a user edit and an external settingsd change
    // converge).
    Binding {
        target: showOnAllToggle
        property: "checked"
        value: root.showOnAllSpaces
    }
    Binding {
        target: fitControl
        property: "currentIndex"
        value: root.fitIndex()
    }

    Connections {
        target: Settings
        function onWallpaperPhotoChosen(path) { root.applyPhoto(path); }
    }

    // One built-in wallpaper. A tile is a radio choice: selecting it writes the
    // image path; the fit stays whatever the user last chose.
    component WallpaperTile: Item {
        id: tile

        required property var modelData

        readonly property bool selected: root.currentSource === tile.modelData.source

        width: (content.width - 2 * Theme.primitive.spacing.md) / 3
        height: thumb.height + caption.implicitHeight + Theme.primitive.spacing.xs
        activeFocusOnTab: true

        function choose() {
            root.chooseSource(tile.modelData.source);
        }

        Component.onCompleted: root.tileItems = root.tileItems.concat([tile])
        Component.onDestruction: root.tileItems = root.tileItems.filter(function(item) {
            return item !== tile;
        })

        Rectangle {
            id: thumb
            width: parent.width
            height: 72
            radius: Theme.controls.button.radius
            color: Theme.color.surfaceMuted
            border.width: tile.selected ? Theme.controls.focusRing.width
                                        : Theme.controls.window.borderWidth
            border.color: tile.selected ? Theme.color.accent : Theme.color.border
            clip: true
            antialiasing: true

            Image {
                anchors.fill: parent
                anchors.margins: 2
                source: tile.modelData.url
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
            }
        }

        Text {
            id: caption
            anchors.top: thumb.bottom
            anchors.topMargin: Theme.primitive.spacing.xs
            width: parent.width
            text: tile.modelData.name
            color: Theme.color.textPrimary
            elide: Text.ElideRight
            font.pixelSize: Theme.primitive.font.sizeSm
        }

        TapHandler {
            onTapped: tile.choose()
        }

        FocusRing {
            target: tile
            cornerRadius: Theme.controls.button.radius
            shown: tile.activeFocus
        }

        Keys.onPressed: (event) => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                    || event.key === Qt.Key_Enter) {
                tile.choose();
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.RadioButton
        Accessible.name: tile.modelData.name
        Accessible.checkable: true
        Accessible.checked: tile.selected
        Accessible.focusable: true
        Accessible.onPressAction: tile.choose()
    }
}