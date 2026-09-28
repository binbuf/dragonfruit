// SPDX-License-Identifier: MIT
// The Settings app's keyboard/mouse/trackpad inventory seam (T-15.4b).
//
// The read-only libinput adapter (T-15.4a) never reaches the pane directly:
// the pane binds the `Settings` singleton, and the bridge forwards the
// inventory read here. The live `DbusInputClient` talks to the bridge host's
// `org.dragonfruit.SystemStatus1.Input` interface (the same view the Control
// Center tile reads), and `MockInputClient` serves a deterministic fixture for
// the headless pane tests (`DF_INPUT_FIXTURE`). Absence is a normal state:
// `available()` is false and the view is empty — never an error. The adapter is
// read-only, so there is no write method.
#pragma once

#include <QObject>
#include <QString>
#include <QVariantMap>

class QDBusServiceWatcher;

class InputClient : public QObject
{
    Q_OBJECT

public:
    explicit InputClient(QObject *parent = nullptr) : QObject(parent) {}
    ~InputClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `input` view (`{state, present, label, glyph, counts,
    // devices}`); empty when absent.
    virtual QVariantMap view() const = 0;
    // Re-read the host once (the pane calls this on open).
    virtual void refresh() = 0;

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
};

// The live client over the user session bus.
class DbusInputClient : public InputClient
{
    Q_OBJECT

public:
    explicit DbusInputClient(QObject *parent = nullptr);
    ~DbusInputClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;

private:
    void applyReply(const QByteArray &json);
    QString m_service;
    QString m_path;
    QString m_interface;
    QVariantMap m_view;
    bool m_available = false;
    QDBusServiceWatcher *m_watcher = nullptr;
};

// The fixture client used by `DF_INPUT_FIXTURE`: a keyboard, a trackpad, and a
// mouse, so the pane renders its inventory with no libinput and no bus.
class MockInputClient : public InputClient
{
    Q_OBJECT

public:
    explicit MockInputClient(QObject *parent = nullptr);

    bool available() const override { return true; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;

private:
    void rebuild();
    QVariantMap m_view;
};