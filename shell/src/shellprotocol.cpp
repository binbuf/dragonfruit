// SPDX-License-Identifier: MIT
#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif

#include "shellprotocol.h"

#include <algorithm>
#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <utility>

#include <QElapsedTimer>
#include <QtMath>
#include <QVariantMap>
#include <QSocketNotifier>

#include <fcntl.h>
#include <poll.h>
#include <sys/mman.h>
#include <unistd.h>

#ifdef __linux__
#include <linux/memfd.h>
#endif

#include <wayland-client.h>

#include "chromescale.h"
#include "chromestats.h"
#include "dockdrops.h"
#include "gpuchromeframeproducer.h"
#include "dockprojection.h"
#include "dragonfruit-core-client-protocol.h"
// wayland-scanner emits `namespace` as a C argument name for the chrome
// factory; it is a C++ keyword, so rename it around this one generated
// header only.
#define namespace df_layer_namespace
#include "dragonfruit-shell-client-protocol.h"
#undef namespace
#include "dragonfruit-toplevel-client-protocol.h"
// The vendored standard protocol (MIT); the shell is the first-party lock UI.
#include "ext-session-lock-v1-client-protocol.h"
// The vendored standard protocols (MIT) for physical-resolution chrome
// buffers (T-21.1b).
#include "fractional-scale-v1-client-protocol.h"
#include "viewporter-client-protocol.h"
// The vendored standard protocol (MIT); the shell's clipboard history
// observes and forwards the selection through it (T-13.5b, ADR 0082).
#include "wlr-data-control-unstable-v1-client-protocol.h"
// The vendored standard protocol (MIT) for the T-21.1d dmabuf transport.
#include "linux-dmabuf-unstable-v1-client-protocol.h"

#ifndef DF_LOCKSTEP_VERSION
#define DF_LOCKSTEP_VERSION 1
#endif

struct ShellShmSlot {
    wl_buffer *buffer = nullptr;
    void *data = nullptr;
    int fd = -1;
    size_t bytes = 0;
};

struct ShellDmabufSlot {
    wl_buffer *buffer = nullptr;
    zwp_linux_buffer_params_v1 *params = nullptr;
    QSize size;
    uint32_t format = 0;
    uint64_t modifier = 0;
    uint64_t serial = 0;
    bool busy = false;
    bool failed = false;
    bool creating = false;
    QString failReason;
};

struct ShellSurfaceBuffer {
    ChromeBufferPool pool{3};
    QHash<int, ShellShmSlot> slotResources;
    // One reusable wl_buffer per producer ring slot (T186c). Keyed by the
    // producer's slot id; a slot is recreated only when its size/format
    // changes or its import failed.
    QHash<int, ShellDmabufSlot> dmabufSlots;
    QImage lastImage;
    QRect lastSourceRect;
    wl_callback *frameCallback = nullptr;
    QElapsedTimer frameRequested;
};

namespace {

constexpr uint32_t kAnchorTop = 1;
constexpr uint32_t kAnchorBottom = 2;
constexpr uint32_t kAnchorLeft = 4;
constexpr uint32_t kAnchorRight = 8;

// The banner card's inset from the output's right edge (T-11.1a).
constexpr int kBannerMargin = 8;

// Replace one layer surface's independently keyed material-region list
// (T197/T203/T205). The caller commit-latches it with the next surface image.
void applyPanelRegions(df_layer_surface *layer, const QVariantList &regions)
{
    if (!layer)
        return;
    df_layer_surface_clear_panel_regions(layer);
    for (const QVariant &value : regions) {
        const QVariantMap region = value.toMap();
        const QString key = region.value(QStringLiteral("key")).toString();
        const QString preset = region.value(QStringLiteral("preset")).toString();
        const int width = qRound(region.value(QStringLiteral("w")).toReal());
        const int height = qRound(region.value(QStringLiteral("h")).toReal());
        if (key.isEmpty() || width <= 0 || height <= 0)
            continue;
        df_layer_surface_set_panel_region(
            layer, key.toUtf8().constData(), preset.toUtf8().constData(),
            qRound(region.value(QStringLiteral("x")).toReal()),
            qRound(region.value(QStringLiteral("y")).toReal()), width, height,
            qRound(region.value(QStringLiteral("tl")).toReal()),
            qRound(region.value(QStringLiteral("tr")).toReal()),
            qRound(region.value(QStringLiteral("br")).toReal()),
            qRound(region.value(QStringLiteral("bl")).toReal()),
            qRound(region.value(QStringLiteral("overscan")).toReal()));
    }
}

// The Dock's surface anchor for a position (T-10 section 5). A bottom Dock
// stretches the full width; a vertical Dock stretches the full height and
// sits on the left or right edge.
uint32_t dockSurfaceAnchor(ShellProtocol::DockPosition position)
{
    switch (position) {
    case ShellProtocol::DockPosition::Left:
        return kAnchorLeft | kAnchorTop | kAnchorBottom;
    case ShellProtocol::DockPosition::Right:
        return kAnchorRight | kAnchorTop | kAnchorBottom;
    case ShellProtocol::DockPosition::Bottom:
    default:
        return kAnchorBottom | kAnchorLeft | kAnchorRight;
    }
}

// The Dock popover's overlay anchor. It is placed with margins from the
// edges the popover hugs: the Dock's edge plus the top of the output for a
// vertical Dock (T-10 section 5).
uint32_t dockPopupAnchor(ShellProtocol::DockPosition position)
{
    switch (position) {
    case ShellProtocol::DockPosition::Left:
        return kAnchorLeft | kAnchorTop;
    case ShellProtocol::DockPosition::Right:
        return kAnchorRight | kAnchorTop;
    case ShellProtocol::DockPosition::Bottom:
    default:
        return kAnchorBottom | kAnchorLeft;
    }
}

int createShmFile(size_t size)
{
    int fd = -1;
#ifdef __linux__
    fd = memfd_create("dragonfruit-shell", MFD_CLOEXEC | MFD_ALLOW_SEALING);
#endif
    if (fd < 0)
        fd = ::open("/dev/shm", O_TMPFILE | O_RDWR | O_CLOEXEC, 0600);
    if (fd < 0)
        return -1;
    if (ftruncate(fd, static_cast<off_t>(size)) < 0) {
        ::close(fd);
        return -1;
    }
    return fd;
}

} // namespace

ShellProtocol::ShellProtocol(QObject *parent)
    : QObject(parent)
{
}

ShellProtocol::~ShellProtocol()
{
    teardown();
}

bool ShellProtocol::fail(const QString &message)
{
    m_error = message;
    emit fatal(message);
    return false;
}

bool ShellProtocol::connectToCompositor(const QString &socketName)
{
    const QByteArray name = socketName.toUtf8();
    m_display = wl_display_connect(socketName.isEmpty() ? nullptr : name.constData());
    if (!m_display)
        return fail(QStringLiteral("cannot connect to the compositor: %1").arg(socketName));

    m_registry = wl_display_get_registry(m_display);
    static const wl_registry_listener registryListener = {
        onRegistryGlobal,
        onRegistryGlobalRemove,
    };
    wl_registry_add_listener(m_registry, &registryListener, this);

    if (wl_display_roundtrip(m_display) < 0)
        return fail(QStringLiteral("compositor registry roundtrip failed"));

    if (!m_compositor || !m_shm)
        return fail(QStringLiteral("compositor is missing wl_compositor/wl_shm"));
    if (!m_coreName)
        return fail(QStringLiteral("df_core is not advertised"));

    m_core = static_cast<df_core *>(
        wl_registry_bind(m_registry, m_coreName, &df_core_interface,
                         std::min(m_coreVersion, 1u)));
    if (!m_core)
        return fail(QStringLiteral("failed to bind df_core"));
    static const df_core_listener coreListener = { onCoreAuthenticated, onCoreRefused };
    df_core_add_listener(m_core, &coreListener, this);

    // T-21.1d capability probe: one extra registry on the same connection,
    // no buffer creation and no commit. A failed probe is a recorded fallback
    // reason, never a startup failure.
    m_gpuCapabilities = GpuChromeProbe::probeDisplay(m_display);
    GpuChromeProbe::logResult(m_gpuCapabilities, QStringLiteral("startup"));
    return true;
}

bool ShellProtocol::authenticate(const QString &tokenHex)
{
    if (!m_core)
        return fail(QStringLiteral("df_core is not bound"));
    const QByteArray token = tokenHex.toUtf8();
    df_core_authenticate(m_core, DF_LOCKSTEP_VERSION, token.constData());
    if (wl_display_roundtrip(m_display) < 0)
        return fail(QStringLiteral("handshake roundtrip failed"));
    if (!m_authenticated)
        return fail(m_error.isEmpty() ? QStringLiteral("launch-token handshake refused")
                                      : m_error);
    bindTrustedGlobals();
    // Flush the trusted-global binds and receive the manager's initial replay
    // (outputs/workspaces/focus) now, before the caller creates any chrome
    // surface. Without this the binds are only flushed by the first layer
    // roundtrip, so the output scale policy cannot reach the compositor before
    // that surface is configured.
    if (wl_display_roundtrip(m_display) < 0)
        return fail(QStringLiteral("trusted-global replay roundtrip failed"));
    return true;
}

void ShellProtocol::bindTrustedGlobals()
{
    if (m_trustedGlobalsBound)
        return;
    if (m_shellName) {
        // Bind through the generated interface's own version, so an additive
        // protocol bump (T-14.7b's `set_panel_rect`) never needs a second
        // hard-coded cap here.
        m_shell = static_cast<df_shell *>(
            wl_registry_bind(m_registry, m_shellName, &df_shell_interface,
                             std::min(m_shellVersion,
                                      static_cast<uint32_t>(df_shell_interface.version))));
    }
    if (m_managerName) {
        // Bind through the generated interface's own version, so a lockstep
    // protocol bump never needs a second hard-coded cap here (T-08.2c caught
    // the v4 cap rejecting the v5 request at runtime).
    m_manager = static_cast<df_toplevel_manager *>(
        wl_registry_bind(m_registry, m_managerName, &df_toplevel_manager_interface,
                         std::min(m_managerVersion,
                                  static_cast<uint32_t>(df_toplevel_manager_interface.version))));
        static const df_toplevel_manager_listener managerListener = {
            onManagerOutput,
            onManagerWorkspace,
            onManagerToplevel,
            onManagerWorkspaceActivated,
            onManagerFocused,
            onManagerAttention,
            onManagerHotCorner,
            onManagerOverview,
            onManagerAppSwitcher,
            onManagerInputAction,
            onManagerProgress,
            onManagerAppAccelerator,
            onManagerDone,
            onManagerAppSwitcherEntry,
            onManagerScreenshotSaved,
            onManagerScreenshotFailed,
            onManagerActivationResult,
        };
        df_toplevel_manager_add_listener(m_manager, &managerListener, this);
    }
    m_trustedGlobalsBound = true;
}

bool ShellProtocol::createMenuBarSurface(int height, int exclusiveZone)
{
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_surface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_surface);
    m_layer = df_shell_get_layer_surface(m_shell, m_surface, nullptr, DF_SHELL_LAYER_TOP,
                                         "menubar");
    if (!m_layer)
        return fail(QStringLiteral("compositor refused the menu-bar layer surface"));
    static const df_layer_surface_listener layerListener = { onLayerConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_layer, &layerListener, this);

    df_layer_surface_set_anchor(m_layer, kAnchorTop | kAnchorLeft | kAnchorRight);
    df_layer_surface_set_size(m_layer, 0, height);
    df_layer_surface_set_exclusive_zone(m_layer, exclusiveZone);
    df_layer_surface_set_keyboard_interaction(
        m_layer, DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    // T201: the policy may have been applied from settings before the surface
    // existed; send the latest value with the first commit.
    df_layer_surface_set_auto_hide(m_layer, m_menuBarAutoHide);
    wl_surface_commit(m_surface);

    if (wl_display_roundtrip(m_display) < 0)
        return fail(QStringLiteral("menu-bar configure roundtrip failed"));
    return true;
}

void ShellProtocol::setMenuBarFrameGeometry(bool active, int width, int height,
                                             int cornerRadius, bool lightVeil)
{
    if (!m_layer)
        return;
    if (!active) {
        // An explicit empty set is the contract for "no material": it cannot
        // fall back to a stale panel or the derived reserved strip (T197).
        // Send it on the first inactive frame too, so a bar that starts
        // hidden/fallback never exposes the compositor's derived panel.
        if (!m_menuBarPanelInitialized || m_menuBarPanelActive) {
            df_layer_surface_clear_panel_regions(m_layer);
            m_menuBarPanelInitialized = true;
            m_menuBarPanelDirty = true;
            m_menuBarPanelActive = false;
            m_menuBarPanelWidth = 0;
            m_menuBarPanelHeight = 0;
            m_menuBarPanelRadius = 0;
        }
        return;
    }
    const int panelWidth = qMax(1, width);
    const int panelHeight = qMax(1, height);
    const int radius = qMax(0, cornerRadius);
    if (m_menuBarPanelActive && panelWidth == m_menuBarPanelWidth
            && panelHeight == m_menuBarPanelHeight
        && radius == m_menuBarPanelRadius && lightVeil == m_menuBarLightVeil)
        return;
    // Polarity travels with the content commit so lettering and veil agree.
    // Clear the previous key when polarity changes; regions otherwise append.
    if (m_menuBarPanelActive && lightVeil != m_menuBarLightVeil)
        df_layer_surface_clear_panel_regions(m_layer);
    df_layer_surface_set_panel_region(m_layer, lightVeil ? "veil-light" : "veil-dark",
                                     "menuBarVeil", 0, 0,
                                      panelWidth, panelHeight, radius, radius,
                                      radius, radius, 0);
    m_menuBarPanelInitialized = true;
    m_menuBarPanelDirty = true;
    m_menuBarPanelActive = true;
    m_menuBarPanelWidth = panelWidth;
    m_menuBarPanelHeight = panelHeight;
    m_menuBarPanelRadius = radius;
    m_menuBarLightVeil = lightVeil;
}

void ShellProtocol::setMenuBarAutoHide(const QString &mode)
{
    uint32_t wire = DF_LAYER_SURFACE_AUTO_HIDE_MODE_FULL_SCREEN;
    if (mode == QLatin1String("never"))
        wire = DF_LAYER_SURFACE_AUTO_HIDE_MODE_NEVER;
    else if (mode == QLatin1String("always"))
        wire = DF_LAYER_SURFACE_AUTO_HIDE_MODE_ALWAYS;
    if (wire == m_menuBarAutoHide)
        return;
    m_menuBarAutoHide = wire;
    if (!m_layer)
        return;
    df_layer_surface_set_auto_hide(m_layer, wire);
    if (m_display)
        wl_display_flush(m_display);
}

