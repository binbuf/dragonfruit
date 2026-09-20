// SPDX-License-Identifier: GPL-3.0-or-later
#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif

#include "shellprotocol.h"

#include <algorithm>
#include <cerrno>
#include <cstdio>
#include <cstring>
#include <utility>

#include <fcntl.h>
#include <sys/mman.h>
#include <unistd.h>

#ifdef __linux__
#include <linux/memfd.h>
#endif

#include <wayland-client.h>

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
constexpr uint32_t kAnchorLeft = 4;
constexpr uint32_t kAnchorRight = 8;

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
                             std::min(m_managerVersion, 1u)));
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

bool ShellProtocol::commitImage(const QImage &image)
{
    if (!m_surface || !m_shm)
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

    wl_surface_attach(m_surface, buffer, 0, 0);
    wl_surface_damage_buffer(m_surface, 0, 0, width, height);
    wl_surface_commit(m_surface);
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
    if (m_layer)
        df_layer_surface_destroy(m_layer);
    if (m_surface)
        wl_surface_destroy(m_surface);
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
    m_manager = nullptr;
    m_shell = nullptr;
    m_core = nullptr;
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
    } else if (iface == QLatin1String("df_core")) {
        self->m_coreName = name;
        self->m_coreVersion = version;
    } else if (iface == QLatin1String("df_shell")) {
        self->m_shellName = name;
        self->m_shellVersion = version;
    } else if (iface == QLatin1String("df_toplevel_manager")) {
        self->m_managerName = name;
        self->m_managerVersion = version;
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

void ShellProtocol::onLayerClosed(void *data, df_layer_surface *)
{
    auto *self = static_cast<ShellProtocol *>(data);
    emit self->surfaceClosed();
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
    self->m_toplevels.insert(id, ToplevelInfo{});
    static const df_toplevel_listener listener = {
        onToplevelTitle,   onToplevelAppId,         onToplevelState,
        onToplevelWorkspaceEntered, onToplevelWorkspaceLeft, onToplevelOutputEntered,
        onToplevelOutputLeft, onToplevelClosed,     onToplevelDone,
    };
    df_toplevel_add_listener(id, &listener, self);
}

void ShellProtocol::onManagerFocused(void *data, df_toplevel_manager *, df_toplevel *id)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_focused = id;
    const ToplevelInfo info = id ? self->m_toplevels.value(id) : ToplevelInfo{};
    emit self->focusedAppChanged(info.appId, info.title);
}

void ShellProtocol::onManagerAttention(void *, df_toplevel_manager *, df_toplevel *)
{
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

void ShellProtocol::onManagerInputAction(void *, df_toplevel_manager *, const char *, const char *,
                                         uint32_t)
{
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
}

void ShellProtocol::onToplevelAppId(void *data, df_toplevel *toplevel, const char *appId)
{
    auto *self = static_cast<ShellProtocol *>(data);
    self->m_toplevels[toplevel].appId = QString::fromUtf8(appId ? appId : "");
    if (self->m_focused == toplevel) {
        const ToplevelInfo info = self->m_toplevels.value(toplevel);
        emit self->focusedAppChanged(info.appId, info.title);
    }
}

void ShellProtocol::onToplevelState(void *, df_toplevel *, uint32_t)
{
}

void ShellProtocol::onToplevelWorkspaceEntered(void *, df_toplevel *, df_workspace *)
{
}

void ShellProtocol::onToplevelWorkspaceLeft(void *, df_toplevel *, df_workspace *)
{
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
    self->m_toplevels.remove(toplevel);
    if (self->m_focused == toplevel) {
        self->m_focused = nullptr;
        emit self->focusedAppChanged(QString(), QString());
    }
}

void ShellProtocol::onToplevelDone(void *, df_toplevel *)
{
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
void ShellProtocol::onWorkspaceName(void *, df_workspace *, const char *) {}
void ShellProtocol::onWorkspaceIndex(void *, df_workspace *, uint32_t) {}
void ShellProtocol::onWorkspaceActivated(void *, df_workspace *, uint32_t) {}
void ShellProtocol::onWorkspaceFullscreen(void *, df_workspace *, uint32_t) {}
void ShellProtocol::onWorkspaceWallpaper(void *, df_workspace *, const char *, uint32_t, uint32_t) {}
void ShellProtocol::onWorkspaceRemoved(void *, df_workspace *) {}
void ShellProtocol::onWorkspaceDone(void *, df_workspace *) {}
