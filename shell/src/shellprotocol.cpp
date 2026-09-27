// SPDX-License-Identifier: MIT
#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif

#include "shellprotocol.h"

#include <algorithm>
#include <cerrno>
#include <cstdio>
#include <cstring>
#include <utility>

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

#include "dockdrops.h"
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
// The vendored standard protocol (MIT); the shell's clipboard history
// observes and forwards the selection through it (T-13.5b, ADR 0082).
#include "wlr-data-control-unstable-v1-client-protocol.h"

#ifndef DF_LOCKSTEP_VERSION
#define DF_LOCKSTEP_VERSION 1
#endif

namespace {

constexpr uint32_t kAnchorTop = 1;
constexpr uint32_t kAnchorBottom = 2;
constexpr uint32_t kAnchorLeft = 4;
constexpr uint32_t kAnchorRight = 8;

// The banner card's inset from the output's right edge (T-11.1a).
constexpr int kBannerMargin = 8;

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
    wl_surface_commit(m_surface);

    if (wl_display_roundtrip(m_display) < 0)
        return fail(QStringLiteral("menu-bar configure roundtrip failed"));
    return true;
}

bool ShellProtocol::createPopupSurface()
{
    if (m_popupSurface || m_popupLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_popupSurface = wl_compositor_create_surface(m_compositor);
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
    m_popupX = x;
    m_popupY = y;
    df_layer_surface_set_margin(m_popupLayer, y, 0, 0, x);
    df_layer_surface_set_size(m_popupLayer, width, height);
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::commitPopupImage(const QImage &image)
{
    if (!m_popupSurface)
        return false;
    if (!commitTo(m_popupSurface, image))
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
    wl_surface_attach(m_popupSurface, nullptr, 0, 0);
    wl_surface_commit(m_popupSurface);
    m_popupMapped = false;
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::commitImage(const QImage &image)
{
    if (!m_surface)
        return false;
    return commitTo(m_surface, image);
}

bool ShellProtocol::createOverviewSurface()
{
    if (m_overviewSurface || m_overviewLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_overviewSurface = wl_compositor_create_surface(m_compositor);
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

bool ShellProtocol::commitOverviewImage(const QImage &image)
{
    if (!m_overviewSurface)
        return false;
    if (!commitTo(m_overviewSurface, image))
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
    wl_surface_attach(m_overviewSurface, nullptr, 0, 0);
    wl_surface_commit(m_overviewSurface);
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

bool ShellProtocol::commitSwitcherImage(const QImage &image)
{
    if (!m_switcherSurface)
        return false;
    if (!commitTo(m_switcherSurface, image))
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
    wl_surface_attach(m_switcherSurface, nullptr, 0, 0);
    wl_surface_commit(m_switcherSurface);
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

bool ShellProtocol::commitBannerImage(const QImage &image)
{
    if (!m_bannerSurface)
        return false;
    if (!commitTo(m_bannerSurface, image))
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
    wl_surface_attach(m_bannerSurface, nullptr, 0, 0);
    wl_surface_commit(m_bannerSurface);
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

// --- Control Center overlay (T-11.3a) --------------------------------------

bool ShellProtocol::createControlCenterSurface(int width, int height, int topMargin)
{
    if (m_controlCenterSurface || m_controlCenterLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_controlCenterSurface = wl_compositor_create_surface(m_compositor);
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

bool ShellProtocol::commitControlCenterImage(const QImage &image)
{
    if (!m_controlCenterSurface)
        return false;
    if (!commitTo(m_controlCenterSurface, image))
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
    wl_surface_attach(m_controlCenterSurface, nullptr, 0, 0);
    wl_surface_commit(m_controlCenterSurface);
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

bool ShellProtocol::commitOsdImage(const QImage &image)
{
    if (!m_osdSurface)
        return false;
    if (!commitTo(m_osdSurface, image))
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
    wl_surface_attach(m_osdSurface, nullptr, 0, 0);
    wl_surface_commit(m_osdSurface);
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

bool ShellProtocol::commitChooserImage(const QImage &image)
{
    if (!m_chooserSurface)
        return false;
    if (!commitTo(m_chooserSurface, image))
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
    wl_surface_attach(m_chooserSurface, nullptr, 0, 0);
    wl_surface_commit(m_chooserSurface);
    m_chooserMapped = false;
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

bool ShellProtocol::commitScreenshotImage(const QImage &image)
{
    if (!m_screenshotSurface)
        return false;
    if (!commitTo(m_screenshotSurface, image))
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
    wl_surface_attach(m_screenshotSurface, nullptr, 0, 0);
    wl_surface_commit(m_screenshotSurface);
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

bool ShellProtocol::commitScreenCastImage(const QImage &image)
{
    if (!m_screencastSurface)
        return false;
    if (!commitTo(m_screencastSurface, image))
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
    wl_surface_attach(m_screencastSurface, nullptr, 0, 0);
    wl_surface_commit(m_screencastSurface);
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

bool ShellProtocol::commitPolkitImage(const QImage &image)
{
    if (!m_polkitSurface)
        return false;
    if (!commitTo(m_polkitSurface, image))
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
    wl_surface_attach(m_polkitSurface, nullptr, 0, 0);
    wl_surface_commit(m_polkitSurface);
    m_polkitMapped = false;
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
        if (it.value().surface)
            wl_surface_destroy(it.value().surface);
    }
    m_lockSurfaces.clear();
    m_sessionLocked = false;
    m_keyboardOnLock = false;
    if (m_display)
        wl_display_flush(m_display);
}

bool ShellProtocol::commitLockImage(quintptr lockSurfaceId, const QImage &image)
{
    auto *lockSurface = reinterpret_cast<ext_session_lock_surface_v1 *>(lockSurfaceId);
    auto it = m_lockSurfaces.find(lockSurface);
    if (it == m_lockSurfaces.end() || !it.value().surface)
        return false;
    return commitTo(it.value().surface, image);
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

bool ShellProtocol::commitDockImage(const QImage &image)
{
    if (!m_dockSurface)
        return false;
    if (!commitTo(m_dockSurface, image))
        return false;
    m_dockMapped = true;
    return true;
}

bool ShellProtocol::setDockInputRegion(const QList<QRect> &rects)
{
    if (!m_dockSurface || !m_compositor)
        return false;
    wl_region *region = wl_compositor_create_region(m_compositor);
    if (!region)
        return false;
    for (const QRect &rect : rects) {
        if (rect.width() > 0 && rect.height() > 0)
            wl_region_add(region, rect.x(), rect.y(), rect.width(), rect.height());
    }
    // An empty region passes every click through (the hidden Dock and the
    // transparent magnified band; T-10 FR-13).
    wl_surface_set_input_region(m_dockSurface, region);
    wl_region_destroy(region);
    return true;
}

bool ShellProtocol::setDockPanelRect(int x, int y, int width, int height)
{
    if (!m_dockLayer)
        return false;
    df_layer_surface_set_panel_rect(m_dockLayer, x, y, width, height);
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::createDockPopupSurface(DockPosition position)
{
    if (m_dockPopupSurface || m_dockPopupLayer)
        return true;
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_dockPopupSurface = wl_compositor_create_surface(m_compositor);
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
                                         int height)
{
    if (!m_dockPopupLayer || !m_dockPopupSurface)
        return false;
    // set_margin(top, right, bottom, left)
    df_layer_surface_set_margin(m_dockPopupLayer, top, right, bottom, left);
    df_layer_surface_set_size(m_dockPopupLayer, width, height);
    if (m_display)
        wl_display_flush(m_display);
    return true;
}

bool ShellProtocol::commitDockPopupImage(const QImage &image)
{
    if (!m_dockPopupSurface)
        return false;
    if (!commitTo(m_dockPopupSurface, image))
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
    wl_surface_attach(m_dockPopupSurface, nullptr, 0, 0);
    wl_surface_commit(m_dockPopupSurface);
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

bool ShellProtocol::commitTo(wl_surface *surface, const QImage &image)
{
    if (!surface || !m_shm)
        return false;

    QImage frame = image.convertToFormat(QImage::Format_ARGB32_Premultiplied);
    const int width = frame.width();
    const int height = frame.height();
    if (width <= 0 || height <= 0)
        return false;
    const int stride = width * 4;
    const size_t size = static_cast<size_t>(stride) * static_cast<size_t>(height);

    int fd = createShmFile(size);
    if (fd < 0)
        return fail(QStringLiteral("cannot allocate a shared-memory buffer: %1")
                        .arg(QString::fromLocal8Bit(strerror(errno))));

    void *data = mmap(nullptr, size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (data == MAP_FAILED) {
        ::close(fd);
        return fail(QStringLiteral("cannot map the shared-memory buffer"));
    }
    std::memcpy(data, frame.constBits(), size);
    munmap(data, size);

    wl_shm_pool *pool = wl_shm_create_pool(m_shm, fd, static_cast<int32_t>(size));
    wl_buffer *buffer =
        wl_shm_pool_create_buffer(pool, 0, width, height, stride, WL_SHM_FORMAT_ARGB8888);
    wl_shm_pool_destroy(pool);
    ::close(fd);

    static const wl_buffer_listener bufferListener = { onBufferRelease };
    wl_buffer_add_listener(buffer, &bufferListener, this);
    m_buffers.insert(buffer);

    wl_surface_attach(surface, buffer, 0, 0);
    wl_surface_damage_buffer(surface, 0, 0, width, height);
    wl_surface_commit(surface);
    wl_display_flush(m_display);
    return true;
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
    for (wl_buffer *buffer : std::as_const(m_buffers))
        wl_buffer_destroy(buffer);
    m_buffers.clear();
    // Session lock (T-12.3a): destroy the lock object only when it is not
    // locked. A locked lock object cannot be destroyed (protocol error), and
    // leaving it alive lets the connection drop keep the compositor locked —
    // the fail-secure path.
    if (m_sessionLock && !m_sessionLocked)
        ext_session_lock_v1_destroy(m_sessionLock);
    m_sessionLock = nullptr;
    for (auto it = m_lockSurfaces.begin(); it != m_lockSurfaces.end(); ++it) {
        if (it.value().surface)
            wl_surface_destroy(it.value().surface);
    }
    m_lockSurfaces.clear();
    m_lockOutputs.clear();
    m_sessionLocked = false;
    m_keyboardOnLock = false;
    m_lockManager = nullptr;
    if (m_switcherLayer)
        df_layer_surface_destroy(m_switcherLayer);
    if (m_switcherSurface)
        wl_surface_destroy(m_switcherSurface);
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
    m_dockSurface = nullptr;
    m_dockMapped = false;
    m_dockPopupLayer = nullptr;
    m_dockPopupSurface = nullptr;
    m_dockPopupMapped = false;
    m_overviewLayer = nullptr;
    m_overviewSurface = nullptr;
    m_overviewMapped = false;
    m_pointerOnOverview = false;
    m_pointerOnBanner = false;
    m_pointerOnControlCenter = false;
    m_pointerOnChooser = false;
    m_pointerOnScreenshot = false;
    m_pointerOnScreenCast = false;
    m_pointerOnPolkit = false;
    m_keyboardOnOverview = false;
    m_keyboardOnControlCenter = false;
    m_keyboardOnChooser = false;
    m_keyboardOnScreenshot = false;
    m_keyboardOnScreenCast = false;
    m_keyboardOnPolkit = false;
    m_screenshotLayer = nullptr;
    m_screenshotSurface = nullptr;
    m_screenshotMapped = false;
    m_screencastLayer = nullptr;
    m_screencastSurface = nullptr;
    m_screencastMapped = false;
    m_polkitLayer = nullptr;
    m_polkitSurface = nullptr;
    m_polkitMapped = false;
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

void ShellProtocol::onLayerClosed(void *data, df_layer_surface *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    emit self->surfaceClosed();
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
    self->m_pointerOnControlCenter =
        self->m_controlCenterSurface && surface == self->m_controlCenterSurface;
    self->m_pointerOnOverview =
        self->m_overviewSurface && surface == self->m_overviewSurface;
    self->m_pointerOnChooser =
        self->m_chooserSurface && surface == self->m_chooserSurface;
    self->m_pointerOnScreenshot =
        self->m_screenshotSurface && surface == self->m_screenshotSurface;
    self->m_pointerOnScreenCast =
        self->m_screencastSurface && surface == self->m_screencastSurface;
    self->m_pointerOnPolkit =
        self->m_polkitSurface && surface == self->m_polkitSurface;
    self->m_pointerX = wl_fixed_to_double(x) + (self->m_pointerOnPopup ? self->m_popupX : 0);
    self->m_pointerY = wl_fixed_to_double(y) + (self->m_pointerOnPopup ? self->m_popupY : 0);
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
    const bool wasControlCenter = self->m_pointerOnControlCenter;
    const bool wasChooser = self->m_pointerOnChooser;
    const bool wasScreenshot = self->m_pointerOnScreenshot;
    const bool wasScreenCast = self->m_pointerOnScreenCast;
    const bool wasPolkit = self->m_pointerOnPolkit;
    const bool wasDockPopup = self->m_pointerOnDockPopup;
    const bool wasDock = self->m_pointerOnDock;
    self->m_pointerOnPopup = false;
    self->m_pointerOnDockPopup = false;
    self->m_pointerOnDock = false;
    self->m_pointerOnBanner = false;
    self->m_pointerOnControlCenter = false;
    self->m_pointerOnOverview = false;
    self->m_pointerOnChooser = false;
    self->m_pointerOnScreenshot = false;
    self->m_pointerOnScreenCast = false;
    self->m_pointerOnPolkit = false;
    if (wasChooser)
        emit self->chooserPointerLeft();
    else if (wasScreenshot)
        emit self->screenshotPointerLeft();
    else if (wasScreenCast)
        emit self->screencastPointerLeft();
    else if (wasPolkit)
        emit self->polkitPointerLeft();
    else if (wasOverview)
        emit self->overviewPointerLeft();
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

void ShellProtocol::onPointerAxis(void *, wl_pointer *, uint32_t, uint32_t, wl_fixed_t)
{
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
    self->m_keyboardOnControlCenter =
        self->m_controlCenterSurface && surface == self->m_controlCenterSurface;
    self->m_keyboardOnChooser =
        self->m_chooserSurface && surface == self->m_chooserSurface;
    self->m_keyboardOnScreenshot =
        self->m_screenshotSurface && surface == self->m_screenshotSurface;
    self->m_keyboardOnScreenCast =
        self->m_screencastSurface && surface == self->m_screencastSurface;
    self->m_keyboardOnPolkit =
        self->m_polkitSurface && surface == self->m_polkitSurface;
    self->m_keyboardOnLock = self->isLockSurface(surface);
    emit self->keyboardFocused(true);
    if (self->m_keyboardOnDock)
        emit self->dockKeyboardFocused(true);
    if (self->m_keyboardOnOverview)
        emit self->overviewKeyboardFocused(true);
    if (self->m_keyboardOnControlCenter)
        emit self->controlCenterKeyboardFocused(true);
    if (self->m_keyboardOnChooser)
        emit self->chooserKeyboardFocused(true);
    if (self->m_keyboardOnScreenshot)
        emit self->screenshotKeyboardFocused(true);
    if (self->m_keyboardOnScreenCast)
        emit self->screencastKeyboardFocused(true);
    if (self->m_keyboardOnPolkit)
        emit self->polkitKeyboardFocused(true);
}

void ShellProtocol::onKeyboardLeave(void *data, wl_keyboard *, uint32_t, wl_surface *surface)
{
    auto *self = static_cast<ShellProtocol *>(data);
    const bool wasDock = self->m_keyboardOnDock
            || (self->m_dockSurface && surface == self->m_dockSurface);
    const bool wasOverview = self->m_keyboardOnOverview
            || (self->m_overviewSurface && surface == self->m_overviewSurface);
    const bool wasControlCenter = self->m_keyboardOnControlCenter
            || (self->m_controlCenterSurface && surface == self->m_controlCenterSurface);
    const bool wasChooser = self->m_keyboardOnChooser
            || (self->m_chooserSurface && surface == self->m_chooserSurface);
    const bool wasScreenshot = self->m_keyboardOnScreenshot
            || (self->m_screenshotSurface && surface == self->m_screenshotSurface);
    const bool wasScreenCast = self->m_keyboardOnScreenCast
            || (self->m_screencastSurface && surface == self->m_screencastSurface);
    const bool wasPolkit = self->m_keyboardOnPolkit
            || (self->m_polkitSurface && surface == self->m_polkitSurface);
    self->m_keyboardOnDock = false;
    self->m_keyboardOnOverview = false;
    self->m_keyboardOnControlCenter = false;
    self->m_keyboardOnChooser = false;
    self->m_keyboardOnScreenshot = false;
    self->m_keyboardOnScreenCast = false;
    self->m_keyboardOnPolkit = false;
    self->m_keyboardOnLock = false;
    emit self->keyboardFocused(false);
    if (wasDock)
        emit self->dockKeyboardFocused(false);
    if (wasOverview)
        emit self->overviewKeyboardFocused(false);
    if (wasControlCenter)
        emit self->controlCenterKeyboardFocused(false);
    if (wasChooser)
        emit self->chooserKeyboardFocused(false);
    if (wasScreenshot)
        emit self->screenshotKeyboardFocused(false);
    if (wasScreenCast)
        emit self->screencastKeyboardFocused(false);
    if (wasPolkit)
        emit self->polkitKeyboardFocused(false);
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
    if (self->m_keyboardOnChooser) {
        emit self->chooserKeyEvent(key, state == WL_KEYBOARD_KEY_STATE_PRESSED);
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
    emit self->keyEvent(key, state == WL_KEYBOARD_KEY_STATE_PRESSED);
}

void ShellProtocol::onKeyboardModifiers(void *data, wl_keyboard *, uint32_t, uint32_t depressed,
                                        uint32_t, uint32_t, uint32_t)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // WL_KEYBOARD_MODIFIER_MASK_SHIFT is bit 0.
    self->m_keyboardShift = (depressed & 0x1u) != 0;
}

// --- df_toplevel_manager ----------------------------------------------------

void ShellProtocol::onManagerOutput(void *data, df_toplevel_manager *, df_output *id)
{
    auto *self = static_cast<ShellProtocol *>(data);
    static const df_output_listener listener = {
        onOutputName,       onOutputGeometry, onOutputMode,        onOutputScale,
        onOutputTransform,  onOutputVrr,      onOutputNightLight,  onOutputReservedZone,
        onOutputDone,       onOutputRemoved,  onOutputBrightness,
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

df_toplevel *ShellProtocol::toplevelForId(quintptr windowId) const
{
    return m_toplevelById.value(windowId, nullptr);
}

// --- wl_buffer --------------------------------------------------------------

void ShellProtocol::onBufferRelease(void *data, wl_buffer *buffer)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_buffers.remove(buffer);
    wl_buffer_destroy(buffer);
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
                                 uint32_t height, uint32_t)
{
    // The last mode announced is the output's current size for the ScreenCast
    // picker (T-13.4a).
    auto *self = static_cast<ShellProtocol *>(data);
    OutputInfo &info = self->m_outputInfo[output];
    info.width = static_cast<int>(width);
    info.height = static_cast<int>(height);
}
void ShellProtocol::onOutputScale(void *, df_output *, int32_t) {}
void ShellProtocol::onOutputTransform(void *, df_output *, uint32_t) {}
void ShellProtocol::onOutputVrr(void *, df_output *, uint32_t) {}
void ShellProtocol::onOutputNightLight(void *, df_output *, uint32_t, uint32_t) {}
void ShellProtocol::onOutputBrightness(void *, df_output *, wl_fixed_t) {}
void ShellProtocol::onOutputReservedZone(void *, df_output *, uint32_t edge, uint32_t thickness)
{
    fprintf(stderr, "dragonfruit-shell: output reserved zone edge=%u thickness=%u\n", edge,
            thickness);
}
void ShellProtocol::onOutputDone(void *, df_output *) {}
void ShellProtocol::onOutputRemoved(void *data, df_output *output)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_outputs.removeAll(output);
    self->m_outputInfo.remove(output);
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
void ShellProtocol::onWorkspaceWallpaper(void *, df_workspace *, const char *, uint32_t, uint32_t) {}
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
