// SPDX-License-Identifier: MIT
#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif

#include "DesktopProtocol.h"

#include <algorithm>
#include <cerrno>
#include <cstdio>
#include <cstring>

#include <fcntl.h>
#include <sys/mman.h>
#include <unistd.h>

#ifdef __linux__
#include <linux/memfd.h>
#endif

#include "dragonfruit-core-client-protocol.h"
// wayland-scanner emits `namespace` as a C argument name for the chrome
// factory; it is a C++ keyword, so rename it around this one generated header.
#define namespace df_layer_namespace
#include "dragonfruit-shell-client-protocol.h"
#undef namespace

#ifndef DF_LOCKSTEP_VERSION
#define DF_LOCKSTEP_VERSION 1
#endif

namespace {

constexpr uint32_t kAnchorTop = 1;
constexpr uint32_t kAnchorBottom = 2;
constexpr uint32_t kAnchorLeft = 4;
constexpr uint32_t kAnchorRight = 8;

// The one dialog the desktop needs is a right-click menu on the empty
// background. It is a regular `overlay` layer surface owned by this process
// (the shell owns the chrome layers); the desktop's context menu is therefore
// rendered into the same surface as the icons. No extra protocol object.

int createShmFile(size_t size)
{
    int fd = -1;
#ifdef __linux__
    fd = memfd_create("dragonfruit-files-desktop", MFD_CLOEXEC | MFD_ALLOW_SEALING);
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

DesktopProtocol::DesktopProtocol(QObject *parent)
    : QObject(parent)
{
}

DesktopProtocol::~DesktopProtocol()
{
    teardown();
}

bool DesktopProtocol::fail(const QString &message)
{
    if (m_error.isEmpty())
        m_error = message;
    emit fatal(message);
    return false;
}

bool DesktopProtocol::connectToCompositor(const QString &socketName, const QString &tokenHex)
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
    if (!m_hasCore)
        return fail(QStringLiteral("df_core is not advertised"));

    m_core = static_cast<df_core *>(
        wl_registry_bind(m_registry, m_coreName, &df_core_interface, std::min(m_coreVersion, 1u)));
    if (!m_core)
        return fail(QStringLiteral("failed to bind df_core"));
    static const df_core_listener coreListener = { onCoreAuthenticated, onCoreRefused };
    df_core_add_listener(m_core, &coreListener, this);

    // Present the one-time token as part of `connectToCompositor`, so the
    // caller only needs one call before creating the surface.
    const QByteArray token = tokenHex.toUtf8();
    df_core_authenticate(m_core, DF_LOCKSTEP_VERSION, token.constData());
    if (wl_display_roundtrip(m_display) < 0)
        return fail(QStringLiteral("launch-token handshake roundtrip failed"));
    if (!m_authenticated)
        return fail(m_error.isEmpty() ? QStringLiteral("launch-token handshake refused")
                                      : m_error);

    bindTrustedGlobals();
    if (!m_shell)
        return fail(QStringLiteral("df_shell is not available to the desktop role"));
    return true;
}

void DesktopProtocol::bindTrustedGlobals()
{
    if (m_trustedGlobalsBound)
        return;
    if (m_hasShell)
        m_shell = static_cast<df_shell *>(
            wl_registry_bind(m_registry, m_shellName, &df_shell_interface,
                             std::min(m_shellVersion,
                                      static_cast<uint32_t>(df_shell_interface.version))));
    m_trustedGlobalsBound = true;
}

bool DesktopProtocol::createDesktopSurface()
{
    if (!m_shell || !m_compositor)
        return fail(QStringLiteral("df_shell is not available"));

    m_surface = wl_compositor_create_surface(m_compositor);
    // Anchor every edge and request size 0x0: the compositor resolves it to the
    // full output (matching the wallpaper band). `exclusive_zone -1` keeps the
    // desktop out of the reserved-zone accounting; OnDemand keyboard lets a
    // click focus it without an always-modal grab.
    m_layer = df_shell_get_layer_surface(m_shell, m_surface, nullptr, DF_SHELL_LAYER_BACKGROUND,
                                         "desktop");
    if (!m_layer)
        return fail(QStringLiteral("compositor refused the desktop layer surface"));
    static const df_layer_surface_listener layerListener = { onLayerConfigure, onLayerClosed };
    df_layer_surface_add_listener(m_layer, &layerListener, this);

    df_layer_surface_set_anchor(m_layer, kAnchorTop | kAnchorBottom | kAnchorLeft | kAnchorRight);
    df_layer_surface_set_size(m_layer, 0, 0);
    df_layer_surface_set_exclusive_zone(m_layer, -1);
    df_layer_surface_set_keyboard_interaction(
        m_layer, DF_LAYER_SURFACE_KEYBOARD_INTERACTION_ON_DEMAND);
    wl_surface_commit(m_surface);

    if (wl_display_roundtrip(m_display) < 0)
        return fail(QStringLiteral("desktop configure roundtrip failed"));
    return true;
}

bool DesktopProtocol::commitImage(const QImage &image)
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

int DesktopProtocol::displayFd() const
{
    return m_display ? wl_display_get_fd(m_display) : -1;
}

bool DesktopProtocol::dispatch()
{
    if (!m_display)
        return false;
    if (wl_display_dispatch(m_display) < 0)
        return fail(QStringLiteral("compositor connection lost"));
    return true;
}

bool DesktopProtocol::flush()
{
    if (!m_display)
        return false;
    if (wl_display_flush(m_display) < 0 && errno != EAGAIN)
        return fail(QStringLiteral("failed to flush the compositor connection"));
    return true;
}

void DesktopProtocol::teardown()
{
    for (wl_buffer *buffer : std::as_const(m_buffers))
        wl_buffer_destroy(buffer);
    m_buffers.clear();
    if (m_layer)
        df_layer_surface_destroy(m_layer);
    if (m_surface)
        wl_surface_destroy(m_surface);
    if (m_pointer)
        wl_pointer_destroy(m_pointer);
    if (m_keyboard)
        wl_keyboard_destroy(m_keyboard);
    if (m_seat)
        wl_seat_destroy(m_seat);
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
    m_registry = nullptr;
    m_compositor = nullptr;
    m_shm = nullptr;
    m_seat = nullptr;
    m_pointer = nullptr;
    m_keyboard = nullptr;
    m_core = nullptr;
    m_shell = nullptr;
    m_surface = nullptr;
    m_layer = nullptr;
    m_authenticated = false;
    m_trustedGlobalsBound = false;
    m_hasShm = false;
    m_hasCore = false;
    m_hasShell = false;
    m_pointerOnSurface = false;
    m_keyboardOnSurface = false;
}

// --- registry ---------------------------------------------------------------

void DesktopProtocol::onRegistryGlobal(void *data, wl_registry *registry, uint32_t name,
                                       const char *interface, uint32_t version)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    const QString iface = QString::fromLatin1(interface);
    if (iface == QLatin1String("wl_compositor")) {
        self->m_compositorVersion = version;
        self->m_compositor = static_cast<wl_compositor *>(
            wl_registry_bind(registry, name, &wl_compositor_interface, std::min(version, 4u)));
    } else if (iface == QLatin1String("wl_shm")) {
        self->m_shmVersion = version;
        self->m_shm = static_cast<wl_shm *>(
            wl_registry_bind(registry, name, &wl_shm_interface, std::min(version, 1u)));
        self->m_hasShm = self->m_shm != nullptr;
    } else if (iface == QLatin1String("wl_seat")) {
        self->m_seatName = name;
        self->m_seatVersion = version;
        self->m_seat = static_cast<wl_seat *>(
            wl_registry_bind(registry, name, &wl_seat_interface, std::min(version, 1u)));
        if (self->m_seat) {
            static const wl_seat_listener seatListener = { onSeatCapabilities, onSeatName };
            wl_seat_add_listener(self->m_seat, &seatListener, self);
        }
    } else if (iface == QLatin1String("df_core")) {
        self->m_coreName = name;
        self->m_coreVersion = version;
        self->m_hasCore = true;
    } else if (iface == QLatin1String("df_shell")) {
        self->m_shellName = name;
        self->m_shellVersion = version;
        self->m_hasShell = true;
    }
}

