// SPDX-License-Identifier: GPL-3.0-or-later
// T338: the wallpaper provider's resolved source properties, read
// asynchronously and coalesced. The shell must never block startup or a
// wallpaper change on a hung `wallpaperd`; a missing or slow provider keeps
// the last known values.
#pragma once
#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantMap>

class QDBusServiceWatcher;

class WallpaperProviderClient : public QObject
{
    Q_OBJECT

public:
    explicit WallpaperProviderClient(
        const QString &service = QStringLiteral("org.dragonfruit.Wallpaper1"),
        const QString &path = QStringLiteral("/org/dragonfruit/Wallpaper1"),
        const QString &interface = QStringLiteral("org.dragonfruit.Wallpaper1"),
        int readTimeoutMs = 2000, QObject *parent = nullptr);
    ~WallpaperProviderClient() override;

    // Subscribe to the provider's property/items signals and start the first
    // read. Safe to call more than once; repeated starts reuse the watcher.
    void start();

    QString defaultSource() const { return m_defaultSource; }
    QString builtinDefaultSource() const { return m_builtinDefaultSource; }

public slots:
    // One coalesced `Properties.GetAll` at a time. A burst of provider
    // signals becomes at most one follow-up read.
    void refresh();
    void onItemsChanged();
    void onPropertiesChanged(const QString &interface, const QVariantMap &values,
                             const QStringList &invalidated);

signals:
    void changed();

private:
    void applyProperties(const QVariantMap &properties);
    void applyProperty(const QString &name, const QVariant &value, bool *changed);

    QString m_service;
    QString m_path;
    QString m_interface;
    int m_readTimeoutMs = 2000;
    QDBusServiceWatcher *m_watcher = nullptr;
    QString m_defaultSource;
    QString m_builtinDefaultSource;
    bool m_subscribed = false;
    bool m_refreshPending = false;
    bool m_refreshQueued = false;
};
