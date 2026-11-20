// SPDX-License-Identifier: MIT
// The Settings app's Accessibility seam (T-15.14b).
//
// The Accessibility pane never touches D-Bus: it binds the `Settings`
// singleton, and the bridge forwards reads here. The live
// `DbusAccessibilityClient` talks to the bridge host's
// `org.dragonfruit.SystemStatus1.Accessibility` interface (the same AT-SPI
// projection the Quick Settings tile reads), and `MockAccessibilityClient`
// serves a deterministic fixture for the headless pane tests
// (`DF_ACCESSIBILITY_FIXTURE`). The adapter is read-only (`org.a11y.Status`
// has no setter, ADR 0144), so there is no write here: the pane's one durable
// preference (`accessibility.reduceMotion`) is a settingsd key the pane writes
// through `Settings.set`. Absence is a normal state: `available()` is false and
// the view is empty — never an error.
#pragma once

#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantMap>

class QDBusServiceWatcher;

class AccessibilityClient : public QObject
{
    Q_OBJECT

public:
    explicit AccessibilityClient(QObject *parent = nullptr) : QObject(parent) {}
    ~AccessibilityClient() override = default;

    // Whether the bridge host owns its session-bus name.
    virtual bool available() const = 0;
    // The decoded `accessibility` view (`{state, glyph, label, present,
    // enabled, enabledLabel, screenReader, screenReaderLabel}`); empty when
    // absent.
    virtual QVariantMap view() const = 0;
    // Re-read the AT-SPI accessibility bus once (the pane calls this on open).
    virtual void refresh() = 0;
    // Test seam: restore the fixture's initial state. A no-op on the live
    // client, which has no fixture to reset.
    virtual void resetForTest() {}

    // T275 delegates. The screen reader, speech engine, and on-screen
    // keyboard are external programs, never reimplemented here: the pane
    // offers a real launch action and disables it when the program is absent.
    // `DF_ACCESSIBILITY_DELEGATE_FIXTURE=present|absent` makes the headless
    // tests and captures deterministic without spawning a process.
    bool screenReaderAvailable() const;
    bool speechAvailable() const;
    bool onScreenKeyboardAvailable() const;
    QString lastDelegate() const { return m_lastDelegate; }
    bool openScreenReader();
    bool speak(const QString &text);
    bool openOnScreenKeyboard();

signals:
    void changed(const QVariantMap &view);
    void availableChanged(bool available);
    void delegateLaunched(const QString &id, bool ok);

private:
    enum class DelegateFixture { Auto, Present, Absent };
    DelegateFixture delegateFixture() const;
    bool delegateAvailable(const QStringList &programs) const;
    bool launchDelegate(const QString &id, const QString &program,
                        const QStringList &arguments);
    QString m_lastDelegate;
};

// The live client over the user session bus.
class DbusAccessibilityClient : public AccessibilityClient
{
    Q_OBJECT

public:
    explicit DbusAccessibilityClient(QObject *parent = nullptr);
    ~DbusAccessibilityClient() override;

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

// The fixture client used by `DF_ACCESSIBILITY_FIXTURE`: a Dragonfruit
// workstation whose AT-SPI accessibility bus reports the toolkit bridge and a
// screen reader on. The adapter is read-only, so the fixture never mutates.
class MockAccessibilityClient : public AccessibilityClient
{
    Q_OBJECT

public:
    explicit MockAccessibilityClient(QObject *parent = nullptr);

    bool available() const override;
    QVariantMap view() const override { return m_view; }
    void refresh() override;
    void resetForTest() override;

private:
    void rebuild();
    bool fixtureAbsent() const;
    bool m_enabled = true;
    bool m_screenReader = true;
    QVariantMap m_view;
};