void DesktopProtocol::onRegistryGlobalRemove(void *, wl_registry *, uint32_t)
{
}

// --- df_core ----------------------------------------------------------------

void DesktopProtocol::onCoreAuthenticated(void *data, df_core *, uint32_t)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    self->m_authenticated = true;
}

void DesktopProtocol::onCoreRefused(void *data, df_core *, uint32_t code, const char *message)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    self->m_authenticated = false;
    self->m_error = QStringLiteral("desktop launch-token refused (code %1): %2")
                        .arg(code)
                        .arg(QString::fromUtf8(message));
    emit self->fatal(self->m_error);
}

// --- df_layer_surface -------------------------------------------------------

void DesktopProtocol::onLayerConfigure(void *data, df_layer_surface *, uint32_t serial,
                                       int32_t width, int32_t height)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    if (self->m_layer)
        df_layer_surface_ack_configure(self->m_layer, serial);
    emit self->configured(width, height);
}

void DesktopProtocol::onLayerClosed(void *data, df_layer_surface *)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    emit self->surfaceClosed();
}

// --- wl_seat / wl_pointer / wl_keyboard -------------------------------------

void DesktopProtocol::onSeatCapabilities(void *data, wl_seat *seat, uint32_t capabilities)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    const bool hasPointer = capabilities & WL_SEAT_CAPABILITY_POINTER;
    if (hasPointer && !self->m_pointer) {
        self->m_pointer = wl_seat_get_pointer(seat);
        static const wl_pointer_listener pointerListener = {
            onPointerEnter, onPointerLeave, onPointerMotion, onPointerButton, onPointerAxis,
        };
        wl_pointer_add_listener(self->m_pointer, &pointerListener, self);
    } else if (!hasPointer && self->m_pointer) {
        wl_pointer_destroy(self->m_pointer);
        self->m_pointer = nullptr;
    }
    const bool hasKeyboard = capabilities & WL_SEAT_CAPABILITY_KEYBOARD;
    if (hasKeyboard && !self->m_keyboard) {
        self->m_keyboard = wl_seat_get_keyboard(seat);
        static const wl_keyboard_listener keyboardListener = {
            onKeyboardKeymap, onKeyboardEnter, onKeyboardLeave, onKeyboardKey,
            onKeyboardModifiers,
        };
        wl_keyboard_add_listener(self->m_keyboard, &keyboardListener, self);
    } else if (!hasKeyboard && self->m_keyboard) {
        wl_keyboard_destroy(self->m_keyboard);
        self->m_keyboard = nullptr;
    }
}

