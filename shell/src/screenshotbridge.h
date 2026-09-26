// SPDX-License-Identifier: MIT
// The shell's half of the Screenshot presenter seam (T-13.3a).
//
// `ScreenshotBridge` watches the portal backend's diagnostic
// `org.dragonfruit.Portal1.ScreenshotOpened` signal and presents the
// selection overlay for the request's mode. When the user makes a selection
// the bridge hands the chosen rectangle to the capture seam
// (`captureRequested`) and returns the captured URI to the waiting portal
// request with `CompleteScreenshot`; a cancellation answers `CancelScreenshot`.
//
// The desktop's own Cmd+Shift+3/4 shortcut drives the same overlay through
// `beginLocal()` — no portal request, one selection UI. The capture bytes and
// save/copy are T-13.3b; this class owns the request lifecycle and the D-Bus
// presenter calls.
//
// The overlay itself is pure QML (`SelectionOverlay.qml`); this class owns the
// state the view binds to. `begin()` is the shared entry point for the live
// signal and the headless/capture fixture (`DF_SCREENSHOT_FIXTURE`).
#pragma once

#include <QObject>
#include <QString>
#include <QVariantMap>

class ScreenshotBridge : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool active READ active NOTIFY changed)
    Q_PROPERTY(QString handle READ handle NOTIFY changed)
    Q_PROPERTY(QString mode READ mode NOTIFY changed)
    Q_PROPERTY(QString appId READ appId NOTIFY changed)
    Q_PROPERTY(QString error READ error NOTIFY changed)

public:
    explicit ScreenshotBridge(QObject *parent = nullptr);
    ~ScreenshotBridge() override;

    // Subscribe to the portal's presenter signal. A missing portal is a normal
    // state: a portal request simply never arrives. Reconnect-safe.
    void connectService();
    bool serviceAvailable() const { return m_serviceAvailable; }

    bool active() const { return m_active; }
    QString handle() const { return m_handle; }
    QString mode() const { return m_mode; }
    QString appId() const { return m_appId; }
    QString error() const { return m_error; }

public slots:
    // Present one portal request. `mode` is `fullscreen`/`region`/`window`.
    void begin(const QString &handle, const QString &mode, const QString &appId,
               const QString &parentWindow);
    // Present the desktop's own capture flow (the Cmd+Shift+3/4 shortcut).
    void beginLocal(const QString &mode);
    // The user made a selection: hand the rectangle to the capture seam.
    void accept(int x, int y, int width, int height);
    // Cancel the request (no selection).
    void cancel();
    // Return the presenter's captured URI to the waiting portal request. This
    // is the T-13.3b capture/output seam.
    void complete(const QString &uri);

signals:
    void changed();
    // A request opened; the shell maps the overlay surface.
    void started();
    // The request resolved and the shell unmaps. `completed` is true when the
    // portal was handed a capture, false on cancellation.
    void finished(bool completed);
    // A selection was made and needs a capture. `mode` names the selection
    // kind; the rectangle is in overlay-local pixels. T-13.3b turns this into
    // a saved image and calls `complete()`.
    void captureRequested(const QString &mode, int x, int y, int width, int height);

private slots:
    // The portal backend's presenter signal.
    void onScreenshotOpened(const QString &handle, const QString &mode, const QString &appId,
                            const QString &parentWindow, const QVariantMap &options);

private:
    // Clear every per-request field.
    void reset();
    // Re-evaluate the portal service presence.
    void checkService();

    bool m_active = false;
    QString m_handle;
    QString m_mode;
    QString m_appId;
    QString m_error;
    bool m_serviceAvailable = false;
    bool m_connected = false;
};