# macOS-style Dock: screenshot-based UI/UX reproduction brief

**Target implementation:** Qt Quick/QML front end + Rust back end, initially tested in Fedora 44 KDE Plasma/Wayland.  
**Reference images:** `image(20261102-191624).png` (rest), `image(20261102-191644).png` (mouse over Apps).  
**Goal:** reproduce visible layout, light glass treatment, icon geometry, running indicators, separators, hover magnification, tooltip, and interaction quality. Existing app icons will be replaced with the project's own artwork.

## 0. Measurement confidence and coordinate system

- Both source images are **2048 pixels wide**, with heights **176** (rest) and **215** (hover). These are *screenshot pixel* coordinates, **not necessarily Qt logical/device-independent pixels**. Convert using the effective display scale/device pixel ratio.
- They are DIFFERENTLY CROPPED vertically. The hover screenshot includes a tooltip and 39 extra pixels of vertical space. Dock bottoms both appear ~9 pixels from their respective image bottoms. **Do not interpret the +40 px shift in the hover screenshot as vertical movement of the Dock.** Align the screenshots on their dock bottoms when comparing behavior.
- Body and icon positions are approximate due to anti-aliasing/shadows; solid interior colors and vertical divider colors can be sampled accurately.
- These screenshots establish two visual *states*, not the movement between them. Animation duration, easing, true translucent material parameters, actual blur radius, pointer velocity handling, hover delay, physics, and dynamic fit rules are **not measurable from these stills**. Proposed numbers for those are **tuning defaults, not observed macOS constants**.

## 1. Screenshot-measured visual metrics

| Feature | Rest screenshot | Hovering over Apps | Remarks |
|---|---|---|---|
| Screenshot dimensions | 2048 x 176 px | 2048 x 215 px | Crops differ vertically |
| Dock panel bounds | x ~103–1969; y ~41–167 | x ~80–1980; y ~83–206 | Panel expands/recenters in width; height ~constant |
| Panel dimensions | ~1866 x 126–127 px | ~1900 x 123–124 px | Hover screenshot's exact apparent edges affected by AA |
| Panel corner radius | ~40–45 px | ~40–45 px | Rounded capsule-like rectangle, but not necessarily full semicircular ends |
| Ordinary icon artwork | ~78–80 px square | ~76–80 px when far from pointer | Slight global fit/compression in hover capture possible |
| Apps artwork | ~76–79 px | ~94–96 px | Approx 1.20–1.23x magnification |
| Finder (adjacent, left) | ~79 px | ~92–94 px | Magnifies with Apps |
| Activity Monitor (adjacent, right) | ~79 px | ~93–95 px | Magnifies with Apps |
| Chrome (two slots to right) | ~77–79 px | ~85–87 px | Partial magnification |
| Discord (three slots to right) | ~79 px | ~79 px | Essentially normal size |
| Normal rest icon center pitch | ~102–103 px | ~99–102 px at distant slots | Hover spacing varies continuously near pointer |
| Running indicator | ~7 px diameter, dark gray | Same diameter | Attached to slot center, not enlarged with artwork |
| Vertical dividers | ~1 visual px dark core; ~98 px high | Same appearance | Antialiased side pixels; not scaled on hover |
| Tooltip | Hidden | x ~222–320, y ~21–69 | ~97x47 including pointer; centered above Apps |

### Pixel-color references on *this white screenshot background*

| Surface/feature | Sample/starting value | Interpretation |
|---|---|---|
| Screenshot outside panel | **#FEFEFE** | Nearly pure white background |
| Dock interior, large unoccluded region | **#E5E5E5** (RGB 229,229,229) | Very uniform neutral light-gray fill in captured output |
| Dock interior lower-edge transition | ~#E6E6E6 → #F0F0F0 → nearly #FEFEFE | Feathered/antialiased lower border, subtle lightening |
| Top hairline / edge highlights | Near white and light gray | Soft glass/glint, not a dark hard border |
| Strong divider pixel | **#ACACAC** (RGB 172) | Single core pixel, with lighter antialias edges |
| Running dot core | **#676767** (RGB 103) | 7–8 px circle, antialiased |
| Dim/inactive dot example | **#C6C6C6** (RGB 198) | State-dependent muted indicator |
| Tooltip fill | Near white | Minimal contrast from background; slim gray outline |
| Tooltip lettering | Approx #252525–#333333 | San Francisco-like regular-weight sans, ~20–22 screenshot px |

