// SPDX-License-Identifier: MIT
// Private-protocol client for the Dragonfruit shell (T-09).
//
// Connects to the compositor over the df_core/df_shell/df_toplevel_manager
// interfaces, performs the launch-token handshake, and owns the menu-bar
// chrome surface. Rendering is deliberately separate (ShellController):
// this class only speaks Wayland.
#pragma once

#include <QByteArray>
#include <QHash>
#include <QImage>
#include <QList>
#include <QObject>
#include <QRect>
#include <QSet>
#include <QString>
#include <QStringList>
#include <QVariantList>

#include <cstdint>

// The listener signatures mirror the libwayland C API exactly (wl_fixed_t,
// wl_array), so pull in the client header rather than re-declaring them.
#include <wayland-client.h>

struct df_core;
struct df_shell;
struct df_layer_surface;
struct df_toplevel_manager;
struct df_toplevel;
struct df_output;
struct df_workspace;

class QSocketNotifier;

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

    // Create the dropdown `overlay` layer surface (T-09). It starts unmapped;
    // `setPopupGeometry` + `commitPopupImage` place and reveal it, `hidePopup`
    // unmaps it. Unlike the bar it reserves no zone and never takes keyboard.
    bool createPopupSurface();

    // Place the popup layer surface at window coordinates `(x, y)` sized
    // `(width, height)`. The compositor answers with a fresh `configure`.
    bool setPopupGeometry(int x, int y, int width, int height);

    // Attach `image` to the popup surface and commit. The image must be
    // ARGB32(_Premultiplied).
    bool commitPopupImage(const QImage &image);

    // Unmap the popup surface (attach a null buffer).
    bool hidePopup();

    // The screen edge the Dock anchors to (T-10 section 5). The surface's
    // `thickness` is its extent perpendicular to the edge (the baseline bar
    // plus the magnify band); it stretches along the edge.
    enum class DockPosition { Bottom, Left, Right };

    // Create the Dock layer surface (T-10): `top` layer, anchored to
    // `position`, namespace "dock". It reserves `exclusiveZone` pixels on its
    // edge and never takes keyboard focus.
    bool createDockSurface(DockPosition position, int thickness, int exclusiveZone);

    // Re-apply the Dock surface's position, extent, and reserved zone after a
    // live `dock.position` / `dock.size` / `dock.autohide` change (T-10
    // section 19). The compositor answers with a fresh configure.
    bool configureDockSurface(DockPosition position, int thickness, int exclusiveZone);

    // Create the Dock's transient `overlay` layer surface (T-10 context menus
    // and the window chooser). Anchored to the Dock's edge (`bottom|left` for
    // a bottom Dock) so it can be placed with margins, reserves nothing
    // (`exclusive_zone = -1`), and starts unmapped.
    bool createDockPopupSurface(DockPosition position);

    // Place the Dock popup with margins (top, right, bottom, left) inset from
    // the popup's anchored edges and size `(width, height)`. The shell
    // computes the margins from the popover's Dock-scene rectangle and the
    // Dock's position (T-10 section 5). The compositor answers with a fresh
    // `dockPopupConfigured`.
    bool setDockPopupGeometry(int top, int right, int bottom, int left, int width, int height);

    // Attach `image` to the Dock popup surface and commit.
    bool commitDockPopupImage(const QImage &image);

    // Unmap the Dock popup surface (attach a null buffer).
    bool hideDockPopup();

    // Attach `image` to the Dock surface and commit. The image must be
    // ARGB32(_Premultiplied).
    bool commitDockImage(const QImage &image);

    // Set the Dock surface's input region to the union of `rects` in
    // surface-local coordinates. An empty list makes the whole surface pass
    // input through (the hidden Dock, the transparent magnified band). Each
    // frame the shell sends the visible bar plus the currently magnified or
    // bouncing icon rectangles (T-10 FR-13).
    bool setDockInputRegion(const QList<QRect> &rects);

    // Attach `image` to the menu-bar surface and commit. The image must be
    // ARGB32(_Premultiplied).
    bool commitImage(const QImage &image);

    // Drive the compositor's Mission Control overview (T-11 entry point).
    void enterMissionControl();

    // App-level activation for a Dock click (T-10): the compositor picks the
    // app's most recent window, switches to its Space, and restores it.
    void activateApp(const QString &appId);

    // Assign every window of `appId` to the currently active Space (the Dock
    // "Options ▸ Assign to This Desktop" action, T-10 section 13). The
    // compositor has no sticky/all-Spaces assignment, so "All Desktops" and
    // "None" are handled by the controller as pending.
    void assignAppToActiveWorkspace(const QString &appId);

    // Window-level activation for the Dock window chooser (T-10 FR-5): the
    // compositor restores the window if minimized, switches to its Space, and
    // focuses it (`select_overview_toplevel` semantics). `windowId` is the
    // opaque handle string the Dock projection carries.
    void selectToplevel(const QString &windowId);

    // Close one window (the chooser/menu `Quit` closes every window of an
    // app; `df_toplevel.close` is per-window).
    void closeToplevel(const QString &windowId);

    // Close every window of `appId` (the interim `Quit` action, T-10 section
    // 13). The compositor does not expose an app-level quit.
    void closeApp(const QString &appId);

    // Release chrome keyboard focus (T-10 section 20): the compositor hands
    // the keyboard back to the active window. Sent when Escape exits Dock
    // keyboard navigation.
    void releaseKeyboardFocus();

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
    void popupConfigured(int width, int height, uint32_t serial);
    void dockConfigured(int width, int height, uint32_t serial);
    void dockPopupConfigured(int width, int height, uint32_t serial);
    void surfaceClosed();
    void focusedAppChanged(const QString &appId, const QString &title);
    // xdg-activation attention for an app's toplevel (T-10 FR-4): the Dock
    // bounces the owning entry and stops on click or focus.
    void attentionRequested(const QString &appId);
    // The running-app projection the Dock renders (T-10): one entry per app
    // with at least one window, plus per-window minimized entries.
    void dockStateChanged(const QVariantList &entries);
    void fatal(const QString &message);
    // Input bridge: pointer/keyboard events delivered to the chrome surface.
    void pointerMoved(qreal x, qreal y);
    void pointerButton(qreal x, qreal y, uint32_t button, bool pressed);
    void pointerLeft();
    void dockPointerMoved(qreal x, qreal y);
    void dockPointerButton(qreal x, qreal y, uint32_t button, bool pressed);
    void dockPointerLeft();
    // Dock popover input, in popover-local coordinates; the shell controller
    // translates it into the Dock scene before forwarding.
    void dockPopupPointerMoved(qreal x, qreal y);
    void dockPopupPointerButton(qreal x, qreal y, uint32_t button, bool pressed);
    void dockPopupPointerLeft();
    void keyboardFocused(bool focused);
    // The Dock surface specifically gained/lost the keyboard (T-10 section
    // 20). Distinct from `keyboardFocused` so the shell can route keys to the
    // Dock while the bar still tracks its own focus.
    void dockKeyboardFocused(bool focused);
    void keyEvent(uint32_t key, bool pressed);
    // External drag-and-drop onto the Dock (T-10 section 12). The shell binds
    // the seat's data device, so a drag from another client (Files, a
    // launcher) is delivered here. Coordinates are Dock-surface-local; the
    // controller offsets them into the offscreen scene like pointer events.
    void dockExternalDragEntered(bool payloadIsApp, qreal x, qreal y);
    void dockExternalDragMoved(qreal x, qreal y);
    void dockExternalDragLeft();
    // A drop arrived. `payloadIsApp` selects the payload: `desktopId` is the
    // application alias (an `application/x-dragonfruit-app` value or a single
    // `.desktop` URI); otherwise `paths` are the dropped files.
    void dockExternalDropped(bool payloadIsApp, const QString &desktopId,
                             const QStringList &paths, qreal x, qreal y);
    // A compositor input action broadcast (`df_toplevel_manager.input_action`,
    // T-07). The Dock uses `focus-dock` and `toggle-dock` (T-10 section 20).
    void inputAction(const QString &action, const QString &source);

