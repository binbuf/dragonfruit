// SPDX-License-Identifier: MIT
// The shell's half of the ScreenCast presenter seam (T-13.4a).
//
// `ScreenCastBridge` watches the portal backend's diagnostic
// `org.dragonfruit.Portal1.ScreenCastOpened` signal and presents the source
// picker (monitors and windows) for the request. When the user chooses the
// bridge returns the selection to the waiting portal request with
// `CompleteScreenCast`; a cancellation answers `CancelScreenCast`.
//
// The set of available sources comes from the compositor, not from this class:
// `ShellController` reads the compositor's monitor/window projection and hands
// it in through `setSources` (the fixture uses the same path). The picker
// itself is pure QML (`ScreenCastPicker.qml`); this class owns the state the
// view binds to.
//
// Streaming is T-13.4b: the portal negotiates each chosen source into a live
// PipeWire node or the named stills fallback. This class reads the backend's
// diagnostic mode (`ScreenCastStreamMode`) so the picker can name the fallback
// before the user chooses; a missing producer is a normal, spoken state, never
// a silent one.
#pragma once

#include <QList>
#include <QMetaType>
#include <QObject>
#include <QString>
#include <QVariantList>

class QDBusPendingCallWatcher;

// One source selection on the wire: the presenter's opaque handle plus the
// source type bit (1 monitor, 2 window). Registered with the D-Bus metatype
// system so `CompleteScreenCast(a(su))` marshals.
struct ScreenCastSelection
{
    QString id;
    uint sourceType = 1;

    bool operator==(const ScreenCastSelection &other) const
    {
        return id == other.id && sourceType == other.sourceType;
    }
};

Q_DECLARE_METATYPE(ScreenCastSelection)
Q_DECLARE_METATYPE(QList<ScreenCastSelection>)

class QDBusArgument;
// The `a(su)` marshalling operators. Declared here so every translation unit
// that registers or exports the metatype (the shell and its tests) can see
// them; defined in `screencastbridge.cpp`.
QDBusArgument &operator<<(QDBusArgument &argument, const ScreenCastSelection &selection);
const QDBusArgument &operator>>(const QDBusArgument &argument, ScreenCastSelection &selection);

class ScreenCastBridge : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool active READ active NOTIFY changed)
    Q_PROPERTY(QString handle READ handle NOTIFY changed)
    Q_PROPERTY(QString sessionHandle READ sessionHandle NOTIFY changed)
    Q_PROPERTY(QString appId READ appId NOTIFY changed)
    Q_PROPERTY(uint types READ types NOTIFY changed)
    Q_PROPERTY(bool multiple READ multiple NOTIFY changed)
    Q_PROPERTY(uint cursorMode READ cursorMode NOTIFY changed)
    Q_PROPERTY(QVariantList sources READ sources NOTIFY changed)
    Q_PROPERTY(int selectedCount READ selectedCount NOTIFY changed)
    Q_PROPERTY(QString error READ error NOTIFY changed)
    // The backend's negotiated stream mode (`pipewire` / `stills`). Defaults to
    // `stills` so a missing producer is never mistaken for a live stream.
    Q_PROPERTY(QString streamMode READ streamMode NOTIFY changed)
    // A human note naming the stills fallback; empty when streaming is live.
    Q_PROPERTY(QString streamNote READ streamNote NOTIFY changed)

public:
    explicit ScreenCastBridge(QObject *parent = nullptr);
    ~ScreenCastBridge() override;

    // Subscribe to the portal's presenter signal. A missing portal is a normal
    // state: a request simply never arrives. Reconnect-safe.
    void connectService();
    bool serviceAvailable() const { return m_serviceAvailable; }

    bool active() const { return m_active; }
    QString handle() const { return m_handle; }
    QString sessionHandle() const { return m_sessionHandle; }
    QString appId() const { return m_appId; }
    uint types() const { return m_types; }
    bool multiple() const { return m_multiple; }
    uint cursorMode() const { return m_cursorMode; }
    QVariantList sources() const { return m_sources; }
    int selectedCount() const;
    QString error() const { return m_error; }
    QString streamMode() const { return m_streamMode; }
    QString streamNote() const;

public slots:
    // Present one portal request. `types` is the source-type bitmask (1
    // monitor, 2 window); `multiple` allows more than one choice.
    void begin(const QString &handle, const QString &sessionHandle, const QString &appId,
               uint types, bool multiple, uint cursorMode);
    // Publish the available sources (one map per source: `id`, `kind`
    // (`monitor`/`window`), `label`, `detail`). The controller supplies these
    // from the compositor projection before the picker is shown.
    void setSources(const QVariantList &sources);
    // Toggle (multiple) or set (single) one source's selection.
    void select(const QString &id);
    // Return the selection to the portal and close.
    void accept();
    // Cancel the request (no selection).
    void cancel();

signals:
    void changed();
    // A request opened; the shell maps the picker surface.
    void started();
    // The request resolved and the shell unmaps. `completed` is true when the
    // portal was handed a selection, false on cancellation.
    void finished(bool completed);

private slots:
    // The portal backend's presenter signal.
    void onScreenCastOpened(const QString &handle, const QString &sessionHandle,
                            const QString &appId, uint types, bool multiple,
                            const QVariantMap &options);
    // The async reply to the diagnostic `ScreenCastStreamMode` call.
    void onStreamModeReply(QDBusPendingCallWatcher *watcher);

private:
    // Clear every per-request field.
    void reset();
    // The chosen selections, in source order.
    QList<ScreenCastSelection> selections() const;
    // Re-evaluate the portal service presence.
    void checkService();
    // Read the backend's diagnostic stream mode (`ScreenCastStreamMode`); a
    // missing method or service leaves the stills default in place.
    void refreshStreamMode();
    // Store a stream mode and announce a change.
    void setStreamMode(const QString &mode);

    bool m_active = false;
    QString m_handle;
    QString m_sessionHandle;
    QString m_appId;
    uint m_types = 1;
    bool m_multiple = false;
    uint m_cursorMode = 1;
    QVariantList m_sources;
    QString m_error;
    QString m_streamMode = QStringLiteral("stills");
    bool m_serviceAvailable = false;
    bool m_connected = false;
};