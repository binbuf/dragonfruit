// SPDX-License-Identifier: MIT
// The Settings app's Sound seam (T-15.3b).
//
// The Sound pane never touches D-Bus: it binds the `Settings` singleton, and
// the bridge forwards reads and writes here. The live `DbusSoundClient` talks
// to the bridge host's `org.dragonfruit.SystemStatus1.Audio` interface (the
// same adapter the Control Center tile reads), and `MockSoundClient` serves a
// deterministic fixture for the headless pane tests (`DF_SOUND_FIXTURE`).
// Absence is a normal state: `available()` is false and the view is empty —
// never an error.
#pragma once

#include <QObject>
#include <QString>
#include <QVariantMap>

class QDBusServiceWatcher;

class SoundClient : public QObject
{
    Q_OBJECT

public:
    explicit SoundClient(QObject *parent = nullptr) : QObject(parent) {}
    ~SoundClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `audio` view (`{state, volume, muted, defaultSink,
    // defaultSource, sinks, sources}`); empty when absent.
    virtual QVariantMap view() const = 0;
    // Re-read the host once (the pane calls this on open).
    virtual void refresh() = 0;
    // The explicit writes; the host re-reads and pushes the new view.
    virtual void setVolume(double volume) = 0;
    virtual void setMute(bool muted) = 0;
    virtual void setDefaultSink(int id) = 0;
    virtual void setDefaultSource(int id) = 0;

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
};

// The live client over the user session bus.
class DbusSoundClient : public SoundClient
{
    Q_OBJECT

public:
    explicit DbusSoundClient(QObject *parent = nullptr);
    ~DbusSoundClient() override;

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setVolume(double volume) override;
    void setMute(bool muted) override;
    void setDefaultSink(int id) override;
    void setDefaultSource(int id) override;

private:
    void call(const QString &method, const QVariantList &arguments);
    void applyReply(const QByteArray &json);
    QString m_service;
    QString m_path;
    QString m_interface;
    QVariantMap m_view;
    bool m_available = false;
    QDBusServiceWatcher *m_watcher = nullptr;
};

// The fixture client used by `DF_SOUND_FIXTURE`: two outputs and two inputs,
// with output volume/mute and default-device routing that mutate in place and
// re-emit, so a pane control's round-trip is observable with no bus.
class MockSoundClient : public SoundClient
{
    Q_OBJECT

public:
    explicit MockSoundClient(QObject *parent = nullptr);

    bool available() const override { return true; }
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void setVolume(double volume) override;
    void setMute(bool muted) override;
    void setDefaultSink(int id) override;
    void setDefaultSource(int id) override;

private:
    void rebuild();
    double m_volume = 0.6;
    bool m_muted = false;
    int m_defaultSinkId = 7;
    int m_defaultSourceId = 20;
    QVariantMap m_view;
};