bool ShellProtocol::createPopupSurface()
{
    if (m_popupSurface || m_popupLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_popupSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_popupSurface);
    m_popupLayer = df_shell_get_layer_surface(m_shell, m_popupSurface, nullptr,
                                              DF_SHELL_LAYER_OVERLAY, "menubar-popup");
    if (!m_popupLayer)
        return fail(QStringLiteral("compositor refused the menu-popup layer surface"));
    static const df_layer_surface_listener popupListener = { onPopupConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_popupLayer, &popupListener, this);

    // Anchor top|left, positioned by margins; no reserved zone (the design
    // lays transient menus out of the reserved-zone accounting) and no
    // keyboard (the bar surface keeps focus for Escape/menu navigation).
    static const uint32_t kAnchorTopLeft = kAnchorTop | kAnchorLeft;
    df_layer_surface_set_anchor(m_popupLayer, kAnchorTopLeft);
    df_layer_surface_set_exclusive_zone(m_popupLayer, -1);
    df_layer_surface_set_keyboard_interaction(m_popupLayer,
                                              DF_LAYER_SURFACE_KEYBOARD_INTERACTION_NONE);
    // Unmapped until the first menu opens.
    wl_surface_attach(m_popupSurface, nullptr, 0, 0);
    wl_surface_commit(m_popupSurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the popup surface creation"));
    return true;
}

bool ShellProtocol::setPopupGeometry(int x, int y, int width, int height)
{
    if (!m_popupLayer || !m_popupSurface)
        return false;
    const QRect geometry(x, y, width, height);
    if (m_popupGeometry == geometry)
        return true;
    m_popupGeometry = geometry;
    m_popupX = x;
    m_popupY = y;
    df_layer_surface_set_margin(m_popupLayer, y, 0, 0, x);
    df_layer_surface_set_size(m_popupLayer, width, height);
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

static void setMenuPanelRegions(df_layer_surface *layer, const QVariantList &regions)
{
    if (!layer)
        return;
    df_layer_surface_clear_panel_regions(layer);
    for (const QVariant &value : regions) {
        const QVariantMap region = value.toMap();
        const QString key = region.value(QStringLiteral("key")).toString();
        const QString preset = region.value(QStringLiteral("preset")).toString();
        const qreal regionX = region.value(QStringLiteral("x")).toReal();
        const qreal regionY = region.value(QStringLiteral("y")).toReal();
        const int x = qFloor(regionX);
        const int y = qFloor(regionY);
        const int width = qCeil(regionX + region.value(QStringLiteral("w")).toReal()) - x;
        const int height = qCeil(regionY + region.value(QStringLiteral("h")).toReal()) - y;
        if (key.isEmpty() || width <= 0 || height <= 0)
            continue;
        df_layer_surface_set_panel_region(
            layer, key.toUtf8().constData(), preset.toUtf8().constData(),
            x, y, width, height,
            qRound(region.value(QStringLiteral("tl")).toReal()),
            qRound(region.value(QStringLiteral("tr")).toReal()),
            qRound(region.value(QStringLiteral("br")).toReal()),
            qRound(region.value(QStringLiteral("bl")).toReal()),
            qRound(region.value(QStringLiteral("overscan")).toReal()));
    }
}

void ShellProtocol::setPopupPanelRegions(const QVariantList &regions)
{
    if (!m_popupLayer || m_popupPanelRegions == regions)
        return;
    setMenuPanelRegions(m_popupLayer, regions);
    m_popupPanelRegions = regions;
}

void ShellProtocol::clearPopupPanelRegions()
{
    setPopupPanelRegions({});
}

void ShellProtocol::setPopupInputActive(bool active)
{
    if (!m_popupSurface || !m_compositor)
        return;
    if (active) {
        wl_surface_set_input_region(m_popupSurface, nullptr);
        return;
    }
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (!region)
        return;
    wl_surface_set_input_region(m_popupSurface, region);
    wl_region_destroy(region);
}

bool ShellProtocol::commitPopupImage(const QImage &image, const QSize &logicalSize,
                                     const QRect &sourceRect)
{
    if (!m_popupSurface)
        return false;
    if (!commitTo(m_popupSurface, image, logicalSize, sourceRect))
        return false;
    m_popupMapped = true;
    return true;
}

bool ShellProtocol::hidePopup()
{
    if (!m_popupSurface || !m_popupLayer)
        return false;
    if (!m_popupMapped)
        return true;
    cancelPendingFrame(QStringLiteral("menubar-popup"));
    wl_surface_attach(m_popupSurface, nullptr, 0, 0);
    wl_surface_commit(m_popupSurface);
    resetSurfaceScale(m_popupSurface);
    m_popupMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::commitImage(const QImage &image, const QSize &logicalSize,
                                const QRect &sourceRect)
{
    if (!m_surface)
        return false;
    return commitTo(m_surface, image, logicalSize, sourceRect);
}

bool ShellProtocol::createOverviewSurface()
{
    if (m_overviewSurface || m_overviewLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_overviewSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_overviewSurface);
    m_overviewLayer = df_shell_get_layer_surface(m_shell, m_overviewSurface, nullptr,
                                                 DF_SHELL_LAYER_OVERLAY, "overview");
    if (!m_overviewLayer)
        return fail(QStringLiteral("compositor refused the overview layer surface"));
    static const df_layer_surface_listener listener = { onOverviewConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_overviewLayer, &listener, this);

    // Cover the whole output; reserve nothing (the overview is transient) and
    // take the keyboard on demand so Escape can dismiss it. It is mapped only
    // while the overview is open, so it never blocks the normal scene.
    df_layer_surface_set_anchor(m_overviewLayer,
                                kAnchorTop | kAnchorBottom | kAnchorLeft | kAnchorRight);
    df_layer_surface_set_exclusive_zone(m_overviewLayer, -1);
    df_layer_surface_set_keyboard_interaction(
        m_overviewLayer, DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    wl_surface_attach(m_overviewSurface, nullptr, 0, 0);
    wl_surface_commit(m_overviewSurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the overview surface creation"));
    return true;
}

bool ShellProtocol::commitOverviewImage(const QImage &image, const QSize &logicalSize)
{
    if (!m_overviewSurface)
        return false;
    if (!commitTo(m_overviewSurface, image, logicalSize))
        return false;
    m_overviewMapped = true;
    return true;
}

bool ShellProtocol::hideOverview()
{
    if (!m_overviewSurface || !m_overviewLayer)
        return false;
    if (!m_overviewMapped)
        return true;
    cancelPendingFrame(QStringLiteral("overview"));
    wl_surface_attach(m_overviewSurface, nullptr, 0, 0);
    wl_surface_commit(m_overviewSurface);
    resetSurfaceScale(m_overviewSurface);
    m_overviewMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::createSwitcherSurface()
{
    if (m_switcherSurface || m_switcherLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_switcherSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_switcherSurface);
    m_switcherLayer = df_shell_get_layer_surface(m_shell, m_switcherSurface, nullptr,
                                                 DF_SHELL_LAYER_OVERLAY, "app-switcher");
    if (!m_switcherLayer)
        return fail(QStringLiteral("compositor refused the app-switcher layer surface"));
    static const df_layer_surface_listener listener = { onSwitcherConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_switcherLayer, &listener, this);

    // Full output, no reserved zone, no keyboard: the compositor owns the
    // Cmd-Tab chord, so the overlay is a pure projection. It is mapped only
    // while the switcher is open, so it never blocks the normal scene.
    df_layer_surface_set_anchor(m_switcherLayer,
                                kAnchorTop | kAnchorBottom | kAnchorLeft | kAnchorRight);
    df_layer_surface_set_exclusive_zone(m_switcherLayer, -1);
    df_layer_surface_set_keyboard_interaction(
        m_switcherLayer, DF_LAYER_SURFACE_KEYBOARD_INTERACTION_NONE);
    wl_surface_attach(m_switcherSurface, nullptr, 0, 0);
    wl_surface_commit(m_switcherSurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the app-switcher surface creation"));
    return true;
}

bool ShellProtocol::commitSwitcherImage(const QImage &image, const QSize &logicalSize)
{
    if (!m_switcherSurface)
        return false;
    if (!commitTo(m_switcherSurface, image, logicalSize))
        return false;
    m_switcherMapped = true;
    return true;
}

bool ShellProtocol::hideSwitcher()
{
    if (!m_switcherSurface || !m_switcherLayer)
        return false;
    if (!m_switcherMapped)
        return true;
    cancelPendingFrame(QStringLiteral("switcher"));
    wl_surface_attach(m_switcherSurface, nullptr, 0, 0);
    wl_surface_commit(m_switcherSurface);
    resetSurfaceScale(m_switcherSurface);
    m_switcherMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::createBannerSurface(int width, int height, int topMargin)
{
    if (m_bannerSurface || m_bannerLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_bannerSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_bannerSurface);
    m_bannerLayer = df_shell_get_layer_surface(m_shell, m_bannerSurface, nullptr,
                                               DF_SHELL_LAYER_OVERLAY, "notification");
    if (!m_bannerLayer)
        return fail(QStringLiteral("compositor refused the notification layer surface"));
    static const df_layer_surface_listener listener = { onBannerConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_bannerLayer, &listener, this);

    // Top-right corner, below the menu bar; reserve nothing (a banner is
    // transient) and never take keyboard. Clicks pass through until T-11.1b
    // adds banner activation.
    df_layer_surface_set_anchor(m_bannerLayer, kAnchorRight | kAnchorTop);
    df_layer_surface_set_margin(m_bannerLayer, topMargin, kBannerMargin, 0, 0);
    df_layer_surface_set_size(m_bannerLayer, width, height);
    df_layer_surface_set_exclusive_zone(m_bannerLayer, -1);
    df_layer_surface_set_keyboard_interaction(m_bannerLayer,
                                              DF_LAYER_SURFACE_KEYBOARD_INTERACTION_NONE);
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (region) {
        wl_surface_set_input_region(m_bannerSurface, region);
        wl_region_destroy(region);
    }
    wl_surface_attach(m_bannerSurface, nullptr, 0, 0);
    wl_surface_commit(m_bannerSurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the notification surface creation"));
    return true;
}

bool ShellProtocol::commitBannerImage(const QImage &image, const QSize &logicalSize)
{
    if (!m_bannerSurface)
        return false;
    if (!commitTo(m_bannerSurface, image, logicalSize))
        return false;
    m_bannerMapped = true;
    return true;
}

bool ShellProtocol::hideBanner()
{
    if (!m_bannerSurface || !m_bannerLayer)
        return false;
    if (!m_bannerMapped)
        return true;
    cancelPendingFrame(QStringLiteral("banner"));
    wl_surface_attach(m_bannerSurface, nullptr, 0, 0);
    wl_surface_commit(m_bannerSurface);
    resetSurfaceScale(m_bannerSurface);
    m_bannerMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::setBannerInputRegion(int width, int height)
{
    if (!m_bannerSurface || !m_compositor)
        return false;
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (!region)
        return false;
    if (width > 0 && height > 0)
        wl_region_add(region, 0, 0, width, height);
    // Applied with the next buffer commit (`commitBannerImage`).
    wl_surface_set_input_region(m_bannerSurface, region);
    wl_region_destroy(region);
    return true;
}

void ShellProtocol::setBannerPanelRegions(const QVariantList &regions)
{
    applyPanelRegions(m_bannerLayer, regions);
}

void ShellProtocol::clearBannerPanelRegions()
{
    if (m_bannerLayer)
        df_layer_surface_clear_panel_regions(m_bannerLayer);
}

// --- Notification Center overlay (T204) ------------------------------------

bool ShellProtocol::createNotificationCenterSurface(int width, int height, int topMargin)
{
    if (m_notificationCenterSurface || m_notificationCenterLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_notificationCenterSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_notificationCenterSurface);
    m_notificationCenterLayer = df_shell_get_layer_surface(
        m_shell, m_notificationCenterSurface, nullptr, DF_SHELL_LAYER_OVERLAY,
        "notification-center");
    if (!m_notificationCenterLayer)
        return fail(QStringLiteral("compositor refused the notification-center layer surface"));
    static const df_layer_surface_listener listener = { onNotificationCenterConfigure,
                                                        onNotificationCenterLayerClosed };
    df_layer_surface_add_listener(m_notificationCenterLayer, &listener, this);

    // Top-right corner, below the menu bar. The overlay is per-output: with no
    // explicit output the compositor places it on the output the invoking
    // interaction happened on (the clock click's chrome focus, or the pointer
    // output for a keyboard/hot-corner trigger). Reserve nothing and take the
    // keyboard on demand so Escape, click-away and traversal work.
    df_layer_surface_set_anchor(m_notificationCenterLayer, kAnchorRight | kAnchorTop);
    df_layer_surface_set_margin(m_notificationCenterLayer, topMargin, kBannerMargin, 0, 0);
    df_layer_surface_set_size(m_notificationCenterLayer, width, height);
    df_layer_surface_set_exclusive_zone(m_notificationCenterLayer, -1);
    df_layer_surface_set_keyboard_interaction(
        m_notificationCenterLayer, DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    // Everything passes through until the Center's own input region is applied
    // with its first committed buffer.
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (region) {
        wl_surface_set_input_region(m_notificationCenterSurface, region);
        wl_region_destroy(region);
    }
    wl_surface_attach(m_notificationCenterSurface, nullptr, 0, 0);
    wl_surface_commit(m_notificationCenterSurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the notification-center surface creation"));
    return true;
}

bool ShellProtocol::setNotificationCenterSize(int width, int height)
{
    if (!m_notificationCenterLayer || width <= 0 || height <= 0)
        return false;
    df_layer_surface_set_size(m_notificationCenterLayer, width, height);
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::setNotificationCenterInputRegion(int width, int height)
{
    if (!m_notificationCenterSurface || !m_compositor)
        return false;
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (!region)
        return false;
    if (width > 0 && height > 0)
        wl_region_add(region, 0, 0, width, height);
    // Applied with the next buffer commit (`commitNotificationCenterImage`).
    wl_surface_set_input_region(m_notificationCenterSurface, region);
    wl_region_destroy(region);
    return true;
}

bool ShellProtocol::setNotificationCenterKeyboardGrab(bool grab)
{
    if (!m_notificationCenterLayer)
        return false;
    df_layer_surface_set_keyboard_interaction(
        m_notificationCenterLayer,
        grab ? DF_LAYER_SURFACE_KEYBOARD_INTERACTION_EXCLUSIVE
             : DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

void ShellProtocol::setNotificationCenterPanelRegions(const QVariantList &regions)
{
    applyPanelRegions(m_notificationCenterLayer, regions);
}

void ShellProtocol::clearNotificationCenterPanelRegions()
{
    if (m_notificationCenterLayer)
        df_layer_surface_clear_panel_regions(m_notificationCenterLayer);
}

bool ShellProtocol::commitNotificationCenterImage(const QImage &image,
                                                  const QSize &logicalSize)
{
    if (!m_notificationCenterSurface)
        return false;
    if (!commitTo(m_notificationCenterSurface, image, logicalSize))
        return false;
    m_notificationCenterMapped = true;
    return true;
}

bool ShellProtocol::hideNotificationCenter()
{
    if (!m_notificationCenterSurface || !m_notificationCenterLayer)
        return false;
    if (!m_notificationCenterMapped)
        return true;
    cancelPendingFrame(QStringLiteral("notification-center"));
    wl_surface_attach(m_notificationCenterSurface, nullptr, 0, 0);
    wl_surface_commit(m_notificationCenterSurface);
    resetSurfaceScale(m_notificationCenterSurface);
    m_notificationCenterMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

// --- Control Center overlay (T-11.3a) --------------------------------------

bool ShellProtocol::createControlCenterSurface(int width, int height, int topMargin)
{
    if (m_controlCenterSurface || m_controlCenterLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_controlCenterSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_controlCenterSurface);
    m_controlCenterLayer = df_shell_get_layer_surface(m_shell, m_controlCenterSurface, nullptr,
                                                      DF_SHELL_LAYER_OVERLAY, "control-center");
    if (!m_controlCenterLayer)
        return fail(QStringLiteral("compositor refused the control-center layer surface"));
    static const df_layer_surface_listener listener = { onControlCenterConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_controlCenterLayer, &listener, this);

    // Top-right corner, below the menu bar; reserve nothing (the panel is
    // transient) and take the keyboard on demand so Escape can dismiss it and
    // a click elsewhere (which moves chrome keyboard focus away) can too.
    df_layer_surface_set_anchor(m_controlCenterLayer, kAnchorRight | kAnchorTop);
    df_layer_surface_set_margin(m_controlCenterLayer, topMargin, kBannerMargin, 0, 0);
    df_layer_surface_set_size(m_controlCenterLayer, width, height);
    df_layer_surface_set_exclusive_zone(m_controlCenterLayer, -1);
    df_layer_surface_set_keyboard_interaction(
        m_controlCenterLayer, DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    // Everything passes through until the panel's own input region is applied
    // with its first committed buffer.
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (region) {
        wl_surface_set_input_region(m_controlCenterSurface, region);
        wl_region_destroy(region);
    }
    wl_surface_attach(m_controlCenterSurface, nullptr, 0, 0);
    wl_surface_commit(m_controlCenterSurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the control-center surface creation"));
    return true;
}

bool ShellProtocol::setControlCenterSize(int width, int height)
{
    if (!m_controlCenterLayer || width <= 0 || height <= 0)
        return false;
    df_layer_surface_set_size(m_controlCenterLayer, width, height);
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::setControlCenterInputRegion(int width, int height)
{
    if (!m_controlCenterSurface || !m_compositor)
        return false;
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (!region)
        return false;
    if (width > 0 && height > 0)
        wl_region_add(region, 0, 0, width, height);
    // Applied with the next buffer commit (`commitControlCenterImage`).
    wl_surface_set_input_region(m_controlCenterSurface, region);
    wl_region_destroy(region);
    return true;
}

void ShellProtocol::setControlCenterPanelRegions(const QVariantList &regions)
{
    applyPanelRegions(m_controlCenterLayer, regions);
}

void ShellProtocol::clearControlCenterPanelRegions()
{
    if (m_controlCenterLayer)
        df_layer_surface_clear_panel_regions(m_controlCenterLayer);
}

bool ShellProtocol::commitControlCenterImage(const QImage &image, const QSize &logicalSize)
{
    if (!m_controlCenterSurface)
        return false;
    if (!commitTo(m_controlCenterSurface, image, logicalSize))
        return false;
    m_controlCenterMapped = true;
    return true;
}

bool ShellProtocol::hideControlCenter()
{
    if (!m_controlCenterSurface || !m_controlCenterLayer)
        return false;
    if (!m_controlCenterMapped)
        return true;
    cancelPendingFrame(QStringLiteral("control-center"));
    wl_surface_attach(m_controlCenterSurface, nullptr, 0, 0);
    wl_surface_commit(m_controlCenterSurface);
    resetSurfaceScale(m_controlCenterSurface);
    m_controlCenterMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

// --- OSD overlay (T-11.4a) --------------------------------------------------

bool ShellProtocol::createOsdSurface(int width, int height)
{
    if (m_osdSurface || m_osdLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_osdSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_osdSurface);
    m_osdLayer = df_shell_get_layer_surface(m_shell, m_osdSurface, nullptr,
                                            DF_SHELL_LAYER_OVERLAY, "osd");
    if (!m_osdLayer)
        return fail(QStringLiteral("compositor refused the OSD layer surface"));
    static const df_layer_surface_listener listener = { onOsdConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_osdLayer, &listener, this);

    // No anchors: the compositor centers the surface on the output. Reserve
    // nothing, never take keyboard, and pass every input event through (the
    // OSD is non-interactive).
    df_layer_surface_set_size(m_osdLayer, width, height);
    df_layer_surface_set_exclusive_zone(m_osdLayer, -1);
    df_layer_surface_set_keyboard_interaction(m_osdLayer,
                                              DF_LAYER_SURFACE_KEYBOARD_INTERACTION_NONE);
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (region) {
        wl_surface_set_input_region(m_osdSurface, region);
        wl_region_destroy(region);
    }
    wl_surface_attach(m_osdSurface, nullptr, 0, 0);
    wl_surface_commit(m_osdSurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the OSD surface creation"));
    return true;
}

bool ShellProtocol::setOsdPanelRect(int x, int y, int width, int height)
{
    if (!m_osdLayer)
        return false;
    df_layer_surface_set_panel_rect(m_osdLayer, x, y, width, height);
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::commitOsdImage(const QImage &image, const QSize &logicalSize)
{
    if (!m_osdSurface)
        return false;
    if (!commitTo(m_osdSurface, image, logicalSize))
        return false;
    m_osdMapped = true;
    return true;
}

bool ShellProtocol::hideOsd()
{
    if (!m_osdSurface || !m_osdLayer)
        return false;
    if (!m_osdMapped)
        return true;
    cancelPendingFrame(QStringLiteral("osd"));
    wl_surface_attach(m_osdSurface, nullptr, 0, 0);
    wl_surface_commit(m_osdSurface);
    resetSurfaceScale(m_osdSurface);
    m_osdMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

// --- FileChooser picker overlay (T-13.2b) ----------------------------------

bool ShellProtocol::createChooserSurface(int width, int height)
{
    if (m_chooserSurface || m_chooserLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_chooserSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_chooserSurface);
    m_chooserLayer = df_shell_get_layer_surface(m_shell, m_chooserSurface, nullptr,
                                                DF_SHELL_LAYER_OVERLAY, "file-chooser");
    if (!m_chooserLayer)
        return fail(QStringLiteral("compositor refused the file-chooser layer surface"));
    static const df_layer_surface_listener listener = { onChooserConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_chooserLayer, &listener, this);

    // No anchors: the compositor centers the dialog on the output. Reserve
    // nothing; take the keyboard on demand so Escape rejects and a click
    // elsewhere (which moves chrome keyboard focus away) dismisses it.
    df_layer_surface_set_size(m_chooserLayer, width, height);
    df_layer_surface_set_exclusive_zone(m_chooserLayer, -1);
    df_layer_surface_set_keyboard_interaction(m_chooserLayer,
                                              DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    // Everything passes through until the dialog's own input region is applied
    // with its first committed buffer.
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (region) {
        wl_surface_set_input_region(m_chooserSurface, region);
        wl_region_destroy(region);
    }
    wl_surface_attach(m_chooserSurface, nullptr, 0, 0);
    wl_surface_commit(m_chooserSurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the file-chooser surface creation"));
    return true;
}

bool ShellProtocol::setChooserInputRegion(int width, int height)
{
    if (!m_chooserSurface || !m_compositor)
        return false;
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (!region)
        return false;
    if (width > 0 && height > 0)
        wl_region_add(region, 0, 0, width, height);
    // Applied with the next buffer commit (`commitChooserImage`).
    wl_surface_set_input_region(m_chooserSurface, region);
    wl_region_destroy(region);
    return true;
}

bool ShellProtocol::commitChooserImage(const QImage &image, const QSize &logicalSize)
{
    if (!m_chooserSurface)
        return false;
    if (!commitTo(m_chooserSurface, image, logicalSize))
        return false;
    m_chooserMapped = true;
    return true;
}

bool ShellProtocol::hideChooser()
{
    if (!m_chooserSurface || !m_chooserLayer)
        return false;
    if (!m_chooserMapped)
        return true;
    cancelPendingFrame(QStringLiteral("chooser"));
    wl_surface_attach(m_chooserSurface, nullptr, 0, 0);
    wl_surface_commit(m_chooserSurface);
    resetSurfaceScale(m_chooserSurface);
    m_chooserMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

// --- widget gallery (T234) --------------------------------------------------

bool ShellProtocol::createWidgetGallerySurface(int width, int height)
{
    if (m_widgetGallerySurface || m_widgetGalleryLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_widgetGallerySurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_widgetGallerySurface);
    m_widgetGalleryLayer = df_shell_get_layer_surface(m_shell, m_widgetGallerySurface, nullptr,
                                                      DF_SHELL_LAYER_OVERLAY,
                                                      "widget-gallery");
    if (!m_widgetGalleryLayer)
        return fail(QStringLiteral("compositor refused the widget-gallery layer surface"));
    static const df_layer_surface_listener listener = { onWidgetGalleryConfigure,
                                                        onLayerClosed };
    df_layer_surface_add_listener(m_widgetGalleryLayer, &listener, this);

    // No anchors: the compositor centers the measured gallery window. Reserve
    // nothing; take the keyboard on demand so Escape and click-away dismiss it.
    df_layer_surface_set_size(m_widgetGalleryLayer, width, height);
    df_layer_surface_set_exclusive_zone(m_widgetGalleryLayer, -1);
    df_layer_surface_set_keyboard_interaction(
        m_widgetGalleryLayer, DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (region) {
        wl_surface_set_input_region(m_widgetGallerySurface, region);
        wl_region_destroy(region);
    }
    wl_surface_attach(m_widgetGallerySurface, nullptr, 0, 0);
    wl_surface_commit(m_widgetGallerySurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the widget-gallery surface creation"));
    return true;
}

bool ShellProtocol::setWidgetGalleryInputRegion(int width, int height)
{
    if (!m_widgetGallerySurface || !m_compositor)
        return false;
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (!region)
        return false;
    if (width > 0 && height > 0)
        wl_region_add(region, 0, 0, width, height);
    wl_surface_set_input_region(m_widgetGallerySurface, region);
    wl_region_destroy(region);
    return true;
}

bool ShellProtocol::commitWidgetGalleryImage(const QImage &image,
                                             const QSize &logicalSize)
{
    if (!m_widgetGallerySurface)
        return false;
    if (!commitTo(m_widgetGallerySurface, image, logicalSize))
        return false;
    m_widgetGalleryMapped = true;
    return true;
}

bool ShellProtocol::hideWidgetGallery()
{
    if (!m_widgetGallerySurface || !m_widgetGalleryLayer)
        return false;
    if (!m_widgetGalleryMapped)
        return true;
    cancelPendingFrame(QStringLiteral("widget-gallery"));
    wl_surface_attach(m_widgetGallerySurface, nullptr, 0, 0);
    wl_surface_commit(m_widgetGallerySurface);
    resetSurfaceScale(m_widgetGallerySurface);
    m_widgetGalleryMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

// --- screenshot selection overlay (T-13.3a) ---------------------------------

bool ShellProtocol::createScreenshotSurface()
{
    if (m_screenshotSurface || m_screenshotLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_screenshotSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_screenshotSurface);
    m_screenshotLayer = df_shell_get_layer_surface(m_shell, m_screenshotSurface, nullptr,
                                                   DF_SHELL_LAYER_OVERLAY, "screenshot");
    if (!m_screenshotLayer)
        return fail(QStringLiteral("compositor refused the screenshot layer surface"));
    static const df_layer_surface_listener listener = { onScreenshotConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_screenshotLayer, &listener, this);

    // Cover the whole output: the selection is drawn over the live desktop.
    // Reserve nothing (the overlay is transient) and take the keyboard on
    // demand so Escape cancels and Return accepts.
    df_layer_surface_set_anchor(m_screenshotLayer,
                                kAnchorTop | kAnchorBottom | kAnchorLeft | kAnchorRight);
    df_layer_surface_set_exclusive_zone(m_screenshotLayer, -1);
    df_layer_surface_set_keyboard_interaction(
        m_screenshotLayer, DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    wl_surface_attach(m_screenshotSurface, nullptr, 0, 0);
    wl_surface_commit(m_screenshotSurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the screenshot surface creation"));
    return true;
}

bool ShellProtocol::setScreenshotInputRegion(int width, int height)
{
    if (!m_screenshotSurface || !m_compositor)
        return false;
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (!region)
        return false;
    if (width > 0 && height > 0)
        wl_region_add(region, 0, 0, width, height);
    // Applied with the next buffer commit (`commitScreenshotImage`).
    wl_surface_set_input_region(m_screenshotSurface, region);
    wl_region_destroy(region);
    return true;
}

bool ShellProtocol::commitScreenshotImage(const QImage &image, const QSize &logicalSize)
{
    if (!m_screenshotSurface)
        return false;
    if (!commitTo(m_screenshotSurface, image, logicalSize))
        return false;
    m_screenshotMapped = true;
    return true;
}

bool ShellProtocol::hideScreenshot()
{
    if (!m_screenshotSurface || !m_screenshotLayer)
        return false;
    if (!m_screenshotMapped)
        return true;
    cancelPendingFrame(QStringLiteral("screenshot"));
    wl_surface_attach(m_screenshotSurface, nullptr, 0, 0);
    wl_surface_commit(m_screenshotSurface);
    resetSurfaceScale(m_screenshotSurface);
    m_screenshotMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

// --- ScreenCast source picker overlay (T-13.4a) -----------------------------

bool ShellProtocol::createScreenCastSurface(int width, int height)
{
    if (m_screencastSurface || m_screencastLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_screencastSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_screencastSurface);
    m_screencastLayer = df_shell_get_layer_surface(m_shell, m_screencastSurface, nullptr,
                                                   DF_SHELL_LAYER_OVERLAY, "screencast");
    if (!m_screencastLayer)
        return fail(QStringLiteral("compositor refused the screencast layer surface"));
    static const df_layer_surface_listener listener = { onScreenCastConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_screencastLayer, &listener, this);

    // No anchors: the compositor centers the picker on the output. Reserve
    // nothing; take the keyboard on demand so Escape cancels, Return accepts,
    // and a click elsewhere (which moves chrome keyboard focus away) dismisses.
    df_layer_surface_set_size(m_screencastLayer, width, height);
    df_layer_surface_set_exclusive_zone(m_screencastLayer, -1);
    df_layer_surface_set_keyboard_interaction(
        m_screencastLayer, DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    // Everything passes through until the picker's own input region is applied
    // with its first committed buffer.
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (region) {
        wl_surface_set_input_region(m_screencastSurface, region);
        wl_region_destroy(region);
    }
    wl_surface_attach(m_screencastSurface, nullptr, 0, 0);
    wl_surface_commit(m_screencastSurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the screencast surface creation"));
    return true;
}

bool ShellProtocol::setScreenCastInputRegion(int width, int height)
{
    if (!m_screencastSurface || !m_compositor)
        return false;
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (!region)
        return false;
    if (width > 0 && height > 0)
        wl_region_add(region, 0, 0, width, height);
    // Applied with the next buffer commit (`commitScreenCastImage`).
    wl_surface_set_input_region(m_screencastSurface, region);
    wl_region_destroy(region);
    return true;
}

bool ShellProtocol::commitScreenCastImage(const QImage &image, const QSize &logicalSize)
{
    if (!m_screencastSurface)
        return false;
    if (!commitTo(m_screencastSurface, image, logicalSize))
        return false;
    m_screencastMapped = true;
    return true;
}

bool ShellProtocol::hideScreenCast()
{
    if (!m_screencastSurface || !m_screencastLayer)
        return false;
    if (!m_screencastMapped)
        return true;
    cancelPendingFrame(QStringLiteral("screencast"));
    wl_surface_attach(m_screencastSurface, nullptr, 0, 0);
    wl_surface_commit(m_screencastSurface);
    resetSurfaceScale(m_screencastSurface);
    m_screencastMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

// --- polkit authentication dialog overlay (T-13.6) ---------------------------

bool ShellProtocol::createPolkitSurface(int width, int height)
{
    if (m_polkitSurface || m_polkitLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_polkitSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_polkitSurface);
    m_polkitLayer = df_shell_get_layer_surface(m_shell, m_polkitSurface, nullptr,
                                               DF_SHELL_LAYER_OVERLAY, "polkit");
    if (!m_polkitLayer)
        return fail(QStringLiteral("compositor refused the polkit layer surface"));
    static const df_layer_surface_listener listener = { onPolkitConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_polkitLayer, &listener, this);

    // No anchors: the compositor centers the dialog on the output. Reserve
    // nothing; take the keyboard on demand so Escape declines and the password
    // keys reach the agent.
    df_layer_surface_set_size(m_polkitLayer, width, height);
    df_layer_surface_set_exclusive_zone(m_polkitLayer, -1);
    df_layer_surface_set_keyboard_interaction(
        m_polkitLayer, DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (region) {
        wl_surface_set_input_region(m_polkitSurface, region);
        wl_region_destroy(region);
    }
    wl_surface_attach(m_polkitSurface, nullptr, 0, 0);
    wl_surface_commit(m_polkitSurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the polkit surface creation"));
    return true;
}

bool ShellProtocol::setPolkitInputRegion(int width, int height)
{
    if (!m_polkitSurface || !m_compositor)
        return false;
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (!region)
        return false;
    if (width > 0 && height > 0)
        wl_region_add(region, 0, 0, width, height);
    wl_surface_set_input_region(m_polkitSurface, region);
    wl_region_destroy(region);
    return true;
}

bool ShellProtocol::commitPolkitImage(const QImage &image, const QSize &logicalSize)
{
    if (!m_polkitSurface)
        return false;
    if (!commitTo(m_polkitSurface, image, logicalSize))
        return false;
    m_polkitMapped = true;
    return true;
}

bool ShellProtocol::hidePolkit()
{
    if (!m_polkitSurface || !m_polkitLayer)
        return false;
    if (!m_polkitMapped)
        return true;
    cancelPendingFrame(QStringLiteral("polkit"));
    wl_surface_attach(m_polkitSurface, nullptr, 0, 0);
    wl_surface_commit(m_polkitSurface);
    resetSurfaceScale(m_polkitSurface);
    m_polkitMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::createAppsDrawerSurface()
{
    if (m_appsDrawerSurface || m_appsDrawerLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_appsDrawerSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_appsDrawerSurface);
    m_appsDrawerLayer = df_shell_get_layer_surface(m_shell, m_appsDrawerSurface, nullptr,
                                                   DF_SHELL_LAYER_OVERLAY, "apps-drawer");
    if (!m_appsDrawerLayer)
        return fail(QStringLiteral("compositor refused the apps-drawer layer surface"));
    static const df_layer_surface_listener listener = { onAppsDrawerConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_appsDrawerLayer, &listener, this);

    // Cover the whole output; reserve nothing (the drawer is transient) and
    // take the keyboard on demand so Escape dismisses and arrow/Return reaches
    // the grid's navigation.
    df_layer_surface_set_anchor(m_appsDrawerLayer,
                                kAnchorTop | kAnchorBottom | kAnchorLeft | kAnchorRight);
    df_layer_surface_set_exclusive_zone(m_appsDrawerLayer, -1);
    df_layer_surface_set_keyboard_interaction(
        m_appsDrawerLayer, DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    wl_surface_attach(m_appsDrawerSurface, nullptr, 0, 0);
    wl_surface_commit(m_appsDrawerSurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the apps-drawer surface creation"));
    return true;
}

bool ShellProtocol::setAppsDrawerInputRegion(int width, int height)
{
    if (!m_appsDrawerSurface || !m_compositor)
        return false;
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (!region)
        return false;
    if (width > 0 && height > 0)
        wl_region_add(region, 0, 0, width, height);
    // Applied with the next buffer commit (`commitAppsDrawerImage`).
    wl_surface_set_input_region(m_appsDrawerSurface, region);
    wl_region_destroy(region);
    return true;
}

bool ShellProtocol::setAppsDrawerKeyboardGrab(bool grab)
{
    if (!m_appsDrawerLayer)
        return false;
    df_layer_surface_set_keyboard_interaction(
        m_appsDrawerLayer,
        grab ? DF_LAYER_SURFACE_KEYBOARD_INTERACTION_EXCLUSIVE
             : DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::setAppsDrawerPanelRect(int x, int y, int width, int height)
{
    if (!m_appsDrawerLayer)
        return false;
    df_layer_surface_set_panel_rect(m_appsDrawerLayer, x, y, width, height);
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::commitAppsDrawerImage(const QImage &image, const QSize &logicalSize)
{
    if (!m_appsDrawerSurface)
        return false;
    if (!commitTo(m_appsDrawerSurface, image, logicalSize))
        return false;
    m_appsDrawerMapped = true;
    return true;
}

bool ShellProtocol::hideAppsDrawer()
{
    if (!m_appsDrawerSurface || !m_appsDrawerLayer)
        return false;
    if (!m_appsDrawerMapped)
        return true;
    cancelPendingFrame(QStringLiteral("apps-drawer"));
    wl_surface_attach(m_appsDrawerSurface, nullptr, 0, 0);
    wl_surface_commit(m_appsDrawerSurface);
    resetSurfaceScale(m_appsDrawerSurface);
    m_appsDrawerMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::captureScreenshot(const QString &path, int x, int y, int width, int height,
                                      const QString &mode)
{
    if (!m_manager || path.isEmpty())
        return false;
    const QByteArray pathUtf8 = path.toUtf8();
    const QByteArray modeUtf8 = mode.toUtf8();
    df_toplevel_manager_capture_screenshot(m_manager, x, y, width, height, modeUtf8.constData(),
                                           pathUtf8.constData());
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

// --- session lock (T-12.3a) -------------------------------------------------

void ShellProtocol::createLockSurfaces()
{
    if (!m_sessionLock || !m_compositor)
        return;
    for (wl_output *output : std::as_const(m_lockOutputs)) {
        bool alreadyCovered = false;
        for (auto it = m_lockSurfaces.constBegin(); it != m_lockSurfaces.constEnd(); ++it) {
            if (it.value().output == output) {
                alreadyCovered = true;
                break;
            }
        }
        if (alreadyCovered)
            continue;
        wl_surface *surface = wl_compositor_create_surface(m_compositor);
        ensureScaleObjects(surface);
        if (!surface)
            continue;
        ext_session_lock_surface_v1 *lockSurface =
            ext_session_lock_v1_get_lock_surface(m_sessionLock, surface, output);
        static const ext_session_lock_surface_v1_listener listener = { onLockSurfaceConfigure };
        ext_session_lock_surface_v1_add_listener(lockSurface, &listener, this);
        LockSurfaceInfo info;
        info.output = output;
        info.surface = surface;
        m_lockSurfaces.insert(lockSurface, info);
    }
}

bool ShellProtocol::isLockSurface(wl_surface *surface) const
{
    if (!surface)
        return false;
    for (auto it = m_lockSurfaces.constBegin(); it != m_lockSurfaces.constEnd(); ++it) {
        if (it.value().surface == surface)
            return true;
    }
    return false;
}

bool ShellProtocol::lockSession()
{
    if (m_sessionLock)
        return true;
    if (!m_lockManager)
        return fail(QStringLiteral("ext_session_lock_manager_v1 is not advertised"));
    if (!m_compositor)
        return fail(QStringLiteral("wl_compositor is not available"));

    m_sessionLock = ext_session_lock_manager_v1_lock(m_lockManager);
    if (!m_sessionLock)
        return fail(QStringLiteral("compositor refused the session lock"));
    static const ext_session_lock_v1_listener listener = { onSessionLockLocked,
                                                            onSessionLockFinished };
    ext_session_lock_v1_add_listener(m_sessionLock, &listener, this);

    // Cover every output the registry has announced so far. An output that
    // appears later gets its lock surface from the registry handler.
    createLockSurfaces();
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

void ShellProtocol::unlockSession()
{
    if (m_sessionLock) {
        ext_session_lock_v1_unlock_and_destroy(m_sessionLock);
        m_sessionLock = nullptr;
    }
    for (auto it = m_lockSurfaces.begin(); it != m_lockSurfaces.end(); ++it) {
        if (it.value().surface) {
            destroySurfaceBuffer(it.value().surface);
            wl_surface_destroy(it.value().surface);
        }
    }
    m_lockSurfaces.clear();
    m_sessionLocked = false;
    m_keyboardOnLock = false;
    if (m_display)
        wl_display_flush(m_display);
}

bool ShellProtocol::commitLockImage(quintptr lockSurfaceId, const QImage &image,
                         const QSize &logicalSize)
{
    auto *lockSurface = reinterpret_cast<ext_session_lock_surface_v1 *>(lockSurfaceId);
    auto it = m_lockSurfaces.find(lockSurface);
    if (it == m_lockSurfaces.end() || !it.value().surface)
        return false;
    return commitTo(it.value().surface, image, logicalSize);
}

bool ShellProtocol::fullscreenOverlayActive() const
{
    // A fullscreen window owns a dedicated Space while it exists, and the
    // compositor activates that Space on the owning output; the per-index
    // projection therefore reports an active Space marked fullscreen.
    for (auto it = m_workspaces.constBegin(); it != m_workspaces.constEnd(); ++it) {
        if (it.value().active && it.value().fullscreen)
            return true;
    }
    // Fallback: the focused toplevel's own state bit (fullscreen = 0x4) when
    // the Space projection has not caught up.
    if (m_focused && (m_toplevels.value(m_focused).state & 0x4u) != 0)
        return true;
    return false;
}

void ShellProtocol::activateWorkspace(int index)
{
    if (!m_display)
        return;
    // Spaces are per-output; activation is lockstep, so any output's Space at
    // this index addresses the same switch (T-05).
    for (auto it = m_workspaces.constBegin(); it != m_workspaces.constEnd(); ++it) {
        if (it.value().index == index) {
            df_workspace_activate(it.key());
            wl_display_flush(m_display);
            return;
        }
    }
}

QVariantList ShellProtocol::overviewWorkspaces() const
{
    // Spaces are per-output and lockstep; the strip shows one card per index.
    // The dedicated fullscreen Space exists (empty) on every output but only
    // the owning output marks it fullscreen, so merge the per-index flags
    // instead of trusting whichever hash entry is seen first.
    QMap<int, QVariantMap> byIndex;
    for (const WorkspaceInfo &info : m_workspaces) {
        if (info.index < 0)
            continue;
        auto it = byIndex.find(info.index);
        if (it == byIndex.end()) {
            QVariantMap map;
            map.insert(QStringLiteral("index"), info.index);
            map.insert(QStringLiteral("name"), info.name);
            map.insert(QStringLiteral("fullscreen"), info.fullscreen);
            map.insert(QStringLiteral("active"), info.active);
            byIndex.insert(info.index, map);
        } else {
            it.value().insert(QStringLiteral("fullscreen"),
                              it.value().value(QStringLiteral("fullscreen")).toBool()
                                  || info.fullscreen);
            it.value().insert(QStringLiteral("active"),
                              it.value().value(QStringLiteral("active")).toBool()
                                  || info.active);
            if (it.value().value(QStringLiteral("name")).toString().isEmpty())
                it.value().insert(QStringLiteral("name"), info.name);
        }
    }
    QVariantList result;
    for (auto it = byIndex.constBegin(); it != byIndex.constEnd(); ++it)
        result.append(it.value());
    return result;
}

QVariantList ShellProtocol::minimizedWindows() const
{
    QVariantList result;
    for (df_toplevel *toplevel : m_toplevelOrder) {
        const ToplevelInfo &info = m_toplevels.value(toplevel);
        if ((info.state & 0x1u) == 0)
            continue;
        QVariantMap map;
        map.insert(QStringLiteral("windowId"), QString::number(info.windowId));
        map.insert(QStringLiteral("title"), info.title);
        map.insert(QStringLiteral("appId"), info.appId);
        result.append(map);
    }
    return result;
}

QVariantList ShellProtocol::overviewWindows() const
{
    // The overview window grid (FR-7): one draggable card per *visible*
    // window. Minimized windows are excluded from the Space layout and live in
    // the bottom strip (FR-6); fullscreen windows own a dedicated transient
    // Space (T-05 FR-3). Neither belongs in the grid. The Space index/name
    // comes from the same projection the Dock uses, so the shell still keeps
    // no copy of workspace state.
    QVariantList result;
    for (df_toplevel *toplevel : m_toplevelOrder) {
        const ToplevelInfo &info = m_toplevels.value(toplevel);
        if ((info.state & 0x1u) != 0 || (info.state & 0x4u) != 0)
            continue;
        const WorkspaceInfo ws = m_workspaces.value(info.workspace);
        QVariantMap map;
        map.insert(QStringLiteral("windowId"), QString::number(info.windowId));
        map.insert(QStringLiteral("title"), info.title);
        map.insert(QStringLiteral("appId"), info.appId);
        map.insert(QStringLiteral("workspaceIndex"), ws.index);
        map.insert(QStringLiteral("workspaceName"), ws.name);
        map.insert(QStringLiteral("focused"), toplevel == m_focused);
        result.append(map);
    }
    return result;
}

QVariantList ShellProtocol::screencastSources() const
{
    // Monitors first, then windows: the picker's section delegate keys off the
    // source order. The shell never invents sources; the compositor's output
    // and toplevel projections are the one source of truth (T-13.4a).
    QVariantList result;
    int monitorIndex = 0;
    for (df_output *output : m_outputs) {
        const OutputInfo info = m_outputInfo.value(output);
        const QString name = info.name.isEmpty()
                ? tr("Monitor %1").arg(++monitorIndex)
                : info.name;
        const QString id = QStringLiteral("monitor:") + name;
        QVariantMap map;
        map.insert(QStringLiteral("id"), id);
        map.insert(QStringLiteral("kind"), QStringLiteral("monitor"));
        map.insert(QStringLiteral("label"), name);
        map.insert(QStringLiteral("detail"),
                   info.width > 0
                       ? QStringLiteral("%1 × %2").arg(info.width).arg(info.height)
                       : QString());
        result.append(map);
    }
    for (df_toplevel *toplevel : m_toplevelOrder) {
        const ToplevelInfo &info = m_toplevels.value(toplevel);
        QVariantMap map;
        map.insert(QStringLiteral("id"),
                   QStringLiteral("window:") + QString::number(info.windowId));
        map.insert(QStringLiteral("kind"), QStringLiteral("window"));
        map.insert(QStringLiteral("label"),
                   info.title.isEmpty() ? info.appId : info.title);
        map.insert(QStringLiteral("detail"), info.appId);
        result.append(map);
    }
    return result;
}

void ShellProtocol::moveToplevelToWorkspace(const QString &windowId, int index)
{
    bool ok = false;
    const quintptr id = windowId.toULongLong(&ok);
    df_toplevel *toplevel = ok ? toplevelForId(id) : nullptr;
    if (!toplevel)
        return;
    // Spaces are lockstep: every output has the same ordered indices, and the
    // compositor resolves the window's own output from its current Space. Any
    // handle with the target index therefore addresses the right Space.
    df_workspace *target = nullptr;
    for (auto it = m_workspaces.constBegin(); it != m_workspaces.constEnd(); ++it) {
        if (it.value().index == index) {
            target = it.key();
            break;
        }
    }
    if (!target)
        return;
    df_toplevel_move_to_workspace(toplevel, target);
    if (m_display)
        wl_display_flush(m_display);
}

bool ShellProtocol::createDockSurface(DockPosition position, int thickness, int exclusiveZone)
{
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_dockSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_dockSurface);
    m_dockLayer = df_shell_get_layer_surface(m_shell, m_dockSurface, nullptr, DF_SHELL_LAYER_TOP,
                                             "dock");
    if (!m_dockLayer)
        return fail(QStringLiteral("compositor refused the Dock layer surface"));
    static const df_layer_surface_listener dockListener = { onDockConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_dockLayer, &dockListener, this);

    // A bottom Dock stretches the full output width; a vertical Dock the full
    // height. Either way it reserves the baseline bar thickness on its edge.
    // It takes keyboard focus only on a click (OnDemand), which lets an open
    // context menu/chooser be dismissed by click-away/focus loss and receive
    // Escape (T-10 sections 13/20); the idle Dock never holds focus.
    df_layer_surface_set_anchor(m_dockLayer, dockSurfaceAnchor(position));
    df_layer_surface_set_keyboard_interaction(
        m_dockLayer, DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    if (!configureDockSurface(position, thickness, exclusiveZone))
        return false;
    // Start unmapped; the first render commits a buffer.
    wl_surface_attach(m_dockSurface, nullptr, 0, 0);
    wl_surface_commit(m_dockSurface);

    if (wl_display_roundtrip(m_display) < 0)
        return fail(QStringLiteral("Dock configure roundtrip failed"));
    return true;
}

bool ShellProtocol::configureDockSurface(DockPosition position, int thickness, int exclusiveZone)
{
    if (!m_dockLayer)
        return false;
    // Re-apply the anchor so a live `dock.position` change moves the surface
    // without a recreate; the compositor honors anchor changes after creation.
    df_layer_surface_set_anchor(m_dockLayer, dockSurfaceAnchor(position));
    if (position == DockPosition::Bottom)
        df_layer_surface_set_size(m_dockLayer, 0, thickness);
    else
        df_layer_surface_set_size(m_dockLayer, thickness, 0);
    df_layer_surface_set_exclusive_zone(m_dockLayer, exclusiveZone);
    // Keep the popover overlay anchored to the same edge if it already exists,
    // so a live `dock.position` change does not leave it hugging the old edge.
    if (m_dockPopupLayer)
        df_layer_surface_set_anchor(m_dockPopupLayer, dockPopupAnchor(position));
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::commitDockImage(const QImage &image, const QSize &logicalSize,
                                    const QRect &sourceRect)
{
    if (!m_dockSurface)
        return false;
    if (!commitTo(m_dockSurface, image, logicalSize, sourceRect))
        return false;
    m_dockMapped = true;
    return true;
}

bool ShellProtocol::dockDmabufBusy() const
{
    if (!m_dockSurface)
        return false;
    auto it = m_surfaceBuffers.constFind(m_dockSurface);
    if (it == m_surfaceBuffers.constEnd())
        return false;
    for (const ShellDmabufSlot &slot : it.value()->dmabufSlots) {
        if (slot.busy)
            return true;
    }
    return false;
}

ShellProtocol::DmabufCommitResult ShellProtocol::commitDockDmabuf(
    const ChromeDmabufFrame &frame, const QSize &logicalSize,
    const QRect &sourceRect, QString *reason)
{
    if (!m_dockSurface) {
        if (reason)
            *reason = QStringLiteral("no-dock-surface");
        return DmabufCommitResult::Failed;
    }
    const DmabufCommitResult result = commitDmabufTo(
        m_dockSurface, frame, logicalSize, sourceRect, reason);
    if (result == DmabufCommitResult::Committed)
        m_dockMapped = true;
    return result;
}

ShellProtocol::DmabufCommitResult ShellProtocol::commitDmabufTo(
    wl_surface *surface, const ChromeDmabufFrame &frame, const QSize &logicalSize,
    const QRect &sourceRect, QString *reason)
{
    if (!surface) {
        if (reason)
            *reason = QStringLiteral("no-surface");
        return DmabufCommitResult::Failed;
    }
    if (!m_dmabuf) {
        if (reason)
            *reason = QStringLiteral("no-dmabuf");
        return DmabufCommitResult::Failed;
    }
    if (frame.slot < 0) {
        if (reason)
            *reason = QStringLiteral("no-slot");
        return DmabufCommitResult::Failed;
    }

    const ChromeDmabufCheck check =
        ChromeDmabuf::validate(frame, m_gpuCapabilities.formats);
    if (!check.valid) {
        if (reason)
            *reason = check.reason;
        return DmabufCommitResult::Failed;
    }

    const QRect source = (sourceRect.isEmpty()
                              ? QRect(QPoint(0, 0), frame.size)
                              : sourceRect)
                             .intersected(QRect(QPoint(0, 0), frame.size));
    if (source.isEmpty() || logicalSize.isEmpty()) {
        if (reason)
            *reason = QStringLiteral("bad-source");
        return DmabufCommitResult::Failed;
    }

    const auto scaleIt = m_surfaceScales.constFind(surface);
    const bool hasViewport =
        scaleIt != m_surfaceScales.constEnd() && scaleIt.value().viewport;
    if (!hasViewport) {
        // Without a viewporter only a full-buffer integer-scale mapping is
        // expressible. The Dock always has a gutter/headroom sub-rect, so this
        // is a recorded fallback rather than a silently stretched frame.
        const bool fullBuffer = source == QRect(QPoint(0, 0), frame.size);
        const bool integerScale =
            logicalSize.width() > 0 && logicalSize.height() > 0
            && frame.size.width() % logicalSize.width() == 0
            && frame.size.height() % logicalSize.height() == 0
            && frame.size.width() / logicalSize.width()
                == frame.size.height() / logicalSize.height();
        if (!fullBuffer || !integerScale) {
            if (reason)
                *reason = QStringLiteral("no-viewporter");
            return DmabufCommitResult::Failed;
        }
    }

    ShellSurfaceBuffer *buffer = surfaceBuffer(surface);
    if (!buffer) {
        if (reason)
            *reason = QStringLiteral("no-buffer-state");
        return DmabufCommitResult::Failed;
    }
    const QString label = surfaceLabel(surface);
    auto slotIt = buffer->dmabufSlots.find(frame.slot);
    if (slotIt != buffer->dmabufSlots.end() && slotIt.value().failed) {
        if (reason)
            *reason = slotIt.value().failReason.isEmpty()
                ? QStringLiteral("import-failed")
                : slotIt.value().failReason;
        return DmabufCommitResult::Failed;
    }

    if (slotIt != buffer->dmabufSlots.end() && slotIt.value().busy) {
        // The producer ring should never hand us a busy slot. If it does, do
        // not attach it again or write into it; retry after the release.
        if (reason)
            *reason = QStringLiteral("slot-busy");
        return DmabufCommitResult::Deferred;
    }

    // A size/format change recreates this slot's buffer. The producer only
    // reuses a retired slot after its release, so this is not a busy buffer.
    if (slotIt != buffer->dmabufSlots.end() && slotIt.value().buffer
        && (slotIt.value().size != frame.size
            || slotIt.value().format != frame.format
            || slotIt.value().modifier != frame.modifier)) {
        destroyDmabufSlot(buffer, frame.slot);
        slotIt = buffer->dmabufSlots.end();
    }

    const bool reused = slotIt != buffer->dmabufSlots.end()
        && slotIt.value().buffer;
    if (!reused) {
        if (!createDmabufSlot(surface, buffer, frame.slot, frame, reason))
            return DmabufCommitResult::Failed;
        slotIt = buffer->dmabufSlots.find(frame.slot);
        if (slotIt == buffer->dmabufSlots.end())
            return DmabufCommitResult::Failed;
    }
    ShellDmabufSlot &slot = slotIt.value();

    const bool statsEnabled = ChromeStats::instance().enabled();
    QElapsedTimer commitTimer;
    if (statsEnabled)
        commitTimer.start();

    applyDmabufScale(surface, logicalSize, source, frame.size);

    // Request the callback before the commit it should pace, exactly like the
    // shm path. The compositor answers it after this frame presents.
    requestFrameInternal(surface, buffer);
    wl_surface_attach(surface, slot.buffer, 0, 0);
    wl_surface_damage_buffer(surface, 0, 0, frame.size.width(), frame.size.height());
    applyDockSurfaceGeometry(surface);
    wl_surface_commit(surface);
    wl_display_flush(m_display);
    if (surface == m_surface)
        m_menuBarPanelDirty = false;
    buffer->lastImage = QImage();
    buffer->lastSourceRect = QRect();
    slot.busy = true;
    slot.serial = frame.serial;
    m_dmabufOwners.insert(slot.buffer, DmabufBufferOwner{surface, frame.slot,
                                                         frame.serial});

    if (statsEnabled) {
        qint64 commitUs = ChromeStats::instance().commitUsSinceRender();
        if (commitUs < 0)
            commitUs = commitTimer.nsecsElapsed() / 1000;
        const qint64 bytes = qint64(frame.stride) * frame.size.height();
        ChromeStats::instance().recordCommit(
            label, logicalSize, frame.size, 1, 1.0, true, commitUs, 0,
            qint64(source.width()) * source.height());
        ChromeStats::instance().recordDmabufCommit(label, bytes);
        if (reused)
            ChromeStats::instance().recordDmabufReuse(label);
    }
    if (reason)
        reason->clear();
    return DmabufCommitResult::Committed;
}

ShellProtocol::DmabufCommitResult ShellProtocol::commitDmabufFor(
    const QString &surfaceLabel, const ChromeDmabufFrame &frame,
    const QSize &logicalSize, const QRect &sourceRect, QString *reason)
{
    wl_surface *surface = surfaceForLabel(surfaceLabel);
    if (!surface) {
        if (reason)
            *reason = QStringLiteral("unknown-surface");
        return DmabufCommitResult::Failed;
    }
    const DmabufCommitResult result =
        commitDmabufTo(surface, frame, logicalSize, sourceRect, reason);
    if (result == DmabufCommitResult::Committed) {
        if (surface == m_dockSurface)
            m_dockMapped = true;
        else if (surface == m_popupSurface)
            m_popupMapped = true;
        else if (surface == m_dockPopupSurface)
            m_dockPopupMapped = true;
        else if (surface == m_overviewSurface)
            m_overviewMapped = true;
        else if (surface == m_switcherSurface)
            m_switcherMapped = true;
        else if (surface == m_bannerSurface)
            m_bannerMapped = true;
        else if (surface == m_notificationCenterSurface)
            m_notificationCenterMapped = true;
        else if (surface == m_controlCenterSurface)
            m_controlCenterMapped = true;
        else if (surface == m_osdSurface)
            m_osdMapped = true;
        else if (surface == m_chooserSurface)
            m_chooserMapped = true;
        else if (surface == m_widgetGallerySurface)
            m_widgetGalleryMapped = true;
        else if (surface == m_screenshotSurface)
            m_screenshotMapped = true;
        else if (surface == m_screencastSurface)
            m_screencastMapped = true;
        else if (surface == m_polkitSurface)
            m_polkitMapped = true;
        else if (surface == m_appsDrawerSurface)
            m_appsDrawerMapped = true;
    }
    return result;
}

ShellProtocol::DmabufCommitResult ShellProtocol::commitLockDmabuf(
    quintptr lockSurfaceId, const ChromeDmabufFrame &frame,
    const QSize &logicalSize, QString *reason)
{
    auto *lockSurface = reinterpret_cast<ext_session_lock_surface_v1 *>(lockSurfaceId);
    auto it = m_lockSurfaces.find(lockSurface);
    if (it == m_lockSurfaces.end() || !it.value().surface) {
        if (reason)
            *reason = QStringLiteral("unknown-lock-surface");
        return DmabufCommitResult::Failed;
    }
    return commitDmabufTo(it.value().surface, frame, logicalSize, QRect(), reason);
}

bool ShellProtocol::createDmabufSlot(wl_surface *surface, ShellSurfaceBuffer *buffer,
                                     int slotIndex, const ChromeDmabufFrame &frame,
                                     QString *reason)
{
    if (!surface || !buffer || !m_dmabuf || !frame.valid() || slotIndex < 0)
        return false;
    ShellDmabufSlot &slot = buffer->dmabufSlots[slotIndex];
    slot.failed = false;
    slot.failReason.clear();
    slot.buffer = nullptr;

    zwp_linux_buffer_params_v1 *params =
        zwp_linux_dmabuf_v1_create_params(m_dmabuf);
    if (!params) {
        if (reason)
            *reason = QStringLiteral("no-params");
        return false;
    }
    slot.params = params;
    m_dmabufParams.insert(params, DmabufParamOwner{surface, slotIndex});

    const int fd = ::dup(frame.fd);
    if (fd < 0) {
        m_dmabufParams.remove(params);
        slot.params = nullptr;
        zwp_linux_buffer_params_v1_destroy(params);
        if (reason)
            *reason = QStringLiteral("dup-failed");
        return false;
    }
    zwp_linux_buffer_params_v1_add(
        params, fd, 0, frame.offset, frame.stride,
        ChromeDmabuf::modifierHigh(frame.modifier),
        ChromeDmabuf::modifierLow(frame.modifier));
    ::close(fd);

    static const zwp_linux_buffer_params_v1_listener paramsListener = {
        onDmabufCreated,
        onDmabufFailed,
    };
    zwp_linux_buffer_params_v1_add_listener(params, &paramsListener, this);

    slot.creating = true;
    const uint32_t flags = frame.yInverted
        ? ZWP_LINUX_BUFFER_PARAMS_V1_FLAGS_Y_INVERT
        : 0;
    zwp_linux_buffer_params_v1_create(params, frame.size.width(), frame.size.height(),
                                      frame.format, flags);
    // `create` reports success/failure asynchronously. This is the only
    // roundtrip on the dmabuf path; the buffer is reused for later frames.
    while (slot.creating && m_display) {
        if (wl_display_roundtrip(m_display) < 0)
            break;
    }
    if (slot.params) {
        m_dmabufParams.remove(slot.params);
        zwp_linux_buffer_params_v1_destroy(slot.params);
        slot.params = nullptr;
    }
    if (!slot.buffer) {
        slot.creating = false;
        slot.failed = true;
        if (slot.failReason.isEmpty())
            slot.failReason = QStringLiteral("import-failed");
        if (reason)
            *reason = slot.failReason;
        return false;
    }
    slot.size = frame.size;
    slot.format = frame.format;
    slot.modifier = frame.modifier;
    slot.serial = frame.serial;
    ChromeStats::instance().recordDmabufBufferCreated(surfaceLabel(surface));
    qInfo().noquote() << QStringLiteral(
        "chrome dmabuf: buffer created slot=%1 format=%2 modifier=%3 stride=%4 "
        "offset=%5 size=%6x%7 surface=%8")
        .arg(slotIndex)
        .arg(GpuChromeCapabilities::formatName(frame.format),
             GpuChromeCapabilities::modifierName(frame.modifier))
        .arg(frame.stride)
        .arg(frame.offset)
        .arg(frame.size.width())
        .arg(frame.size.height())
        .arg(surfaceLabel(surface));
    return true;
}

void ShellProtocol::destroyDmabufSlot(ShellSurfaceBuffer *buffer, int slotIndex,
                                      bool destroyBuffer)
{
    if (!buffer)
        return;
    auto it = buffer->dmabufSlots.find(slotIndex);
    if (it == buffer->dmabufSlots.end())
        return;
    ShellDmabufSlot &slot = it.value();
    if (slot.params) {
        m_dmabufParams.remove(slot.params);
        zwp_linux_buffer_params_v1_destroy(slot.params);
        slot.params = nullptr;
    }
    if (slot.buffer) {
        const auto owner = m_dmabufOwners.constFind(slot.buffer);
        const QString label = owner != m_dmabufOwners.constEnd() && owner.value().surface
            ? surfaceLabel(owner.value().surface)
            : QString();
        m_dmabufOwners.remove(slot.buffer);
        ChromeStats::instance().recordDmabufBufferDestroyed(label);
        if (destroyBuffer)
            wl_buffer_destroy(slot.buffer);
        slot.buffer = nullptr;
    }
    buffer->dmabufSlots.erase(it);
}

void ShellProtocol::applyDmabufScale(wl_surface *surface, const QSize &logicalSize,
                                     const QRect &sourceRect, const QSize &bufferSize)
{
    if (!surface)
        return;
    auto it = m_surfaceScales.find(surface);
    if (it != m_surfaceScales.end() && it.value().viewport) {
        // The dmabuf is the whole render target; the viewport selects the Dock
        // sub-rect and maps it onto the logical surface. This also covers
        // fractional scales, so the buffer scale stays 1.
        wl_surface_set_buffer_scale(surface, 1);
        wp_viewport_set_source(it.value().viewport,
                               wl_fixed_from_int(sourceRect.x()),
                               wl_fixed_from_int(sourceRect.y()),
                               wl_fixed_from_int(sourceRect.width()),
                               wl_fixed_from_int(sourceRect.height()));
        wp_viewport_set_destination(it.value().viewport,
                                    logicalSize.width(), logicalSize.height());
        return;
    }
    // Without a viewporter only a full-buffer integer-scale mapping is
    // expressible. The Dock always has a gutter/headroom sub-rect, so this is
    // a recorded fallback rather than a silently stretched frame.
    if (sourceRect == QRect(QPoint(0, 0), bufferSize)
        && bufferSize.width() % logicalSize.width() == 0
        && bufferSize.height() % logicalSize.height() == 0
        && bufferSize.width() / logicalSize.width()
            == bufferSize.height() / logicalSize.height()) {
        wl_surface_set_buffer_scale(surface, bufferSize.width() / logicalSize.width());
    }
}

void ShellProtocol::setDockFrameGeometry(const QRect &panel, const QList<QRect> &inputRects)
{
    m_nextDockGeometry = DockFrameGeometry{panel, inputRects};
}

bool ShellProtocol::applyMenuBarSurfaceGeometry(wl_surface *surface)
{
    if (surface != m_surface || !m_menuBarPanelDirty)
        return false;
    m_menuBarPanelDirty = false;
    return true;
}

bool ShellProtocol::applyDockSurfaceGeometry(wl_surface *surface)
{
    if (surface == m_dockPopupSurface && m_nextDockPopupPanel
        && m_nextDockPopupPanel != m_committedDockPopupPanel) {
        const QRect &panel = *m_nextDockPopupPanel;
        df_layer_surface_set_panel_rect(m_dockPopupLayer, panel.x(), panel.y(),
                                       panel.width(), panel.height());
        m_committedDockPopupPanel = panel;
        return true;
    }
    if (surface != m_dockSurface || !m_nextDockGeometry)
        return false;
    const DockFrameGeometry &next = *m_nextDockGeometry;
    const bool inputChanged = !m_committedDockGeometry
        || next.inputRects != m_committedDockGeometry->inputRects;
    const bool panelChanged = !m_committedDockGeometry
        || next.panel != m_committedDockGeometry->panel;
    if (inputChanged) {
        wl_region *region = wl_compositor_create_region(m_compositor);
        if (!region)
            return false;
        for (const QRect &rect : next.inputRects) {
            if (!rect.isEmpty())
                wl_region_add(region, rect.x(), rect.y(), rect.width(), rect.height());
        }
        wl_surface_set_input_region(m_dockSurface, region);
        wl_region_destroy(region);
    }
    if (panelChanged)
        df_layer_surface_set_panel_rect(m_dockLayer, next.panel.x(), next.panel.y(),
                                         next.panel.width(), next.panel.height());
    m_committedDockGeometry = next;
    return inputChanged || panelChanged;
}

bool ShellProtocol::createDockPopupSurface(DockPosition position)
{
    if (m_dockPopupSurface || m_dockPopupLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_dockPopupSurface = wl_compositor_create_surface(m_compositor);
    ensureScaleObjects(m_dockPopupSurface);
    m_dockPopupLayer = df_shell_get_layer_surface(m_shell, m_dockPopupSurface, nullptr,
                                                  DF_SHELL_LAYER_OVERLAY, "dock-popup");
    if (!m_dockPopupLayer)
        return fail(QStringLiteral("compositor refused the Dock popup layer surface"));
    static const df_layer_surface_listener listener = { onDockPopupConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_dockPopupLayer, &listener, this);

    // Anchored to the Dock's edge so the popover can be placed with margins
    // measured from that edge; no reserved zone and no keyboard (the Dock
    // surface owns pointer input).
    df_layer_surface_set_anchor(m_dockPopupLayer, dockPopupAnchor(position));
    df_layer_surface_set_exclusive_zone(m_dockPopupLayer, -1);
    df_layer_surface_set_keyboard_interaction(m_dockPopupLayer,
                                              DF_LAYER_SURFACE_KEYBOARD_INTERACTION_NONE);
    wl_surface_attach(m_dockPopupSurface, nullptr, 0, 0);
    wl_surface_commit(m_dockPopupSurface);
    if (wl_display_flush(m_display) < 0)
        return fail(QStringLiteral("failed to flush the Dock popup surface creation"));
    return true;
}

bool ShellProtocol::setDockPopupGeometry(int top, int right, int bottom, int left, int width,
                                         int height, const QRect &panel)
{
    if (!m_dockPopupLayer || !m_dockPopupSurface)
        return false;
    m_nextDockPopupPanel = panel;
    // Hover repaints do not change layout. Avoid reconfiguring the overlay
    // and rebuilding its material metadata for every highlight/animation frame.
    const std::array<int, 6> geometry{top, right, bottom, left, width, height};
    if (m_dockPopupGeometry == geometry)
        return true;
    m_dockPopupGeometry = geometry;
    // set_margin(top, right, bottom, left)
    df_layer_surface_set_margin(m_dockPopupLayer, top, right, bottom, left);
    df_layer_surface_set_size(m_dockPopupLayer, width, height);
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

void ShellProtocol::setDockPopupPanelRegions(const QVariantList &regions)
{
    if (!m_dockPopupLayer || m_dockPopupPanelRegions == regions)
        return;
    setMenuPanelRegions(m_dockPopupLayer, regions);
    m_dockPopupPanelRegions = regions;
}

void ShellProtocol::clearDockPopupPanelRegions()
{
    setDockPopupPanelRegions({});
}

void ShellProtocol::setDockPopupInputActive(bool active)
{
    if (!m_dockPopupSurface || !m_compositor)
        return;
    if (active) {
        wl_surface_set_input_region(m_dockPopupSurface, nullptr);
        return;
    }
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (!region)
        return;
    wl_surface_set_input_region(m_dockPopupSurface, region);
    wl_region_destroy(region);
}

bool ShellProtocol::commitDockPopupImage(const QImage &image, const QSize &logicalSize,
                                         const QRect &sourceRect)
{
    if (!m_dockPopupSurface)
        return false;
    if (!commitTo(m_dockPopupSurface, image, logicalSize, sourceRect))
        return false;
    m_dockPopupMapped = true;
    return true;
}

bool ShellProtocol::hideDockPopup()
{
    if (!m_dockPopupSurface || !m_dockPopupLayer)
        return false;
    if (!m_dockPopupMapped)
        return true;
    cancelPendingFrame(QStringLiteral("dock-popup"));
    wl_surface_attach(m_dockPopupSurface, nullptr, 0, 0);
    wl_surface_commit(m_dockPopupSurface);
    resetSurfaceScale(m_dockPopupSurface);
    m_dockPopupMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

void ShellProtocol::activateApp(const QString &appId)
{
    if (!m_manager)
        return;
    const QByteArray id = appId.toUtf8();
    df_toplevel_manager_activate_app(m_manager, id.constData());
    if (m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::assignAppToActiveWorkspace(const QString &appId)
{
    if (appId.isEmpty() || !m_activeWorkspace)
        return;
    // Move every window of the app to the Space the compositor last activated
    // (T-10 section 13, "Assign to This Desktop"). The compositor has no
    // sticky/all-Spaces or "no assignment" state, so those two options are
    // controller-level pending actions.
    QList<df_toplevel *> targets;
    for (auto it = m_toplevels.constBegin(); it != m_toplevels.constEnd(); ++it) {
        if (it.value().appId == appId)
            targets.append(it.key());
    }
    for (df_toplevel *toplevel : std::as_const(targets))
        df_toplevel_move_to_workspace(toplevel, m_activeWorkspace);
    if (!targets.isEmpty() && m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::releaseKeyboardFocus()
{
    if (!m_manager)
        return;
    // T-10 section 20: Escape exits Dock keyboard navigation; ask the
    // compositor to restore the active window's keyboard focus.
    df_toplevel_manager_release_keyboard_focus(m_manager);
    if (m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::selectToplevel(const QString &windowId)
{
    bool ok = false;
    const quintptr id = windowId.toULongLong(&ok);
    df_toplevel *toplevel = ok ? toplevelForId(id) : nullptr;
    if (!toplevel || !m_manager)
        return;
    // `select_overview_toplevel` restores a minimized window, activates its
    // Space, and focuses it — exactly the chooser's selection semantics
    // (T-10 FR-5).
    df_toplevel_manager_select_overview_toplevel(m_manager, toplevel);
    if (m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::closeToplevel(const QString &windowId)
{
    bool ok = false;
    const quintptr id = windowId.toULongLong(&ok);
    df_toplevel *toplevel = ok ? toplevelForId(id) : nullptr;
    if (!toplevel)
        return;
    df_toplevel_close(toplevel);
    if (m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::setToplevelMinimized(const QString &windowId, bool minimized)
{
    bool ok = false;
    const quintptr id = windowId.toULongLong(&ok);
    df_toplevel *toplevel = ok ? toplevelForId(id) : nullptr;
    if (!toplevel)
        return;
    // The chooser's stateful row action (T-14.7m): `minimized` is the state
    // the row asked for, never a toggle derived shell-side.
    if (minimized)
        df_toplevel_minimize(toplevel);
    else
        df_toplevel_unminimize(toplevel);
    if (m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::closeApp(const QString &appId)
{
    if (appId.isEmpty())
        return;
    // The private protocol has no app-level quit; closing every window of the
    // app is the closest supported action (T-10 section 13, interim).
    QList<df_toplevel *> targets;
    for (auto it = m_toplevels.constBegin(); it != m_toplevels.constEnd(); ++it) {
        if (it.value().appId == appId)
            targets.append(it.key());
    }
    for (df_toplevel *toplevel : std::as_const(targets))
        df_toplevel_close(toplevel);
    if (!targets.isEmpty() && m_display)
        wl_display_flush(m_display);
}

QString ShellProtocol::surfaceLabel(wl_surface *surface) const
{
    if (surface == m_surface)
        return QStringLiteral("menubar");
    if (surface == m_popupSurface)
        return QStringLiteral("menubar-popup");
    if (surface == m_dockSurface)
        return QStringLiteral("dock");
    if (surface == m_dockPopupSurface)
        return QStringLiteral("dock-popup");
    if (surface == m_overviewSurface)
        return QStringLiteral("overview");
    if (surface == m_switcherSurface)
        return QStringLiteral("switcher");
    if (surface == m_bannerSurface)
        return QStringLiteral("banner");
    if (surface == m_notificationCenterSurface)
        return QStringLiteral("notification-center");
    if (surface == m_controlCenterSurface)
        return QStringLiteral("control-center");
    if (surface == m_osdSurface)
        return QStringLiteral("osd");
    if (surface == m_chooserSurface)
        return QStringLiteral("chooser");
    if (surface == m_widgetGallerySurface)
        return QStringLiteral("widget-gallery");
    if (surface == m_screenshotSurface)
        return QStringLiteral("screenshot");
    if (surface == m_screencastSurface)
        return QStringLiteral("screencast");
    if (surface == m_polkitSurface)
        return QStringLiteral("polkit");
    if (surface == m_appsDrawerSurface)
        return QStringLiteral("apps-drawer");
    for (auto it = m_lockSurfaces.constBegin(); it != m_lockSurfaces.constEnd(); ++it) {
        if (it.value().surface == surface)
            return QStringLiteral("lock");
    }
    return QStringLiteral("unknown");
}

wl_surface *ShellProtocol::surfaceForLabel(const QString &label) const
{
    if (label == QLatin1String("menubar"))
        return m_surface;
    if (label == QLatin1String("menubar-popup"))
        return m_popupSurface;
    if (label == QLatin1String("dock"))
        return m_dockSurface;
    if (label == QLatin1String("dock-popup"))
        return m_dockPopupSurface;
    if (label == QLatin1String("overview"))
        return m_overviewSurface;
    if (label == QLatin1String("switcher"))
        return m_switcherSurface;
    if (label == QLatin1String("banner"))
        return m_bannerSurface;
    if (label == QLatin1String("notification-center"))
        return m_notificationCenterSurface;
    if (label == QLatin1String("control-center"))
        return m_controlCenterSurface;
    if (label == QLatin1String("osd"))
        return m_osdSurface;
    if (label == QLatin1String("chooser"))
        return m_chooserSurface;
    if (label == QLatin1String("widget-gallery"))
        return m_widgetGallerySurface;
    if (label == QLatin1String("screenshot"))
        return m_screenshotSurface;
    if (label == QLatin1String("screencast"))
        return m_screencastSurface;
    if (label == QLatin1String("polkit"))
        return m_polkitSurface;
    if (label == QLatin1String("apps-drawer"))
        return m_appsDrawerSurface;
    // `lock` is intentionally not resolved here: every output has its own lock
    // surface and the caller must pass the id to `commitLockDmabuf`.
    return nullptr;
}

QSize ShellProtocol::outputLogicalSize() const
{
    if (m_outputs.isEmpty())
        return QSize();
    const OutputInfo info = m_outputInfo.value(m_outputs.first());
    if (info.geometryWidth <= 0 || info.geometryHeight <= 0)
        return QSize();
    return QSize(info.geometryWidth, info.geometryHeight);
}

qreal ShellProtocol::preferredScaleFor(const QString &surface) const
{
    if (surface.isEmpty())
        return 1.0;
    for (auto it = m_surfaceScales.constBegin(); it != m_surfaceScales.constEnd(); ++it) {
        if (surfaceLabel(it.key()) == surface)
            return it.value().preferredKnown ? it.value().preferredScale : 1.0;
    }
    return 1.0;
}

void ShellProtocol::ensureScaleObjects(wl_surface *surface)
{
    if (!surface || m_surfaceScales.contains(surface))
        return;
    SurfaceScale scale;
    if (m_fractionalScaleManager) {
        scale.fractional =
            wp_fractional_scale_manager_v1_get_fractional_scale(m_fractionalScaleManager,
                                                                surface);
        if (scale.fractional) {
            static const wp_fractional_scale_v1_listener listener = { onPreferredScale };
            wp_fractional_scale_v1_add_listener(scale.fractional, &listener, this);
        }
    }
    if (m_viewporter)
        scale.viewport = wp_viewporter_get_viewport(m_viewporter, surface);
    m_surfaceScales.insert(surface, scale);
}

void ShellProtocol::applySurfaceScale(wl_surface *surface, const QSize &logicalSize,
                                      const QSize &bufferSize)
{
    if (!surface)
        return;
    auto it = m_surfaceScales.find(surface);
    if (it == m_surfaceScales.end())
        return;

    const qreal scale = it.value().preferredKnown ? it.value().preferredScale
                                                   : m_outputScale;
    const ChromeScale::Geometry geometry =
        ChromeScale::geometry(logicalSize.isEmpty() ? bufferSize : logicalSize, scale);

    // `wl_surface.set_buffer_scale` accepts integers only. Exact integers use
    // it directly; fractional scales keep scale 1 and map the physical buffer
    // onto the logical surface with the viewport.
    wl_surface_set_buffer_scale(surface, geometry.bufferScale);
    if (it.value().viewport) {
        if (geometry.useViewport) {
            wp_viewport_set_source(it.value().viewport, wl_fixed_from_int(0),
                                   wl_fixed_from_int(0),
                                   wl_fixed_from_int(geometry.viewportSource.width()),
                                   wl_fixed_from_int(geometry.viewportSource.height()));
            wp_viewport_set_destination(it.value().viewport,
                                        geometry.viewportDestination.width(),
                                        geometry.viewportDestination.height());
        } else {
            // Unset the viewport: -1 in both source and destination restores
            // the surface's native buffer-scale mapping.
            wp_viewport_set_source(it.value().viewport, wl_fixed_from_int(-1),
                                   wl_fixed_from_int(-1), wl_fixed_from_int(-1),
                                   wl_fixed_from_int(-1));
            wp_viewport_set_destination(it.value().viewport, -1, -1);
        }
    }
}

void ShellProtocol::resetSurfaceScale(wl_surface *surface)
{
    if (!surface)
        return;
    wl_surface_set_buffer_scale(surface, 1);
    auto it = m_surfaceScales.find(surface);
    if (it != m_surfaceScales.end() && it.value().viewport) {
        wp_viewport_set_source(it.value().viewport, wl_fixed_from_int(-1),
                               wl_fixed_from_int(-1), wl_fixed_from_int(-1),
                               wl_fixed_from_int(-1));
        wp_viewport_set_destination(it.value().viewport, -1, -1);
    }
}

void ShellProtocol::destroyScaleObjects()
{
    for (auto it = m_surfaceScales.begin(); it != m_surfaceScales.end(); ++it) {
        if (it.value().fractional)
            wp_fractional_scale_v1_destroy(it.value().fractional);
        if (it.value().viewport)
            wp_viewport_destroy(it.value().viewport);
    }
    m_surfaceScales.clear();
}

bool ShellProtocol::commitTo(wl_surface *surface, const QImage &image,
                             const QSize &logicalSize, const QRect &sourceRect)
{
    if (!surface || !m_shm || image.isNull())
        return false;

    const bool statsEnabled = ChromeStats::instance().enabled();
    QElapsedTimer commitTimer;
    if (statsEnabled)
        commitTimer.start();

    // QQuickRenderControl already hands us ARGB32_Premultiplied; only convert
    // when a caller actually passes another format (the conversion is a full
    // copy, so it must not run on the normal path).
    QImage frame = image;
    if (frame.format() != QImage::Format_ARGB32_Premultiplied)
        frame = frame.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    const QRect source = (sourceRect.isEmpty() ? frame.rect() : sourceRect)
                             .intersected(frame.rect());
    if (source.isEmpty())
        return false;

    const int width = source.width();
    const int height = source.height();
    // The logical surface size is the compositor contract. Callers pass it
    // explicitly because a fractional physical buffer cannot be divided back
    // to an exact logical size (1.5x: 188 physical -> 125.33 logical).
    const QSize logical = logicalSize.isEmpty() ? QSize(width, height) : logicalSize;
    const auto scaleIt = m_surfaceScales.constFind(surface);
    const qreal surfaceScale =
        (scaleIt != m_surfaceScales.constEnd() && scaleIt.value().preferredKnown)
            ? scaleIt.value().preferredScale
            : m_outputScale;
    const ChromeScale::Geometry geometry = ChromeScale::geometry(logical, surfaceScale);
    // A first-party QML scene should never produce more than an 8K output,
    // but a bad geometry property must not overflow the shm/pool arithmetic
    // or ask the compositor for an absurd allocation.
    constexpr int kMaxCommittedDimension = 16384;
    if (width > kMaxCommittedDimension || height > kMaxCommittedDimension)
        return fail(QStringLiteral("refusing an oversized surface image: %1x%2")
                        .arg(width)
                        .arg(height));
    const size_t stride = static_cast<size_t>(width) * 4u;
    const size_t bytes = stride * static_cast<size_t>(height);
    if (bytes > static_cast<size_t>(INT32_MAX))
        return fail(QStringLiteral("surface image is too large for wl_shm: %1x%2")
                        .arg(width)
                        .arg(height));

    ShellSurfaceBuffer *buffer = surfaceBuffer(surface);
    if (!buffer)
        return false;
    const QString label = surfaceLabel(surface);

    // A resize retires the old pool. Free old buffers are destroyed now; busy
    // ones are destroyed from `onBufferRelease` when the compositor returns
    // them, so the pool never grows past its bound.
    if (buffer->pool.size() != QSize(width, height)) {
        buffer->pool.setSize(QSize(width, height));
        const QVector<int> retired = buffer->pool.takeRetired();
        for (int slot : retired)
            destroyShmSlot(buffer, slot);
        buffer->lastImage = QImage();
        buffer->lastSourceRect = QRect();
    }

    // Damage is the bounding box of changed pixels against the previous frame
    // this surface committed. The previous image is a shallow reference to the
    // producer's other ping-pong target, so this comparison costs no copy.
    QRect damage = changedPixelRect(buffer->lastImage, buffer->lastSourceRect,
                                    frame, source);
    if (damage.isEmpty() && !buffer->lastImage.isNull()) {
        // Reuse the attached buffer. Input or plate geometry can change even
        // when the pixels match, so commit that state on its own.
        if (applyDockSurfaceGeometry(surface)
                || applyMenuBarSurfaceGeometry(surface)) {
            wl_surface_commit(surface);
            wl_display_flush(m_display);
        }
        // Release the old producer target for reuse.
        buffer->lastImage = frame;
        buffer->lastSourceRect = source;
        ChromeStats::instance().recordSkippedCommit(label, logical, QSize(width, height),
                                                    geometry.bufferScale, geometry.scale,
                                                    geometry.useViewport);
        return true;
    }
    if (damage.isEmpty())
        damage = QRect(0, 0, width, height);

    const int slot = buffer->pool.acquire();
    if (slot < 0) {
        // Every buffer is still held by the compositor. Do not write into a
        // busy buffer; the controller keeps the surface dirty and retries on
        // `bufferReleased`. This is a deferred frame, not a transport error.
        emit commitDeferred(label);
        return true;
    }
    if (!ensureShmSlot(surface, buffer, slot, QSize(width, height))) {
        buffer->pool.release(slot);
        return false;
    }
    ShellShmSlot &resource = buffer->slotResources[slot];

    // Refresh the selected slot with all changes since it last held a frame.
    // Wayland damage still describes the difference from the last commit.
    const QRect copyRect = buffer->pool.damageForCommit(slot, damage);
    const size_t copyOffset = static_cast<size_t>(copyRect.x()) * 4;
    const size_t copyStride = static_cast<size_t>(copyRect.width()) * 4;
    for (int y = copyRect.top(); y <= copyRect.bottom(); ++y) {
        const uchar *sourceLine = frame.constScanLine(source.y() + y)
            + static_cast<qsizetype>(source.x()) * 4 + copyOffset;
        std::memcpy(static_cast<uchar *>(resource.data)
                        + static_cast<size_t>(y) * stride + copyOffset,
                    sourceLine, copyStride);
    }

    // Buffer scale / viewport must be set before the attach+commit that makes
    // them current. At 1x this is the default (scale 1, viewport unset), so
    // the existing frame is bit-identical.
    applySurfaceScale(surface, logical, QSize(width, height));

    // Request the callback before the commit it should pace. The compositor
    // answers it after this frame presents; the controller then schedules the
    // next QML render.
    requestFrameInternal(surface, buffer);

    wl_surface_attach(surface, resource.buffer, 0, 0);
    wl_surface_damage_buffer(surface, damage.x(), damage.y(),
                             damage.width(), damage.height());
    applyDockSurfaceGeometry(surface);
    wl_surface_commit(surface);
    wl_display_flush(m_display);
    if (surface == m_surface)
        m_menuBarPanelDirty = false;
    buffer->lastImage = frame;
    buffer->lastSourceRect = source;
    if (statsEnabled) {
        // `commit_us` is measured from the render completion to the surface
        // commit; the fallback covers a commit with no preceding render.
        qint64 commitUs = ChromeStats::instance().commitUsSinceRender();
        if (commitUs < 0)
            commitUs = commitTimer.nsecsElapsed() / 1000;
        ChromeStats::instance().recordCommit(label, logical, QSize(width, height),
                                             geometry.bufferScale, geometry.scale,
                                             geometry.useViewport, commitUs,
                                             qint64(copyStride) * copyRect.height(),
                                             qint64(damage.width()) * damage.height());
    }
    return true;
}

ShellSurfaceBuffer *ShellProtocol::surfaceBuffer(wl_surface *surface)
{
    if (!surface)
        return nullptr;
    auto it = m_surfaceBuffers.find(surface);
    if (it != m_surfaceBuffers.end())
        return it.value();
    auto *buffer = new ShellSurfaceBuffer;
    m_surfaceBuffers.insert(surface, buffer);
    return buffer;
}

bool ShellProtocol::ensureShmSlot(wl_surface *surface, ShellSurfaceBuffer *buffer, int slot,
                                  const QSize &size)
{
    if (!surface || !buffer || !m_shm || size.isEmpty())
        return false;
    auto it = buffer->slotResources.find(slot);
    if (it != buffer->slotResources.end()) {
        ShellShmSlot &existing = it.value();
        if (existing.buffer && existing.bytes
                == static_cast<size_t>(size.width()) * 4u
                       * static_cast<size_t>(size.height()))
            return true;
        destroyShmSlot(buffer, slot);
    }

    const int width = size.width();
    const int height = size.height();
    const size_t stride = static_cast<size_t>(width) * 4u;
    const size_t bytes = stride * static_cast<size_t>(height);
    int fd = createShmFile(bytes);
    if (fd < 0) {
        fail(QStringLiteral("cannot allocate a shared-memory buffer: %1")
                 .arg(QString::fromLocal8Bit(strerror(errno))));
        return false;
    }
    void *data = mmap(nullptr, bytes, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (data == MAP_FAILED) {
        ::close(fd);
        fail(QStringLiteral("cannot map the shared-memory buffer"));
        return false;
    }
    wl_shm_pool *pool = wl_shm_create_pool(m_shm, fd, static_cast<int32_t>(bytes));
    if (!pool) {
        munmap(data, bytes);
        ::close(fd);
        fail(QStringLiteral("cannot create the shared-memory pool"));
        return false;
    }
    wl_buffer *wlBuffer = wl_shm_pool_create_buffer(
        pool, 0, width, height, static_cast<int32_t>(stride), WL_SHM_FORMAT_ARGB8888);
    wl_shm_pool_destroy(pool);
    ::close(fd);
    if (!wlBuffer) {
        munmap(data, bytes);
        fail(QStringLiteral("cannot create the shared-memory buffer"));
        return false;
    }

    static const wl_buffer_listener bufferListener = { onBufferRelease };
    wl_buffer_add_listener(wlBuffer, &bufferListener, this);

    ShellShmSlot resource;
    resource.buffer = wlBuffer;
    resource.data = data;
    resource.fd = -1;
    resource.bytes = bytes;
    buffer->slotResources.insert(slot, resource);
    m_bufferOwners.insert(wlBuffer, qMakePair(surface, slot));
    ChromeStats::instance().recordShmAlloc(surfaceLabel(surface),
                                           static_cast<qint64>(bytes));
    return true;
}

void ShellProtocol::destroyShmSlot(ShellSurfaceBuffer *buffer, int slot, bool destroyBuffer)
{
    if (!buffer)
        return;
    auto it = buffer->slotResources.find(slot);
    if (it == buffer->slotResources.end())
        return;
    ShellShmSlot &resource = it.value();
    if (resource.buffer) {
        m_bufferOwners.remove(resource.buffer);
        if (destroyBuffer)
            wl_buffer_destroy(resource.buffer);
    }
    if (resource.data)
        munmap(resource.data, resource.bytes);
    if (resource.fd >= 0)
        ::close(resource.fd);
    buffer->slotResources.erase(it);
}

void ShellProtocol::destroySurfaceBuffer(wl_surface *surface)
{
    auto it = m_surfaceBuffers.find(surface);
    if (it == m_surfaceBuffers.end())
        return;
    ShellSurfaceBuffer *buffer = it.value();
    m_surfaceBuffers.erase(it);
    if (buffer->frameCallback) {
        m_frameCallbackSurfaces.remove(buffer->frameCallback);
        wl_callback_destroy(buffer->frameCallback);
        buffer->frameCallback = nullptr;
    }
    const QList<int> dmabufSlotIds = buffer->dmabufSlots.keys();
    for (int slot : dmabufSlotIds)
        destroyDmabufSlot(buffer, slot);
    const QList<int> slotIds = buffer->slotResources.keys();
    for (int slot : slotIds)
        destroyShmSlot(buffer, slot);
    delete buffer;
}

void ShellProtocol::requestFrameInternal(wl_surface *surface, ShellSurfaceBuffer *buffer)
{
    if (!surface || !buffer || buffer->frameCallback || !m_display)
        return;
    wl_callback *callback = wl_surface_frame(surface);
    if (!callback)
        return;
    static const wl_callback_listener listener = { onFrameCallback };
    wl_callback_add_listener(callback, &listener, this);
    buffer->frameCallback = callback;
    buffer->frameRequested.start();
    m_frameCallbackSurfaces.insert(callback, surface);
}

void ShellProtocol::requestFrame(wl_surface *surface)
{
    if (!surface)
        return;
    requestFrameInternal(surface, surfaceBuffer(surface));
}

bool ShellProtocol::hasPendingFrame(const QString &surface, qint64 minAgeMs) const
{
    if (surface.isEmpty())
        return false;
    for (auto it = m_surfaceBuffers.constBegin(); it != m_surfaceBuffers.constEnd(); ++it) {
        if (it.value()->frameCallback && surfaceLabel(it.key()) == surface
            && (minAgeMs == 0 || it.value()->frameRequested.hasExpired(minAgeMs)))
            return true;
    }
    return false;
}

void ShellProtocol::cancelPendingFrame(const QString &surface)
{
    if (surface.isEmpty())
        return;
    QList<wl_surface *> matches;
    for (auto it = m_surfaceBuffers.constBegin(); it != m_surfaceBuffers.constEnd(); ++it) {
        if (surfaceLabel(it.key()) == surface)
            matches.append(it.key());
    }
    for (wl_surface *match : matches) {
        ShellSurfaceBuffer *buffer = m_surfaceBuffers.value(match);
        if (!buffer)
            continue;
        if (buffer->frameCallback) {
            wl_callback *callback = buffer->frameCallback;
            buffer->frameCallback = nullptr;
            m_frameCallbackSurfaces.remove(callback);
            wl_callback_destroy(callback);
        }
        // The hidden surface's damage baseline is stale; the next commit after
        // it is shown again must be a full frame.
        buffer->lastImage = QImage();
        buffer->lastSourceRect = QRect();
    }
}

QRect ShellProtocol::changedPixelRect(const QImage &previous, const QRect &previousRect,
                                      const QImage &current, const QRect &currentRect)
{
    const QSize size = currentRect.size();
    if (size.isEmpty())
        return QRect();
    if (previous.isNull() || previousRect.size() != size
            || previous.format() != current.format())
        return QRect(QPoint(0, 0), size);

    int minX = size.width();
    int minY = size.height();
    int maxX = -1;
    int maxY = -1;
    const int rowBytes = size.width() * 4;
    for (int y = 0; y < size.height(); ++y) {
        const uchar *a = previous.constScanLine(previousRect.y() + y)
            + static_cast<qsizetype>(previousRect.x()) * 4;
        const uchar *b = current.constScanLine(currentRect.y() + y)
            + static_cast<qsizetype>(currentRect.x()) * 4;
        if (std::memcmp(a, b, static_cast<size_t>(rowBytes)) == 0)
            continue;
        // Once some rows changed, only look for differences that would
        // extend the horizontal damage bounds.
        if (std::memcmp(a, b, static_cast<size_t>(minX) * 4) != 0) {
            int first = 0;
            while (std::memcmp(a + first * 4, b + first * 4, 4) == 0)
                ++first;
            minX = first;
        }
        const int endOfDamage = (maxX + 1) * 4;
        if (std::memcmp(a + endOfDamage, b + endOfDamage,
                        static_cast<size_t>(rowBytes - endOfDamage)) != 0) {
            int last = size.width() - 1;
            while (std::memcmp(a + last * 4, b + last * 4, 4) == 0)
                --last;
            maxX = last;
        }
        minY = qMin(minY, y);
        maxY = qMax(maxY, y);
    }
    if (maxX < minX || maxY < minY)
        return QRect();
    return QRect(minX, minY, maxX - minX + 1, maxY - minY + 1);
}

void ShellProtocol::onFrameCallback(void *data, wl_callback *callback, uint32_t time)
{
    auto *self = static_cast<ShellProtocol *>(data);
    wl_surface *surface = self->m_frameCallbackSurfaces.take(callback);
    if (!surface) {
        wl_callback_destroy(callback);
        return;
    }
    auto it = self->m_surfaceBuffers.find(surface);
    qint64 callbackUs = -1;
    if (it != self->m_surfaceBuffers.end()) {
        ShellSurfaceBuffer *buffer = it.value();
        if (buffer->frameCallback == callback) {
            buffer->frameCallback = nullptr;
            if (buffer->frameRequested.isValid())
                callbackUs = buffer->frameRequested.nsecsElapsed() / 1000;
        }
    }
    wl_callback_destroy(callback);
    ChromeStats::instance().recordFrameCallback(self->surfaceLabel(surface), callbackUs);
    emit self->framePresented(self->surfaceLabel(surface), time);
}


bool ShellProtocol::dispatch()
{
    if (!m_display)
        return false;
    if (wl_display_dispatch(m_display) < 0)
        return fail(QStringLiteral("compositor connection lost"));
    return true;
}

bool ShellProtocol::flush()
{
    if (!m_display)
        return false;
    if (wl_display_flush(m_display) < 0 && errno != EAGAIN)
        return fail(QStringLiteral("failed to flush the compositor connection"));
    return true;
}

void ShellProtocol::enterMissionControl()
{
    if (m_manager)
        df_toplevel_manager_enter_mission_control(m_manager);
    if (m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::exitMissionControl()
{
    if (m_manager)
        df_toplevel_manager_exit_mission_control(m_manager);
    if (m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::setReducedMotion(bool enabled)
{
    // T-11 U-1 / FR-9: mirror the design-system reduced-motion policy so the
    // compositor's transitions take the single-step path. Additive in v3.
    if (m_manager)
        df_toplevel_manager_set_reduced_motion(m_manager, enabled ? 1u : 0u);
    if (m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::setMotionPolicy(const QString &colorScheme,
                                    const QString &titlebarDoubleClick,
                                    const QString &minimizedAnimation)
{
    // T-08.2c: the compositor's motion/appearance keys, forwarded from the one
    // settingsd owner. Additive in v5; older compositors ignore it.
    if (m_manager && m_managerVersion >= 5) {
        const QByteArray scheme = colorScheme.toUtf8();
        const QByteArray titlebar = titlebarDoubleClick.toUtf8();
        const QByteArray minimized = minimizedAnimation.toUtf8();
        df_toplevel_manager_set_motion_policy(m_manager, scheme.constData(), titlebar.constData(),
                                              minimized.constData());
    }
    if (m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::setAppearancePolicy(const QString &colorScheme, double transparency,
                                        bool reduceTransparency, bool increaseContrast)
{
    // T196: the resolved scheme and the glass/accessibility preferences in one
    // coherent snapshot. Additive in v9; older compositors ignore it and keep
    // the v5 motion policy.
    if (m_manager && m_managerVersion >= 9) {
        const QByteArray scheme = colorScheme.toUtf8();
        const double bounded = qBound(0.0, transparency, 1.0);
        const uint32_t millionths =
            static_cast<uint32_t>(qRound(bounded * 1000000.0));
        df_toplevel_manager_set_appearance_policy(
            m_manager, scheme.constData(), millionths,
            reduceTransparency ? 1u : 0u, increaseContrast ? 1u : 0u);
    }
    if (m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::setInputPolicy(int repeatDelayMs, int repeatRateHz, bool gesturesEnabled,
                                   bool gestureSpaceSwitch, bool gestureMissionControl)
{
    // T-08.2c: keyboard repeat and gesture gating, applied live by the
    // compositor. Additive in v5.
    if (m_manager && m_managerVersion >= 5) {
        df_toplevel_manager_set_input_policy(
            m_manager, static_cast<uint32_t>(qMax(0, repeatDelayMs)),
            static_cast<uint32_t>(qMax(0, repeatRateHz)), gesturesEnabled ? 1u : 0u,
            gestureSpaceSwitch ? 1u : 0u, gestureMissionControl ? 1u : 0u);
    }
    if (m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::setPointerPolicy(double pointerSpeed, bool naturalScroll, bool tapToClick,
                                     bool leftHanded, const QString &scrollMethod,
                                     const QString &secondaryClick, double scrollSpeed,
                                     int doubleClickMs, bool haptics)
{
    // T229: the pointer half of the Keyboard/Mouse/Trackpad pane. Additive in
    // v10; older compositors keep the settingsd-only state.
    if (m_manager && m_managerVersion >= 10) {
        const auto millionths = [](double value) {
            return static_cast<int32_t>(qRound(qBound(-1.0, value, 1.0) * 1000000.0));
        };
        const QByteArray scroll = scrollMethod.toUtf8();
        const QByteArray secondary = secondaryClick.toUtf8();
        df_toplevel_manager_set_pointer_policy(
            m_manager, millionths(pointerSpeed), naturalScroll ? 1u : 0u,
            tapToClick ? 1u : 0u, leftHanded ? 1u : 0u,
            scrollMethod.isEmpty() ? nullptr : scroll.constData(),
            secondaryClick.isEmpty() ? nullptr : secondary.constData(),
            millionths(scrollSpeed), static_cast<uint32_t>(qMax(0, doubleClickMs)),
            haptics ? 1u : 0u);
    }
    if (m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::setKeyboardLayout(const QString &layout, const QString &variant)
{
    // T229: the active input source. Additive in v10; a null layout follows
    // the host XKB_DEFAULT_* configuration.
    if (m_manager && m_managerVersion >= 10) {
        const QByteArray layoutBytes = layout.toUtf8();
        const QByteArray variantBytes = variant.toUtf8();
        df_toplevel_manager_set_keyboard_layout(
            m_manager, layout.isEmpty() ? nullptr : layoutBytes.constData(),
            variant.isEmpty() ? nullptr : variantBytes.constData());
    }
    if (m_display)
        wl_display_flush(m_display);
}

void ShellProtocol::setWallpaper(const QString &source, uint32_t fit, bool showOnAllSpaces)
{
    // T-09.3: remember the selection and push it to the announced Spaces. When
    // no Space is known yet, `onManagerDone` applies it after the replay.
    m_wallpaperKnown = true;
    m_wallpaperSource = source;
    m_wallpaperFit = fit;
    m_wallpaperShowOnAllSpaces = showOnAllSpaces;
    applyWallpaper();
}

void ShellProtocol::applyWallpaper()
{
    if (!m_display || !m_wallpaperKnown)
        return;
    // `source` empty is legal (the protocol keeps the Space's solid color), so
    // a null pointer is sent rather than an empty string.
    const QByteArray source = m_wallpaperSource.toUtf8();
    const char *rawSource = m_wallpaperSource.isEmpty() ? nullptr : source.constData();
    for (auto it = m_workspaces.constBegin(); it != m_workspaces.constEnd(); ++it) {
        if (!m_wallpaperShowOnAllSpaces && it.key() != m_activeWorkspace)
            continue;
        // The color argument is retained by the compositor; the schema owns
        // only source/fit, so 0 leaves the Space's existing fallback.
        df_workspace_set_wallpaper(it.key(), rawSource, m_wallpaperFit, 0u);
    }
    wl_display_flush(m_display);
}

QString ShellProtocol::activeWallpaperSource() const
{
    if (!m_activeWorkspace)
        return QString();
    return m_workspaces.value(m_activeWorkspace).wallpaperSource;
}

QColor ShellProtocol::activeWallpaperColor() const
{
    if (!m_activeWorkspace)
        return QColor();
    return m_workspaces.value(m_activeWorkspace).wallpaperColor;
}

void ShellProtocol::setDisplayPolicy(double scale, uint32_t transform, double brightness)
{
    // T-09.5/T-11.3a: remember the selection and push it to the announced
    // outputs. When no output is known yet, `onManagerDone`/`onManagerOutput`
    // applies it after the replay.
    m_displayKnown = true;
    m_displayScale = scale;
    m_displayTransform = transform;
    m_displayBrightness = brightness;
    applyDisplayPolicy();
}

void ShellProtocol::applyDisplayPolicy()
{
    if (!m_display || !m_displayKnown || m_outputs.isEmpty())
        return;
    // The shell is the forwarder; the compositor is the applier (ADR 0034).
    // Wave 1 has no per-display selection, so the policy reaches every output;
    // per-output targeting is a T-16 item. `wl_fixed_from_double` is the wire
    // form of `df_output.set_scale` / `set_brightness`.
    for (df_output *output : std::as_const(m_outputs)) {
        df_output_set_scale(output, wl_fixed_from_double(m_displayScale));
        df_output_set_transform(output, m_displayTransform);
        df_output_set_brightness(output, wl_fixed_from_double(m_displayBrightness));
    }
    wl_display_flush(m_display);
}

void ShellProtocol::setLaunchOrigin(const QString &appId, int x, int y, int width, int height)
{
    // T-02.1b: hand the Dock entry's tile rectangle to the compositor so a
    // launching app's window appears from it. Additive in v4; the compositor
    // falls back to a centered origin when it never arrives.
    if (m_manager && m_managerVersion >= 4) {
        const QByteArray app = appId.toUtf8();
        df_toplevel_manager_set_launch_origin(m_manager, app.constData(), x, y, width, height);
    }
    if (m_display)
        wl_display_flush(m_display);
}

QRect ShellProtocol::primaryOutputGeometry() const
{
    if (m_outputs.isEmpty())
        return QRect();
    const OutputInfo info = m_outputInfo.value(m_outputs.first());
    if (info.geometryWidth <= 0 || info.geometryHeight <= 0)
        return QRect();
    return QRect(info.x, info.y, info.geometryWidth, info.geometryHeight);
}

void ShellProtocol::setAppAccelerators(const QString &appId, const QString &accelerators)
{
    // T-14.2b: mirror the menu-broker's focus-scoped accelerator table. The
    // compositor only matches an app's table while it is focused and its system
    // shortcuts still win. Additive in v7; older compositors ignore it.
    if (m_manager && m_managerVersion >= 7) {
        const QByteArray app = appId.toUtf8();
        const QByteArray table = accelerators.toUtf8();
        df_toplevel_manager_set_app_accelerators(m_manager, app.constData(), table.constData());
    }
    if (m_display)
        wl_display_flush(m_display);
}

int ShellProtocol::displayFd() const
{
    return m_display ? wl_display_get_fd(m_display) : -1;
}

void ShellProtocol::teardown()
{
    // Destroy the per-surface scale helpers before their wl_surfaces.
    destroyScaleObjects();
    // Pooled buffers and their pending frame callbacks are per surface.
    const QList<wl_surface *> bufferedSurfaces = m_surfaceBuffers.keys();
    for (wl_surface *surface : bufferedSurfaces)
        destroySurfaceBuffer(surface);
    m_bufferOwners.clear();
    m_dmabufOwners.clear();
    m_dmabufParams.clear();
    m_frameCallbackSurfaces.clear();
    // Session lock (T-12.3a): while locked, neither the lock object nor its
    // surfaces may be destroyed — that is a protocol error, and leaving them
    // alive lets the connection drop keep the compositor locked (the
    // fail-secure path). They are owned by the connection and disappear with
    // `wl_display_disconnect`. When unlocked, destroy the surfaces first and
    // the lock object last, which is the order the protocol requires.
    if (!m_sessionLocked) {
        for (auto it = m_lockSurfaces.begin(); it != m_lockSurfaces.end(); ++it) {
            if (it.value().surface)
                wl_surface_destroy(it.value().surface);
        }
        if (m_sessionLock)
            ext_session_lock_v1_destroy(m_sessionLock);
    }
    m_sessionLock = nullptr;
    m_lockSurfaces.clear();
    m_lockOutputs.clear();
    m_sessionLocked = false;
    m_keyboardOnLock = false;
    m_lockManager = nullptr;
    if (m_switcherLayer)
        df_layer_surface_destroy(m_switcherLayer);
    if (m_switcherSurface)
        wl_surface_destroy(m_switcherSurface);
    if (m_notificationCenterLayer)
        df_layer_surface_destroy(m_notificationCenterLayer);
    if (m_notificationCenterSurface)
        wl_surface_destroy(m_notificationCenterSurface);
    if (m_controlCenterLayer)
        df_layer_surface_destroy(m_controlCenterLayer);
    if (m_controlCenterSurface)
        wl_surface_destroy(m_controlCenterSurface);
    if (m_osdLayer)
        df_layer_surface_destroy(m_osdLayer);
    if (m_osdSurface)
        wl_surface_destroy(m_osdSurface);
    if (m_chooserLayer)
        df_layer_surface_destroy(m_chooserLayer);
    if (m_chooserSurface)
        wl_surface_destroy(m_chooserSurface);
    if (m_widgetGalleryLayer)
        df_layer_surface_destroy(m_widgetGalleryLayer);
    if (m_widgetGallerySurface)
        wl_surface_destroy(m_widgetGallerySurface);
    if (m_screenshotLayer)
        df_layer_surface_destroy(m_screenshotLayer);
    if (m_screenshotSurface)
        wl_surface_destroy(m_screenshotSurface);
    if (m_bannerLayer)
        df_layer_surface_destroy(m_bannerLayer);
    if (m_bannerSurface)
        wl_surface_destroy(m_bannerSurface);
    if (m_overviewLayer)
        df_layer_surface_destroy(m_overviewLayer);
    if (m_overviewSurface)
        wl_surface_destroy(m_overviewSurface);
    if (m_dockPopupLayer)
        df_layer_surface_destroy(m_dockPopupLayer);
    if (m_dockPopupSurface)
        wl_surface_destroy(m_dockPopupSurface);
    if (m_dockLayer)
        df_layer_surface_destroy(m_dockLayer);
    if (m_dockSurface)
        wl_surface_destroy(m_dockSurface);
    if (m_popupLayer)
        df_layer_surface_destroy(m_popupLayer);
    if (m_popupSurface)
        wl_surface_destroy(m_popupSurface);
    if (m_layer)
        df_layer_surface_destroy(m_layer);
    if (m_surface)
        wl_surface_destroy(m_surface);
    resetExternalDrag();
    if (m_dndReadNotifier) {
        m_dndReadNotifier->setEnabled(false);
        delete m_dndReadNotifier;
        m_dndReadNotifier = nullptr;
    }
    if (m_dataDevice)
        wl_data_device_destroy(m_dataDevice);
    if (m_dataDeviceManager)
        wl_data_device_manager_destroy(m_dataDeviceManager);
    cancelClipboardRead();
    if (m_clipboardSource)
        zwlr_data_control_source_v1_destroy(m_clipboardSource);
    if (m_dataControlOffer)
        zwlr_data_control_offer_v1_destroy(m_dataControlOffer);
    if (m_dataControlDevice)
        zwlr_data_control_device_v1_destroy(m_dataControlDevice);
    if (m_dataControlManager)
        zwlr_data_control_manager_v1_destroy(m_dataControlManager);
    if (m_pointer)
        wl_pointer_destroy(m_pointer);
    if (m_keyboard)
        wl_keyboard_destroy(m_keyboard);
    if (m_seat)
        wl_seat_destroy(m_seat);
    if (m_manager)
        df_toplevel_manager_destroy(m_manager);
    if (m_shell)
        df_shell_destroy(m_shell);
    if (m_core)
        df_core_destroy(m_core);
    if (m_shm)
        wl_shm_destroy(m_shm);
    if (m_dmabuf)
        zwp_linux_dmabuf_v1_destroy(m_dmabuf);
    if (m_viewporter)
        wp_viewporter_destroy(m_viewporter);
    if (m_fractionalScaleManager)
        wp_fractional_scale_manager_v1_destroy(m_fractionalScaleManager);
    if (m_compositor)
        wl_compositor_destroy(m_compositor);
    if (m_registry)
        wl_registry_destroy(m_registry);
    if (m_display) {
        wl_display_flush(m_display);
        wl_display_disconnect(m_display);
    }
    m_display = nullptr;
    m_layer = nullptr;
    m_surface = nullptr;
    m_popupLayer = nullptr;
    m_popupSurface = nullptr;
    m_popupMapped = false;
    m_dockLayer = nullptr;
    m_popupGeometry.reset();
    m_popupPanelRegions.reset();
    m_dockSurface = nullptr;
    m_dockMapped = false;
    m_nextDockGeometry.reset();
    m_committedDockGeometry.reset();
    m_dockPopupLayer = nullptr;
    m_dockPopupSurface = nullptr;
    m_dockPopupMapped = false;
    m_nextDockPopupPanel.reset();
    m_dockPopupGeometry.reset();
    m_dockPopupPanelRegions.reset();
    m_committedDockPopupPanel.reset();
    m_overviewLayer = nullptr;
    m_overviewSurface = nullptr;
    m_overviewMapped = false;
    m_pointerOnOverview = false;
    m_pointerOnBanner = false;
    m_pointerOnNotificationCenter = false;
    m_pointerOnControlCenter = false;
    m_pointerOnChooser = false;
    m_pointerOnScreenshot = false;
    m_pointerOnScreenCast = false;
    m_pointerOnPolkit = false;
    m_pointerOnAppsDrawer = false;
    m_keyboardOnOverview = false;
    m_keyboardOnNotificationCenter = false;
    m_keyboardOnControlCenter = false;
    m_keyboardOnChooser = false;
    m_keyboardOnScreenshot = false;
    m_keyboardOnScreenCast = false;
    m_keyboardOnPolkit = false;
    m_keyboardOnAppsDrawer = false;
    m_screenshotLayer = nullptr;
    m_screenshotSurface = nullptr;
    m_screenshotMapped = false;
    m_screencastLayer = nullptr;
    m_screencastSurface = nullptr;
    m_screencastMapped = false;
    m_polkitLayer = nullptr;
    m_polkitSurface = nullptr;
    m_polkitMapped = false;
    m_appsDrawerLayer = nullptr;
    m_appsDrawerSurface = nullptr;
    m_appsDrawerMapped = false;
    m_notificationCenterLayer = nullptr;
    m_notificationCenterSurface = nullptr;
    m_notificationCenterMapped = false;
    m_controlCenterLayer = nullptr;
    m_controlCenterSurface = nullptr;
    m_controlCenterMapped = false;
    m_switcherLayer = nullptr;
    m_switcherSurface = nullptr;
    m_switcherMapped = false;
    m_switcherActive = false;
    m_switcherPending = false;
    m_switcherEntries.clear();
    // The manager destroy above freed every df_output; drop our handles.
    m_outputs.clear();
    m_outputInfo.clear();
    m_displayKnown = false;
    m_manager = nullptr;
    m_shell = nullptr;
    m_core = nullptr;
    m_fractionalScaleManager = nullptr;
    m_viewporter = nullptr;
    m_dmabuf = nullptr;
    m_surfaceScales.clear();
    m_pointer = nullptr;
    m_keyboard = nullptr;
    m_seat = nullptr;
    m_dataDevice = nullptr;
    m_dataDeviceManager = nullptr;
}

// --- registry ---------------------------------------------------------------

void ShellProtocol::onRegistryGlobal(void *data, wl_registry *registry, uint32_t name,
                                     const char *interface, uint32_t version)
{
    auto *self = static_cast<ShellProtocol *>(data);
    const QString iface = QString::fromLatin1(interface);
    if (iface == QLatin1String("wl_compositor")) {
        self->m_compositorVersion = version;
        self->m_compositor = static_cast<wl_compositor *>(
            wl_registry_bind(registry, name, &wl_compositor_interface, std::min(version, 4u)));
    } else if (iface == QLatin1String("wl_shm")) {
        self->m_shmVersion = version;
        self->m_shm = static_cast<wl_shm *>(
            wl_registry_bind(registry, name, &wl_shm_interface, std::min(version, 1u)));
    } else if (iface == QLatin1String("wl_seat")) {
        self->m_seatName = name;
        self->m_seatVersion = version;
        self->m_seat = static_cast<wl_seat *>(
            wl_registry_bind(registry, name, &wl_seat_interface, std::min(version, 1u)));
        static const wl_seat_listener seatListener = { onSeatCapabilities, onSeatName };
        wl_seat_add_listener(self->m_seat, &seatListener, self);
    } else if (iface == QLatin1String("df_core")) {
        self->m_coreName = name;
        self->m_coreVersion = version;
    } else if (iface == QLatin1String("df_shell")) {
        self->m_shellName = name;
        self->m_shellVersion = version;
    } else if (iface == QLatin1String("df_toplevel_manager")) {
        self->m_managerName = name;
        self->m_managerVersion = version;
    } else if (iface == QLatin1String("wl_data_device_manager")) {
        self->m_dataDeviceManager = static_cast<wl_data_device_manager *>(
            wl_registry_bind(registry, name, &wl_data_device_manager_interface,
                             std::min(version, 3u)));
        self->maybeCreateDataDevice();
    } else if (iface == QLatin1String("zwlr_data_control_manager_v1")) {
        self->m_dataControlManager = static_cast<zwlr_data_control_manager_v1 *>(
            wl_registry_bind(registry, name, &zwlr_data_control_manager_v1_interface,
                             std::min(version, 2u)));
        self->maybeCreateDataControlDevice();
    } else if (iface == QLatin1String("wl_output")) {
        auto *output = static_cast<wl_output *>(
            wl_registry_bind(registry, name, &wl_output_interface, std::min(version, 4u)));
        if (output) {
            self->m_lockOutputs.append(output);
            // A lock requested before the output was announced still covers
            // it (T-12.3a).
            if (self->m_sessionLock)
                self->createLockSurfaces();
        }
    } else if (iface == QLatin1String("ext_session_lock_manager_v1")) {
        self->m_lockManager = static_cast<ext_session_lock_manager_v1 *>(
            wl_registry_bind(registry, name, &ext_session_lock_manager_v1_interface, 1u));
    } else if (iface == QLatin1String("wp_fractional_scale_manager_v1")) {
        self->m_fractionalScaleManager = static_cast<wp_fractional_scale_manager_v1 *>(
            wl_registry_bind(registry, name, &wp_fractional_scale_manager_v1_interface,
                             std::min(version, 1u)));
    } else if (iface == QLatin1String("wp_viewporter")) {
        self->m_viewporter = static_cast<wp_viewporter *>(
            wl_registry_bind(registry, name, &wp_viewporter_interface,
                             std::min(version, 1u)));
    } else if (iface == QLatin1String("zwp_linux_dmabuf_v1")) {
        // T-21.1d: bind the transport object when the GPU rail is requested
        // (GPU-first default, or an explicit 1/on/auto). A forced 0/off/false
        // keeps the shell on the pooled-shm path and creates no dmabuf.
        if (GpuChromeFrameProducer::environmentRequested()) {
            self->m_dmabuf = static_cast<zwp_linux_dmabuf_v1 *>(
                wl_registry_bind(registry, name, &zwp_linux_dmabuf_v1_interface,
                                 std::min(version, 5u)));
        }
    }
}

void ShellProtocol::onRegistryGlobalRemove(void *, wl_registry *, uint32_t)
{
}

// --- df_core ----------------------------------------------------------------

void ShellProtocol::onCoreAuthenticated(void *data, df_core *, uint32_t version)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_authenticated = true;
    self->m_error.clear();
    emit self->authenticated(version);
}

void ShellProtocol::onCoreRefused(void *data, df_core *, uint32_t code, const char *message)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_authenticated = false;
    self->m_error = QString::fromUtf8(message);
    emit self->refused(code, self->m_error);
}

// --- df_layer_surface -------------------------------------------------------

void ShellProtocol::onLayerConfigure(void *data, df_layer_surface *, uint32_t serial,
                                     int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_layer)
        df_layer_surface_ack_configure(self->m_layer, serial);
    emit self->configured(width, height, serial);
}

void ShellProtocol::onPopupConfigure(void *data, df_layer_surface *, uint32_t serial,
                                     int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_popupLayer)
        df_layer_surface_ack_configure(self->m_popupLayer, serial);
    emit self->popupConfigured(width, height, serial);
}

void ShellProtocol::onDockConfigure(void *data, df_layer_surface *, uint32_t serial,
                                    int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_dockLayer)
        df_layer_surface_ack_configure(self->m_dockLayer, serial);
    emit self->dockConfigured(width, height, serial);
}

void ShellProtocol::onDockPopupConfigure(void *data, df_layer_surface *, uint32_t serial,
                                         int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_dockPopupLayer)
        df_layer_surface_ack_configure(self->m_dockPopupLayer, serial);
    emit self->dockPopupConfigured(width, height, serial);
}

void ShellProtocol::onOverviewConfigure(void *data, df_layer_surface *, uint32_t serial,
                                        int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_overviewLayer)
        df_layer_surface_ack_configure(self->m_overviewLayer, serial);
    emit self->overviewConfigured(width, height, serial);
}

void ShellProtocol::onSwitcherConfigure(void *data, df_layer_surface *, uint32_t serial,
                                        int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_switcherLayer)
        df_layer_surface_ack_configure(self->m_switcherLayer, serial);
    emit self->switcherConfigured(width, height, serial);
}

void ShellProtocol::onBannerConfigure(void *data, df_layer_surface *, uint32_t serial,
                                      int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_bannerLayer)
        df_layer_surface_ack_configure(self->m_bannerLayer, serial);
    emit self->bannerConfigured(width, height, serial);
}

void ShellProtocol::onNotificationCenterConfigure(void *data, df_layer_surface *,
                                                  uint32_t serial, int32_t width,
                                                  int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_notificationCenterLayer)
        df_layer_surface_ack_configure(self->m_notificationCenterLayer, serial);
    emit self->notificationCenterConfigured(width, height, serial);
}

void ShellProtocol::onLayerClosed(void *data, df_layer_surface *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    emit self->surfaceClosed();
}

void ShellProtocol::onNotificationCenterLayerClosed(void *data, df_layer_surface *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_notificationCenterLayer = nullptr;
    self->m_notificationCenterSurface = nullptr;
    self->m_notificationCenterMapped = false;
    self->m_pointerOnNotificationCenter = false;
    self->m_keyboardOnNotificationCenter = false;
    emit self->notificationCenterSurfaceClosed();
}

void ShellProtocol::onControlCenterConfigure(void *data, df_layer_surface *, uint32_t serial,
                                             int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_controlCenterLayer)
        df_layer_surface_ack_configure(self->m_controlCenterLayer, serial);
    emit self->controlCenterConfigured(width, height, serial);
}

void ShellProtocol::onOsdConfigure(void *data, df_layer_surface *, uint32_t serial,
                                   int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_osdLayer)
        df_layer_surface_ack_configure(self->m_osdLayer, serial);
    emit self->osdConfigured(width, height, serial);
}

void ShellProtocol::onChooserConfigure(void *data, df_layer_surface *, uint32_t serial,
                                       int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_chooserLayer)
        df_layer_surface_ack_configure(self->m_chooserLayer, serial);
    emit self->chooserConfigured(width, height, serial);
}

void ShellProtocol::onWidgetGalleryConfigure(void *data, df_layer_surface *, uint32_t serial,
                                             int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_widgetGalleryLayer)
        df_layer_surface_ack_configure(self->m_widgetGalleryLayer, serial);
    emit self->widgetGalleryConfigured(width, height, serial);
}

void ShellProtocol::onScreenshotConfigure(void *data, df_layer_surface *, uint32_t serial,
                                          int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_screenshotLayer)
        df_layer_surface_ack_configure(self->m_screenshotLayer, serial);
    emit self->screenshotConfigured(width, height, serial);
}

void ShellProtocol::onScreenCastConfigure(void *data, df_layer_surface *, uint32_t serial,
                                          int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_screencastLayer)
        df_layer_surface_ack_configure(self->m_screencastLayer, serial);
    emit self->screencastConfigured(width, height, serial);
}

void ShellProtocol::onPolkitConfigure(void *data, df_layer_surface *, uint32_t serial,
                                      int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_polkitLayer)
        df_layer_surface_ack_configure(self->m_polkitLayer, serial);
    emit self->polkitConfigured(width, height, serial);
}

void ShellProtocol::onAppsDrawerConfigure(void *data, df_layer_surface *, uint32_t serial,
                                          int32_t width, int32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_appsDrawerLayer)
        df_layer_surface_ack_configure(self->m_appsDrawerLayer, serial);
    emit self->appsDrawerConfigured(width, height, serial);
}

// --- ext_session_lock_v1 (T-12.3a) -----------------------------------------

void ShellProtocol::onSessionLockLocked(void *data, ext_session_lock_v1 *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_sessionLocked = true;
    emit self->sessionLocked();
}

void ShellProtocol::onSessionLockFinished(void *data, ext_session_lock_v1 *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_sessionLocked = false;
    emit self->sessionFinished();
}

void ShellProtocol::onLockSurfaceConfigure(void *data, ext_session_lock_surface_v1 *surface,
                                           uint32_t serial, uint32_t width, uint32_t height)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // Ack before the buffer commit; the compositor rejects a commit that
    // precedes the first ack.
    ext_session_lock_surface_v1_ack_configure(surface, serial);
    auto it = self->m_lockSurfaces.find(surface);
    if (it != self->m_lockSurfaces.end()) {
        it.value().width = static_cast<int>(width);
        it.value().height = static_cast<int>(height);
    }
    emit self->lockSurfaceConfigured(reinterpret_cast<quintptr>(surface), static_cast<int>(width),
                                     static_cast<int>(height), serial);
}

// --- wl_seat / wl_pointer / wl_keyboard (input bridge) ----------------------

void ShellProtocol::onSeatCapabilities(void *data, wl_seat *seat, uint32_t capabilities)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // The data device needs the seat, not a capability; bind it once both the
    // seat and the manager global are known (T-10 external drops).
    self->maybeCreateDataDevice();
    self->maybeCreateDataControlDevice();
    if ((capabilities & WL_SEAT_CAPABILITY_POINTER) && !self->m_pointer) {
        self->m_pointer = wl_seat_get_pointer(seat);
        static const wl_pointer_listener pointerListener = {
            onPointerEnter, onPointerLeave, onPointerMotion, onPointerButton, onPointerAxis,
        };
        wl_pointer_add_listener(self->m_pointer, &pointerListener, self);
    }
    if ((capabilities & WL_SEAT_CAPABILITY_KEYBOARD) && !self->m_keyboard) {
        self->m_keyboard = wl_seat_get_keyboard(seat);
        static const wl_keyboard_listener keyboardListener = {
            onKeyboardKeymap, onKeyboardEnter, onKeyboardLeave, onKeyboardKey,
            onKeyboardModifiers,
        };
        wl_keyboard_add_listener(self->m_keyboard, &keyboardListener, self);
    }
}

void ShellProtocol::onSeatName(void *, wl_seat *, const char *)
{
}

void ShellProtocol::maybeCreateDataDevice()
{
    if (!m_dataDeviceManager || !m_seat || m_dataDevice)
        return;
    m_dataDevice = wl_data_device_manager_get_data_device(m_dataDeviceManager, m_seat);
    static const wl_data_device_listener listener = {
        onDataDeviceOffer, onDataDeviceEnter, onDataDeviceLeave,
        onDataDeviceMotion, onDataDeviceDrop, onDataDeviceSelection,
    };
    wl_data_device_add_listener(m_dataDevice, &listener, this);
}

void ShellProtocol::onDataDeviceOffer(void *data, wl_data_device *, wl_data_offer *offer)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // A new offer (a drag or a selection) supersedes the previous one. The
    // shell only consumes drags; a stale selection offer is harmless to drop.
    // A superseding offer invalidates any in-flight enter-time read (T-14.7f).
    if (self->m_dndOffer && self->m_dndOffer != offer) {
        wl_data_offer_destroy(self->m_dndOffer);
        self->m_dndOffer = nullptr;
        self->resetDndRead();
    }
    self->m_dndOffer = offer;
    self->m_dndMimeTypes.clear();
    static const wl_data_offer_listener offerListener = {
        onDataOfferMimeType, onDataOfferSourceActions, onDataOfferAction,
    };
    wl_data_offer_add_listener(offer, &offerListener, self);
}

void ShellProtocol::onDataOfferMimeType(void *data, wl_data_offer *, const char *mimeType)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (mimeType)
        self->m_dndMimeTypes.append(QString::fromLatin1(mimeType));
}

void ShellProtocol::onDataOfferSourceActions(void *, wl_data_offer *, uint32_t)
{
}

void ShellProtocol::onDataOfferAction(void *, wl_data_offer *, uint32_t)
{
}

void ShellProtocol::onDataDeviceEnter(void *data, wl_data_device *, uint32_t, wl_surface *surface,
                                      wl_fixed_t x, wl_fixed_t y, wl_data_offer *offer)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_dndOffer != offer) {
        if (self->m_dndOffer)
            wl_data_offer_destroy(self->m_dndOffer);
        self->m_dndOffer = offer;
        self->resetDndRead();
    }
    // Only the Dock is a drop target; a drag over the popup or another chrome
    // surface is ignored (the shell owns the payload).
    if (!self->m_dockSurface || surface != self->m_dockSurface) {
        self->m_dndActive = false;
        return;
    }
    self->m_dndActive = true;
    self->m_dndX = wl_fixed_to_double(x);
    self->m_dndY = wl_fixed_to_double(y);
    // Read the payload now, at enter, so the hover ghost and target
    // affordances show real identity and the drop reuses this one receive
    // (a data offer is readable once, T-14.7f).
    self->beginDndRead();
    // Until the read finishes the mime set is all we know: the app-alias mime
    // is unambiguous, while a `text/uri-list` may still turn out to be a
    // `.desktop` alias once parsed.
    if (!self->m_dndDataReady)
        self->m_dndPayloadIsApp = self->m_dndMimeTypes.contains(
            QStringLiteral("application/x-dragonfruit-app"));
    emit self->dockExternalDragEntered(self->m_dndPayloadIsApp, self->m_dndX, self->m_dndY);
    // A re-enter of a drag whose read already completed (or a synchronous
    // cache hit) reports the resolved payload immediately.
    if (self->m_dndDataReady) {
        emit self->dockExternalDragPayload(self->m_dndPayloadIsApp, self->m_dndDesktopId,
                                           self->m_dndPaths);
    }
}

void ShellProtocol::onDataDeviceLeave(void *data, wl_data_device *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (!self->m_dndActive)
        return;
    self->m_dndActive = false;
    emit self->dockExternalDragLeft();
}

void ShellProtocol::onDataDeviceMotion(void *data, wl_data_device *, uint32_t, wl_fixed_t x,
                                       wl_fixed_t y)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (!self->m_dndActive)
        return;
    self->m_dndX = wl_fixed_to_double(x);
    self->m_dndY = wl_fixed_to_double(y);
    emit self->dockExternalDragMoved(self->m_dndX, self->m_dndY);
}

void ShellProtocol::onDataDeviceDrop(void *data, wl_data_device *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (!self->m_dndActive || !self->m_dndOffer) {
        self->resetExternalDrag();
        return;
    }
    // The enter-time read is the single receive for this drag. If the source
    // has not finished writing, complete it now so a payload still in flight
    // is never dropped unread (T-14.7f).
    if (!self->ensureDndPayloadReady()) {
        wl_data_offer_finish(self->m_dndOffer);
        self->resetExternalDrag();
        return;
    }
    const bool payloadIsApp = self->m_dndPayloadIsApp;
    const QString desktopId = self->m_dndDesktopId;
    const QStringList paths = self->m_dndPaths;
    const qreal x = self->m_dndX;
    const qreal y = self->m_dndY;
    wl_data_offer_finish(self->m_dndOffer);
    self->resetExternalDrag();
    emit self->dockExternalDropped(payloadIsApp, desktopId, paths, x, y);
}

void ShellProtocol::onDataDeviceSelection(void *, wl_data_device *, wl_data_offer *)
{
}

void ShellProtocol::beginDndRead()
{
    // One receive per drag: never re-request a mime already read or in flight.
    if (m_dndDataReady || m_dndReadStarted || m_dndReadFd >= 0 || !m_dndOffer)
        return;
    QString mime;
    if (m_dndMimeTypes.contains(QStringLiteral("text/uri-list")))
        mime = QStringLiteral("text/uri-list");
    else if (m_dndMimeTypes.contains(QStringLiteral("application/x-dragonfruit-app")))
        mime = QStringLiteral("application/x-dragonfruit-app");
    if (mime.isEmpty())
        return;

    int fds[2];
    if (pipe2(fds, O_CLOEXEC) != 0)
        return;
    fcntl(fds[0], F_SETFL, O_NONBLOCK);
    wl_data_offer_receive(m_dndOffer, mime.toUtf8().constData(), fds[1]);
    ::close(fds[1]);

    m_dndMime = mime;
    m_dndData.clear();
    m_dndReadStarted = true;
    m_dndReadFd = fds[0];
    if (!m_dndReadNotifier) {
        m_dndReadNotifier = new QSocketNotifier(m_dndReadFd, QSocketNotifier::Read, this);
        QObject::connect(m_dndReadNotifier, &QSocketNotifier::activated, this,
                         &ShellProtocol::onDndReadable);
    } else {
        m_dndReadNotifier->setSocket(m_dndReadFd);
        m_dndReadNotifier->setEnabled(true);
    }
}

void ShellProtocol::onDndReadable()
{
    if (m_dndReadFd < 0)
        return;
    char buffer[4096];
    const ssize_t n = ::read(m_dndReadFd, buffer, sizeof(buffer));
    if (n > 0) {
        m_dndData.append(buffer, static_cast<int>(n));
        return;
    }
    if (n < 0 && (errno == EINTR || errno == EAGAIN || errno == EWOULDBLOCK))
        return;
    // EOF (0) or an error: the source has finished writing the payload.
    if (m_dndReadNotifier)
        m_dndReadNotifier->setEnabled(false);
    ::close(m_dndReadFd);
    m_dndReadFd = -1;
    finishDndRead();
    if (m_dndActive) {
        emit dockExternalDragPayload(m_dndPayloadIsApp, m_dndDesktopId, m_dndPaths);
    }
}

void ShellProtocol::finishDndRead()
{
    const DockDropPayloadData payload = parseDockDropPayload(m_dndMime, m_dndData);
    m_dndPayloadIsApp = payload.valid && payload.kind == DockDropPayload::Application;
    m_dndDesktopId = payload.desktopId;
    m_dndPaths = payload.paths;
    m_dndDataReady = payload.valid;
    m_dndData.clear();
}

bool ShellProtocol::ensureDndPayloadReady()
{
    if (m_dndDataReady)
        return true;
    if (m_dndReadFd < 0)
        return false;
    char buffer[4096];
    for (;;) {
        const ssize_t n = ::read(m_dndReadFd, buffer, sizeof(buffer));
        if (n > 0) {
            m_dndData.append(buffer, static_cast<int>(n));
            continue;
        }
        if (n == 0)
            break;
        if (errno == EINTR)
            continue;
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            // The source has not written yet (it writes then closes after
            // servicing the receive). Wait briefly rather than drop a
            // half-written payload.
            struct pollfd pfd;
            pfd.fd = m_dndReadFd;
            pfd.events = POLLIN;
            pfd.revents = 0;
            if (::poll(&pfd, 1, 250) > 0)
                continue;
        }
        break;
    }
    if (m_dndReadNotifier)
        m_dndReadNotifier->setEnabled(false);
    ::close(m_dndReadFd);
    m_dndReadFd = -1;
    finishDndRead();
    return m_dndDataReady;
}

void ShellProtocol::resetDndRead()
{
    if (m_dndReadFd >= 0) {
        ::close(m_dndReadFd);
        m_dndReadFd = -1;
    }
    if (m_dndReadNotifier)
        m_dndReadNotifier->setEnabled(false);
    m_dndReadStarted = false;
    m_dndDataReady = false;
    m_dndDesktopId.clear();
    m_dndPaths.clear();
    m_dndMime.clear();
    m_dndData.clear();
}

void ShellProtocol::resetExternalDrag()
{
    if (m_dndOffer) {
        wl_data_offer_destroy(m_dndOffer);
        m_dndOffer = nullptr;
    }
    resetDndRead();
    m_dndActive = false;
    m_dndPayloadIsApp = false;
    m_dndMimeTypes.clear();
}

// --- wlr-data-control clipboard observation (T-13.5b) -----------------------

namespace {

// The MIME types the history store can classify and keep. Everything else in
// the offer is ignored; the selection itself is untouched, so this filter
// never changes what a paste sees (ADR 0082).
bool shellKeepsClipboardMime(const QString &mime)
{
    return mime == QLatin1String("text/uri-list")
        || mime.startsWith(QLatin1String("image/"))
        || mime == QLatin1String("text/plain;charset=utf-8")
        || mime == QLatin1String("text/plain")
        || mime == QLatin1String("UTF8_STRING")
        || mime == QLatin1String("TEXT");
}

} // namespace

void ShellProtocol::maybeCreateDataControlDevice()
{
    if (!m_dataControlManager || !m_seat || m_dataControlDevice)
        return;
    m_dataControlDevice =
        zwlr_data_control_manager_v1_get_data_device(m_dataControlManager, m_seat);
    static const zwlr_data_control_device_v1_listener listener = {
        onDataControlOffer,
        onDataControlSelection,
        onDataControlFinished,
        onDataControlPrimarySelection,
    };
    zwlr_data_control_device_v1_add_listener(m_dataControlDevice, &listener, this);
}

void ShellProtocol::onDataControlOffer(void *data, zwlr_data_control_device_v1 *,
                                       zwlr_data_control_offer_v1 *offer)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_dataControlOffer && self->m_dataControlOffer != offer)
        zwlr_data_control_offer_v1_destroy(self->m_dataControlOffer);
    self->m_dataControlOffer = offer;
    self->m_clipboardMimes.clear();
    static const zwlr_data_control_offer_v1_listener offerListener = {
        onDataControlOfferMime,
    };
    zwlr_data_control_offer_v1_add_listener(offer, &offerListener, self);
}

void ShellProtocol::onDataControlOfferMime(void *data, zwlr_data_control_offer_v1 *,
                                           const char *mimeType)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (mimeType)
        self->m_clipboardMimes.append(QString::fromLatin1(mimeType));
}

void ShellProtocol::onDataControlSelection(void *data, zwlr_data_control_device_v1 *,
                                           zwlr_data_control_offer_v1 *offer)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // A new selection supersedes any read still in flight.
    self->cancelClipboardRead();
    if (!offer) {
        if (self->m_dataControlOffer) {
            zwlr_data_control_offer_v1_destroy(self->m_dataControlOffer);
            self->m_dataControlOffer = nullptr;
        }
        self->m_clipboardMimes.clear();
        return;
    }

    QStringList readable;
    for (const QString &mime : self->m_clipboardMimes) {
        if (shellKeepsClipboardMime(mime) && !readable.contains(mime))
            readable.append(mime);
    }
    if (readable.isEmpty())
        return;

    self->m_clipboardReadPayloads.clear();
    for (const QString &mime : readable) {
        int fds[2];
        if (pipe2(fds, O_CLOEXEC) != 0)
            continue;
        fcntl(fds[0], F_SETFL, O_NONBLOCK);
        zwlr_data_control_offer_v1_receive(offer, mime.toUtf8().constData(), fds[1]);
        ::close(fds[1]);

        self->m_clipboardFdMime.insert(fds[0], mime);
        self->m_clipboardPayloads.insert(fds[0], QByteArray());
        auto *notifier = new QSocketNotifier(fds[0], QSocketNotifier::Read, self);
        self->m_clipboardNotifierFd.insert(notifier, fds[0]);
        QObject::connect(notifier, &QSocketNotifier::activated, self,
                         [self, fd = fds[0]]() { self->onClipboardReadable(fd); });
    }
}

void ShellProtocol::onDataControlFinished(void *data, zwlr_data_control_device_v1 *device)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->cancelClipboardRead();
    if (self->m_dataControlOffer) {
        zwlr_data_control_offer_v1_destroy(self->m_dataControlOffer);
        self->m_dataControlOffer = nullptr;
    }
    self->m_clipboardMimes.clear();
    if (self->m_dataControlDevice == device)
        self->m_dataControlDevice = nullptr;
    zwlr_data_control_device_v1_destroy(device);
}

void ShellProtocol::onDataControlPrimarySelection(void *, zwlr_data_control_device_v1 *,
                                                  zwlr_data_control_offer_v1 *offer)
{
    // Primary selection is deliberately not part of the history (T-13.5b
    // stores the clipboard only); release the offer so it does not leak.
    if (offer)
        zwlr_data_control_offer_v1_destroy(offer);
}

void ShellProtocol::onClipboardReadable(int fd)
{
    if (!m_clipboardFdMime.contains(fd)) {
        ::close(fd);
        return;
    }
    char buffer[8192];
    const ssize_t n = ::read(fd, buffer, sizeof(buffer));
    if (n > 0) {
        m_clipboardPayloads[fd].append(buffer, static_cast<int>(n));
        return;
    }
    // EOF (0) or an error: this MIME's payload is complete.
    const QString mime = m_clipboardFdMime.take(fd);
    const QByteArray payload = m_clipboardPayloads.take(fd);
    if (!mime.isEmpty())
        m_clipboardReadPayloads.insert(mime, payload);
    ::close(fd);
    for (auto it = m_clipboardNotifierFd.begin(); it != m_clipboardNotifierFd.end(); ++it) {
        if (it.value() == fd) {
            it.key()->setEnabled(false);
            it.key()->deleteLater();
            m_clipboardNotifierFd.erase(it);
            break;
        }
    }
    if (m_clipboardNotifierFd.isEmpty() && !m_clipboardReadPayloads.isEmpty()) {
        const QMap<QString, QByteArray> payloads = m_clipboardReadPayloads;
        m_clipboardReadPayloads.clear();
        const QStringList mimes = m_clipboardMimes;
        emit clipboardObserved(mimes, payloads);
    }
}

void ShellProtocol::cancelClipboardRead()
{
    for (auto it = m_clipboardNotifierFd.begin(); it != m_clipboardNotifierFd.end(); ++it) {
        it.key()->setEnabled(false);
        it.key()->deleteLater();
        ::close(it.value());
    }
    m_clipboardNotifierFd.clear();
    m_clipboardFdMime.clear();
    m_clipboardPayloads.clear();
    m_clipboardReadPayloads.clear();
}

void ShellProtocol::onDataControlSourceSend(void *data, zwlr_data_control_source_v1 *,
                                            const char *mimeType, int32_t fd)
{
    auto *self = static_cast<ShellProtocol *>(data);
    const QByteArray payload =
        self->m_clipboardSourcePayloads.value(QString::fromUtf8(mimeType));
    ssize_t written = 0;
    while (written < payload.size()) {
        const ssize_t n = ::write(fd, payload.constData() + written, payload.size() - written);
        if (n <= 0)
            break;
        written += n;
    }
    ::close(fd);
}

void ShellProtocol::onDataControlSourceCancelled(void *data, zwlr_data_control_source_v1 *source)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_clipboardSource == source)
        self->m_clipboardSource = nullptr;
    zwlr_data_control_source_v1_destroy(source);
    self->m_clipboardSourcePayloads.clear();
}

bool ShellProtocol::offerClipboard(const QStringList &mimes,
                                   const QMap<QString, QByteArray> &payloads)
{
    if (!m_dataControlManager || !m_dataControlDevice)
        return false;

    m_clipboardSourcePayloads.clear();
    for (const QString &mime : mimes) {
        if (payloads.contains(mime))
            m_clipboardSourcePayloads.insert(mime, payloads.value(mime));
    }
    if (m_clipboardSourcePayloads.isEmpty())
        return false;

    if (m_clipboardSource) {
        zwlr_data_control_source_v1_destroy(m_clipboardSource);
        m_clipboardSource = nullptr;
    }
    m_clipboardSource = zwlr_data_control_manager_v1_create_data_source(m_dataControlManager);
    static const zwlr_data_control_source_v1_listener sourceListener = {
        onDataControlSourceSend,
        onDataControlSourceCancelled,
    };
    zwlr_data_control_source_v1_add_listener(m_clipboardSource, &sourceListener, this);
    for (const QString &mime : m_clipboardSourcePayloads.keys())
        zwlr_data_control_source_v1_offer(m_clipboardSource, mime.toUtf8().constData());
    zwlr_data_control_device_v1_set_selection(m_dataControlDevice, m_clipboardSource);
    wl_display_flush(m_display);
    return true;
}

void ShellProtocol::onPointerEnter(void *data, wl_pointer *, uint32_t, wl_surface *surface,
                                   wl_fixed_t x, wl_fixed_t y)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_pointerOnPopup = self->m_popupSurface && surface == self->m_popupSurface;
    self->m_pointerOnDockPopup =
        self->m_dockPopupSurface && surface == self->m_dockPopupSurface;
    self->m_pointerOnDock = self->m_dockSurface && surface == self->m_dockSurface;
    self->m_pointerOnBanner = self->m_bannerSurface && surface == self->m_bannerSurface;
    self->m_pointerOnNotificationCenter =
        self->m_notificationCenterSurface && surface == self->m_notificationCenterSurface;
    self->m_pointerOnControlCenter =
        self->m_controlCenterSurface && surface == self->m_controlCenterSurface;
    self->m_pointerOnOverview =
        self->m_overviewSurface && surface == self->m_overviewSurface;
    self->m_pointerOnChooser =
        self->m_chooserSurface && surface == self->m_chooserSurface;
    self->m_pointerOnWidgetGallery =
        self->m_widgetGallerySurface && surface == self->m_widgetGallerySurface;
    self->m_pointerOnScreenshot =
        self->m_screenshotSurface && surface == self->m_screenshotSurface;
    self->m_pointerOnScreenCast =
        self->m_screencastSurface && surface == self->m_screencastSurface;
    self->m_pointerOnPolkit =
        self->m_polkitSurface && surface == self->m_polkitSurface;
    self->m_pointerOnAppsDrawer =
        self->m_appsDrawerSurface && surface == self->m_appsDrawerSurface;
    self->m_pointerX = wl_fixed_to_double(x) + (self->m_pointerOnPopup ? self->m_popupX : 0);
    self->m_pointerY = wl_fixed_to_double(y) + (self->m_pointerOnPopup ? self->m_popupY : 0);
    if (self->m_pointerOnAppsDrawer) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->appsDrawerPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnPolkit) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->polkitPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnChooser) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->chooserPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnWidgetGallery) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->widgetGalleryPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnScreenshot) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->screenshotPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnScreenCast) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->screencastPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnNotificationCenter) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->notificationCenterPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnControlCenter) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->controlCenterPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnOverview) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->overviewPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnBanner) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->bannerPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnDockPopup) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->dockPopupPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnDock) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->dockPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    emit self->pointerMoved(self->m_pointerX, self->m_pointerY);
}

void ShellProtocol::onPointerLeave(void *data, wl_pointer *, uint32_t, wl_surface *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    const bool wasOverview = self->m_pointerOnOverview;
    const bool wasBanner = self->m_pointerOnBanner;
    const bool wasNotificationCenter = self->m_pointerOnNotificationCenter;
    const bool wasControlCenter = self->m_pointerOnControlCenter;
    const bool wasChooser = self->m_pointerOnChooser;
    const bool wasWidgetGallery = self->m_pointerOnWidgetGallery;
    const bool wasScreenshot = self->m_pointerOnScreenshot;
    const bool wasScreenCast = self->m_pointerOnScreenCast;
    const bool wasPolkit = self->m_pointerOnPolkit;
    const bool wasAppsDrawer = self->m_pointerOnAppsDrawer;
    const bool wasDockPopup = self->m_pointerOnDockPopup;
    const bool wasDock = self->m_pointerOnDock;
    self->m_pointerOnPopup = false;
    self->m_pointerOnDockPopup = false;
    self->m_pointerOnDock = false;
    self->m_pointerOnBanner = false;
    self->m_pointerOnNotificationCenter = false;
    self->m_pointerOnControlCenter = false;
    self->m_pointerOnOverview = false;
    self->m_pointerOnChooser = false;
    self->m_pointerOnWidgetGallery = false;
    self->m_pointerOnScreenshot = false;
    self->m_pointerOnScreenCast = false;
    self->m_pointerOnPolkit = false;
    self->m_pointerOnAppsDrawer = false;
    if (wasChooser)
        emit self->chooserPointerLeft();
    else if (wasWidgetGallery)
        emit self->widgetGalleryPointerLeft();
    else if (wasScreenshot)
        emit self->screenshotPointerLeft();
    else if (wasScreenCast)
        emit self->screencastPointerLeft();
    else if (wasPolkit)
        emit self->polkitPointerLeft();
    else if (wasAppsDrawer)
        emit self->appsDrawerPointerLeft();
    else if (wasOverview)
        emit self->overviewPointerLeft();
    else if (wasNotificationCenter)
        emit self->notificationCenterPointerLeft();
    else if (wasControlCenter)
        emit self->controlCenterPointerLeft();
    else if (wasBanner)
        emit self->bannerPointerLeft();
    else if (wasDockPopup)
        emit self->dockPopupPointerLeft();
    else if (wasDock)
        emit self->dockPointerLeft();
    else
        emit self->pointerLeft();
}

void ShellProtocol::onPointerMotion(void *data, wl_pointer *, uint32_t, wl_fixed_t x,
                                    wl_fixed_t y)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_pointerX = wl_fixed_to_double(x) + (self->m_pointerOnPopup ? self->m_popupX : 0);
    self->m_pointerY = wl_fixed_to_double(y) + (self->m_pointerOnPopup ? self->m_popupY : 0);
    if (self->m_pointerOnAppsDrawer) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->appsDrawerPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnPolkit) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->polkitPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnChooser) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->chooserPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnWidgetGallery) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->widgetGalleryPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnScreenshot) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->screenshotPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnScreenCast) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->screencastPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnNotificationCenter) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->notificationCenterPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnControlCenter) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->controlCenterPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnOverview) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->overviewPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnBanner) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->bannerPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnDockPopup) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->dockPopupPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    if (self->m_pointerOnDock) {
        self->m_pointerX = wl_fixed_to_double(x);
        self->m_pointerY = wl_fixed_to_double(y);
        emit self->dockPointerMoved(self->m_pointerX, self->m_pointerY);
        return;
    }
    emit self->pointerMoved(self->m_pointerX, self->m_pointerY);
}

void ShellProtocol::onPointerButton(void *data, wl_pointer *, uint32_t, uint32_t, uint32_t button,
                                    uint32_t state)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (self->m_pointerOnAppsDrawer) {
        emit self->appsDrawerPointerButton(self->m_pointerX, self->m_pointerY, button,
                                           state == WL_POINTER_BUTTON_STATE_PRESSED);
        return;
    }
    if (self->m_pointerOnPolkit) {
        emit self->polkitPointerButton(self->m_pointerX, self->m_pointerY, button,
                                       state == WL_POINTER_BUTTON_STATE_PRESSED);
        return;
    }
    if (self->m_pointerOnChooser) {
        emit self->chooserPointerButton(self->m_pointerX, self->m_pointerY, button,
                                        state == WL_POINTER_BUTTON_STATE_PRESSED);
        return;
    }
    if (self->m_pointerOnWidgetGallery) {
        emit self->widgetGalleryPointerButton(self->m_pointerX, self->m_pointerY, button,
                                              state == WL_POINTER_BUTTON_STATE_PRESSED);
        return;
    }
    if (self->m_pointerOnScreenshot) {
        emit self->screenshotPointerButton(self->m_pointerX, self->m_pointerY, button,
                                           state == WL_POINTER_BUTTON_STATE_PRESSED);
        return;
    }
    if (self->m_pointerOnScreenCast) {
        emit self->screencastPointerButton(self->m_pointerX, self->m_pointerY, button,
                                           state == WL_POINTER_BUTTON_STATE_PRESSED);
        return;
    }
    if (self->m_pointerOnNotificationCenter) {
        emit self->notificationCenterPointerButton(self->m_pointerX, self->m_pointerY, button,
                                                   state == WL_POINTER_BUTTON_STATE_PRESSED);
        return;
    }
    if (self->m_pointerOnControlCenter) {
        emit self->controlCenterPointerButton(self->m_pointerX, self->m_pointerY, button,
                                              state == WL_POINTER_BUTTON_STATE_PRESSED);
        return;
    }
    if (self->m_pointerOnOverview) {
        emit self->overviewPointerButton(self->m_pointerX, self->m_pointerY, button,
                                         state == WL_POINTER_BUTTON_STATE_PRESSED);
        return;
    }
    if (self->m_pointerOnBanner) {
        emit self->bannerPointerButton(self->m_pointerX, self->m_pointerY, button,
                                       state == WL_POINTER_BUTTON_STATE_PRESSED);
        return;
    }
    if (self->m_pointerOnDockPopup) {
        emit self->dockPopupPointerButton(self->m_pointerX, self->m_pointerY, button,
                                          state == WL_POINTER_BUTTON_STATE_PRESSED);
        return;
    }
    if (self->m_pointerOnDock) {
        emit self->dockPointerButton(self->m_pointerX, self->m_pointerY, button,
                                     state == WL_POINTER_BUTTON_STATE_PRESSED);
        return;
    }
    emit self->pointerButton(self->m_pointerX, self->m_pointerY, button,
                             state == WL_POINTER_BUTTON_STATE_PRESSED);
}

ShellProtocol::PointerSurface ShellProtocol::currentPointerSurface() const
{
    if (m_pointerOnAppsDrawer)
        return PointerSurface::AppsDrawer;
    if (m_pointerOnPolkit)
        return PointerSurface::Polkit;
    if (m_pointerOnChooser)
        return PointerSurface::Chooser;
    if (m_pointerOnScreenshot)
        return PointerSurface::Screenshot;
    if (m_pointerOnScreenCast)
        return PointerSurface::ScreenCast;
    if (m_pointerOnNotificationCenter)
        return PointerSurface::NotificationCenter;
    if (m_pointerOnControlCenter)
        return PointerSurface::ControlCenter;
    if (m_pointerOnOverview)
        return PointerSurface::Overview;
    if (m_pointerOnBanner)
        return PointerSurface::Banner;
    if (m_pointerOnDockPopup)
        return PointerSurface::DockPopup;
    if (m_pointerOnDock)
        return PointerSurface::Dock;
    if (m_pointerOnPopup)
        return PointerSurface::Popup;
    return PointerSurface::MenuBar;
}

void ShellProtocol::onPointerAxis(void *data, wl_pointer *, uint32_t, uint32_t axis,
                                  wl_fixed_t value)
{
    auto *self = static_cast<ShellProtocol *>(data);
    const PointerSurface surface = self->currentPointerSurface();
    if (surface == PointerSurface::None)
        return;
    // Wayland axis 0 is vertical, 1 is horizontal. Positive is down/right.
    emit self->pointerAxis(surface, self->m_pointerX, self->m_pointerY, axis,
                           wl_fixed_to_double(value));
}

void ShellProtocol::onKeyboardKeymap(void *, wl_keyboard *, uint32_t, int32_t fd, uint32_t)
{
    if (fd >= 0)
        ::close(fd);
}

void ShellProtocol::onKeyboardEnter(void *data, wl_keyboard *, uint32_t, wl_surface *surface,
                                    wl_array *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_keyboardOnDock = self->m_dockSurface && surface == self->m_dockSurface;
    self->m_keyboardOnOverview =
        self->m_overviewSurface && surface == self->m_overviewSurface;
    self->m_keyboardOnNotificationCenter =
        self->m_notificationCenterSurface && surface == self->m_notificationCenterSurface;
    self->m_keyboardOnControlCenter =
        self->m_controlCenterSurface && surface == self->m_controlCenterSurface;
    self->m_keyboardOnChooser =
        self->m_chooserSurface && surface == self->m_chooserSurface;
    self->m_keyboardOnWidgetGallery =
        self->m_widgetGallerySurface && surface == self->m_widgetGallerySurface;
    self->m_keyboardOnScreenshot =
        self->m_screenshotSurface && surface == self->m_screenshotSurface;
    self->m_keyboardOnScreenCast =
        self->m_screencastSurface && surface == self->m_screencastSurface;
    self->m_keyboardOnPolkit =
        self->m_polkitSurface && surface == self->m_polkitSurface;
    self->m_keyboardOnAppsDrawer =
        self->m_appsDrawerSurface && surface == self->m_appsDrawerSurface;
    self->m_keyboardOnLock = self->isLockSurface(surface);
    emit self->keyboardFocused(true);
    if (self->m_keyboardOnDock)
        emit self->dockKeyboardFocused(true);
    if (self->m_keyboardOnOverview)
        emit self->overviewKeyboardFocused(true);
    if (self->m_keyboardOnNotificationCenter)
        emit self->notificationCenterKeyboardFocused(true);
    if (self->m_keyboardOnControlCenter)
        emit self->controlCenterKeyboardFocused(true);
    if (self->m_keyboardOnChooser)
        emit self->chooserKeyboardFocused(true);
    if (self->m_keyboardOnWidgetGallery)
        emit self->widgetGalleryKeyboardFocused(true);
    if (self->m_keyboardOnScreenshot)
        emit self->screenshotKeyboardFocused(true);
    if (self->m_keyboardOnScreenCast)
        emit self->screencastKeyboardFocused(true);
    if (self->m_keyboardOnPolkit)
        emit self->polkitKeyboardFocused(true);
    if (self->m_keyboardOnAppsDrawer)
        emit self->appsDrawerKeyboardFocused(true);
}

void ShellProtocol::onKeyboardLeave(void *data, wl_keyboard *, uint32_t, wl_surface *surface)
{
    auto *self = static_cast<ShellProtocol *>(data);
    const bool wasDock = self->m_keyboardOnDock
            || (self->m_dockSurface && surface == self->m_dockSurface);
    const bool wasOverview = self->m_keyboardOnOverview
            || (self->m_overviewSurface && surface == self->m_overviewSurface);
    const bool wasNotificationCenter = self->m_keyboardOnNotificationCenter
            || (self->m_notificationCenterSurface
                && surface == self->m_notificationCenterSurface);
    const bool wasControlCenter = self->m_keyboardOnControlCenter
            || (self->m_controlCenterSurface && surface == self->m_controlCenterSurface);
    const bool wasChooser = self->m_keyboardOnChooser
            || (self->m_chooserSurface && surface == self->m_chooserSurface);
    const bool wasWidgetGallery = self->m_keyboardOnWidgetGallery
            || (self->m_widgetGallerySurface && surface == self->m_widgetGallerySurface);
    const bool wasScreenshot = self->m_keyboardOnScreenshot
            || (self->m_screenshotSurface && surface == self->m_screenshotSurface);
    const bool wasScreenCast = self->m_keyboardOnScreenCast
            || (self->m_screencastSurface && surface == self->m_screencastSurface);
    const bool wasPolkit = self->m_keyboardOnPolkit
            || (self->m_polkitSurface && surface == self->m_polkitSurface);
    const bool wasAppsDrawer = self->m_keyboardOnAppsDrawer
            || (self->m_appsDrawerSurface && surface == self->m_appsDrawerSurface);
    self->m_keyboardOnDock = false;
    self->m_keyboardOnOverview = false;
    self->m_keyboardOnNotificationCenter = false;
    self->m_keyboardOnControlCenter = false;
    self->m_keyboardOnChooser = false;
    self->m_keyboardOnWidgetGallery = false;
    self->m_keyboardOnScreenshot = false;
    self->m_keyboardOnScreenCast = false;
    self->m_keyboardOnPolkit = false;
    self->m_keyboardOnAppsDrawer = false;
    self->m_keyboardOnLock = false;
    emit self->keyboardFocused(false);
    if (wasDock)
        emit self->dockKeyboardFocused(false);
    if (wasOverview)
        emit self->overviewKeyboardFocused(false);
    if (wasNotificationCenter)
        emit self->notificationCenterKeyboardFocused(false);
    if (wasControlCenter)
        emit self->controlCenterKeyboardFocused(false);
    if (wasChooser)
        emit self->chooserKeyboardFocused(false);
    if (wasWidgetGallery)
        emit self->widgetGalleryKeyboardFocused(false);
    if (wasScreenshot)
        emit self->screenshotKeyboardFocused(false);
    if (wasScreenCast)
        emit self->screencastKeyboardFocused(false);
    if (wasPolkit)
        emit self->polkitKeyboardFocused(false);
    if (wasAppsDrawer)
        emit self->appsDrawerKeyboardFocused(false);
}

void ShellProtocol::onKeyboardKey(void *data, wl_keyboard *, uint32_t, uint32_t, uint32_t key,
                                  uint32_t state)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // While locked, keys are the lock UI's password input and are never
    // routed to the chrome scenes (T-12.3c).
    if (self->m_keyboardOnLock) {
        emit self->lockKeyEvent(key, state == WL_KEYBOARD_KEY_STATE_PRESSED,
                                self->m_keyboardShift);
        return;
    }
    if (self->m_keyboardOnPolkit) {
        emit self->polkitKeyEvent(key, state == WL_KEYBOARD_KEY_STATE_PRESSED,
                                  self->m_keyboardShift);
        return;
    }
    if (self->m_keyboardOnAppsDrawer) {
        emit self->appsDrawerKeyEvent(key, state == WL_KEYBOARD_KEY_STATE_PRESSED);
        return;
    }
    if (self->m_keyboardOnNotificationCenter) {
        emit self->notificationCenterKeyEvent(key,
                                              state == WL_KEYBOARD_KEY_STATE_PRESSED);
        return;
    }
    if (self->m_keyboardOnChooser) {
        emit self->chooserKeyEvent(key, state == WL_KEYBOARD_KEY_STATE_PRESSED);
        return;
    }
    if (self->m_keyboardOnWidgetGallery) {
        emit self->widgetGalleryKeyEvent(key, state == WL_KEYBOARD_KEY_STATE_PRESSED);
        return;
    }
    if (self->m_keyboardOnScreenshot) {
        emit self->screenshotKeyEvent(key, state == WL_KEYBOARD_KEY_STATE_PRESSED);
        return;
    }
    if (self->m_keyboardOnScreenCast) {
        emit self->screencastKeyEvent(key, state == WL_KEYBOARD_KEY_STATE_PRESSED);
        return;
    }
    emit self->keyEvent(key, state == WL_KEYBOARD_KEY_STATE_PRESSED,
                        self->m_keyboardModifiers);
}

void ShellProtocol::onKeyboardModifiers(void *data, wl_keyboard *, uint32_t, uint32_t depressed,
                                        uint32_t, uint32_t, uint32_t)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // WL_KEYBOARD_MODIFIER_MASK_SHIFT is bit 0.
    self->m_keyboardShift = (depressed & 0x1u) != 0;
    // Keep the full mask for the chrome chords (T-14.7t).
    self->m_keyboardModifiers = depressed;
}

// --- df_toplevel_manager ----------------------------------------------------

void ShellProtocol::onManagerOutput(void *data, df_toplevel_manager *, df_output *id)
{
    auto *self = static_cast<ShellProtocol *>(data);
    static const df_output_listener listener = {
        onOutputName,       onOutputGeometry, onOutputMode,        onOutputScale,
        onOutputTransform,  onOutputVrr,      onOutputNightLight,  onOutputReservedZone,
        onOutputDone,       onOutputRemoved,  onOutputBrightness,  onOutputMaterialState,
        onOutputMaterialRoleState,
    };
    df_output_add_listener(id, &listener, self);
    // Remember the output so a display policy can reach it (T-09.5). A policy
    // that arrived before the replay is applied now.
    self->m_outputs.append(id);
    if (self->m_displayKnown)
        self->applyDisplayPolicy();
}

void ShellProtocol::onManagerWorkspace(void *data, df_toplevel_manager *, df_workspace *id)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_workspaces.insert(id, WorkspaceInfo{});
    static const df_workspace_listener listener = {
        onWorkspaceName,     onWorkspaceIndex,  onWorkspaceActivated,
        onWorkspaceFullscreen, onWorkspaceWallpaper, onWorkspaceRemoved,
        onWorkspaceDone,
    };
    df_workspace_add_listener(id, &listener, self);
}

void ShellProtocol::onManagerWorkspaceActivated(void *data, df_toplevel_manager *,
                                                df_workspace *workspace, uint32_t index)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // The manager's activation event is the per-switch signal (the
    // `df_workspace.activated` property is only re-sent on structural syncs).
    // Activation is lockstep, so every Space with this index is active and
    // every other index is not (T-05/T-11).
    self->m_activeWorkspace = workspace;
    for (auto it = self->m_workspaces.begin(); it != self->m_workspaces.end(); ++it)
        it.value().active = (it.value().index == static_cast<int>(index));
    fprintf(stderr, "dragonfruit-shell: workspace activated index=%u\n", index);
    emit self->overviewDataChanged();
}

void ShellProtocol::onManagerToplevel(void *data, df_toplevel_manager *, df_toplevel *id)
{
    auto *self = static_cast<ShellProtocol *>(data);
    ToplevelInfo info;
    info.windowId = reinterpret_cast<quintptr>(id);
    self->m_toplevels.insert(id, info);
    self->m_toplevelOrder.append(id);
    self->m_toplevelById.insert(info.windowId, id);
    static const df_toplevel_listener listener = {
        onToplevelTitle,   onToplevelAppId,         onToplevelState,
        onToplevelWorkspaceEntered, onToplevelWorkspaceLeft, onToplevelOutputEntered,
        onToplevelOutputLeft, onToplevelClosed,     onToplevelDone,
    };
    df_toplevel_add_listener(id, &listener, self);
    self->emitDockState();
}

void ShellProtocol::onManagerFocused(void *data, df_toplevel_manager *, df_toplevel *id)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_focused = id;
    const ToplevelInfo info = id ? self->m_toplevels.value(id) : ToplevelInfo{};
    emit self->focusedAppChanged(info.appId, info.title);
    // The window chooser marks the frontmost window; a focus change updates it.
    self->emitDockState();
}

void ShellProtocol::onManagerAttention(void *data, df_toplevel_manager *, df_toplevel *id)
{
    auto *self = static_cast<ShellProtocol *>(data);
    if (!id)
        return;
    // The attention event carries the toplevel; the Dock keys on the app
    // identity (T-10 section 8.1). An id whose app_id has not arrived yet is
    // ignored rather than crashing.
    const ToplevelInfo info = self->m_toplevels.value(id);
    if (!info.appId.isEmpty())
        emit self->attentionRequested(info.appId);
}

void ShellProtocol::onManagerHotCorner(void *, df_toplevel_manager *, uint32_t, const char *)
{
}

void ShellProtocol::onManagerOverview(void *data, df_toplevel_manager *, uint32_t active,
                                      df_toplevel *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // The compositor's single overview state machine is the source of truth
    // (T-11); the shell only mirrors it for the chrome.
    self->m_overviewActive = active != 0;
    emit self->overviewChanged(self->m_overviewActive);
    emit self->overviewDataChanged();
}

void ShellProtocol::onManagerAppSwitcher(void *data, df_toplevel_manager *, uint32_t active,
                                         const char *appId, int32_t direction)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // The compositor-owned machine is the source of truth (T-06.1); the shell
    // only renders the projection. The recency cards follow before the batch
    // `done`, so hold them and emit one signal per completed batch.
    self->m_switcherActive = active != 0;
    self->m_switcherSelectedApp = QString::fromUtf8(appId ? appId : "");
    self->m_switcherDirection = direction;
    self->m_switcherPending = true;
    if (self->m_switcherActive)
        self->m_switcherEntries.clear();
}

void ShellProtocol::onManagerAppSwitcherEntry(void *data, df_toplevel_manager *, uint32_t index,
                                              const char *appId)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_switcherPending = true;
    QVariantMap entry;
    entry.insert(QStringLiteral("index"), static_cast<int>(index));
    entry.insert(QStringLiteral("appId"), QString::fromUtf8(appId ? appId : ""));
    self->m_switcherEntries.append(entry);
}

void ShellProtocol::onManagerInputAction(void *data, df_toplevel_manager *, const char *action,
                                         const char *source, uint32_t)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // The Dock consumes `focus-dock` / `toggle-dock` (T-10 section 20); every
    // trigger (shortcut, gesture, hot corner) arrives here.
    emit self->inputAction(QString::fromUtf8(action ? action : ""),
                           QString::fromUtf8(source ? source : ""));
}

void ShellProtocol::onManagerProgress(void *data, df_toplevel_manager *, const char *action,
                                      int32_t progress, int32_t, int32_t, uint32_t, uint32_t,
                                      uint32_t)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // One shared pipeline sample for the in-flight transition (T-11 FR-1):
    // the overview chrome fades/slides with the same curve as the gesture.
    self->m_overviewProgress = wl_fixed_to_double(progress);
    emit self->overviewProgress(self->m_overviewProgress,
                                QString::fromUtf8(action ? action : ""));
}

void ShellProtocol::onManagerAppAccelerator(void *data, df_toplevel_manager *, const char *appId,
                                            const char *action, const char *source, uint32_t serial)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // T-14.2b: the compositor matched a focus-scoped application accelerator
    // and hands it back; the owning app executes `action`. The shell is the
    // router, not the executor.
    emit self->appAccelerator(QString::fromUtf8(appId ? appId : ""),
                              QString::fromUtf8(action ? action : ""),
                              QString::fromUtf8(source ? source : ""), serial);
}

void ShellProtocol::onManagerScreenshotSaved(void *data, df_toplevel_manager *, const char *path)
{
    auto *self = static_cast<ShellProtocol *>(data);
    emit self->screenshotSaved(QString::fromUtf8(path ? path : ""));
}

void ShellProtocol::onManagerScreenshotFailed(void *data, df_toplevel_manager *,
                                              const char *reason)
{
    auto *self = static_cast<ShellProtocol *>(data);
    emit self->screenshotFailed(QString::fromUtf8(reason ? reason : ""));
}

void ShellProtocol::onManagerActivationResult(void *data, df_toplevel_manager *,
                                              const char *appId, uint32_t found)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // T-14.7g: the compositor answered an `activate_app`; `found` false means
    // there was no window to bring forward.
    emit self->activationResult(QString::fromUtf8(appId ? appId : ""), found != 0);
}

void ShellProtocol::onManagerDone(void *data, df_toplevel_manager *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // The replay is complete: the Space list is now known, so a wallpaper
    // selection that arrived before it can be forwarded (T-09.3). The output
    // list is known too, so a display selection can be applied (T-09.5).
    self->applyWallpaper();
    self->applyDisplayPolicy();
    if (!self->m_switcherPending)
        return;
    self->m_switcherPending = false;
    emit self->appSwitcherChanged(self->m_switcherActive, self->m_switcherEntries,
                                  self->m_switcherSelectedApp, self->m_switcherDirection);
}

// --- df_toplevel ------------------------------------------------------------

void ShellProtocol::onToplevelTitle(void *data, df_toplevel *toplevel, const char *title)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_toplevels[toplevel].title = QString::fromUtf8(title ? title : "");
    if (self->m_focused == toplevel) {
        const ToplevelInfo info = self->m_toplevels.value(toplevel);
        emit self->focusedAppChanged(info.appId, info.title);
    }
    self->emitDockState();
}

void ShellProtocol::onToplevelAppId(void *data, df_toplevel *toplevel, const char *appId)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_toplevels[toplevel].appId = QString::fromUtf8(appId ? appId : "");
    if (self->m_focused == toplevel) {
        const ToplevelInfo info = self->m_toplevels.value(toplevel);
        emit self->focusedAppChanged(info.appId, info.title);
    }
    self->emitDockState();
}

void ShellProtocol::onToplevelState(void *data, df_toplevel *toplevel, uint32_t state)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_toplevels[toplevel].state = state;
    self->emitDockState();
}

void ShellProtocol::onToplevelWorkspaceEntered(void *data, df_toplevel *toplevel,
                                               df_workspace *workspace)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_toplevels[toplevel].workspace = workspace;
    self->emitDockState();
}

void ShellProtocol::onToplevelWorkspaceLeft(void *data, df_toplevel *toplevel, df_workspace *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_toplevels[toplevel].workspace = nullptr;
    self->emitDockState();
}

void ShellProtocol::onToplevelOutputEntered(void *, df_toplevel *, df_output *)
{
}

void ShellProtocol::onToplevelOutputLeft(void *, df_toplevel *, df_output *)
{
}

void ShellProtocol::onToplevelClosed(void *data, df_toplevel *toplevel)
{
    auto *self = static_cast<ShellProtocol *>(data);
    const auto info = self->m_toplevels.constFind(toplevel);
    if (info != self->m_toplevels.constEnd() && info->windowId)
        self->m_toplevelById.remove(info->windowId);
    self->m_toplevels.remove(toplevel);
    self->m_toplevelOrder.removeAll(toplevel);
    if (self->m_focused == toplevel) {
        self->m_focused = nullptr;
        emit self->focusedAppChanged(QString(), QString());
    }
    self->emitDockState();
}

void ShellProtocol::onToplevelDone(void *, df_toplevel *)
{
}

// The running projection is built by the pure `buildDockProjection` core
// (dockprojection.h): the section 22 lifecycle grouping (app_id changes,
// cross-Space windows, minimize state, rapid open/close) is unit-tested by
// tst_dockcore. This method only resolves the private-protocol data.
void ShellProtocol::emitDockState()
{
    QList<DockWindow> windows;
    windows.reserve(m_toplevelOrder.size());
    for (df_toplevel *toplevel : std::as_const(m_toplevelOrder)) {
        const ToplevelInfo &info = m_toplevels.value(toplevel);
        const WorkspaceInfo ws = m_workspaces.value(info.workspace);
        DockWindow window;
        window.windowId = info.windowId;
        window.appId = info.appId;
        window.title = info.title;
        window.minimized = (info.state & 0x1u) != 0;
        window.focused = toplevel == m_focused;
        window.workspaceIndex = ws.index;
        window.workspaceName = ws.name;
        windows.append(window);
    }
    emit dockStateChanged(buildDockProjection(windows));
    // The overview strip is a projection of the same window/workspace state
    // (T-11); refresh it whenever the running set changes.
    emit overviewDataChanged();
}

quintptr ShellProtocol::focusedWindowId() const
{
    if (!m_focused)
        return 0;
    return m_toplevels.value(m_focused).windowId;
}

bool ShellProtocol::focusedWindowZoomed() const
{
    if (!m_focused)
        return false;
    return (m_toplevels.value(m_focused).state & 0x2u) != 0;
}

void ShellProtocol::setToplevelZoomed(const QString &windowId, bool zoomed)
{
    bool ok = false;
    const quintptr id = windowId.toULongLong(&ok);
    df_toplevel *toplevel = ok ? toplevelForId(id) : nullptr;
    if (!toplevel)
        return;
    if (zoomed)
        df_toplevel_zoom(toplevel);
    else
        df_toplevel_unzoom(toplevel);
}

df_toplevel *ShellProtocol::toplevelForId(quintptr windowId) const
{
    return m_toplevelById.value(windowId, nullptr);
}

// --- wl_buffer --------------------------------------------------------------

void ShellProtocol::onDmabufCreated(void *data, zwp_linux_buffer_params_v1 *params,
                                   wl_buffer *buffer)
{
    auto *self = static_cast<ShellProtocol *>(data);
    const DmabufParamOwner owner = self->m_dmabufParams.take(params);
    if (owner.surface && owner.slot >= 0) {
        auto it = self->m_surfaceBuffers.find(owner.surface);
        if (it != self->m_surfaceBuffers.end()) {
            auto slotIt = it.value()->dmabufSlots.find(owner.slot);
            if (slotIt != it.value()->dmabufSlots.end()) {
                ShellDmabufSlot &slot = slotIt.value();
                slot.buffer = buffer;
                slot.creating = false;
                slot.params = nullptr;
                self->m_dmabufOwners.insert(
                    buffer, DmabufBufferOwner{owner.surface, owner.slot, 0});
                static const wl_buffer_listener bufferListener = { onBufferRelease };
                wl_buffer_add_listener(buffer, &bufferListener, self);
                zwp_linux_buffer_params_v1_destroy(params);
                return;
            }
        }
    }
    if (buffer)
        wl_buffer_destroy(buffer);
    zwp_linux_buffer_params_v1_destroy(params);
}

void ShellProtocol::onDmabufFailed(void *data, zwp_linux_buffer_params_v1 *params)
{
    auto *self = static_cast<ShellProtocol *>(data);
    const DmabufParamOwner owner = self->m_dmabufParams.take(params);
    if (owner.surface && owner.slot >= 0) {
        auto it = self->m_surfaceBuffers.find(owner.surface);
        if (it != self->m_surfaceBuffers.end()) {
            auto slotIt = it.value()->dmabufSlots.find(owner.slot);
            if (slotIt != it.value()->dmabufSlots.end()) {
                ShellDmabufSlot &slot = slotIt.value();
                slot.creating = false;
                slot.failed = true;
                slot.params = nullptr;
                slot.failReason = QStringLiteral("import-failed");
            }
        }
    }
    zwp_linux_buffer_params_v1_destroy(params);
}

void ShellProtocol::onBufferRelease(void *data, wl_buffer *buffer)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // T-21.1d: a dmabuf buffer is reusable and stays owned by the shell. Its
    // release clears the slot's busy flag and returns it to the producer ring;
    // the next frame may render into that FBO again.
    auto dmabufOwner = self->m_dmabufOwners.constFind(buffer);
    if (dmabufOwner != self->m_dmabufOwners.constEnd()) {
        const DmabufBufferOwner owner = dmabufOwner.value();
        QString label;
        if (owner.surface) {
            label = self->surfaceLabel(owner.surface);
            auto it = self->m_surfaceBuffers.find(owner.surface);
            if (it != self->m_surfaceBuffers.end()) {
                auto slotIt = it.value()->dmabufSlots.find(owner.slot);
                if (slotIt != it.value()->dmabufSlots.end()
                    && slotIt.value().serial == owner.serial) {
                    slotIt.value().busy = false;
                }
            }
        }
        if (!label.isEmpty()) {
            emit self->dmabufReleased(label, owner.slot, owner.serial);
            emit self->bufferReleased(label);
        }
        return;
    }

    // The owner mapping lives for the whole buffer lifetime; a current-size
    // buffer returns to the pool and will be released again on a later frame.
    const auto owner = self->m_bufferOwners.value(buffer);
    wl_surface *surface = owner.first;
    const int slot = owner.second;
    QString label;
    bool destroyBuffer = true;
    if (surface) {
        label = self->surfaceLabel(surface);
        auto it = self->m_surfaceBuffers.find(surface);
        if (it != self->m_surfaceBuffers.end()) {
            ShellSurfaceBuffer *surfaceBuffer = it.value();
            surfaceBuffer->pool.release(slot);
            if (surfaceBuffer->pool.isRetired(slot)) {
                // The buffer belongs to a retired size era; its memory can go
                // now that the compositor is done with it. The release
                // listener owns the single wl_buffer_destroy below.
                self->destroyShmSlot(surfaceBuffer, slot, false);
                surfaceBuffer->pool.remove(slot);
            } else {
                // A current-size buffer returns to the pool with its
                // wl_buffer intact; the next commit attaches it again.
                destroyBuffer = false;
            }
        }
    }
    if (destroyBuffer)
        wl_buffer_destroy(buffer);
    if (!label.isEmpty())
        emit self->bufferReleased(label);
}

// --- df_output / df_workspace (unused properties) ---------------------------

void ShellProtocol::onOutputName(void *data, df_output *output, const char *name)
{
    // Remember the connector/name so the ScreenCast picker can label monitors
    // (T-13.4a).
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_outputInfo[output].name = QString::fromUtf8(name ? name : "");
}
void ShellProtocol::onOutputGeometry(void *data, df_output *output, int32_t x, int32_t y,
                                     int32_t width, int32_t height)
{
    // Remember the output's global layout rect so the Dock can translate its
    // surface-local tile into the compositor's coordinates (T-14.7l).
    auto *self = static_cast<ShellProtocol *>(data);
    OutputInfo &info = self->m_outputInfo[output];
    info.x = static_cast<int>(x);
    info.y = static_cast<int>(y);
    info.geometryWidth = static_cast<int>(width);
    info.geometryHeight = static_cast<int>(height);
}
void ShellProtocol::onOutputMode(void *data, df_output *output, uint32_t, uint32_t width,
                                 uint32_t height, uint32_t refresh)
{
    // The last mode announced is the output's current size for the ScreenCast
    // picker (T-13.4a).
    auto *self = static_cast<ShellProtocol *>(data);
    OutputInfo &info = self->m_outputInfo[output];
    info.width = static_cast<int>(width);
    info.height = static_cast<int>(height);
    const uint32_t previous = info.refreshMilliHz;
    info.refreshMilliHz = refresh == 0 ? 60'000 : refresh;
    if (!self->m_outputs.isEmpty() && output == self->m_outputs.first()
        && previous != info.refreshMilliHz)
        emit self->outputRefreshChanged();
}

uint32_t ShellProtocol::outputRefreshMilliHz() const
{
    if (!m_outputs.isEmpty()) {
        auto info = m_outputInfo.constFind(m_outputs.first());
        if (info != m_outputInfo.cend())
            return info->refreshMilliHz;
    }
    return 60'000;
}
void ShellProtocol::onOutputScale(void *data, df_output *output, wl_fixed_t scale)
{
    auto *self = static_cast<ShellProtocol *>(data);
    OutputInfo &info = self->m_outputInfo[output];
    info.scale = wl_fixed_to_double(scale);
    if (!qIsFinite(info.scale) || info.scale <= 0.0)
        info.scale = 1.0;
    // The chrome surfaces are anchored to the output set as a whole; the
    // first announced output is the primary and owns the shared chrome
    // scale. Per-output differing scales remain a shell limitation (the
    // surfaces themselves are single Wayland surfaces).
    if (self->m_outputs.isEmpty() || output == self->m_outputs.first()) {
        if (!qFuzzyCompare(self->m_outputScale, info.scale)) {
            self->m_outputScale = info.scale;
            emit self->outputScaleChanged(info.scale);
        } else {
            self->m_outputScale = info.scale;
        }
    }
}

void ShellProtocol::onPreferredScale(void *data, wp_fractional_scale_v1 *fractional,
                                     uint32_t scale)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // The protocol sends the numerator of a fraction with denominator 120.
    const qreal preferred = qreal(scale) / 120.0;
    if (!qIsFinite(preferred) || preferred <= 0.0)
        return;
    for (auto it = self->m_surfaceScales.begin(); it != self->m_surfaceScales.end(); ++it) {
        if (it.value().fractional != fractional)
            continue;
        if (it.value().preferredKnown
            && qFuzzyCompare(it.value().preferredScale, preferred)) {
            return;
        }
        it.value().preferredScale = preferred;
        it.value().preferredKnown = true;
        emit self->preferredScaleChanged(self->surfaceLabel(it.key()), preferred);
        return;
    }
}
void ShellProtocol::onOutputTransform(void *, df_output *, uint32_t) {}
void ShellProtocol::onOutputVrr(void *, df_output *, uint32_t) {}
void ShellProtocol::onOutputNightLight(void *, df_output *, uint32_t, uint32_t) {}
void ShellProtocol::onOutputBrightness(void *, df_output *, wl_fixed_t) {}
void ShellProtocol::onOutputReservedZone(void *, df_output *, uint32_t edge, uint32_t thickness)
{
    fprintf(stderr, "dragonfruit-shell: output reserved zone edge=%u thickness=%u\n", edge,
            thickness);
}
void ShellProtocol::onOutputMaterialState(void *data, df_output *, uint32_t tier,
                                          uint32_t blurActive, uint32_t fallback)
{
    auto *self = static_cast<ShellProtocol *>(data);
    const bool active = blurActive != 0;
    if (self->m_materialTier == tier && self->m_materialBlurActive == active
            && self->m_materialFallback == fallback)
        return;
    self->m_materialTier = tier;
    self->m_materialBlurActive = active;
    self->m_materialFallback = fallback;
    emit self->materialStateChanged(tier, active, fallback);
}

void ShellProtocol::onOutputMaterialRoleState(void *data, df_output *, uint32_t role,
                                              uint32_t tier, uint32_t blurActive,
                                              uint32_t fallback)
{
    auto *self = static_cast<ShellProtocol *>(data);
    const bool active = blurActive != 0;
    auto &state = self->m_materialRoles[role];
    if (state.tier == tier && state.blurActive == active && state.fallback == fallback)
        return;
    state.tier = tier;
    state.blurActive = active;
    state.fallback = fallback;
    emit self->materialRoleStateChanged(role, tier, active, fallback);
}
void ShellProtocol::onOutputDone(void *, df_output *) {}
void ShellProtocol::onOutputRemoved(void *data, df_output *output)
{
    auto *self = static_cast<ShellProtocol *>(data);
    const uint32_t previous = self->outputRefreshMilliHz();
    self->m_outputs.removeAll(output);
    self->m_outputInfo.remove(output);
    if (previous != self->outputRefreshMilliHz())
        emit self->outputRefreshChanged();
    // Transient overlays are per-output; close them rather than leaving a
    // stale configure anchored to a display that no longer exists.
    emit self->outputRemoved();
}
void ShellProtocol::onWorkspaceName(void *data, df_workspace *workspace, const char *name)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_workspaces[workspace].name = QString::fromUtf8(name ? name : "");
    self->emitDockState();
}
void ShellProtocol::onWorkspaceIndex(void *data, df_workspace *workspace, uint32_t index)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_workspaces[workspace].index = static_cast<int>(index);
    self->emitDockState();
}
void ShellProtocol::onWorkspaceActivated(void *data, df_workspace *workspace, uint32_t active)
{
    // The event names the Space that just became active (T-10 section 13);
    // remember it so "Assign to This Desktop" can target it. The strip also
    // highlights the active card (T-11).
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_activeWorkspace = workspace;
    self->m_workspaces[workspace].active = active != 0;
    if (active != 0)
        emit self->activeWallpaperChanged();
    emit self->overviewDataChanged();
}
void ShellProtocol::onWorkspaceFullscreen(void *data, df_workspace *workspace, uint32_t fullscreen)
{
    // A fullscreen window owns a dedicated Space while it exists; the
    // workspace strip marks it (T-11 FR-10).
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_workspaces[workspace].fullscreen = fullscreen != 0;
    emit self->overviewDataChanged();
}
void ShellProtocol::onWorkspaceWallpaper(void *data, df_workspace *workspace,
                                         const char *source, uint32_t fit, uint32_t color)
{
    auto *self = static_cast<ShellProtocol *>(data);
    auto &info = self->m_workspaces[workspace];
    info.wallpaperSource = QString::fromUtf8(source ? source : "");
    info.wallpaperFit = fit;
    info.wallpaperColor = QColor::fromRgba(color);
    if (self->m_activeWorkspace == workspace)
        emit self->activeWallpaperChanged();
}
void ShellProtocol::onWorkspaceRemoved(void *data, df_workspace *workspace)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_workspaces.remove(workspace);
    // Never keep a dangling active-Space pointer for "Assign to This Desktop".
    if (self->m_activeWorkspace == workspace)
        self->m_activeWorkspace = nullptr;
    emit self->overviewDataChanged();
}
void ShellProtocol::onWorkspaceDone(void *, df_workspace *) {}