void DesktopProtocol::onSeatName(void *, wl_seat *, const char *)
{
}

void DesktopProtocol::onPointerEnter(void *data, wl_pointer *, uint32_t, wl_surface *surface,
                                     wl_fixed_t x, wl_fixed_t y)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    self->m_pointerOnSurface = self->m_surface && surface == self->m_surface;
    self->m_pointerX = wl_fixed_to_double(x);
    self->m_pointerY = wl_fixed_to_double(y);
    if (self->m_pointerOnSurface)
        emit self->pointerMoved(self->m_pointerX, self->m_pointerY);
}

void DesktopProtocol::onPointerLeave(void *data, wl_pointer *, uint32_t, wl_surface *)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    self->m_pointerOnSurface = false;
    emit self->pointerLeft();
}

void DesktopProtocol::onPointerMotion(void *data, wl_pointer *, uint32_t, wl_fixed_t x,
                                      wl_fixed_t y)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    self->m_pointerX = wl_fixed_to_double(x);
    self->m_pointerY = wl_fixed_to_double(y);
    if (self->m_pointerOnSurface)
        emit self->pointerMoved(self->m_pointerX, self->m_pointerY);
}

void DesktopProtocol::onPointerButton(void *data, wl_pointer *, uint32_t, uint32_t,
                                      uint32_t button, uint32_t state)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    if (!self->m_pointerOnSurface)
        return;
    emit self->pointerButton(self->m_pointerX, self->m_pointerY, button,
                             state == WL_POINTER_BUTTON_STATE_PRESSED);
}

void DesktopProtocol::onPointerAxis(void *, wl_pointer *, uint32_t, uint32_t, wl_fixed_t)
{
}

void DesktopProtocol::onKeyboardKeymap(void *, wl_keyboard *, uint32_t, int32_t fd, uint32_t)
{
    if (fd >= 0)
        ::close(fd);
}

void DesktopProtocol::onKeyboardEnter(void *data, wl_keyboard *, uint32_t, wl_surface *surface,
                                      wl_array *)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    self->m_keyboardOnSurface = self->m_surface && surface == self->m_surface;
    emit self->keyboardFocused(true);
}

void DesktopProtocol::onKeyboardLeave(void *data, wl_keyboard *, uint32_t, wl_surface *)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    self->m_keyboardOnSurface = false;
    emit self->keyboardFocused(false);
}

void DesktopProtocol::onKeyboardKey(void *data, wl_keyboard *, uint32_t, uint32_t, uint32_t key,
                                    uint32_t state)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    if (!self->m_keyboardOnSurface)
        return;
    emit self->keyEvent(key, state == WL_KEYBOARD_KEY_STATE_PRESSED, self->m_keyboardModifiers);
}

void DesktopProtocol::onKeyboardModifiers(void *data, wl_keyboard *, uint32_t, uint32_t depressed,
                                          uint32_t, uint32_t, uint32_t)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    self->m_keyboardModifiers = depressed;
}

void DesktopProtocol::onBufferRelease(void *data, wl_buffer *buffer)
{
    auto *self = static_cast<DesktopProtocol *>(data);
    self->m_buffers.remove(buffer);
    wl_buffer_destroy(buffer);
}