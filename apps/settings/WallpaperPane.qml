// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Settings

// The Wallpaper pane (T-09.3, restructured by T-18.2 and ADR 0240). Every
// control is a stock design-system component bound to the `Settings` singleton
// (never D-Bus directly) with the T-09.1b write-on-interaction / bind-to-
// `Settings.values` pattern, so a user pick and an external settingsd change
// converge without a restart.
//
// The layout follows the measured macOS Wallpaper pane: a flat hero (preview,
// current name, Fit, Show on all Workspaces, Screen Saver/Clock Appearance),
// then flat collection sections with four tiles per row and a Show All/Less
// toggle. The provider categories are grouped into rows; Abstract is first,
// Space is the NASA source. Built-in and Your Photos stay local sections.
//
// Apply is an explicit state machine: clicking a tile keeps the old preview,
// shows a spinner on the tile and in the hero, downloads a deferred full image
// when needed, then writes `wallpaper.source` and swaps the preview only once
// the image is ready. A failure keeps the previous wallpaper.
Item {
    id: root

    readonly property string currentSource: Settings.values["wallpaper.source"] || ""
    readonly property string currentFit: Settings.values["wallpaper.fit"] || "fill"
    readonly property bool showOnAllSpaces:
        Settings.values["wallpaper.showOnAllSpaces"] !== false
    readonly property var presets: Settings.wallpaperPresets

    // The provider surface (T-18.1b / ADR 0240). Absence is normal: status is
    // empty and the catalogue is empty, never an error.
    readonly property var providerItems: Settings.providerItems
    readonly property string providerStatus: Settings.providerStatus
    readonly property string builtinDefault: Settings.wallpaperBuiltinDefault
    readonly property string providerDefault: Settings.providerDefault

    readonly property var fitOptions: [
        { value: "fill", label: qsTr("Fill") },
        { value: "fit", label: qsTr("Fit") },
        { value: "stretch", label: qsTr("Stretch") },
        { value: "center", label: qsTr("Center") }
    ]

    // The display order. Abstract is the special first row (the macOS
    // "Dynamic Wallpapers" equivalent); Space is the NASA source.
    readonly property var categoryDefs: [
        { slug: "abstract", title: qsTr("Abstract") },
        { slug: "nature", title: qsTr("Nature") },
        { slug: "water", title: qsTr("Water") },
        { slug: "cityscapes", title: qsTr("Cityscapes") },
        { slug: "earth", title: qsTr("Earth") },
        { slug: "space", title: qsTr("Space") },
        { slug: "scenery", title: qsTr("Scenery") },
        { slug: "underwater", title: qsTr("Underwater") }
    ]

    // The provider is fetching. Empty categories show skeletons; a category
    // with cached items keeps showing them while the refresh runs.
    readonly property bool featuredFetching: root.providerStatus === "fetching"
    // Nothing to show for the online catalogue (no provider, cold cache,
    // offline, or an error): the note explains that absence is normal.
    readonly property bool featuredEmpty:
        root.providerItems.length === 0 && !root.featuredFetching
    readonly property string featuredMessage:
        qsTr("Featured pictures will be available soon.")

    // The provider catalogue, each entry gaining a display `name`, stable
    // `key`, grid thumbnail, and the best preview the hero can show.
    readonly property var featuredItems: {
        var out = [];
        for (var i = 0; i < root.providerItems.length; ++i) {
            var entry = root.providerItems[i];
            var thumb = entry.thumbUrl || entry.url || "";
            var preview = entry.previewUrl || thumb;
            var pageid = entry.pageid !== undefined ? entry.pageid : -1;
            out.push({
                key: pageid >= 0 ? "p" + pageid : "s" + (entry.source || ""),
                pageid: pageid,
                name: root.displayName(entry),
                source: entry.source || "",
                thumbUrl: thumb,
                previewUrl: preview,
                title: entry.title || "",
                category: entry.category || "",
                provider: entry.provider || "wikimedia",
                artist: entry.artist || "",
                licenseShortName: entry.licenseShortName || "",
                licenseUrl: entry.licenseUrl || "",
                pageUrl: entry.pageUrl || "",
                fetched: true
            });
        }
        return out;
    }

    // The built-in row: the shipped default first, then the original
    // gradients. `Default` is named literally so the out-of-box tile reads
    // exactly as the reference and ADR 0094 describe.
    readonly property var builtinItems: {
        var out = [];
        if (root.builtinDefault.length > 0) {
            var defaultUrl = root.fileUrl(root.builtinDefault);
            out.push({
                key: "s" + root.builtinDefault,
                name: qsTr("Default"),
                source: root.builtinDefault,
                thumbUrl: defaultUrl,
                previewUrl: defaultUrl,
                builtin: true,
                provider: ""
            });
        }
        for (var i = 0; i < root.presets.length; ++i) {
            var preset = root.presets[i];
            out.push({
                key: "s" + preset.source,
                name: preset.name,
                source: preset.source,
                thumbUrl: preset.url,
                previewUrl: preset.url,
                builtin: true,
                provider: ""
            });
        }
        return out;
    }

    readonly property var addPhotoItem: ({
        key: "add-photo",
        name: qsTr("Add Photo…"),
        addPhoto: true,
        enabled: Settings.wallpaperChooserAvailable,
        provider: ""
    })

    // Your Photos: the Add Photo tile first, then the current custom photo
    // (when the user's choice is not a provider or built-in item).
    readonly property var customItems: {
        var out = [root.addPhotoItem];
        if (root.currentSource.length > 0 && !root.isKnownSource(root.currentSource)) {
            var url = root.fileUrl(root.currentSource);
            out.push({
                key: "s" + root.currentSource,
                name: root.fileName(root.currentSource),
                source: root.currentSource,
                thumbUrl: url,
                previewUrl: url,
                custom: true,
                provider: ""
            });
        }
        return out;
    }

    readonly property var allItems: {
        var out = root.featuredItems.concat(root.builtinItems);
        for (var i = 0; i < root.customItems.length; ++i) {
            if (!root.customItems[i].addPhoto)
                out.push(root.customItems[i]);
        }
        return out;
    }

    // The effective source the shell applies: the user's choice, then the
    // shipped default, then the fetched fallback (ADR 0094). The pane uses it
    // for the selected tile and the hero, while `currentSource` remains the
    // raw user setting the tests and writes observe.
    readonly property string selectedSource: {
        if (root.currentSource.length > 0)
            return root.currentSource;
        if (root.builtinDefault.length > 0)
            return root.builtinDefault;
        return root.providerDefault || "";
    }

    readonly property var currentItem: {
        for (var i = 0; i < root.allItems.length; ++i) {
            var item = root.allItems[i];
            if (item.source && item.source === root.selectedSource)
                return item;
        }
        return null;
    }

    readonly property string currentName: {
        if (root.currentItem)
            return root.currentItem.name;
        if (root.selectedSource.length > 0)
            return root.fileName(root.selectedSource);
        return qsTr("Default");
    }

    readonly property string currentPreviewUrl: {
        if (root.currentItem && root.currentItem.previewUrl)
            return root.currentItem.previewUrl;
        if (root.selectedSource.length > 0)
            return root.fileUrl(root.selectedSource);
        return "";
    }
    // The old pane's name for the hero image URL; kept for tests and callers.
    readonly property string currentUrl: root.currentPreviewUrl

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
    readonly property string currentSourceLabel: {
        if (!root.currentItem)
            return "";
        if (root.currentItem.fetched)
            return root.currentItem.provider === "nasa"
                ? qsTr("NASA Image and Video Library")
                : qsTr("Wikimedia Commons Featured Pictures");
        if (root.currentItem.builtin)
            return qsTr("Built-in");
        if (root.currentItem.custom)
            return qsTr("Custom photo");
        return "";
    }

    // -- Apply state --------------------------------------------------------
    property bool pending: false
    property string pendingKey: ""
    property string pendingName: ""
    property int pendingPageId: -1
    property string pendingSource: ""
    property string pendingPreviewUrl: ""
    property string pendingPhase: "" // "downloading" | "applying"
    property string pendingBasePreviewUrl: ""
    property bool pendingImageReady: false
    property bool pendingMinElapsed: false
    property bool pendingWriteStarted: false
    property string pendingPhotoNotice: ""
    property string applyError: ""
    property string applyErrorKey: ""

    readonly property string heroPreviewUrl:
        root.pending ? root.pendingBasePreviewUrl : root.currentPreviewUrl
    readonly property string heroName:
        root.pending ? root.pendingName : root.currentName

    // Every instantiated tile/placeholder, in declaration order, for the
    // headless tests.
    property var tileItems: []
    property var featuredTiles: []
    property var builtinTiles: []
    property var customTiles: []
    property var skeletonItems: []
    property var sections: []
    property var addPhotoTile: null

    readonly property var photoButton: root.addPhotoTile
    readonly property var photoRow: root.yourPhotosSection
    readonly property string photoDescription:
        Settings.wallpaperChooserAvailable
            ? qsTr("Choose an image file for this workspace.")
            : qsTr("No file chooser is available.")

    // Test surface (used by tst_settings_wallpaper.qml and the absence suite).
    property alias hero: hero
    property alias heroNameText: heroNameText
    property alias heroSpinner: heroSpinner
    property alias heroPreview: preview
    property alias fitControl: fitControl
    property alias showOnAllToggle: showOnAllToggle
    property alias showOnAllRow: showOnAllRow
    property alias photoNotice: photoNotice
    property alias featuredGroup: providerSections
    property var featuredGrid:
        root.sections.length > 0 ? root.sections[0].grid : null
    property alias featuredMessageItem: featuredMessageItem
    property alias builtinGroup: builtinSection
    property alias customGroup: yourPhotosSection
    property alias attributionColumn: attributionColumn
    property alias attributionArtist: attributionArtist
    property alias attributionLicense: attributionLicense
    property alias attributionPage: attributionPage
    property alias screenSaverRow: screenSaverRow
    property alias screenSaverButton: screenSaverButton
    property alias clockAppearanceRow: clockAppearanceRow
    property alias clockAppearanceButton: clockAppearanceButton
    property alias retryButton: retryButton

    signal navigateRequested(string paneId)

    implicitWidth: 480
    implicitHeight: content.implicitHeight

    // Fill the detail pane slot (see AppearancePane).
    width: parent ? parent.width : implicitWidth

    // Opening the pane (the shell's Loader instantiates this body) requests an
    // eager provider load. The desktop already shows the shipped default, so
    // this never blocks; leaving the pane destroys this item and the provider
    // returns to idle.
    Component.onCompleted: Settings.preloadWallpapers()

    onCurrentSourceChanged: {
        // An external change wins over an in-flight apply, unless our own write
        // has already started (that change is the one we are committing).
        if (root.pending && !root.pendingWriteStarted)
            root.cancelPending();
    }

    function fileUrl(path) {
        return path && path.length > 0 ? "file://" + path : "";
    }

    function fileName(path) {
        if (!path)
            return "";
        var parts = path.split("/");
        return parts[parts.length - 1];
    }

    function displayName(entry) {
        if (entry.name)
            return entry.name;
        var title = entry.title || "";
        if (title.indexOf("File:") === 0)
            title = title.substring(5);
        if (/\.(jpg|jpeg|png|webp|gif|tif|tiff)$/i.test(title))
            title = title.substring(0, title.lastIndexOf("."));
        return title.replace(/_/g, " ");
    }

    function categoryTitle(slug) {
        for (var i = 0; i < root.categoryDefs.length; ++i) {
            if (root.categoryDefs[i].slug === slug)
                return root.categoryDefs[i].title;
        }
        if (!slug || slug.length === 0)
            return "";
        return slug.charAt(0).toUpperCase() + slug.slice(1);
    }

    function itemsFor(slug) {
        var out = [];
        for (var i = 0; i < root.featuredItems.length; ++i) {
            if (root.featuredItems[i].category === slug)
                out.push(root.featuredItems[i]);
        }
        return out;
    }

    function isKnownSource(source) {
        for (var i = 0; i < root.featuredItems.length; ++i) {
            if (root.featuredItems[i].source === source)
                return true;
        }
        for (var j = 0; j < root.builtinItems.length; ++j) {
            if (root.builtinItems[j].source === source)
                return true;
        }
        return false;
    }

    function itemForPageId(pageid) {
        for (var i = 0; i < root.featuredItems.length; ++i) {
            if (root.featuredItems[i].pageid === pageid)
                return root.featuredItems[i];
        }
        return null;
    }

    function sectionFor(slug) {
        for (var i = 0; i < root.sections.length; ++i) {
            if (root.sections[i].slug === slug)
                return root.sections[i];
        }
        return null;
    }

    function fitIndex() {
        for (var i = 0; i < root.fitOptions.length; ++i) {
            if (root.fitOptions[i].value === root.currentFit)
                return i;
        }
        return 0;
    }

    function chooseItem(item) {
        if (!item)
            return;
        if (item.addPhoto) {
            if (item.enabled !== false)
                Settings.chooseWallpaperPhoto();
            return;
        }
        if (root.pending && root.pendingKey === item.key)
            return;
        // Compare the raw user setting, not the effective source: clicking the
        // shipped Default while `wallpaper.source` is empty should pin it
        // explicitly rather than being a no-op.
        if (!root.pending && item.source && item.source === root.currentSource)
            return;
        root.pendingPhotoNotice = "";
        photoNotice.text = "";
        root.beginPending(item);
    }

    function loadPendingImage(url) {
        root.pendingImageReady = false;
        // Reset first so re-applying the same image still produces a load
        // status and cannot hang waiting for a Ready that already happened.
        pendingImage.source = "";
        pendingImage.source = url;
        if (pendingImage.status === Image.Ready) {
            root.pendingImageReady = true;
            root.maybeCommit();
        }
    }

    function beginPending(item) {
        root.pendingBasePreviewUrl = root.currentPreviewUrl;
        root.pending = true;
        root.pendingKey = item.key;
        root.pendingName = item.name;
        root.pendingPageId = item.pageid !== undefined ? item.pageid : -1;
        root.pendingSource = item.source || "";
        root.pendingPreviewUrl = item.previewUrl || "";
        root.pendingPhase = (item.fetched && root.pendingSource.length === 0)
            ? "downloading" : "applying";
        root.pendingImageReady = false;
        root.pendingMinElapsed = false;
        root.pendingWriteStarted = false;
        root.applyError = "";
        root.applyErrorKey = "";
        applyMinTimer.restart();

        if (root.pendingPhase === "downloading") {
            if (root.pendingPageId < 0) {
                root.failPending(qsTr("The wallpaper could not be downloaded."));
                return;
            }
            Settings.fetchWallpaperItem(root.pendingPageId);
        } else {
            if (root.pendingPreviewUrl.length === 0) {
                root.failPending(qsTr("The wallpaper image is unavailable."));
                return;
            }
            root.loadPendingImage(root.pendingPreviewUrl);
        }
    }

    function maybeCommit() {
        if (!root.pending || root.pendingPhase !== "applying"
                || !root.pendingImageReady || !root.pendingMinElapsed
                || root.pendingWriteStarted)
            return;
        root.pendingWriteStarted = true;
        if (!Settings.set("wallpaper.source", root.pendingSource)) {
            root.failPending(Settings.errorFor("wallpaper.source"));
            return;
        }
        if (root.pending)
            pendingWriteTimer.restart();
    }

    function finishPending() {
        root.pending = false;
        root.pendingKey = "";
        root.pendingPhase = "";
        root.pendingPageId = -1;
        root.pendingImageReady = false;
        root.pendingMinElapsed = false;
        root.pendingWriteStarted = false;
        applyMinTimer.stop();
        pendingWriteTimer.stop();
        pendingImage.source = "";
        if (root.pendingPhotoNotice.length > 0) {
            photoNotice.text = root.pendingPhotoNotice;
            root.pendingPhotoNotice = "";
        }
    }

    function cancelPending() {
        root.pending = false;
        root.pendingKey = "";
        root.pendingPhase = "";
        root.pendingPageId = -1;
        root.pendingImageReady = false;
        root.pendingMinElapsed = false;
        root.pendingWriteStarted = false;
        root.pendingPhotoNotice = "";
        applyMinTimer.stop();
        pendingWriteTimer.stop();
        pendingImage.source = "";
    }

    function failPending(detail) {
        if (detail && detail.length > 0)
            console.warn("wallpaper apply failed: " + detail);
        root.applyError = root.pendingPhase === "downloading"
            ? qsTr("The wallpaper could not be downloaded.")
            : qsTr("Could not apply this wallpaper.");
        root.applyErrorKey = root.pendingKey;
        root.pending = false;
        root.pendingKey = "";
        root.pendingPhase = "";
        root.pendingPageId = -1;
        root.pendingImageReady = false;
        root.pendingMinElapsed = false;
        root.pendingWriteStarted = false;
        root.pendingPhotoNotice = "";
        applyMinTimer.stop();
        pendingWriteTimer.stop();
        pendingImage.source = "";
    }

    function onFetchFinished(pageid, ok, message) {
        if (!root.pending || root.pendingPageId !== pageid)
            return;
        if (!ok) {
            root.failPending(message);
            return;
        }
        var item = root.itemForPageId(pageid);
        if (!item || item.source.length === 0) {
            root.failPending(qsTr("The wallpaper could not be downloaded."));
            return;
        }
        root.pendingSource = item.source;
        root.pendingPreviewUrl = item.previewUrl;
        root.pendingPhase = "applying";
        root.loadPendingImage(root.pendingPreviewUrl);
    }

    function onWriteFinished(key, value, ok, message) {
        if (key !== "wallpaper.source" || !root.pending
                || !root.pendingWriteStarted)
            return;
        if (!ok) {
            root.failPending(message);
            return;
        }
        root.finishPending();
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
        if (!path || path.length === 0)
            return;
        var item = {
            key: "s" + path,
            name: root.fileName(path),
            source: path,
            thumbUrl: root.fileUrl(path),
            previewUrl: root.fileUrl(path),
            custom: true,
            provider: ""
        };
        root.chooseItem(item);
        if (root.pending)
            root.pendingPhotoNotice = qsTr("Using %1").arg(item.name);
    }

    Timer {
        id: applyMinTimer
        interval: Theme.controls.wallpaper.applyMinMs
        repeat: false
        onTriggered: {
            root.pendingMinElapsed = true;
            root.maybeCommit();
        }
    }

    Timer {
        id: pendingWriteTimer
        interval: 2000
        repeat: false
        onTriggered: {
            if (root.pending && root.pendingWriteStarted)
                root.finishPending();
        }
    }

    // The pending preview is loaded offscreen so the hero keeps the previous
    // image until the new one is ready (the macOS behavior).
    Image {
        id: pendingImage
        visible: false
        asynchronous: true
        source: ""
        onStatusChanged: {
            if (!root.pending || root.pendingPhase !== "applying")
                return;
            if (status === Image.Ready) {
                root.pendingImageReady = true;
                root.maybeCommit();
            } else if (status === Image.Error) {
                root.failPending(qsTr("The wallpaper image could not be loaded."));
            }
        }
    }

    Column {
        id: content
        width: root.width
        spacing: Theme.primitive.spacing.xl

        // -- Current wallpaper hero ----------------------------------------
        Row {
            id: hero
            width: parent.width
            spacing: Theme.primitive.spacing.lg

            Rectangle {
                id: preview
                width: Theme.controls.wallpaper.heroWidth
                height: Theme.controls.wallpaper.heroHeight
                radius: Theme.controls.settingsGroup.radius
                color: Theme.color.surfaceMuted
                clip: true
                antialiasing: true
                property string settingsId: "current"

                Image {
                    anchors.fill: parent
                    source: root.heroPreviewUrl
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    Accessible.role: Accessible.Graphic
                    Accessible.name: root.heroName
                }

                Text {
                    anchors.centerIn: parent
                    visible: root.heroPreviewUrl.length === 0
                    text: qsTr("Solid color")
                    color: Theme.color.textSecondary
                    font.pixelSize: Theme.controls.button.fontSize
                }
            }

            Column {
                width: Math.max(0, hero.width - preview.width - hero.spacing)
                spacing: Theme.primitive.spacing.sm

                Row {
                    width: parent.width
                    spacing: Theme.primitive.spacing.sm

                    Text {
                        id: heroNameText
                        width: Math.max(0, parent.width
                                        - (root.pending ? heroSpinnerSlot.width
                                                        : fitControl.implicitWidth)
                                        - parent.spacing)
                        height: fitControl.implicitHeight
                        text: root.heroName
                        color: Theme.color.textPrimary
                        elide: Text.ElideRight
                        verticalAlignment: Text.AlignVCenter
                        font.pixelSize: Theme.primitive.font.sizeLg
                        font.weight: Theme.primitive.font.weightSemibold
                    }

                    Select {
                        id: fitControl
                        visible: !root.pending
                        model: root.fitOptions.map(function(option) { return option.label; })
                        accessibleName: qsTr("Fit")
                        property string settingsId: "fit"
                        onActivated: (index) => Settings.set("wallpaper.fit",
                                                            root.fitOptions[index].value)
                        function focusFirstControl() { forceActiveFocus(); }
                    }

                    Item {
                        id: heroSpinnerSlot
                        visible: root.pending
                        width: Theme.controls.spinner.size
                        height: fitControl.implicitHeight

                        Spinner {
                            id: heroSpinner
                            anchors.centerIn: parent
                            visible: root.pending
                            running: root.pending
                            size: Theme.controls.spinner.size
                            accessibleName: root.pendingPhase === "downloading"
                                ? qsTr("Downloading %1").arg(root.pendingName)
                                : qsTr("Applying %1").arg(root.pendingName)
                        }
                    }
                }

                Text {
                    width: parent.width
                    visible: text.length > 0 && !root.pending
                    text: root.currentSourceLabel
                    color: Theme.color.textTertiary
                    elide: Text.ElideRight
                    font.pixelSize: Theme.primitive.font.sizeSm
                }

                Text {
                    width: parent.width
                    visible: root.pending
                    text: root.pendingPhase === "downloading"
                          ? qsTr("Downloading…") : qsTr("Applying…")
                    color: Theme.color.textSecondary
                    elide: Text.ElideRight
                    font.pixelSize: Theme.primitive.font.sizeSm
                }

                Column {
                    id: attributionColumn
                    width: parent.width
                    spacing: Theme.primitive.spacing.xxs
                    visible: root.hasAttribution && !root.pending

                    Text {
                        id: attributionArtist
                        width: parent.width
                        text: root.currentItem && root.currentItem.provider === "nasa"
                              ? qsTr("Image credit: %1").arg(root.currentItem.artist || "")
                              : qsTr("Photo by %1").arg(root.currentItem
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
                            text: root.currentItem && root.currentItem.provider === "nasa"
                                  ? qsTr("View on NASA") : qsTr("View file page")
                            color: Theme.color.accent
                            textFormat: Text.PlainText
                            font.pixelSize: Theme.primitive.font.sizeSm
                            font.underline: pageHover.hovered

                            HoverHandler { id: pageHover }
                            TapHandler { onTapped: root.openPage() }
                            Accessible.role: Accessible.Link
                            Accessible.name: text
                            Accessible.onPressAction: root.openPage()
                        }
                    }
                }

                Text {
                    width: parent.width
                    visible: root.applyError.length > 0
                    text: root.applyError
                    color: Theme.color.danger
                    wrapMode: Text.WordWrap
                    font.pixelSize: Theme.primitive.font.sizeSm
                }

                Row {
                    id: showOnAllRow
                    width: parent.width
                    spacing: Theme.primitive.spacing.sm
                    property string settingsId: "showAll"
                    property alias control: showOnAllToggle

                    function focusFirstControl() { showOnAllToggle.forceActiveFocus(); }

                    Text {
                        width: Math.max(0, parent.width - showOnAllToggle.implicitWidth
                                        - parent.spacing)
                        text: qsTr("Show on all Workspaces")
                        color: Theme.color.textPrimary
                        elide: Text.ElideRight
                        anchors.verticalCenter: parent.verticalCenter
                        font.pixelSize: Theme.controls.button.fontSize
                    }

                    Toggle {
                        id: showOnAllToggle
                        text: ""
                        accessibleName: qsTr("Show on all Workspaces")
                        anchors.verticalCenter: parent.verticalCenter
                        onToggled: (checked) => Settings.set("wallpaper.showOnAllSpaces",
                                                            checked)
                    }
                }

                Row {
                    width: parent.width
                    spacing: Theme.primitive.spacing.sm

                    Item {
                        id: screenSaverRow
                        width: screenSaverButton.implicitWidth
                        height: screenSaverButton.implicitHeight
                        property string settingsId: "screenSaver"

                        function focusFirstControl() { screenSaverButton.forceActiveFocus(); }

                        Button {
                            id: screenSaverButton
                            anchors.fill: parent
                            text: qsTr("Screen Saver…")
                            accessibleName: qsTr("Open Screen Saver settings")
                            onClicked: root.navigateRequested("screen-saver")
                        }
                    }

                    Item {
                        id: clockAppearanceRow
                        width: clockAppearanceButton.implicitWidth
                        height: clockAppearanceButton.implicitHeight
                        property string settingsId: "clock"

                        function focusFirstControl() { clockAppearanceButton.forceActiveFocus(); }

                        Button {
                            id: clockAppearanceButton
                            anchors.fill: parent
                            text: qsTr("Clock Appearance…")
                            accessibleName: qsTr("Open clock appearance settings")
                            onClicked: root.navigateRequested("screen-saver")
                        }
                    }
                }
            }
        }

        // -- Online catalogue status / empty note ---------------------------
        Row {
            id: providerStatusNote
            width: parent.width
            spacing: Theme.primitive.spacing.sm
            visible: root.featuredEmpty

            Text {
                id: featuredMessageItem
                width: Math.max(0, parent.width
                                - (retryButton.visible
                                   ? retryButton.width + parent.spacing : 0))
                text: root.featuredMessage
                color: Theme.color.textSecondary
                wrapMode: Text.WordWrap
                font.pixelSize: Theme.primitive.font.sizeSm
            }

            Button {
                id: retryButton
                text: qsTr("Try Again")
                visible: Settings.providerStatus.length > 0
                enabled: Settings.providerStatus !== "fetching"
                onClicked: Settings.refreshWallpapers()
            }
        }

        // -- Provider category rows ----------------------------------------
        Column {
            id: providerSections
            width: parent.width
            spacing: Theme.primitive.spacing.xl
            visible: root.providerItems.length > 0 || root.featuredFetching

            Item {
                id: providerStatusRow
                width: parent.width
                height: Math.max(statusSpinner.height, statusText.implicitHeight,
                                 statusRetry.implicitHeight)
                visible: root.providerItems.length > 0
                         && (root.providerStatus === "fetching"
                             || root.providerStatus === "offline")

                Spinner {
                    id: statusSpinner
                    visible: root.providerStatus === "fetching"
                    running: visible
                    size: Theme.controls.spinner.size
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    accessibleName: qsTr("Refreshing wallpapers")
                }

                Text {
                    id: statusText
                    anchors.left: statusSpinner.visible
                                  ? statusSpinner.right : parent.left
                    anchors.leftMargin: statusSpinner.visible
                                       ? Theme.primitive.spacing.sm : 0
                    anchors.right: statusRetry.visible
                                   ? statusRetry.left : parent.right
                    anchors.rightMargin: statusRetry.visible
                                         ? Theme.primitive.spacing.sm : 0
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.providerStatus === "fetching"
                          ? qsTr("Refreshing wallpapers…")
                          : qsTr("Showing cached wallpapers.")
                    color: Theme.color.textSecondary
                    elide: Text.ElideRight
                    font.pixelSize: Theme.primitive.font.sizeSm
                }

                Button {
                    id: statusRetry
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    visible: root.providerStatus === "offline"
                    text: qsTr("Try Again")
                    onClicked: Settings.refreshWallpapers()
                }
            }

            Repeater {
                model: root.categoryDefs
                delegate: WallpaperSection {
                    width: providerSections.width
                    slug: modelData.slug
                    title: modelData.title
                    settingsId: modelData.slug
                    items: root.itemsFor(modelData.slug)
                    fetching: root.featuredFetching && items.length === 0
                }
            }
        }

        // -- Built-in (the shipped original default + original gradients) ----
        WallpaperSection {
            id: builtinSection
            width: parent.width
            title: qsTr("Built-in")
            settingsId: "builtin"
            items: root.builtinItems
            expanded: true
        }

        // -- Your Photos (the portal chooser + the current custom photo) -----
        WallpaperSection {
            id: yourPhotosSection
            width: parent.width
            title: qsTr("Your Photos")
            settingsId: "photos"
            items: root.customItems
            expanded: true
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

        Text {
            width: parent.width
            visible: !Settings.wallpaperChooserAvailable
            text: root.photoDescription
            color: Theme.color.textSecondary
            wrapMode: Text.WordWrap
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
        function onWallpaperFetchFinished(pageid, ok, message) {
            root.onFetchFinished(pageid, ok, message);
        }
        function onWriteFinished(key, value, ok, message) {
            root.onWriteFinished(key, ok, message);
        }
    }

    // One titled collection row. The header carries Show All (N) / Show Less;
    // the grid is four responsive 3:2 tiles. A fetching section shows
    // skeletons instead of stale empty content.
    component WallpaperSection: Column {
        id: section

        property string slug: ""
        property string title: ""
        property var items: []
        property bool fetching: false
        property bool expanded: false
        property string settingsId: ""

        property alias grid: grid
        property alias tileRepeater: tileRepeater
        property alias showAllButton: showAllButton

        readonly property int columns: Theme.controls.wallpaper.columns
        readonly property real tileGap: Theme.controls.wallpaper.tileGap
        readonly property real tileWidth: Math.max(
            0, (section.width - (section.columns - 1) * section.tileGap)
               / section.columns)
        readonly property real thumbHeight: Math.round(
            section.tileWidth / Theme.controls.wallpaper.tileAspect)
        readonly property real captionHeight:
            Theme.primitive.font.sizeSm + 2 * Theme.primitive.spacing.xxs
        readonly property real tileHeight:
            section.thumbHeight + Theme.primitive.spacing.xs + section.captionHeight
        readonly property var visibleItems:
            section.fetching
            ? []
            : (section.expanded ? section.items
                                : section.items.slice(0, section.columns))
        readonly property int skeletonCount: section.fetching ? section.columns : 0
        readonly property bool showAllVisible:
            !section.fetching && section.items.length > section.columns
        readonly property real showAllWidth:
            showAllButton.visible ? showAllButton.implicitWidth : 0

        width: parent ? parent.width : 0
        spacing: Theme.primitive.spacing.sm
        visible: section.fetching || section.items.length > 0

        Accessible.role: Accessible.Grouping
        Accessible.name: section.title

        function toggleExpanded() {
            section.expanded = !section.expanded;
        }

        function focusFirstControl() {
            if (tileRepeater.count > 0)
                tileRepeater.itemAt(0).forceActiveFocus();
        }

        Component.onCompleted: root.sections = root.sections.concat([section])
        Component.onDestruction: root.sections = root.sections.filter(
            function(item) { return item !== section; })

        Item {
            width: parent.width
            height: Math.max(titleText.implicitHeight, showAllButton.implicitHeight)

            Text {
                id: titleText
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                width: Math.max(0, parent.width - section.showAllWidth
                                - (section.showAllWidth > 0
                                   ? Theme.primitive.spacing.sm : 0))
                text: section.title
                color: Theme.color.textSecondary
                elide: Text.ElideRight
                font.pixelSize: Theme.primitive.font.sizeMd
                font.weight: Theme.primitive.font.weightMedium
            }

            Button {
                id: showAllButton
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                visible: section.showAllVisible
                variant: "ghost"
                text: section.expanded
                      ? qsTr("Show Less")
                      : qsTr("Show All (%1)").arg(section.items.length)
                accessibleName: section.expanded
                    ? qsTr("Show less %1 wallpapers").arg(section.title)
                    : qsTr("Show all %1 wallpapers").arg(section.title)
                onClicked: section.toggleExpanded()
            }
        }

        Grid {
            id: grid
            width: parent.width
            visible: !section.fetching
            columns: section.columns
            columnSpacing: section.tileGap
            rowSpacing: Theme.primitive.spacing.md

            Repeater {
                id: tileRepeater
                model: section.visibleItems
                delegate: WallpaperTile {
                    width: section.tileWidth
                    modelData: modelData
                    tileIndex: index
                    tileRepeater: tileRepeater
                    sectionColumns: section.columns
                }
            }
        }

        Grid {
            width: parent.width
            visible: section.fetching
            columns: section.columns
            columnSpacing: section.tileGap
            rowSpacing: Theme.primitive.spacing.md

            Repeater {
                model: section.skeletonCount
                delegate: Skeleton {
                    id: skeletonTile
                    width: section.tileWidth
                    height: section.tileHeight
                    active: section.fetching
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
    }

    // One wallpaper tile. A tile is a radio choice; the Add Photo tile is a
    // button. Selecting writes the full-resolution path through the apply state
    // machine; a deferred item downloads first.
    component WallpaperTile: Item {
        id: tile

        required property var modelData
        property int tileIndex: -1
        property var tileRepeater: null
        property int sectionColumns: Theme.controls.wallpaper.columns

        readonly property bool isAddPhoto: tile.modelData.addPhoto === true
        readonly property bool selected:
            !tile.isAddPhoto && tile.modelData.source !== undefined
            && tile.modelData.source.length > 0
            && tile.modelData.source === root.selectedSource
        readonly property bool pending:
            root.pending && root.pendingKey === tile.modelData.key
        readonly property string pendingPhase:
            tile.pending ? root.pendingPhase : ""
        readonly property bool highlighted:
            root.pending ? tile.pending : tile.selected
        readonly property string errorText:
            root.applyErrorKey === tile.modelData.key ? root.applyError : ""
        readonly property bool hovered: hoverHandler.hovered
        readonly property string accessibleLabel: {
            if (tile.isAddPhoto)
                return qsTr("Add Photo…");
            var parts = [];
            if (tile.modelData.fetched && tile.modelData.category)
                parts.push(root.categoryTitle(tile.modelData.category));
            parts.push(tile.modelData.name);
            if (tile.modelData.fetched && tile.modelData.licenseShortName)
                parts.push(tile.modelData.licenseShortName);
            if (tile.pending)
                parts.push(tile.pendingPhase === "downloading"
                           ? qsTr("Downloading") : qsTr("Applying"));
            return parts.join(", ");
        }
        readonly property string accessibleDescription: {
            if (tile.errorText.length > 0)
                return tile.errorText;
            if (tile.pending)
                return tile.pendingPhase === "downloading"
                    ? qsTr("Downloading") : qsTr("Applying");
            if (tile.isAddPhoto && tile.modelData.enabled === false)
                return qsTr("No file chooser is available.");
            return "";
        }

        width: 0 // the section sets the responsive width
        height: thumb.height + Theme.primitive.spacing.xs + caption.height
        enabled: !tile.isAddPhoto || tile.modelData.enabled !== false
        activeFocusOnTab: true
        opacity: enabled ? 1 : 0.45

        function choose() {
            if (!tile.enabled)
                return;
            if (tile.isAddPhoto) {
                Settings.chooseWallpaperPhoto();
                return;
            }
            root.chooseItem(tile.modelData);
        }

        Component.onCompleted: {
            root.tileItems = root.tileItems.concat([tile]);
            if (tile.isAddPhoto) {
                root.customTiles = root.customTiles.concat([tile]);
                root.addPhotoTile = tile;
            } else if (tile.modelData.fetched) {
                root.featuredTiles = root.featuredTiles.concat([tile]);
            } else if (tile.modelData.builtin) {
                root.builtinTiles = root.builtinTiles.concat([tile]);
            } else if (tile.modelData.custom) {
                root.customTiles = root.customTiles.concat([tile]);
            }
        }
        Component.onDestruction: {
            root.tileItems = root.tileItems.filter(function(item) {
                return item !== tile;
            });
            root.featuredTiles = root.featuredTiles.filter(function(item) {
                return item !== tile;
            });
            root.builtinTiles = root.builtinTiles.filter(function(item) {
                return item !== tile;
            });
            root.customTiles = root.customTiles.filter(function(item) {
                return item !== tile;
            });
            if (root.addPhotoTile === tile)
                root.addPhotoTile = null;
        }

        Rectangle {
            id: thumb
            width: parent.width
            height: Math.round(width / Theme.controls.wallpaper.tileAspect)
            radius: Theme.controls.button.radius
            color: Theme.color.surfaceMuted
            border.width: tile.highlighted ? Theme.controls.focusRing.width
                                           : Theme.controls.window.borderWidth
            border.color: tile.highlighted ? Theme.color.accent : Theme.color.border
            clip: true
            antialiasing: true

            Image {
                anchors.fill: parent
                anchors.margins: 2
                source: tile.isAddPhoto ? "" : (tile.modelData.thumbUrl || "")
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                visible: !tile.isAddPhoto
            }

            Icon {
                visible: tile.isAddPhoto
                anchors.centerIn: parent
                name: "wallpaper"
                size: Theme.primitive.spacing.xxl
                color: Theme.color.textTertiary
            }

            Rectangle {
                anchors.fill: parent
                radius: thumb.radius
                color: Theme.color.controlHover
                opacity: tile.hovered && !tile.pending ? 0.16 : 0
            }

            Rectangle {
                anchors.fill: parent
                radius: thumb.radius
                color: Qt.rgba(0, 0, 0, 0.38)
                visible: tile.pending
            }

            Spinner {
                anchors.centerIn: parent
                visible: tile.pending
                running: tile.pending
                size: Theme.primitive.spacing.xl
                accessibleName: tile.pendingPhase === "downloading"
                    ? qsTr("Downloading %1").arg(tile.modelData.name)
                    : qsTr("Applying %1").arg(tile.modelData.name)
            }

            Rectangle {
                visible: tile.selected && !tile.pending
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: Theme.primitive.spacing.xs
                width: Theme.primitive.spacing.md
                height: width
                radius: width / 2
                color: Theme.color.accent

                Icon {
                    anchors.centerIn: parent
                    name: "check"
                    size: Theme.primitive.font.sizeSm
                    color: Theme.color.accentContent
                }
            }

            Rectangle {
                visible: tile.errorText.length > 0
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.margins: Theme.primitive.spacing.xs
                width: Theme.primitive.spacing.md
                height: width
                radius: width / 2
                color: Theme.color.danger

                Text {
                    anchors.centerIn: parent
                    text: "!" // df-allow-untranslated: error badge glyph
                    color: Theme.color.accentContent
                    font.pixelSize: Theme.primitive.font.sizeSm
                    font.weight: Theme.primitive.font.weightBold
                }
            }
        }

        Text {
            id: caption
            anchors.top: thumb.bottom
            anchors.topMargin: Theme.primitive.spacing.xs
            width: parent.width
            height: Theme.primitive.font.sizeSm + 2 * Theme.primitive.spacing.xxs
            text: tile.pending
                  ? (tile.pendingPhase === "downloading"
                     ? qsTr("Downloading…") : qsTr("Applying…"))
                  : tile.modelData.name
            color: tile.pending ? Theme.color.textSecondary : Theme.color.textPrimary
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
            font.pixelSize: Theme.primitive.font.sizeSm
        }

        HoverHandler { id: hoverHandler }

        TapHandler {
            enabled: tile.enabled
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

        KeyNavigation.left: tile.tileIndex > 0 && tile.tileRepeater
                            ? tile.tileRepeater.itemAt(tile.tileIndex - 1) : null
        KeyNavigation.right: tile.tileRepeater
                             && tile.tileIndex + 1 < tile.tileRepeater.count
                             ? tile.tileRepeater.itemAt(tile.tileIndex + 1) : null
        KeyNavigation.up: tile.tileRepeater
                          && tile.tileIndex >= tile.sectionColumns
                          ? tile.tileRepeater.itemAt(tile.tileIndex
                                                     - tile.sectionColumns) : null
        KeyNavigation.down: tile.tileRepeater
                            && tile.tileIndex + tile.sectionColumns
                               < tile.tileRepeater.count
                            ? tile.tileRepeater.itemAt(tile.tileIndex
                                                       + tile.sectionColumns) : null

        Accessible.role: tile.isAddPhoto ? Accessible.Button : Accessible.RadioButton
        Accessible.name: tile.accessibleLabel
        Accessible.description: tile.accessibleDescription
        Accessible.checkable: !tile.isAddPhoto
        Accessible.checked: tile.selected
        Accessible.focusable: true
        Accessible.onPressAction: tile.choose()
    }
}
