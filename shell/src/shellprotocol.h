// SPDX-License-Identifier: GPL-3.0-or-later
// Private-protocol client for the Dragonfruit shell (T-09).
//
// Connects to the compositor over the df_core/df_shell/df_toplevel_manager
// interfaces, performs the launch-token handshake, and owns the menu-bar
// chrome surface. Rendering is deliberately separate (ShellController):
// this class only speaks Wayland.
#pragma once

#include <QHash>
#include <QImage>
#include <QObject>
#include <QSet>
#include <QString>

#include <cstdint>

struct wl_display;
struct wl_registry;
struct wl_compositor;
struct wl_shm;
struct wl_surface;
struct wl_buffer;
struct df_core;
struct df_shell;
struct df_layer_surface;
struct df_toplevel_manager;
struct df_toplevel;
struct df_output;
struct df_workspace;

class ShellProtocol : public QObject
{
    Q_OBJECT

public:
    explicit ShellProtocol(QObject *parent = nullptr);
    ~ShellProtocol() override;

    // Connect to `socketName` (empty = $WAYLAND_DISPLAY) and enumerate globals.
    bool connectToCompositor(const QString &socketName);
    // Present the one-time launch token and wait for the handshake result.
    bool authenticate(const QString &tokenHex);
    bool isAuthenticated() const { return m_authenticated; }

    // Create the menu-bar layer surface (top | left | right) with the given
    // height and exclusive zone. The compositor answers with `configure`.
    bool createMenuBarSurface(int height, int exclusiveZone);

    // Attach `image` to the chrome surface and commit. The image must be
    // ARGB32(_Premultiplied).
    bool commitImage(const QImage &image);

    // Drive the compositor's Mission Control overview (T-11 entry point).
    void enterMissionControl();

    // The compositor connection fd, for the Qt event-loop notifier.
    int displayFd() const;

    // Drain pending Wayland events without blocking (used from the fd
    // notifier). Returns false on a fatal connection error.
    bool dispatch();
    bool flush();

    QString lastError() const { return m_error; }

signals:
    void authenticated(uint32_t lockstepVersion);
    void refused(uint32_t code, const QString &message);
    void configured(int width, int height, uint32_t serial);
    void surfaceClosed();
    void focusedAppChanged(const QString &appId, const QString &title);
    void fatal(const QString &message);

private:
    struct ToplevelInfo {
        QString appId;
        QString title;
    };

    void bindTrustedGlobals();
    void teardown();
    bool fail(const QString &message);

