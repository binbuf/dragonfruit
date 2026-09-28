// SPDX-License-Identifier: MIT
import QtQuick
import QtQuick.Shapes
import QtQuick.VectorImage
import Dragonfruit

// Dock entry artwork (T-10). Three shapes:
//   * an app tile — a real themed icon resolved by app-index (T-14.1a) when
//     one is available, otherwise a rounded, deterministically coloured square
//     carrying the application's initial (an original placeholder, never a
//     bitmap asset).
//   * a folder stack — a macOS-like folder silhouette (T-14.7h), no text.
//   * the Trash — our own geometry (lid, handle, bin) with an empty/full
//     state. No Apple artwork is copied (14-risks.md).
Item {
    id: root

    // "app" | "trash"
    property string kind: "app"
    property string name: ""
    property string appId: ""
    // The themed icon file from `org.dragonfruit.AppIndex1`; empty when the
    // service is absent or no theme provides the name, in which case the
    // initial tile is drawn.
    property string iconPath: ""
    property bool trashFull: false
    property real size: Theme.controls.dock.iconSize

    implicitWidth: size
    implicitHeight: size

    readonly property bool hasThemedIconHint:
        kind === "app" && iconPath.length > 0
    // Most Linux icon themes ship app icons as SVG; `Image` decodes both the
    // raster and the SVG form when the SVG imageformat plugin is present. When
    // it is not, an SVG still renders through the `QtQuick.VectorImage`
    // fallback below (unclipped, the pre-T-14.7w inset+fit look) rather than
    // resolving to the initial tile.
    readonly property bool isSvgIcon: iconPath.toLowerCase().endsWith(".svg")
    // The themed artwork currently on screen: the masked Canvas, or the
    // unmasked VectorImage fallback when the SVG cannot be decoded as an image.
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
                 && (!root.hasThemedIconHint
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
        source: root.hasThemedIconHint ? "file://" + root.iconPath : ""
        // No `sourceSize`: the loader and the Canvas drawImage share one
        // natural-size pixmap-cache entry, so the artwork is decoded once.
        onStatusChanged: {
            if (status === Image.Ready)
                artwork.requestPaint();
        }
    }

    readonly property bool maskedArtwork:
        root.hasThemedIconHint && iconLoader.status === Image.Ready
    readonly property bool vectorFallback:
        root.hasThemedIconHint && root.isSvgIcon
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
        width: root.tileW
        height: root.tileH
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
            var r = Math.min(root.tileRadius, Math.min(w, h) / 2);
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
            ctx.drawImage("file://" + root.iconPath,
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
        source: visible ? "file://" + root.iconPath : ""
        fillMode: VectorImage.PreserveAspectFit
    }

    // -- Folder stack ----------------------------------------------------
    // A macOS-like folder silhouette (T-14.7h): a back tab, a front face with
    // a subtle vertical gradient, a rim edge, and an inner sheen — all our own
    // geometry, sized like the app tiles. The folder name belongs to the hover
    // label and the popover, never the artwork, so there is deliberately no
    // `Text` here (ADR 0092).
    Item {
        id: stack
        objectName: "stackArtwork"
        visible: root.kind === "stack"
        anchors.fill: parent

        readonly property real s: root.size

        // The back tab, peeking above the front face on the left.
        Rectangle {
            objectName: "stackFolderTab"
            x: stack.s * 0.10
            y: stack.s * 0.20
            width: stack.s * 0.42
            height: stack.s * 0.18
            radius: stack.s * 0.045
            color: Theme.color.folderTab
            border.width: 1
            border.color: Theme.color.folderRim
        }

        // The front face: gradient body, rim edge, and a soft top sheen.
        Rectangle {
            objectName: "stackFolderFront"
            x: stack.s * 0.08
            y: stack.s * 0.30
            width: stack.s * 0.84
            height: stack.s * 0.48
            radius: stack.s * 0.09
            border.width: 1
            border.color: Theme.color.folderRim
            gradient: Gradient {
                GradientStop { position: 0.0; color: Theme.color.folderFillTop }
                GradientStop { position: 1.0; color: Theme.color.folderFillBottom }
            }

            Rectangle {
                objectName: "stackFolderSheen"
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: Math.max(1, stack.s * 0.05)
                height: Math.max(1, stack.s * 0.06)
                radius: height / 2
                color: Theme.color.folderHighlight
                opacity: 0.35
            }
        }
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

    // -- Trash -----------------------------------------------------------
    // A designed, original bin (T-14.7d): a metallic body with a vertical
    // fill/gradient and a rim edge, a lid that overhangs the body, and a clean
    // handle. Empty is a tidy neutral bin; full adds a crumpled-paper
    // silhouette overflowing behind the lid plus an accent rim cue — a shape
    // change, not just a tint, so it stays legible in grayscale. Drawn in a
    // `trashSize`-derived box, centred in the entry's iconSize box so it scales
    // with magnification and shares the app tiles' baseline.
    Item {
        id: trash
        visible: root.kind === "trash"
        anchors.centerIn: parent
        width: root.size * Theme.controls.dock.trashSize
               / Theme.controls.dock.iconSize
        height: width

        readonly property real t: width
        // The defining edge; the accent cue is reserved for a full bin.
        readonly property color edge:
            root.trashFull ? Theme.color.accent : Theme.color.trashRim

        // Crumpled paper behind the rim, visible only above the lid. A jagged
        // silhouette so the full state reads without colour.
        Shape {
            visible: root.trashFull
            width: 100
            height: 100
            scale: trash.t / 100
            transformOrigin: Item.TopLeft
            antialiasing: true
            preferredRendererType: Shape.GeometryRenderer

            ShapePath {
                fillColor: Theme.color.trashPaper
                strokeColor: Theme.color.trashPaperEdge
                strokeWidth: 2.0
                PathSvg {
                    path: "M27 46 L24 22 L31 27 L35 13 L42 22 L47 8 "
                        + "L53 21 L59 11 L65 23 L71 15 L77 28 L74 46 Z"
                }
            }
        }

        // Body: rounded, subtly tapered by its gradient, rim edge.
        Rectangle {
            id: body
            width: trash.t * 0.62
            height: trash.t * 0.52
            radius: trash.t * 0.09
            x: (trash.t - width) / 2
            y: trash.t * 0.34
            border.width: Math.max(1, trash.t * 0.025)
            border.color: trash.edge
            gradient: Gradient {
                GradientStop { position: 0.0; color: Theme.color.trashFillTop }
                GradientStop { position: 1.0; color: Theme.color.trashFillBottom }
            }

            // A soft vertical sheen so the body reads as metal/glass, not fill.
            Rectangle {
                width: body.width * 0.20
                height: body.height * 0.62
                radius: width / 2
                x: body.width * 0.16
                y: body.height * 0.18
                color: Theme.color.trashHighlight
                opacity: 0.22
            }
        }

        // Lid: overhangs the body, with its own rim and a top highlight.
        Rectangle {
            id: lid
            width: trash.t * 0.76
            height: trash.t * 0.105
            radius: trash.t * 0.035
            x: (trash.t - width) / 2
            y: trash.t * 0.255
            border.width: Math.max(1, trash.t * 0.025)
            border.color: trash.edge
            gradient: Gradient {
                GradientStop { position: 0.0; color: Theme.color.trashFillTop }
                GradientStop { position: 1.0; color: Theme.color.trashFillBottom }
            }

            Rectangle {
                width: lid.width * 0.78
                height: Math.max(1, lid.height * 0.18)
                radius: height / 2
                x: (lid.width - width) / 2
                y: lid.height * 0.16
                color: Theme.color.trashHighlight
                opacity: 0.55
            }
        }

        // Handle: a clean rounded bar above the lid.
        Rectangle {
            width: trash.t * 0.24
            height: trash.t * 0.055
            radius: height / 2
            x: (trash.t - width) / 2
            y: trash.t * 0.155
            color: trash.edge
        }
    }
}
