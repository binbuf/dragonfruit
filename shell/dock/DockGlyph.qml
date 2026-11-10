// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.VectorImage
import QtQuick.Window
import Dragonfruit

// Dock entry artwork (T-10). The shapes:
//   * an app tile — a real themed icon resolved by app-index (T-14.1a) when
//     one is available, otherwise a rounded, deterministically coloured square
//     carrying the application's initial (an original placeholder, never a
//     bitmap asset).
//   * a folder stack — a macOS-like folder silhouette (T-14.7h), no text.
//   * the Applications launcher — a neutral squircle with our Tabler grid
//     mark (T-19.2 follow-up).
//   * the Trash — our own geometry (lid, handle, bin) with an empty/full
//     state. No Apple artwork is copied (14-risks.md).
Item {
    id: root

    // "app" | "trash"
    property string kind: "app"
    property string name: ""
    property string appId: ""
    // The app-index record id (e.g. "org.dragonfruit.Files.desktop"). The
    // first-party bundled-tile map (T-19.1d) is keyed by this, not `appId`.
    property string desktopId: ""
    // The themed icon file from `org.dragonfruit.AppIndex1`; empty when the
    // service is absent or no theme provides the name, in which case the
    // initial tile is drawn.
    property string iconPath: ""
    property bool trashFull: false
    property real size: Theme.controls.dock.iconSize
    property real rasterSize: size

    // The source artwork is decoded at the physical size the Canvas paints,
    // not at the icon's intrinsic size. Without this an SVG written for a
    // 16/48px directory is decoded at that size and then magnified (the
    // pixelated Konsole tile), and a raster icon is upscaled on a HiDPI
    // screen. The Canvas itself already paints at the window DPR.
    //
    // The target is the largest tile the Dock can ever render, so an animated
    // `iconSize` resize does not reload the artwork on every frame; a caller
    // that asks for a larger raster (the QML tests) still wins.
    readonly property real artworkPixelSize: Math.max(1, Math.ceil(
        Math.max(Math.ceil(root.rasterSize),
                 Math.ceil(Theme.controls.dock.iconSizeMax
                           * Theme.controls.dock.magnifyPeakMax))
        * Screen.devicePixelRatio))
    // The URL and pixel size currently primed into the Canvas pixmap cache.
    property string artworkCacheUrl: ""
    property int artworkCachePixels: 0

    implicitWidth: size
    implicitHeight: size

    // The bundled first-party tile (T-19.1d). Our own apps ship their artwork
    // under `assets/icons/apps/` and the id -> file map is keyed by desktop id,
    // so nested dev (before an install) shows the right tile even when
    // app-index has no themed path yet. Third-party apps keep using `iconPath`.
    readonly property string firstPartyId: {
        var id = root.desktopId.toLowerCase();
        if (id.endsWith(".desktop"))
            id = id.substring(0, id.length - 8);
        return id;
    }
    readonly property string bundledIcon: {
        if (root.kind !== "app")
            return "";
        if (root.firstPartyId === "org.dragonfruit.files")
            return "qrc:/icons/apps/org.dragonfruit.Files.svg";
        if (root.firstPartyId === "org.dragonfruit.settings")
            return "qrc:/icons/apps/org.dragonfruit.Settings.svg";
        return "";
    }
    readonly property bool hasBundledIcon:
        root.kind === "app" && root.bundledIcon.length > 0
    // The resolved artwork URL: the bundled first-party tile wins for our own
    // apps, then app-index's themed path. Empty means the placeholder initial.
    readonly property string iconUrl:
        root.hasBundledIcon ? root.bundledIcon
        : (root.iconPath.length > 0 ? "file://" + root.iconPath : "")
    readonly property bool hasAppArtwork:
        root.kind === "app" && root.iconUrl.length > 0
    // A themed path from app-index (kept distinct from the bundled first-party
    // tile so callers can tell the two apart; T-14.1a).
    readonly property bool hasThemedIconHint:
        kind === "app" && iconPath.length > 0
    // Most Linux icon themes ship app icons as SVG; `Image` decodes both the
    // raster and the SVG form when the SVG imageformat plugin is present. When
    // it is not, an SVG still renders through the `QtQuick.VectorImage`
    // fallback below (unclipped, the pre-T-14.7w inset+fit look) rather than
    // resolving to the initial tile.
    readonly property bool isSvgIcon: iconUrl.toLowerCase().endsWith(".svg")
    // The app artwork currently on screen: the masked Canvas, or the unmasked
    // VectorImage fallback when the SVG cannot be decoded as an image.
    readonly property bool hasThemedIcon: maskedArtwork || vectorFallback

    readonly property color tileColor: {
        var palette = [
            Theme.primitive.color.magenta500,
            Theme.primitive.color.violet500,
            Theme.primitive.color.jade500,
            Theme.primitive.color.gold500,
            Theme.primitive.color.coral500,
            Theme.primitive.color.sky500
        ];
        var key = appId.length > 0 ? appId : name;
        var hash = 0;
        for (var i = 0; i < key.length; ++i)
            hash = (hash * 31 + key.charCodeAt(i)) & 0x7fffffff;
        return palette[hash % palette.length];
    }

    readonly property string initial:
        name.length > 0 ? name.charAt(0).toUpperCase() : "?"

    // The one tile every app entry's artwork sits in (T-14.7w): a rounded
    // square (squircle) reaching the tile edge, with the radius and inset from
    // tokens, never literals. The placeholder tile, the masked themed artwork,
    // and the unmasked fallback all share this geometry so they cannot drift.
    readonly property real iconInset:
        root.size * Theme.controls.dock.icon.inset
    readonly property real tileW: root.size - 2 * root.iconInset
    readonly property real tileH: root.size - 2 * root.iconInset
    readonly property real tileRadius:
        root.tileW * Theme.controls.dock.icon.radiusRatio

    // -- App tile (placeholder) -----------------------------------------
    Rectangle {
        objectName: "appTile"
        // The placeholder shows only when there is no themed artwork to draw:
        // no path at all, or a raster path that failed to decode. An SVG that
        // fails to decode still shows through the VectorImage fallback below.
        visible: root.kind === "app"
                 && (!root.hasAppArtwork
                     || (iconLoader.status === Image.Error && !root.isSvgIcon))
        x: root.iconInset
        y: root.iconInset
        width: root.tileW
        height: root.tileH
        radius: root.tileRadius
        color: root.tileColor
        border.width: 1
        border.color: Qt.rgba(0, 0, 0, 0.18)

        Text {
            anchors.centerIn: parent
            text: root.initial
            color: Theme.color.accentContent
            font.pixelSize: Math.round(root.size * 0.44)
            font.weight: Theme.primitive.font.weightSemibold
        }
    }

    // -- Themed app icon (T-14.1a, masked T-14.7w) ----------------------
    // app-index resolves the real icon from the active theme. The hidden
    // `Image` loader is the load-state oracle — it reports `Ready` so the
    // Canvas repaints once the source (raster or SVG) is decoded, and `Error`
    // so the placeholder or the SVG fallback shows. The artwork itself is
    // painted by the Canvas below, clipped to the tile squircle.
    Image {
        id: iconLoader
        objectName: "iconLoader"
        visible: false
        source: root.hasAppArtwork ? root.iconUrl : ""
        // Render SVGs at the physical artwork size and bound large rasters.
        // The Canvas `loadImage()` below uses the same sourceSize, so the
        // loader and the Canvas share one pixmap-cache entry.
        sourceSize.width: root.artworkPixelSize
        sourceSize.height: root.artworkPixelSize
        onStatusChanged: {
            if (status === Image.Ready)
                artwork.requestPaint();
        }
    }

    // Prime the Canvas's own URL -> pixmap cache at the physical artwork size.
    // `ctx.drawImage(url)` otherwise loads the source at its intrinsic size,
    // which leaves SVG and magnified raster icons blurry. Canvas `loadImage`
    // ignores a repeat call for the same URL, so a resize unloads first.
    function syncArtworkPixmap() {
        if (!root.hasAppArtwork) {
            if (root.artworkCacheUrl.length > 0)
                artwork.unloadImage(root.artworkCacheUrl);
            root.artworkCacheUrl = "";
            root.artworkCachePixels = 0;
            return;
        }
        var pixels = Math.ceil(root.artworkPixelSize);
        if (root.iconUrl === root.artworkCacheUrl
                && pixels === root.artworkCachePixels)
            return;
        if (root.artworkCacheUrl.length > 0)
            artwork.unloadImage(root.artworkCacheUrl);
        root.artworkCacheUrl = root.iconUrl;
        root.artworkCachePixels = pixels;
        artwork.loadImage(root.iconUrl, Qt.size(pixels, pixels));
    }

    Component.onCompleted: root.syncArtworkPixmap()
    onIconUrlChanged: root.syncArtworkPixmap()
    onArtworkPixelSizeChanged: root.syncArtworkPixmap()

    readonly property bool maskedArtwork:
        root.hasAppArtwork && iconLoader.status === Image.Ready
    readonly property bool vectorFallback:
        root.hasAppArtwork && root.isSvgIcon
        && iconLoader.status === Image.Error

    // The themed artwork clipped to the tile squircle. A Canvas clip is
    // executed by the headless software scene graph used by `tst_dock.qml`,
    // so the masked corners are real pixels a test can assert — unlike
    // MultiEffect/OpacityMask/ShaderEffect, which silently no-op on that
    // backend (Shadow.qml documents the same tier constraint). The clip path
    // is the same rounded rect the placeholder tile draws.
    Canvas {
        id: artwork
        objectName: "artwork"
        visible: root.maskedArtwork
        x: root.iconInset
        y: root.iconInset
        // Rasterize the mask at a stable size and scale the finished texture
        // as the tile zooms. Resizing a Canvas repaints and uploads the icon.
        width: Math.max(1, root.rasterSize * (1 - 2 * Theme.controls.dock.icon.inset))
        height: width
        scale: root.tileW / width
        transformOrigin: Item.TopLeft
        renderStrategy: Canvas.Immediate
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        // `drawImage(url)` loads through the canvas pixmap cache; if it had
        // not been decoded yet the first paint draws nothing, so repaint once
        // it lands.
        onImageLoaded: requestPaint()

        onPaint: {
            var ctx = getContext("2d");
            ctx.clearRect(0, 0, width, height);
            if (iconLoader.status !== Image.Ready)
                return;

            // The tile squircle: a rounded rect traced at the token radius.
            var w = width;
            var h = height;
            var r = Math.min(w * Theme.controls.dock.icon.radiusRatio, w / 2);
            ctx.beginPath();
            ctx.moveTo(r, 0);
            ctx.lineTo(w - r, 0);
            ctx.arcTo(w, 0, w, r, r);
            ctx.lineTo(w, h - r);
            ctx.arcTo(w, h, w - r, h, r);
            ctx.lineTo(r, h);
            ctx.arcTo(0, h, 0, h - r, r);
            ctx.lineTo(0, r);
            ctx.arcTo(0, 0, r, 0, r);
            ctx.closePath();
            ctx.save();
            ctx.clip();

            // Preserve the artwork's aspect within the tile (the pre-existing
            // fit), then let the clip round the corners.
            var iw = iconLoader.implicitWidth;
            var ih = iconLoader.implicitHeight;
            if (iw <= 0 || ih <= 0) {
                iw = w;
                ih = h;
            }
            var scale = Math.min(w / iw, h / ih);
            var dw = iw * scale;
            var dh = ih * scale;
            ctx.drawImage(root.iconUrl,
                          (w - dw) / 2, (h - dh) / 2, dw, dh);
            ctx.restore();
        }
    }

    // Degrade tier: when the SVG cannot be decoded as an image (the optional
    // imageformat plugin is absent), draw it through VectorImage instead of
    // the placeholder. It reaches the same tile but is not squircle-masked —
    // the pre-T-14.7w inset+fit look, never an unmasked square with a
    // different tile size.
    VectorImage {
        id: vectorFallbackArtwork
        objectName: "vectorIcon"
        visible: root.vectorFallback
        x: root.iconInset
        y: root.iconInset
        width: root.tileW
        height: root.tileH
        source: visible ? root.iconUrl : ""
        fillMode: VectorImage.PreserveAspectFit
    }

    // -- Folder stack ----------------------------------------------------
    // The vendored WhiteSur folder artwork (T-14.7h keeps the no-text rule).
    // The folder name belongs to the hover label and the popover, never the
    // artwork, so there is deliberately no `Text` here (ADR 0092).
    Image {
        id: stack
        objectName: "stackArtwork"
        visible: root.kind === "stack"
        anchors.fill: parent
        source: "qrc:/icons/whitesur/folder.svg"
        sourceSize.width: root.artworkPixelSize
        sourceSize.height: root.artworkPixelSize
        fillMode: Image.PreserveAspectFit
        asynchronous: true
        cache: true
        smooth: true
    }

    // -- Overflow cell ---------------------------------------------------
    // The terminal overflow cell (T-14.7q): a grid of small window tiles on a
    // neutral squircle, our own geometry (ADR 0092 — no text in the artwork).
    // The hidden-group count rides the T-14.7o badge path in `DockEntry`, never
    // this glyph.
    Item {
        id: overflow
        objectName: "overflowArtwork"
        visible: root.kind === "overflow"
        anchors.fill: parent

        readonly property real s: root.size
        readonly property real inset: s * Theme.controls.dock.overflow.gridInset
        readonly property real gap: s * Theme.controls.dock.overflow.gridGap
        readonly property real cell: s * Theme.controls.dock.overflow.gridCell

        Rectangle {
            objectName: "overflowPanel"
            anchors.fill: parent
            radius: root.tileRadius
            color: Theme.color.controlFill
            border.width: 1
            border.color: Theme.color.border
        }

        // A 3x3 grid reads as "many windows"; the centre tile is accented so
        // the cell is legible even at the minimum icon size.
        Repeater {
            model: 9
            delegate: Rectangle {
                required property int index
                width: overflow.cell
                height: overflow.cell
                radius: Math.max(1, overflow.cell * 0.24)
                color: index === 4 ? Theme.color.accent : Theme.color.textSecondary
                x: overflow.inset + (index % 3) * (overflow.cell + overflow.gap)
                y: overflow.inset + Math.floor(index / 3) * (overflow.cell + overflow.gap)
            }
        }
    }

    // -- Applications launcher (T-19.2 follow-up) ------------------------
    // The permanent Dock tile that opens the Applications drawer. WhiteSur's
    // app-grid mark carries its own white rounded tile, so it is drawn as one
    // resource rather than composited over a separate squircle.
    Image {
        id: launcher
        objectName: "launcherArtwork"
        visible: root.kind === "launcher"
        anchors.fill: parent
        source: "qrc:/icons/whitesur/view-app-grid.svg"
        sourceSize.width: root.artworkPixelSize
        sourceSize.height: root.artworkPixelSize
        fillMode: Image.PreserveAspectFit
        asynchronous: true
        cache: true
        smooth: true
    }

    // -- Trash -----------------------------------------------------------
    // WhiteSur's empty/full bin pair (T-14.7d keeps the state distinction: the
    // full state adds visible contents above the lid, not just a tint). Drawn
    // in the same `trashSize`-derived box, centred in the entry's iconSize box
    // so it scales with magnification and shares the app tiles' baseline.
    Image {
        id: trash
        objectName: "trashArtwork"
        visible: root.kind === "trash"
        anchors.centerIn: parent
        width: root.size * Theme.controls.dock.trashSize
               / Theme.controls.dock.iconSize
        height: width
        source: root.trashFull
            ? "qrc:/icons/whitesur/user-trash-full.svg"
            : "qrc:/icons/whitesur/user-trash.svg"
        sourceSize.width: Math.ceil(width * 2)
        sourceSize.height: Math.ceil(width * 2)
        fillMode: Image.PreserveAspectFit
        asynchronous: true
        cache: true
        smooth: true
    }
}
