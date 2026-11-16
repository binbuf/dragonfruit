// SPDX-License-Identifier: MIT
#include "AccessibilityClient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QProcess>
#include <QStandardPaths>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.SystemStatus1");
const QString kPath = QStringLiteral("/org/dragonfruit/SystemStatus1");
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Accessibility");

} // namespace

// --- AccessibilityClient delegates (T275) ----------------------------------

AccessibilityClient::DelegateFixture AccessibilityClient::delegateFixture() const
{
    const QString mode =
        qEnvironmentVariable("DF_ACCESSIBILITY_DELEGATE_FIXTURE").trimmed().toLower();
    if (mode == QLatin1String("present"))
        return DelegateFixture::Present;
    if (mode == QLatin1String("absent"))
        return DelegateFixture::Absent;
    return DelegateFixture::Auto;
}

bool AccessibilityClient::delegateAvailable(const QStringList &programs) const
{
    switch (delegateFixture()) {
    case DelegateFixture::Present:
        return true;
    case DelegateFixture::Absent:
        return false;
    case DelegateFixture::Auto:
        break;
    }
    for (const QString &program : programs) {
        if (!QStandardPaths::findExecutable(program).isEmpty())
            return true;
    }
    return false;
}

bool AccessibilityClient::launchDelegate(const QString &id, const QString &program,
                                         const QStringList &arguments)
{
    const DelegateFixture fixture = delegateFixture();
    if (fixture == DelegateFixture::Absent) {
        emit delegateLaunched(id, false);
        return false;
    }
    if (fixture == DelegateFixture::Present) {
        m_lastDelegate = id;
        emit delegateLaunched(id, true);
        return true;
    }
    if (program.isEmpty()) {
        emit delegateLaunched(id, false);
        return false;
    }
    const bool ok = QProcess::startDetached(program, arguments);
    if (ok)
        m_lastDelegate = id;
    emit delegateLaunched(id, ok);
    return ok;
}

bool AccessibilityClient::screenReaderAvailable() const
{
    return delegateAvailable({QStringLiteral("orca")});
}

bool AccessibilityClient::speechAvailable() const
{
    return delegateAvailable({QStringLiteral("spd-say"), QStringLiteral("espeak-ng")});
}

bool AccessibilityClient::onScreenKeyboardAvailable() const
{
    return delegateAvailable({QStringLiteral("onboard"),
                              QStringLiteral("wvkbd-mobintl"),
                              QStringLiteral("wvkbd")});
}

bool AccessibilityClient::openScreenReader()
{
    return launchDelegate(QStringLiteral("screen-reader"), QStringLiteral("orca"), {});
}

bool AccessibilityClient::speak(const QString &text)
{
    if (text.trimmed().isEmpty())
        return false;
    if (!QStandardPaths::findExecutable(QStringLiteral("spd-say")).isEmpty()
        || delegateFixture() == DelegateFixture::Present) {
        return launchDelegate(QStringLiteral("speech"), QStringLiteral("spd-say"), {text});
    }
    return launchDelegate(QStringLiteral("speech"), QStringLiteral("espeak-ng"), {text});
}

bool AccessibilityClient::openOnScreenKeyboard()
{
    for (const QString &program : {QStringLiteral("onboard"),
                                   QStringLiteral("wvkbd-mobintl"),
                                   QStringLiteral("wvkbd")}) {
        if (delegateFixture() == DelegateFixture::Present
            || !QStandardPaths::findExecutable(program).isEmpty()) {
            return launchDelegate(QStringLiteral("on-screen-keyboard"), program, {});
        }
    }
    return launchDelegate(QStringLiteral("on-screen-keyboard"), QString(), {});
}

// --- DbusAccessibilityClient -----------------------------------------------

DbusAccessibilityClient::DbusAccessibilityClient(QObject *parent)
    : AccessibilityClient(parent)
    , m_service(kService)
    , m_path(kPath)
    , m_interface(kInterface)
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    m_available = available();
    m_watcher = new QDBusServiceWatcher(
        m_service, bus, QDBusServiceWatcher::WatchForRegistration
                          | QDBusServiceWatcher::WatchForUnregistration,
        this);
    connect(m_watcher, &QDBusServiceWatcher::serviceRegistered, this,
            [this](const QString &) {
                m_available = true;
                emit availableChanged(true);
                refresh();
            });
    connect(m_watcher, &QDBusServiceWatcher::serviceUnregistered, this,
            [this](const QString &) {
                m_available = false;
                emit availableChanged(false);
                if (!m_view.isEmpty()) {
                    m_view.clear();
                    emit changed(m_view);
                }
            });
    refresh();
}

DbusAccessibilityClient::~DbusAccessibilityClient() = default;

bool DbusAccessibilityClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusAccessibilityClient::refresh()
{
    auto *iface = new QDBusInterface(m_service, m_path, m_interface,
                                     QDBusConnection::sessionBus(), this);
    if (!iface->isValid()) {
        iface->deleteLater();
        if (m_available) {
            m_available = false;
            emit availableChanged(false);
        }
        return;
    }
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(QStringLiteral("State"), {}), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid())
                    return;
                applyReply(reply.value().toUtf8());
            });
}

void DbusAccessibilityClient::applyReply(const QByteArray &json)
{
    QJsonParseError parseError{};
    const QJsonDocument document = QJsonDocument::fromJson(json, &parseError);
    QVariantMap view;
    if (parseError.error == QJsonParseError::NoError && document.isObject())
        view = document.object().toVariantMap();
    if (view == m_view)
        return;
    m_view = view;
    emit changed(m_view);
}

// --- MockAccessibilityClient -----------------------------------------------

MockAccessibilityClient::MockAccessibilityClient(QObject *parent)
    : AccessibilityClient(parent)
{
    resetForTest();
}

void MockAccessibilityClient::resetForTest()
{
    m_enabled = true;
    m_screenReader = true;
    rebuild();
}

bool MockAccessibilityClient::fixtureAbsent() const
{
    return qEnvironmentVariable("DF_ACCESSIBILITY_FIXTURE").trimmed().toLower()
           == QLatin1String("absent");
}

bool MockAccessibilityClient::available() const
{
    return !fixtureAbsent();
}

void MockAccessibilityClient::refresh()
{
    rebuild();
}

void MockAccessibilityClient::rebuild()
{
    if (fixtureAbsent()) {
        if (!m_view.isEmpty()) {
            m_view.clear();
            emit changed(m_view);
        }
        return;
    }
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("accessibility"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("accessibility"));
    view.insert(QStringLiteral("present"), m_enabled || m_screenReader);
    view.insert(QStringLiteral("enabled"), m_enabled);
    view.insert(QStringLiteral("enabledLabel"),
                m_enabled ? QStringLiteral("On") : QStringLiteral("Off"));
    view.insert(QStringLiteral("screenReader"), m_screenReader);
    view.insert(QStringLiteral("screenReaderLabel"),
                m_screenReader ? QStringLiteral("On") : QStringLiteral("Off"));
    view.insert(QStringLiteral("label"),
                m_screenReader ? QStringLiteral("Screen Reader On")
                               : (m_enabled ? QStringLiteral("On")
                                            : QStringLiteral("Off")));
    m_view = view;
    emit changed(m_view);
}