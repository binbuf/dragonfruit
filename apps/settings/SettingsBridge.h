// SPDX-License-Identifier: MIT
// The Settings app's live settings client, exposed to QML as the `Settings`
// singleton (T-09.1b).
//
// Panes never touch D-Bus: `Settings.values` is a reactive map of every
// `org.dragonfruit.Settings1` key, and `Settings.set(key, value)` writes one
// through settingsd (optimistically local first, then mirrored). A control
// binds to the map and writes on change, so the key and the consumer both
// update without restart:
//
//     Toggle {
//         onToggled: (checked) => Settings.set("accessibility.reduceMotion", checked)
//         Binding {
//             target: reduceMotion
//             property: "checked"
//             value: Settings.values["accessibility.reduceMotion"] === true
//         }
//     }
//
// With no daemon on the bus the underlying `DbusSettingsClient` serves the
// schema defaults and keeps writes in memory, so panes still work. Tests set
// `DF_SETTINGS_FIXTURE` to force the deterministic `MockSettingsClient`.
#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariant>
#include <QVariantMap>
#include <QtQml/qqmlregistration.h>

class SettingsClient;

class SettingsBridge : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(Settings)
    QML_SINGLETON
    // Every key's current value. QML bindings such as
    // `Settings.values["dock.size"]` re-evaluate when this changes; it is the
    // one reactive surface, so no pane polls.
    Q_PROPERTY(QVariantMap values READ values NOTIFY valuesChanged)
    // Whether the daemon currently owns the bus name. False means defaults +
    // in-memory writes (the absent-provider state panes must degrade under).
    Q_PROPERTY(bool available READ available NOTIFY availableChanged)

public:
    explicit SettingsBridge(QObject *parent = nullptr);
    ~SettingsBridge() override;

    QVariantMap values() const;
    bool available() const;

    // Read one key (with an optional fallback for an unknown key).
    Q_INVOKABLE QVariant value(const QString &key, const QVariant &fallback = {}) const;
    // Write one key through settingsd. The local store updates on the same
    // event-loop turn; the daemon's `Changed` echo is de-duplicated.
    Q_INVOKABLE void set(const QString &key, const QVariant &value);
    // Re-read the daemon snapshot (`GetAll`); a no-op when no daemon is up.
    Q_INVOKABLE void refresh();
    // Every key the schema (and the defaults table) knows.
    Q_INVOKABLE QStringList keys() const;

signals:
    void valuesChanged();
    void availableChanged(bool available);
    // One key really changed (local write or daemon signal). `values` also
    // changes; listen to this only when a key's identity matters.
    void changed(const QString &key, const QVariant &value);

private:
    SettingsClient *m_client = nullptr;
};