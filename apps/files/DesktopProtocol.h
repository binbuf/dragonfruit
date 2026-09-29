// SPDX-License-Identifier: MIT
// Minimal private-protocol client for the Files-owned desktop (T-19.3).
//
// The desktop process is not a normal toolkit window: it is the second trusted
// session process (after the shell) and speaks the compositor's `df_core` /
// `df_shell` protocol directly, exactly like `shell/src/shellprotocol.*`. This
// class is deliberately trimmed to the desktop's needs — connect, present the
// `desktop:` launch token, create one full-output `background` layer surface,
// commit shared-memory frames, and receive pointer/keyboard input. It owns no
// QML and no scene: `FilesDesktop` renders the offscreen scene and hands this
// class the image.
#pragma once

#include <QByteArray>
#include <QImage>
#include <QObject>
#include <QSet>
#include <QString>

#include <cstdint>

#include <wayland-client.h>

struct df_core;
struct df_shell;
struct df_layer_surface;

class DesktopProtocol : public QObject
{
    Q_OBJECT

public:
    explicit DesktopProtocol(QObject *parent = nullptr);
    ~DesktopProtocol() override;

    // Connect to `socketName` (empty = $WAYLAND_DISPLAY) and enumerate globals.
    bool connectToCompositor(const QString &socketName, const QString &tokenHex);
    bool isAuthenticated() const { return m_authenticated; }

    // Create the full-output `background` layer surface, anchored to every
    // edge, reserving nothing, and taking keyboard on demand. The compositor
    // answers with a `configure`.
    bool createDesktopSurface();

    // Attach `image` to the desktop surface and commit. ARGB32(_Premultiplied).
    bool commitImage(const QImage &image);

    int displayFd() const;
    bool dispatch();
    bool flush();

    QString lastError() const { return m_error; }

signals:
    void configured(int width, int height);
    void pointerMoved(qreal x, qreal y);
    void pointerButton(qreal x, qreal y, quint32 button, bool pressed);
    void pointerLeft();
    void keyboardFocused(bool focused);
    void keyEvent(quint32 key, bool pressed, quint32 modifiers);
    void surfaceClosed();
    void fatal(const QString &message);

private:
    bool fail(const QString &message);
    void bindTrustedGlobals();
    void teardown();

    static void onRegistryGlobal(void *data, wl_registry *registry, uint32_t name,
                                 const char *interface, uint32_t version);
    static void onRegistryGlobalRemove(void *data, wl_registry *registry, uint32_t name);
    static void onCoreAuthenticated(void *data, df_core *core, uint32_t version);
    static void onCoreRefused(void *data, df_core *core, uint32_t code, const char *message);
    static void onLayerConfigure(void *data, df_layer_surface *layer, uint32_t serial,
                                 int32_t width, int32_t height);
    static void onLayerClosed(void *data, df_layer_surface *layer);
    static void onSeatCapabilities(void *data, wl_seat *seat, uint32_t capabilities);
    static void onSeatName(void *data, wl_seat *seat, const char *name);
    static void onPointerEnter(void *data, wl_pointer *pointer, uint32_t serial,
                               wl_surface *surface, wl_fixed_t x, wl_fixed_t y);
    static void onPointerLeave(void *data, wl_pointer *pointer, uint32_t serial,
                               wl_surface *surface);
    static void onPointerMotion(void *data, wl_pointer *pointer, uint32_t time,
                                wl_fixed_t x, wl_fixed_t y);
    static void onPointerButton(void *data, wl_pointer *pointer, uint32_t serial,
                                uint32_t time, uint32_t button, uint32_t state);
    static void onPointerAxis(void *data, wl_pointer *pointer, uint32_t time, uint32_t axis,
                              wl_fixed_t value);
    static void onKeyboardKeymap(void *data, wl_keyboard *keyboard, uint32_t format, int32_t fd,
                                 uint32_t size);
    static void onKeyboardEnter(void *data, wl_keyboard *keyboard, uint32_t serial,
                                wl_surface *surface, wl_array *keys);
    static void onKeyboardLeave(void *data, wl_keyboard *keyboard, uint32_t serial,
                                wl_surface *surface);
    static void onKeyboardKey(void *data, wl_keyboard *keyboard, uint32_t serial, uint32_t time,
                              uint32_t key, uint32_t state);
    static void onKeyboardModifiers(void *data, wl_keyboard *keyboard, uint32_t serial,
                                    uint32_t depressed, uint32_t latched, uint32_t locked,
                                    uint32_t group);
    static void onBufferRelease(void *data, wl_buffer *buffer);

    wl_display *m_display = nullptr;
    wl_registry *m_registry = nullptr;
    wl_compositor *m_compositor = nullptr;
    wl_shm *m_shm = nullptr;
    wl_seat *m_seat = nullptr;
    wl_pointer *m_pointer = nullptr;
    wl_keyboard *m_keyboard = nullptr;
    df_core *m_core = nullptr;
    df_shell *m_shell = nullptr;
    wl_surface *m_surface = nullptr;
    df_layer_surface *m_layer = nullptr;

    uint32_t m_compositorVersion = 1;
    uint32_t m_shmVersion = 1;
    uint32_t m_seatVersion = 1;
    uint32_t m_shmName = 0;
    uint32_t m_seatName = 0;
    uint32_t m_coreName = 0;
    uint32_t m_coreVersion = 1;
    uint32_t m_shellName = 0;
    uint32_t m_shellVersion = 1;
    bool m_hasShm = false;
    bool m_hasCore = false;
    bool m_hasShell = false;

    bool m_authenticated = false;
    bool m_trustedGlobalsBound = false;

    qreal m_pointerX = 0;
    qreal m_pointerY = 0;
    bool m_pointerOnSurface = false;
    bool m_keyboardOnSurface = false;
    uint32_t m_keyboardModifiers = 0;

    QSet<wl_buffer *> m_buffers;
    QString m_error;
};