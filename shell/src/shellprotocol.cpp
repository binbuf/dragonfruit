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

#ifndef DF_LOCKSTEP_VERSION
#define DF_LOCKSTEP_VERSION 1
#endif

namespace {

constexpr uint32_t kAnchorTop = 1;
constexpr uint32_t kAnchorBottom = 2;
constexpr uint32_t kAnchorLeft = 4;
constexpr uint32_t kAnchorRight = 8;

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
        m_shell = static_cast<df_shell *>(
            wl_registry_bind(m_registry, m_shellName, &df_shell_interface,
                             std::min(m_shellVersion, 1u)));
    }
    if (m_managerName) {
        m_manager = static_cast<df_toplevel_manager *>(
            wl_registry_bind(m_registry, m_managerName, &df_toplevel_manager_interface,
                             std::min(m_managerVersion, 2u)));
        static const df_toplevel_manager_listener managerListener = {
            onManagerOutput,
            onManagerWorkspace,
            onManagerToplevel,
            nullptr, // workspace_activated (unused)
            onManagerFocused,
            onManagerAttention,
            onManagerHotCorner,
            onManagerOverview,
            onManagerAppSwitcher,
            onManagerInputAction,
            onManagerProgress,
            onManagerAppAccelerator,
            onManagerDone,
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

int ShellProtocol::displayFd() const
{
    return m_display ? wl_display_get_fd(m_display) : -1;
}

void ShellProtocol::teardown()
{
    for (wl_buffer *buffer : std::as_const(m_buffers))
        wl_buffer_destroy(buffer);
    m_buffers.clear();
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

void ShellProtocol::onLayerClosed(void *data, df_layer_surface *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    emit self->surfaceClosed();
}

// --- wl_seat / wl_pointer / wl_keyboard (input bridge) ----------------------

void ShellProtocol::onSeatCapabilities(void *data, wl_seat *seat, uint32_t capabilities)
{
    auto *self = static_cast<ShellProtocol *>(data);
    // The data device needs the seat, not a capability; bind it once both the
    // seat and the manager global are known (T-10 external drops).
    self->maybeCreateDataDevice();
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
    if (self->m_dndOffer && self->m_dndOffer != offer) {
        wl_data_offer_destroy(self->m_dndOffer);
        self->m_dndOffer = nullptr;
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
    }
    // Only the Dock is a drop target; a drag over the popup or another chrome
    // surface is ignored (the shell owns the payload).
    if (!self->m_dockSurface || surface != self->m_dockSurface) {
        self->m_dndActive = false;
        return;
    }
    self->m_dndActive = true;
    self->m_dndPayloadIsApp = self->m_dndMimeTypes.contains(
        QStringLiteral("application/x-dragonfruit-app"));
    self->m_dndX = wl_fixed_to_double(x);
    self->m_dndY = wl_fixed_to_double(y);
    emit self->dockExternalDragEntered(self->m_dndPayloadIsApp, self->m_dndX, self->m_dndY);
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
    QString mime;
    if (self->m_dndMimeTypes.contains(QStringLiteral("text/uri-list")))
        mime = QStringLiteral("text/uri-list");
    else if (self->m_dndMimeTypes.contains(QStringLiteral("application/x-dragonfruit-app")))
        mime = QStringLiteral("application/x-dragonfruit-app");
    if (mime.isEmpty()) {
        wl_data_offer_finish(self->m_dndOffer);
        self->resetExternalDrag();
        return;
    }

    int fds[2];
    if (pipe2(fds, O_CLOEXEC) != 0) {
        wl_data_offer_finish(self->m_dndOffer);
        self->resetExternalDrag();
        return;
    }
    fcntl(fds[0], F_SETFL, O_NONBLOCK);
    wl_data_offer_receive(self->m_dndOffer, mime.toUtf8().constData(), fds[1]);
    wl_data_offer_finish(self->m_dndOffer);
    ::close(fds[1]);

    self->m_dndMime = mime;
    self->m_dndData.clear();
    self->m_dndReadFd = fds[0];
    if (!self->m_dndReadNotifier) {
        self->m_dndReadNotifier =
            new QSocketNotifier(self->m_dndReadFd, QSocketNotifier::Read, self);
        QObject::connect(self->m_dndReadNotifier, &QSocketNotifier::activated, self,
                         &ShellProtocol::onDndReadable);
    } else {
        self->m_dndReadNotifier->setSocket(self->m_dndReadFd);
        self->m_dndReadNotifier->setEnabled(true);
    }
}

void ShellProtocol::onDataDeviceSelection(void *, wl_data_device *, wl_data_offer *)
{
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
    // EOF (0) or an error: the source has finished writing the payload.
    if (m_dndReadNotifier)
        m_dndReadNotifier->setEnabled(false);
    ::close(m_dndReadFd);
    m_dndReadFd = -1;

    const bool appMime = m_dndMime == QLatin1String("application/x-dragonfruit-app");
    QString desktopId;
    QStringList paths;
    if (appMime) {
        desktopId = QString::fromUtf8(m_dndData).trimmed();
    } else {
        paths = parseUriList(m_dndData);
        if (uriListIsApplication(paths)) {
            desktopId = desktopIdForFile(paths.first());
            paths.clear();
        }
    }
    const bool payloadIsApp = appMime || !desktopId.isEmpty();
    const qreal x = m_dndX;
    const qreal y = m_dndY;

    if (m_dndOffer) {
        wl_data_offer_destroy(m_dndOffer);
        m_dndOffer = nullptr;
    }
    m_dndActive = false;
    m_dndPayloadIsApp = false;
    m_dndMimeTypes.clear();
    m_dndMime.clear();
    m_dndData.clear();
    emit dockExternalDropped(payloadIsApp, desktopId, paths, x, y);
}

void ShellProtocol::resetExternalDrag()
{
    if (m_dndOffer) {
        wl_data_offer_destroy(m_dndOffer);
        m_dndOffer = nullptr;
    }
    if (m_dndReadFd >= 0) {
        ::close(m_dndReadFd);
        m_dndReadFd = -1;
    }
    if (m_dndReadNotifier)
        m_dndReadNotifier->setEnabled(false);
    m_dndActive = false;
    m_dndPayloadIsApp = false;
    m_dndMimeTypes.clear();
    m_dndMime.clear();
    m_dndData.clear();
}

void ShellProtocol::onPointerEnter(void *data, wl_pointer *, uint32_t, wl_surface *surface,
                                   wl_fixed_t x, wl_fixed_t y)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_pointerOnPopup = self->m_popupSurface && surface == self->m_popupSurface;
    self->m_pointerOnDockPopup =
        self->m_dockPopupSurface && surface == self->m_dockPopupSurface;
    self->m_pointerOnDock = self->m_dockSurface && surface == self->m_dockSurface;
    self->m_pointerX = wl_fixed_to_double(x) + (self->m_pointerOnPopup ? self->m_popupX : 0);
    self->m_pointerY = wl_fixed_to_double(y) + (self->m_pointerOnPopup ? self->m_popupY : 0);
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
    const bool wasDockPopup = self->m_pointerOnDockPopup;
    const bool wasDock = self->m_pointerOnDock;
    self->m_pointerOnPopup = false;
    self->m_pointerOnDockPopup = false;
    self->m_pointerOnDock = false;
    if (wasDockPopup)
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
    emit self->keyboardFocused(true);
    if (self->m_keyboardOnDock)
        emit self->dockKeyboardFocused(true);
}

void ShellProtocol::onKeyboardLeave(void *data, wl_keyboard *, uint32_t, wl_surface *surface)
{
    auto *self = static_cast<ShellProtocol *>(data);
    const bool wasDock = self->m_keyboardOnDock
            || (self->m_dockSurface && surface == self->m_dockSurface);
    self->m_keyboardOnDock = false;
    emit self->keyboardFocused(false);
    if (wasDock)
        emit self->dockKeyboardFocused(false);
}

void ShellProtocol::onKeyboardKey(void *data, wl_keyboard *, uint32_t, uint32_t, uint32_t key,
                                  uint32_t state)
{
    auto *self = static_cast<ShellProtocol *>(data);
    emit self->keyEvent(key, state == WL_KEYBOARD_KEY_STATE_PRESSED);
}

void ShellProtocol::onKeyboardModifiers(void *, wl_keyboard *, uint32_t, uint32_t, uint32_t,
                                        uint32_t, uint32_t)
{
}

// --- df_toplevel_manager ----------------------------------------------------

void ShellProtocol::onManagerOutput(void *data, df_toplevel_manager *, df_output *id)
{
    auto *self = static_cast<ShellProtocol *>(data);
    static const df_output_listener listener = {
        onOutputName,       onOutputGeometry, onOutputMode,       onOutputScale,
        onOutputTransform,  onOutputVrr,      onOutputNightLight, onOutputReservedZone,
        onOutputDone,       onOutputRemoved,
    };
    df_output_add_listener(id, &listener, self);
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

void ShellProtocol::onManagerOverview(void *, df_toplevel_manager *, uint32_t, df_toplevel *)
{
}

void ShellProtocol::onManagerAppSwitcher(void *, df_toplevel_manager *, uint32_t, const char *,
                                         int32_t)
{
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

void ShellProtocol::onManagerProgress(void *, df_toplevel_manager *, const char *, int32_t, int32_t,
                                      int32_t, uint32_t, uint32_t, uint32_t)
{
}

void ShellProtocol::onManagerAppAccelerator(void *, df_toplevel_manager *, const char *,
                                            const char *, const char *, uint32_t)
{
}

void ShellProtocol::onManagerDone(void *, df_toplevel_manager *)
{
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

void ShellProtocol::onOutputName(void *, df_output *, const char *) {}
void ShellProtocol::onOutputGeometry(void *, df_output *, int32_t, int32_t, int32_t, int32_t) {}
void ShellProtocol::onOutputMode(void *, df_output *, uint32_t, uint32_t, uint32_t, uint32_t) {}
void ShellProtocol::onOutputScale(void *, df_output *, int32_t) {}
void ShellProtocol::onOutputTransform(void *, df_output *, uint32_t) {}
void ShellProtocol::onOutputVrr(void *, df_output *, uint32_t) {}
void ShellProtocol::onOutputNightLight(void *, df_output *, uint32_t, uint32_t) {}
void ShellProtocol::onOutputReservedZone(void *, df_output *, uint32_t edge, uint32_t thickness)
{
    fprintf(stderr, "dragonfruit-shell: output reserved zone edge=%u thickness=%u\n", edge,
            thickness);
}
void ShellProtocol::onOutputDone(void *, df_output *) {}
void ShellProtocol::onOutputRemoved(void *, df_output *) {}
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
void ShellProtocol::onWorkspaceActivated(void *data, df_workspace *workspace, uint32_t)
{
    // The event names the Space that just became active (T-10 section 13);
    // remember it so "Assign to This Desktop" can target it.
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_activeWorkspace = workspace;
}
void ShellProtocol::onWorkspaceFullscreen(void *, df_workspace *, uint32_t) {}
void ShellProtocol::onWorkspaceWallpaper(void *, df_workspace *, const char *, uint32_t, uint32_t) {}
void ShellProtocol::onWorkspaceRemoved(void *data, df_workspace *workspace)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_workspaces.remove(workspace);
    // Never keep a dangling active-Space pointer for "Assign to This Desktop".
    if (self->m_activeWorkspace == workspace)
        self->m_activeWorkspace = nullptr;
}
void ShellProtocol::onWorkspaceDone(void *, df_workspace *) {}