**Critical material limitation:** #E5E5E5 is the COMPOSITED output against a white background, not proof the Dock is an opaque #E5E5E5 rectangle. Frosted-glass blur strength, tint opacity, and saturation CANNOT be recovered from these captures. Begin with a faithful solid/milky-gray implementation. Add compositor blur only after reproducing geometry; compare against colorful/contrasty wallpaper separately.

## 2. Content layout and alignment

The screenshot contains **17 icon positions** in three groups:

- **Group 1:** 10 slots, then divider.
- **Group 2:** 5 slots, then divider.
- **Group 3:** 2 slots, visually a printer/utility item and Trash in the reference. These can be replaced with the project's own utility icons.

**At rest**, approximate icon center x coordinates in the 2048-pixel reference:

`[168, 270, 373, 476, 578, 681, 783, 886, 989, 1091 | 1241, 1343, 1446, 1549, 1651 | 1803, 1907]`

Approximate first and second divider centers at `x=1168` and `x=1726`.

**While hovering Apps**, approximate centers:

`[154, 273, 393, 506, 614, 713, 814, 913, 1014, 1111 | 1256, 1355, 1453, 1554, 1655 | 1798, 1896]`

Approximate first and second divider centers at `x=1194` and `x=1743`. The separator **moves with reflow** but does not grow with the icons.

### Geometric explanation / reconstruction

- Resting icon center-to-center distance is ~103 px. With a ~79 px icon, this gives a **~23–24 px edge-to-edge gap**.
- Each divider adds ~47–49 px of *extra group-break space* relative to a normal center-to-center icon interval.
- The Dock's left padding from panel edge to the first icon's *artwork* edge is ~26 px; right padding to last visible artwork edge is ~29 px. Use **~26 px horizontal inner padding** on each end at reference scale.
- At rest the Dock is approximately **126 px tall**. Icon artwork's bottom edge sits ~22–24 px above the panel bottom; the running indicator's center is ~8–9 px above the panel bottom.
- The **icon bottom edges align**, instead of vertically centering icons as they grow. Rest icon top is about y=65 and icon bottom y=144. Aligning the captures by +40 px, this corresponds to y=105..184 in the hover image. Hover Apps begins near y=88, with bottom near y=183: **increased height extends UP, with its bottom edge almost fixed**.
- Running dots are aligned on the same horizontal line, at rest y≈158–159, in the hover screenshot y≈198–199. Their centers remain at a consistent distance beneath each icon.
- Tooltip and icons are above the glass backdrop; icons are not clipped by the glass rounded shape.

**Reference starting geometry, screenshot-pixel units:**

```text
baseIconSize      = 79
maximumIconSize   = 94–96
normalIconGap     = 24
extraGroupGap     = 48 (added to an ordinary inter-icon gap)
endPadding        = 26
panelHeight       = 126
panelRadius       = 43
iconBottomInset   = 23   (relative to panel bottom)
indicatorBottom   = 9    (center relative to panel bottom)
indicatorDiameter = 7
separatorWidth    = 1–2 rendered pixels (core ~1)
separatorHeight   = 98
```

Recompute and **center the entire content strip** as icon widths change. Do not use fixed absolute x coordinates; the measurements are validation fixtures only. Keep some optional width-fit logic: these screenshots contain 17 slots across 2048 px, very close to the usable width, so distant icons may shrink a couple of pixels in the hover state as the overall Dock reflows.

## 3. Panel/"glass" drawing order