private:
    struct ToplevelInfo {
        QString appId;
        QString title;
        // df_toplevel.state bitfield (minimized=1, zoomed=2, fullscreen=4,
        // focused=8); the Dock uses minimized.
        uint32_t state = 0;
        // Stable handle the shell uses to address this window in the Dock's
        // window chooser/menus (the `df_toplevel` pointer as an integer).
        quintptr windowId = 0;
        // The Space the window currently lives on, resolved to an index/name
        // for the chooser's row label.
        df_workspace *workspace = nullptr;
    };

    struct WorkspaceInfo {
        int index = -1;
        QString name;
    };

    void bindTrustedGlobals();
    void teardown();
    bool fail(const QString &message);
    // Bind the seat's data device once both the manager and the seat exist.
    void maybeCreateDataDevice();
    // Rebuild the Dock's running-app projection and emit `dockStateChanged`.
    void emitDockState();
    // Resolve a window handle back to its `df_toplevel` (null when gone).
    df_toplevel *toplevelForId(quintptr windowId) const;
    // Attach `image` to `surface` as a fresh shm buffer and commit it.
    bool commitTo(wl_surface *surface, const QImage &image);

    // Wayland listener trampolines.
    static void onRegistryGlobal(void *data, wl_registry *registry, uint32_t name,
                                 const char *interface, uint32_t version);
    static void onRegistryGlobalRemove(void *data, wl_registry *registry, uint32_t name);
    static void onCoreAuthenticated(void *data, df_core *core, uint32_t version);
    static void onCoreRefused(void *data, df_core *core, uint32_t code, const char *message);
    static void onLayerConfigure(void *data, df_layer_surface *layer, uint32_t serial,
                                 int32_t width, int32_t height);
    static void onPopupConfigure(void *data, df_layer_surface *layer, uint32_t serial,
                                 int32_t width, int32_t height);
    static void onDockConfigure(void *data, df_layer_surface *layer, uint32_t serial,
                                int32_t width, int32_t height);
    static void onDockPopupConfigure(void *data, df_layer_surface *layer, uint32_t serial,
                                     int32_t width, int32_t height);
    static void onLayerClosed(void *data, df_layer_surface *layer);
    static void onSeatCapabilities(void *data, wl_seat *seat, uint32_t capabilities);
    static void onSeatName(void *data, wl_seat *seat, const char *name);
    static void onPointerEnter(void *data, wl_pointer *pointer, uint32_t serial,
                               wl_surface *surface, wl_fixed_t x, wl_fixed_t y);
    static void onPointerLeave(void *data, wl_pointer *pointer, uint32_t serial,
                               wl_surface *surface);
    static void onPointerMotion(void *data, wl_pointer *pointer, uint32_t time, wl_fixed_t x,
                                wl_fixed_t y);
    static void onPointerButton(void *data, wl_pointer *pointer, uint32_t serial, uint32_t time,
                                uint32_t button, uint32_t state);
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
                                    uint32_t modsDepressed, uint32_t modsLatched,
                                    uint32_t modsLocked, uint32_t group);
    // wl_data_device / wl_data_offer (external DnD target, T-10 section 12).
    static void onDataDeviceOffer(void *data, wl_data_device *device, wl_data_offer *offer);
    static void onDataDeviceEnter(void *data, wl_data_device *device, uint32_t serial,
                                  wl_surface *surface, wl_fixed_t x, wl_fixed_t y,
                                  wl_data_offer *offer);
    static void onDataDeviceLeave(void *data, wl_data_device *device);
    static void onDataDeviceMotion(void *data, wl_data_device *device, uint32_t time,
                                   wl_fixed_t x, wl_fixed_t y);
    static void onDataDeviceDrop(void *data, wl_data_device *device);
    static void onDataDeviceSelection(void *data, wl_data_device *device, wl_data_offer *offer);
    static void onDataOfferMimeType(void *data, wl_data_offer *offer, const char *mimeType);
    static void onDataOfferSourceActions(void *data, wl_data_offer *offer, uint32_t actions);
    static void onDataOfferAction(void *data, wl_data_offer *offer, uint32_t action);
    // Read the drop payload from the pipe once the source has written it.
    void onDndReadable();
    void resetExternalDrag();
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
    wl_surface *m_popupSurface = nullptr;
    df_layer_surface *m_popupLayer = nullptr;
    wl_surface *m_dockSurface = nullptr;
    df_layer_surface *m_dockLayer = nullptr;
    bool m_dockMapped = false;
    // The Dock's transient popover surface (context menus, window chooser).
    wl_surface *m_dockPopupSurface = nullptr;
    df_layer_surface *m_dockPopupLayer = nullptr;
    bool m_dockPopupMapped = false;
    // Window-space origin of the popup surface, used to translate pointer
    // coordinates delivered relative to the popup into window coordinates.
    int m_popupX = 0;
    int m_popupY = 0;
    bool m_popupMapped = false;
    // True while the pointer is over the popup surface, so its surface-local
    // coordinates are translated into window coordinates.
    bool m_pointerOnPopup = false;
    // True while the pointer is over the Dock surface; its coordinates are
    // already Dock-window-local.
    bool m_pointerOnDock = false;
    // True while the pointer is over the Dock popover; its coordinates are
    // popover-local and translated by the shell controller.
    bool m_pointerOnDockPopup = false;
    // True while the Dock surface holds the keyboard (T-10 section 20), so
    // key events are routed to the Dock scene.
    bool m_keyboardOnDock = false;
    wl_seat *m_seat = nullptr;
    wl_pointer *m_pointer = nullptr;
    wl_keyboard *m_keyboard = nullptr;
    qreal m_pointerX = 0;
    qreal m_pointerY = 0;

    // External drag-and-drop target (T-10 section 12): the seat's data device
    // and the in-flight offer. The shell reads a `text/uri-list` (files) or an
    // application alias on drop.
    wl_data_device_manager *m_dataDeviceManager = nullptr;
    wl_data_device *m_dataDevice = nullptr;
    wl_data_offer *m_dndOffer = nullptr;
    QStringList m_dndMimeTypes;
    QString m_dndMime;
    bool m_dndActive = false;
    bool m_dndPayloadIsApp = false;
    qreal m_dndX = 0;
    qreal m_dndY = 0;
    int m_dndReadFd = -1;
    QSocketNotifier *m_dndReadNotifier = nullptr;
    QByteArray m_dndData;

    uint32_t m_coreName = 0;
    uint32_t m_coreVersion = 0;
    uint32_t m_shellName = 0;
    uint32_t m_shellVersion = 0;
    uint32_t m_managerName = 0;
    uint32_t m_managerVersion = 0;
    uint32_t m_compositorVersion = 0;
    uint32_t m_shmVersion = 0;
    uint32_t m_seatName = 0;
    uint32_t m_seatVersion = 0;

    bool m_authenticated = false;
    bool m_trustedGlobalsBound = false;
    QString m_error;

    QHash<df_toplevel *, ToplevelInfo> m_toplevels;
    // Announcement order (oldest first) so the Dock can present windows
    // most-recent-first; the compositor does not expose its recency to us.
    QList<df_toplevel *> m_toplevelOrder;
    // Stable window handle -> toplevel, for the Dock window chooser.
    QHash<quintptr, df_toplevel *> m_toplevelById;
    // Announced Spaces, resolved to an index/name for chooser row labels.
    QHash<df_workspace *, WorkspaceInfo> m_workspaces;
    // The Space the last `workspace_activated` event named (T-10 section 13);
    // "Assign to This Desktop" moves an app's windows here.
    df_workspace *m_activeWorkspace = nullptr;
    df_toplevel *m_focused = nullptr;
    QSet<wl_buffer *> m_buffers;
};