    // Wayland listener trampolines.
    static void onRegistryGlobal(void *data, wl_registry *registry, uint32_t name,
                                 const char *interface, uint32_t version);
    static void onRegistryGlobalRemove(void *data, wl_registry *registry, uint32_t name);
    static void onCoreAuthenticated(void *data, df_core *core, uint32_t version);
    static void onCoreRefused(void *data, df_core *core, uint32_t code, const char *message);
    static void onLayerConfigure(void *data, df_layer_surface *layer, uint32_t serial,
                                 int32_t width, int32_t height);
    static void onLayerClosed(void *data, df_layer_surface *layer);
    static void onManagerToplevel(void *data, df_toplevel_manager *manager, df_toplevel *id);
    static void onManagerFocused(void *data, df_toplevel_manager *manager, df_toplevel *id);
    static void onManagerOutput(void *data, df_toplevel_manager *manager, df_output *id);
    static void onManagerWorkspace(void *data, df_toplevel_manager *manager, df_workspace *id);
    static void onManagerAttention(void *data, df_toplevel_manager *manager, df_toplevel *id);
    static void onManagerHotCorner(void *data, df_toplevel_manager *manager, uint32_t corner,
                                   const char *output);
    static void onManagerOverview(void *data, df_toplevel_manager *manager, uint32_t active,
                                  df_toplevel *selected);
    static void onManagerAppSwitcher(void *data, df_toplevel_manager *manager, uint32_t active,
                                     const char *appId, int32_t direction);
    static void onManagerInputAction(void *data, df_toplevel_manager *manager, const char *action,
                                     const char *source, uint32_t serial);
    static void onManagerProgress(void *data, df_toplevel_manager *manager, const char *action,
                                  int32_t progress, int32_t rawProgress, int32_t velocity,
                                  uint32_t phase, uint32_t committed, uint32_t cancelled);
    static void onManagerAppAccelerator(void *data, df_toplevel_manager *manager,
                                        const char *appId, const char *acceleratorId,
                                        const char *source, uint32_t serial);
    static void onManagerDone(void *data, df_toplevel_manager *manager);
    static void onToplevelTitle(void *data, df_toplevel *toplevel, const char *title);
    static void onToplevelAppId(void *data, df_toplevel *toplevel, const char *appId);
    static void onToplevelState(void *data, df_toplevel *toplevel, uint32_t state);
    static void onToplevelWorkspaceEntered(void *data, df_toplevel *toplevel, df_workspace *ws);
    static void onToplevelWorkspaceLeft(void *data, df_toplevel *toplevel, df_workspace *ws);
    static void onToplevelOutputEntered(void *data, df_toplevel *toplevel, df_output *output);
    static void onToplevelOutputLeft(void *data, df_toplevel *toplevel, df_output *output);
    static void onToplevelClosed(void *data, df_toplevel *toplevel);
    static void onToplevelDone(void *data, df_toplevel *toplevel);
    static void onBufferRelease(void *data, wl_buffer *buffer);
    // df_output/df_workspace are announced by the manager; the menu bar does
    // not consume their properties yet, but every event needs a handler.
    static void onOutputName(void *data, df_output *output, const char *name);
    static void onOutputGeometry(void *data, df_output *output, int32_t x, int32_t y,
                                 int32_t width, int32_t height);
    static void onOutputMode(void *data, df_output *output, uint32_t flags, uint32_t width,
                             uint32_t height, uint32_t refresh);
    static void onOutputScale(void *data, df_output *output, int32_t scale);
    static void onOutputTransform(void *data, df_output *output, uint32_t transform);
    static void onOutputVrr(void *data, df_output *output, uint32_t enabled);
    static void onOutputNightLight(void *data, df_output *output, uint32_t enabled,
                                   uint32_t temperature);
    static void onOutputReservedZone(void *data, df_output *output, uint32_t edge,
                                     uint32_t thickness);
    static void onOutputDone(void *data, df_output *output);
    static void onOutputRemoved(void *data, df_output *output);
    static void onWorkspaceName(void *data, df_workspace *workspace, const char *name);
    static void onWorkspaceIndex(void *data, df_workspace *workspace, uint32_t index);
    static void onWorkspaceActivated(void *data, df_workspace *workspace, uint32_t active);
    static void onWorkspaceFullscreen(void *data, df_workspace *workspace, uint32_t fullscreen);
    static void onWorkspaceWallpaper(void *data, df_workspace *workspace, const char *source,
                                     uint32_t fit, uint32_t color);
    static void onWorkspaceRemoved(void *data, df_workspace *workspace);
    static void onWorkspaceDone(void *data, df_workspace *workspace);

    wl_display *m_display = nullptr;
    wl_registry *m_registry = nullptr;
    wl_compositor *m_compositor = nullptr;
    wl_shm *m_shm = nullptr;
    df_core *m_core = nullptr;
    df_shell *m_shell = nullptr;
    df_toplevel_manager *m_manager = nullptr;
    wl_surface *m_surface = nullptr;
    df_layer_surface *m_layer = nullptr;

    uint32_t m_coreName = 0;
    uint32_t m_coreVersion = 0;
    uint32_t m_shellName = 0;
    uint32_t m_shellVersion = 0;
    uint32_t m_managerName = 0;
    uint32_t m_managerVersion = 0;
    uint32_t m_compositorVersion = 0;
    uint32_t m_shmVersion = 0;

    bool m_authenticated = false;
    bool m_trustedGlobalsBound = false;
    QString m_error;

    QHash<df_toplevel *, ToplevelInfo> m_toplevels;
    df_toplevel *m_focused = nullptr;
    QSet<wl_buffer *> m_buffers;
};
