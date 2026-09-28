// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Wallpaper pane (T-09.3, restructured by T-18.2). Every control is a
// stock design-system component bound to the `Settings` singleton (never
// D-Bus directly) with the T-09.1b write-on-interaction / bind-to-`Settings.
// values` pattern, so a user pick and an external settingsd change converge
// without a restart.
//
// Three source rows (ADR 0055 / 0094):
//   * Featured — the provider's fetched Wikimedia Commons catalogue, rendered
//     from `Settings.providerItems`. While the first catalogue is downloading
//     (`Status=fetching`, no items) the row shows design-system skeletons.
//   * Built-in — the shipped original `Default.jpg` "Default" tile plus the
//     original gradient presets; no third-party attribution.
//   * Custom — the portal "Add Photo…" chooser (disabled when absent).
//
// The selection stays per Space (`wallpaper.showOnAllSpaces`): the shell
// forwards the chosen `wallpaper.source` to the active Space and the
// compositor renders it. Opening the pane asks the provider for an eager
// `Preload`; with no open pane the provider stays lazy.
Item {
    id: root

    readonly property string currentSource: Settings.values["wallpaper.source"] || ""
    readonly property string currentFit: Settings.values["wallpaper.fit"] || "fill"
    readonly property bool showOnAllSpaces:
        Settings.values["wallpaper.showOnAllSpaces"] !== false
    readonly property var presets: Settings.wallpaperPresets

    // The provider surface (T-18.1b). Absence is normal: status is empty and
    // the catalogue is empty, never an error.
    readonly property var providerItems: Settings.providerItems
    readonly property string providerStatus: Settings.providerStatus
    readonly property string builtinDefault: Settings.wallpaperBuiltinDefault

    readonly property var fitOptions: [
        { value: "fill", label: qsTr("Fill") },
        { value: "fit", label: qsTr("Fit") },
        { value: "stretch", label: qsTr("Stretch") },
        { value: "center", label: qsTr("Center") }
    ]

    // Featured is loading: the provider is fetching and nothing is cached yet.
    readonly property bool featuredFetching:
        root.providerStatus === "fetching" && root.providerItems.length === 0
    // Nothing to show for Featured (no provider, cold cache, offline, or an
    // error): the row shows a translated "available soon" note.
    readonly property bool featuredEmpty:
        root.providerItems.length === 0 && !root.featuredFetching
    readonly property string featuredMessage:
        qsTr("Featured pictures will be available soon.")

    // The provider catalogue, each entry gaining a display `name` and the
    // `fetched` marker the tile uses to show attribution.
    readonly property var featuredItems: {
        var out = [];
        for (var i = 0; i < root.providerItems.length; ++i) {
            var entry = root.providerItems[i];
            var item = {
                name: root.displayName(entry),
                source: entry.source,
                url: entry.url,
                title: entry.title || "",
                category: entry.category || "",
                artist: entry.artist || "",
                licenseShortName: entry.licenseShortName || "",
                licenseUrl: entry.licenseUrl || "",
                pageUrl: entry.pageUrl || "",
                fetched: true
            };
            out.push(item);
        }
        return out;
    }

    // The built-in row: the shipped default first, then the original
    // gradients. `Default` is named literally so the out-of-box tile reads
    // exactly as the reference and ADR 0094 describe.
    readonly property var builtinItems: {
        var out = [];
        if (root.builtinDefault.length > 0) {
            out.push({
                name: qsTr("Default"),
                source: root.builtinDefault,
                url: "file://" + root.builtinDefault,
                builtin: true
            });
        }
        for (var i = 0; i < root.presets.length; ++i) {
            var preset = root.presets[i];
            out.push({
                name: preset.name,
                source: preset.source,
                url: preset.url,
                builtin: true
            });
        }
        return out;
    }

    readonly property var allItems: root.featuredItems.concat(root.builtinItems)

    readonly property var currentItem: {
        for (var i = 0; i < root.allItems.length; ++i) {
            if (root.allItems[i].source === root.currentSource)
                return root.allItems[i];
        }
        return null;
    }

    readonly property string currentName: {
        if (root.currentItem)
            return root.currentItem.name;
        if (root.currentSource.length === 0)
            return qsTr("Default");
        var parts = root.currentSource.split("/");
        return parts[parts.length - 1];
    }

    readonly property string currentUrl:
        root.currentItem ? root.currentItem.url
                         : (root.currentSource.length > 0
                            ? "file://" + root.currentSource : "")

    // Attribution is shown only for a fetched (third-party) picture, never for
    // a built-in one and never for a user photo.
    readonly property bool hasAttribution:
        root.currentItem !== null && root.currentItem.fetched === true
        && (root.currentItem.artist.length > 0
            || root.currentItem.licenseShortName.length > 0)
    readonly property string currentLicenseUrl:
        root.currentItem ? (root.currentItem.licenseUrl || "") : ""
    readonly property string currentPageUrl:
        root.currentItem ? (root.currentItem.pageUrl || "") : ""

    // Every instantiated tile/placeholder, in declaration order, for the
    // headless tests.
    property var tileItems: []
    property var featuredTiles: []
    property var builtinTiles: []
    property var skeletonItems: []

    // Test surface (used by tst_settings_wallpaper.qml).
    property alias showOnAllToggle: showOnAllToggle
    property alias showOnAllRow: showOnAllRow
    property alias fitControl: fitControl
    property alias photoRow: photoRow
    property alias photoButton: photoButton
    property alias photoNotice: photoNotice
    property alias featuredGroup: featuredGroup
    property alias featuredGrid: featuredGrid
    property alias featuredMessageItem: featuredMessageItem
    property alias builtinGroup: builtinGroup
    property alias builtinGrid: builtinGrid
    property alias customGroup: customGroup
    property alias attributionColumn: attributionColumn
    property alias attributionArtist: attributionArtist
    property alias attributionLicense: attributionLicense
    property alias attributionPage: attributionPage

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    // Fill the detail pane slot (see AppearancePane).
    width: parent ? parent.width : implicitWidth

    // Opening the pane (the shell's Loader instantiates this body) requests an
    // eager provider load. The desktop already shows the shipped default, so
    // this never blocks; leaving the pane destroys this item and the provider
    // returns to idle.
    Component.onCompleted: Settings.preloadWallpapers()

    function displayName(entry) {
        if (entry.name)
            return entry.name;
        var title = entry.title || "";
        if (title.indexOf("File:") === 0)
            title = title.substring(5);
        var dot = title.lastIndexOf(".");
        if (dot > 0)
            title = title.substring(0, dot);
        return title.replace(/_/g, " ");
    }

    // The provider publishes the category as a lowercase slug; the accessible
    // name announces the human label ("Nature", "Water", …).
    function categoryLabel(slug) {
        if (!slug || slug.length === 0)
            return "";
        return slug.charAt(0).toUpperCase() + slug.slice(1);
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

    function openLicense() {
        if (root.currentLicenseUrl.length > 0)
            Qt.openUrlExternally(root.currentLicenseUrl);
    }

    function openPage() {
        if (root.currentPageUrl.length > 0)
            Qt.openUrlExternally(root.currentPageUrl);
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

            // Attribution for the current fetched picture: the artist (plain
            // text — the provider's HTML is stripped before it reaches QML),
            // the license short name linked to its URL, and the file page.
            // Built-in tiles and user photos show nothing here.
            Column {
                id: attributionColumn
                width: parent.width
                spacing: Theme.primitive.spacing.xxs
                visible: root.hasAttribution

                Text {
                    id: attributionArtist
                    width: parent.width
                    text: qsTr("Photo by %1").arg(root.currentItem
                                                  ? (root.currentItem.artist || "") : "")
                    color: Theme.color.textSecondary
                    elide: Text.ElideRight
                    textFormat: Text.PlainText
                    font.pixelSize: Theme.primitive.font.sizeSm
                }

                Row {
                    spacing: Theme.primitive.spacing.sm

                    Text {
                        id: attributionLicense
                        text: root.currentItem
                              ? (root.currentItem.licenseShortName || "") : ""
                        color: Theme.color.accent
                        textFormat: Text.PlainText
                        font.pixelSize: Theme.primitive.font.sizeSm
                        font.underline: licenseHover.hovered

                        HoverHandler { id: licenseHover }
                        TapHandler { onTapped: root.openLicense() }
                        Accessible.role: Accessible.Link
                        Accessible.name: qsTr("License: %1").arg(text)
                        Accessible.onPressAction: root.openLicense()
                    }

                    Text {
                        id: attributionPage
                        text: qsTr("View file page")
                        color: Theme.color.accent
                        textFormat: Text.PlainText
                        font.pixelSize: Theme.primitive.font.sizeSm
                        font.underline: pageHover.hovered

                        HoverHandler { id: pageHover }
                        TapHandler { onTapped: root.openPage() }
                        Accessible.role: Accessible.Link
                        Accessible.name: qsTr("View file page")
                        Accessible.onPressAction: root.openPage()
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

        // -- Featured (the fetched provider catalogue) -----------------------
        SettingsGroup {
            id: featuredGroup
            width: parent.width
            title: qsTr("Featured")

            // Downloading: grey shimmer placeholders. `active` follows the
            // provider state; reduced motion resolves to the static variant
            // inside Skeleton.
            Grid {
                id: featuredGrid
                width: parent.width
                visible: root.featuredFetching
                columns: 3
                columnSpacing: Theme.primitive.spacing.md
                rowSpacing: Theme.primitive.spacing.md

                Repeater {
                    model: root.featuredFetching ? 6 : 0
                    delegate: Skeleton {
                        id: skeletonTile
                        width: (featuredGrid.width - 2 * Theme.primitive.spacing.md) / 3
                        height: 72
                        active: root.featuredFetching
                        accessibleName: qsTr("Downloading wallpapers")

                        Component.onCompleted: root.skeletonItems =
                            root.skeletonItems.concat([skeletonTile])
                        Component.onDestruction: root.skeletonItems =
                            root.skeletonItems.filter(function(item) {
                                return item !== skeletonTile;
                            })
                    }
                }
            }

            Grid {
                width: parent.width
                visible: root.providerItems.length > 0
                columns: 3
                columnSpacing: Theme.primitive.spacing.md
                rowSpacing: Theme.primitive.spacing.md

                Repeater {
                    model: root.featuredItems
                    delegate: WallpaperTile { }
                }
            }

            Text {
                id: featuredMessageItem
                width: parent.width
                visible: root.featuredEmpty
                text: root.featuredMessage
                color: Theme.color.textSecondary
                wrapMode: Text.WordWrap
                font.pixelSize: Theme.primitive.font.sizeSm
            }
        }

        // -- Built-in (the shipped original default + original gradients) ----
        SettingsGroup {
            id: builtinGroup
            width: parent.width
            title: qsTr("Built-in")

            Grid {
                id: builtinGrid
                width: parent.width
                columns: 3
                columnSpacing: Theme.primitive.spacing.md
                rowSpacing: Theme.primitive.spacing.md

                Repeater {
                    model: root.builtinItems
                    delegate: WallpaperTile { }
                }
            }
        }

        // -- Custom (the portal chooser; degrades when absent) --------------
        SettingsGroup {
            id: customGroup
            width: parent.width
            title: qsTr("Custom")

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

    // One wallpaper tile. A tile is a radio choice: selecting it writes the
    // image path; the fit stays whatever the user last chose. Fetched tiles
    // carry the category/title/license in their accessible name; built-in
    // tiles are just their name.
    component WallpaperTile: Item {
        id: tile

        required property var modelData

        readonly property bool selected: root.currentSource === tile.modelData.source
        readonly property bool fetched: tile.modelData.fetched === true
        readonly property string accessibleLabel: {
            if (!tile.fetched)
                return tile.modelData.name;
            var parts = [];
            if (tile.modelData.category)
                parts.push(root.categoryLabel(tile.modelData.category));
            parts.push(tile.modelData.name);
            if (tile.modelData.licenseShortName)
                parts.push(tile.modelData.licenseShortName);
            return parts.join(", ");
        }

        width: (parent.width - 2 * Theme.primitive.spacing.md) / 3
        height: thumb.height + caption.implicitHeight + Theme.primitive.spacing.xs
        activeFocusOnTab: true

        function choose() {
            root.chooseSource(tile.modelData.source);
        }

        Component.onCompleted: {
            root.tileItems = root.tileItems.concat([tile]);
            if (tile.fetched)
                root.featuredTiles = root.featuredTiles.concat([tile]);
            else
                root.builtinTiles = root.builtinTiles.concat([tile]);
        }
        Component.onDestruction: {
            root.tileItems = root.tileItems.filter(function(item) { return item !== tile; });
            root.featuredTiles = root.featuredTiles.filter(function(item) { return item !== tile; });
            root.builtinTiles = root.builtinTiles.filter(function(item) { return item !== tile; });
        }

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
        Accessible.name: tile.accessibleLabel
        Accessible.checkable: true
        Accessible.checked: tile.selected
        Accessible.focusable: true
        Accessible.onPressAction: tile.choose()
    }
}