1. Optional ultra-soft backdrop separation shadow below the rounded panel. Keep subtle: broad ~4–10 px falloff, low opacity. Do **not** make a hard black shadow.
2. Rounded light neutral-gray panel (rest output #E5E5E5), with rounded corners ~43px at the reference scale.
3. Very subtle upper/inner pale highlight; 1px-ish upper outline; slightly faded/lighter bottom outline and sides. Rounded edges should antialias softly, not appear as an opaque gray stroke.
4. Thin vertical dividers, anchored within the panel. The 1px dark center can be #ACACAC with nearby antialias pixels.
5. Icon artwork, each with its **own** alpha, squircle silhouette or irregular shape, and faint drop shadow (if not included in asset).
6. Running-state dots below artwork.
7. Optional app notification badges, floating on top/right of artwork.
8. Hover tooltip above all, outside the panel body.

The interior of the reference is strikingly flat and uniform: **avoid loud gradients, blue tints, heavy borders, or obvious noise textures**. Much of the glass effect is the gently rounded translucent-seeming container, low-contrast edges, and app-icon shadows.

## 4. Custom icon authoring policy

- Each app slot has a **square logical artwork box** with a consistent bottom edge. The artwork need NOT be a rectangle: a photo, trash bin, freeform silhouette, or folder can keep transparency.
- Do **not** draw a universal white rounded rectangle behind every icon. Several references have intrinsic white/gray squircles, but others are freeform shapes. Custom assets decide their own background and silhouette.
- For modern macOS-looking custom square tiles, favor a **soft squircle/superellipse** (not a hard-cornered square), with radius appearance roughly ~20–25% of artwork side and smooth antialiasing. This is an artistic approximation, not a measured Apple mask formula.
- Use source PNG/SVG at least **2x the maximum rendered dimension** for sharp magnification. Prefer SVG where feasible or 256/512px premultiplied-alpha raster assets.
- Preserve internal artwork whitespace, optical centering, icon-specific lighting, and existing shadow. Two nominal 79px icons may have different visible opaque bounds because their source artwork differs.
- Suggested additional fallback shadow if asset lacks one: ~2–4px downward offset, blur radius ~3–5px, opacity 0.10–0.20. Tune by screenshot; don't double-shadow icons with embedded effects.
- Do not add hover gray overlays, borders, tints, or glow to an icon. The main hover signal is **scale + position + tooltip**.
- Notification badge example: red near #FF3B30 with white text, circular, floating partly outside Settings at top/right. Screenshot badge is roughly ~36–38px across, with white '3'. Keep badges independent of running dots and align to the current (possibly magnified) icon bounds.

## 5. Hover magnification behavior — the most important part

### Directly observed

- Apps grows from about 78px to about 94–96px.
- Its **left neighbor (Finder)** and **right neighbor (Activity Monitor)** also grow to about 92–94px, almost the same size as Apps.
- The next icon (Chrome, two to the right) grows moderately, roughly 85–87px.
- By the third neighbor the icon is nearly its normal size.
- Icons reflow horizontally, so neighbors are **pushed away without overlap**. The entire strip remains centered. The right-hand groups also shift slightly.
- The artwork grows **upward** while its bottom edge and running dots remain vertically anchored.
- The glass backing increases in width but not meaningfully in height. Do not enlarge the background vertically to follow icon tops.
- Hover does not add a selected outline or background directly behind Apps. Tooltip points to it.

### Suggested continuous algorithm (tune to reference)

Calculate magnification based on **continuous pointer x**, not a boolean hovered/not-hovered state. Use a compact smooth influence curve with these sample weights, where `d` is horizontal distance from pointer to item's baseline center, divided by resting icon pitch (~103px):

```text
 d (slot pitches)       0.0     1.0     2.0     3.0+
 magnification weight   1.00    0.90    0.45    0.00
```

Interpolate the knots smoothly (e.g., monotonic cubic interpolation or a C1-smooth piecewise curve). These are calibration targets fitted to the captured visual sizes, **not a claim about Apple's exact formula**.

```text
nominalSize = 79
peakSize    = 95
size_i      = nominalSize + (peakSize - nominalSize) * weight(distance_i)
```

As the pointer moves over a slot, the peak follows it continuously. Shift and resize *all* icon slots using recalculated target widths, preserving ~24px visual gaps. Divider extra gaps remain ~48px; divider x positions follow reflow. At/near screen boundaries dynamically constrain width, with smooth fallbacks (uniform base-size reduction, lower peak size, or bounded reflow) instead of clipping.

**Avoid feedback jitter:** Using each icon's newly shifted center to recompute hover distance can cause a chasing/oscillating layout. Use stable baseline centers for distance calculation, or a proven stable iterative mapping with hysteresis. Keep pointer coordinates in one well-defined window/display-local space. Hit tests should follow the current icon/slot region without unwanted enter/exit oscillation.

**Do not** animate a centered `scale` transform while leaving layout positions/slots static: this makes icons overlap and grows them below the baseline. Recompute explicit rendered size and x/y positions, or use bottom-origin scaling with matching layout reflow.

### Suggested motion tuning — NOT measurable from stills

- Scale/reflow to new target: ~130–180 ms, ease-out cubic or an appropriately damped continuous spring, without visible overshoot.
- Return to rest: ~150–220 ms, ease-out; matched to entry, no snap.
- Track fast pointer motion with a smooth time-dependent response, not a series of stop/start animations. Qt Quick `SmoothedAnimation` is intended for changing targets; tune velocity/maximum easing time. `NumberAnimation` is good for fixed state transitions; `SpringAnimation` is available when damping is desired. **Do not use bouncy/elastic overshoot by default.**
- Maintain 60fps minimum, ideally support 120Hz+ compositing. Avoid per-frame bitmap regeneration and texture uploads.
- On mouse exit, weights fade back to zero and the panel smoothly returns to resting width/spacing.

## 6. Running-state indicators

- Small filled circles, approximately **7px diameter**, directly beneath the horizontal center of their icon; round, antialiased.
- Dark visible sample core about **#676767**. A muted example about **#C6C6C6** is visible beneath one icon; implement meaningful per-app running/active/inactive states if your app model needs them.
- Dot centers about **14px vertically below the icon bottom edge**, around **9px above the panel bottom**.
- Dot positions move horizontally with the slot, **but dot sizes do not magnify** with the icon. No running dot beneath every icon: they denote state, not a universal decoration.
- Separate the running indicator's display state from hover/magnification state. Dot should not jump up/down with icon scaling.

## 7. Section dividers

- Two 1–2px antialiased vertical separators, visually core #ACACAC, height ~98px.
- At rest: x≈1168 and x≈1726. Hover: x≈1194 and x≈1743.
- Vertically around panel top +15px to panel bottom -13px; they stop well within the rounded body.
- They consume a group-break gap ~48px wider than a regular inter-icon gap. The divider sits approximately in the middle of the gap between group icons.
- Vertical size and stroke thickness remain fixed during hover, but **horizontal position reflows** with icon widths.

## 8. Tooltip/hover label

In the hover screenshot, a small speech-bubble tooltip reads `Apps`:

- ~97px wide, ~47px total high including its downward pointer, at x~222–320, y~21–69.
- Top portion ~38px high; radius ~18–20px; a small centered point/tail extends ~8–10px down.
- Centered near hovered icon x~273, not at arbitrary mouse coordinates.
- Near-white fill, hairline pale gray outline, almost imperceptible shadow; no saturated colors.
- Dark charcoal text, regular-weight San Francisco-like system sans, approximately **20–22 screenshot px**. Use Qt's closest clean system font; typography should be optically centered.
- Bubble tip ends about **14px above the Dock panel top**. The title should not cover the magnified artwork.
- Label width follows text with ~19–24px horizontal internal padding. For longer app labels allow a wider bubble, not fixed 97px.
- Suggested delay 150–300ms and fade 80–140ms; **timings unobservable in screenshot**. It should follow the currently targeted item without a trail of labels.
- Tooltip must render outside the Dock's rounded glass clip. If popup is a separate Wayland surface, keep it reliably stacked, anchored, and closed on exit/click.

## 9. Qt Quick + Rust + Fedora 44 KDE/Wayland engineering guidance

**Division of responsibilities**

- QML/Qt Quick: visual scene, pointer hover sampling, size/position interpolation, image rendering, glass panel, tooltip, separators, dots, badges, animations.
- Rust: model of pinned/running apps and their launch actions, app IDs, icon lookup/cache, ordering, persistence, running-state updates, application lifecycle, configuration. Expose stable model roles to QML through the project's existing bridge.
- Native surface integration is separate from the visual drawing. On Fedora 44 with KDE Plasma on Wayland, look at **KDE LayerShellQt**, available as Fedora package `layer-shell-qt` (and `layer-shell-qt-devel` if building native integration). It provides `wl-layer-shell` integration for a dock/panel-style Wayland surface. The upstream API also supports a C++ shim (`LayerShellQt::Shell::useLayerShell`, `LayerShellQt::Window::get(...)`) if necessary. There is a QML module under `org.kde.layershell`; verify exact runtime API for installed version.
- A normal frameless always-on-top `Window` is fine for a **visual prototype**, but may not behave like a real anchored/panel surface on Wayland. Prefer the supported compositor protocol when integrating into the desktop.
- For LayerShellQt, configure the desired bottom anchoring, layering, margins, mouse focus, optional exclusive zone, and per-monitor surface behavior. Test interactions with Plasma's existing panel/autohide before replacing it.
- **Do not assume that** QML opacity or a local `MultiEffect` blur blurs the wallpaper *behind another Wayland surface*. On KWin, background blur is a compositor-mediated effect (e.g., KWin blur protocol/regions, when supported). It may need native integration. Solid #E5E5E5 with subtle highlights is the reliable pixel-fidelity baseline for these captures.
- Constrain the Wayland surface/input region. A huge transparent fullscreen input-catching QML window will block clicks on apps underneath. Still provide enough *renderable* space above the Dock for icons and tooltip and handle surface bounds as they reflow.
- On multi-monitor and fractional-scale setups, convert screenshot physical px to Qt logical px; account for `devicePixelRatio`, per-output geometry, output scale, and pointer coordinate space. Optimize for stable true bottom anchoring independent of resolution.
- Avoid clipping during maximum growth and tooltip display. Ensure alpha edges render correctly (premultiplied alpha where appropriate, high-resolution source assets, antialiasing and mipmapping when needed).

**Possible QML component structure**

```text
DockSurface / LayerShell integration
  └─ DockRoot Item (transparent scene area with appropriate hit region)
      ├─ DockBackdrop: rounded glass panel + edge highlights
      ├─ Layout computation: icon sizes, x centers, group break gaps
      ├─ Repeater/DockIcon: art, alpha shadow, badge, running dot
      ├─ DockSeparators: lines at reflowed group boundaries
      └─ DockTooltip: separate above-body overlay, above all icons

Rust AppModel: id, displayName, iconURL, launchAction,
               running, active, badgeCount, group, order
```

**Central layout pseudocode (conceptual, not drop-in QML):**

```text
const BASE = 79
const PEAK = 95
const NORMAL_GAP = 24
const GROUP_EXTRA_GAP = 48
const PANEL_H = 126
const END_PAD = 26

for each frame / pointer update:
    for each icon i:
        d = abs(mouseX - restCenterX[i]) / 103
        w = interpolate_smoothly([(0,1), (1,.90), (2,.45), (3,0)], d)
        targetSize[i] = BASE + (PEAK - BASE) * w

    totalWidth = sum(targetSize)
               + (iconCount - 1) * NORMAL_GAP
               + (groupBreakCount * GROUP_EXTRA_GAP)
               + 2 * END_PAD
    left = (outputWidth - totalWidth) / 2

    x = left + END_PAD
    for i in icon order:
        if i starts a new group:
            x += GROUP_EXTRA_GAP
        iconLeft[i] = x
        iconWidth[i] = smooth(targetSize[i])
        iconBottom[i] = panelBottom - 23
        x += iconWidth[i] + NORMAL_GAP

    panel bounds follow totalWidth, centered on output
    dotCenterX[i] = iconLeft[i] + iconWidth[i]/2
    dotCenterY = panelBottom - 9
    tooltipX = dotCenterX[focusedIcon] - tooltipWidth/2
```

**Implementation nuance:** The pseudocode expresses layout shape. A real implementation must use actual icon-center computations with `size_i/2` and layout accounting for group gaps. Ensure the *same interpolated size values* feed both artwork and x layout each frame, to avoid visual gaps appearing/reflow running ahead of the rendered icon. Compute stable baseline centers for hover weights separately from animated centers. After the normal gap following the final icon, do not accidentally count it inside width.

## 10. Visual acceptance checklist

1. Under a nearly white background, captured interior of panel reads ~#E5E5E5; border is exceptionally subtle and softened.
2. With 17 app slots / 2 separators on a 2048px-wide reference image, at rest: panel ~1866x126, icon pitch ~103, icon visible bounds around 79px, icon bottoms aligned, running dots centered ~14px below icon bottom.
3. Hover Apps: Apps, Finder, Activity Monitor reach ~94px; Chrome ~86px; third neighbor returns near nominal. **No overlap**.
4. Hovered and neighboring icons grow **up**; their bottom edges stay aligned (roughly within ±2px). Glass panel is still ~125px tall and widens/recenters horizontally.
5. Dividers slide horizontally with the reflow but retain line thickness/height; running dots maintain fixed size and vertical baseline.
6. A near-white `Apps` tooltip appears over hovered Apps, centered, with rounded bubble and small downward pointer; no tooltip at rest.
7. As the pointer sweeps left-to-right, there is no hard jump between discrete hovered indices. All icons reposition smoothly. No vibrating selection, sticky ghost hover, jumps on exit, or tooltip clipping.
8. When pointer leaves the Dock, icon sizes, positions, and panel width settle back to rest. Static/low-motion preference should disable or sharply reduce movement without breaking layout.
9. Works at different desktop widths, many app counts, and fractional display scaling. Does not block unrelated desktop pointer input.
10. Screenshot test compares **geometry and aesthetic**, not matching the reference's trademarked app artwork; custom icon assets must still obey the same logical bounding boxes and baseline.

## 11. Highest-priority implementation sequence

1. Build a static 17-slot Dock with backdrop, group breaks, running indicators and top tooltip layer. Tune screenshot at-rest pixel geometry.
2. Add continuous mouse-x influence weights with icon sizes bottom-anchored. Recompute horizontal layout and panel width from sizes.
3. Animate reflow and sizes coherently. Test sweeps across adjacent icons and ends. Fix jitter and capture issues.
4. Add independent badge/dot/tooltip logic, press/click activation and app state updates from Rust.
5. Integrate actual Plasma/Wayland panel surface through LayerShellQt; verify stacking, workspace/fullscreen behavior, multiple displays and input regions.
6. Only then attempt compositor blur/material improvements; tune against both pale and colorful wallpapers.

## 12. Source links for implementation mechanics (not for macOS pixel measurements)

- Qt Quick animation choices: https://doc.qt.io/qt-6/qtquick-statesanimations-animations.html
- Qt Quick `SmoothedAnimation`: https://doc.qt.io/qt-6/qml-qtquick-smoothedanimation.html
- KDE `LayerShellQt` upstream: https://github.com/KDE/layer-shell-qt
- Fedora 44 LayerShellQt packages: https://packages.fedoraproject.org/pkgs/layer-shell-qt/layer-shell-qt/
- KWin blur implementation reference: https://github.com/KDE/kwin/blob/master/src/plugins/blur/blur.cpp

**Instruction to implementation LLM:** Prioritize screenshot visual match and interaction continuity, not arbitrary 'Apple-inspired' styling. Treat direct measurements as regression criteria and proposed animation/material defaults as parameters to tune. Do not substitute center-based scaling, a fixed-width panel, generic blur, or a single-hovered-icon effect for the multi-icon reflow shown in the references.
