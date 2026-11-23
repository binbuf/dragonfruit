// SPDX-License-Identifier: MIT
#include "sessionactions.h"

#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDebug>
#include <QFileInfo>
#include <QProcess>
#include <QStandardPaths>
#include <QStringList>

namespace {

constexpr const char *kLogindService = "org.freedesktop.login1";
constexpr const char *kLogindPath = "/org/freedesktop/login1";
constexpr const char *kLogindManager = "org.freedesktop.login1.Manager";
constexpr const char *kSessionTarget = "dragonfruit-session.target";
constexpr const char *kTestConnection = "dragonfruit-sessionactions-test";

} // namespace

SessionActions::SessionActions(QObject *parent)
    : QObject(parent)
{
}

void SessionActions::setBusAddress(const QString &address)
{
    if (m_busAddress == address)
        return;
    if (!m_busAddress.isEmpty())
        QDBusConnection::disconnectFromBus(QString::fromLatin1(kTestConnection));
    m_busAddress = address;
}

void SessionActions::setSessionId(const QString &sessionId)
{
    m_sessionId = sessionId;
}

void SessionActions::setSystemctlPath(const QString &path)
{
    m_systemctlPath = path;
}

QDBusConnection SessionActions::bus() const
{
    if (m_busAddress.isEmpty())
        return QDBusConnection::systemBus();
    return QDBusConnection::connectToBus(m_busAddress,
                                         QString::fromLatin1(kTestConnection));
}

QString SessionActions::sessionId() const
{
    return m_sessionId.isEmpty() ? qEnvironmentVariable("XDG_SESSION_ID")
                                 : m_sessionId;
}

QString SessionActions::systemctlPath() const
{
    if (m_systemctlPath.isEmpty())
        return QStandardPaths::findExecutable(QStringLiteral("systemctl"));
    return QFileInfo(m_systemctlPath).isExecutable() ? m_systemctlPath : QString();
}

bool SessionActions::noLogoutFixture()
{
    const QString value = qEnvironmentVariable("DRAGONFRUIT_NO_LOGOUT")
                              .trimmed().toLower();
    return !value.isEmpty() && value != QLatin1String("0")
           && value != QLatin1String("false") && value != QLatin1String("no");
}

bool SessionActions::logindAvailable()
{
    const QDBusConnection connection = bus();
    if (!connection.isConnected())
        return false;
    QDBusConnectionInterface *interface = connection.interface();
    return interface
           && interface->isServiceRegistered(QString::fromLatin1(kLogindService));
}

QString SessionActions::unavailableReason()
{
    if (noLogoutFixture())
        return QString();
    if (sessionId().isEmpty()) {
        return QStringLiteral(
            "No session id is available (XDG_SESSION_ID is unset).");
    }
    if (logindAvailable())
        return QString();
    if (!systemctlPath().isEmpty())
        return QString();
    return QStringLiteral(
        "logind is not available and systemctl could not be found.");
}

bool SessionActions::requestLogout()
{
    const bool fixture = noLogoutFixture();
    QString id = sessionId();
    if (id.isEmpty() && fixture)
        id = QStringLiteral("fixture-session");
    if (id.isEmpty()) {
        const QString reason = QStringLiteral("XDG_SESSION_ID is not set; "
                                              "cannot log out.");
        qWarning() << "shell:" << reason;
        emit failed(reason);
        return false;
    }

    m_lastSessionId = id;
    m_lastWasFixture = fixture;

    if (fixture) {
        // Test/capture only: record the exact production call without touching
        // the host bus. This is the guard that keeps a test from ending the
        // developer's session.
        m_lastMethod = QStringLiteral("TerminateSession");
        qInfo() << "shell: log out suppressed by DRAGONFRUIT_NO_LOGOUT"
                << "(would call" << m_lastMethod << id << ")";
        emit dispatched(m_lastMethod, m_lastSessionId);
        return true;
    }

    if (logindAvailable()) {
        QDBusMessage call = QDBusMessage::createMethodCall(
            QString::fromLatin1(kLogindService),
            QString::fromLatin1(kLogindPath),
            QString::fromLatin1(kLogindManager),
            QStringLiteral("TerminateSession"));
        // `TerminateSession` takes only the session id. The interactive flag
        // belongs to the power methods (Reboot/PowerOff/Suspend); the shell's
        // confirmation dialog is what makes this request interactive.
        call << id;
        m_lastMethod = QStringLiteral("TerminateSession");
        auto *watcher = new QDBusPendingCallWatcher(bus().asyncCall(call), this);
        connect(watcher, &QDBusPendingCallWatcher::finished, this,
                [this](QDBusPendingCallWatcher *call) {
                    const QDBusPendingReply<> reply = *call;
                    if (reply.isError()) {
                        const QString reason =
                            QStringLiteral("logind TerminateSession failed: %1")
                                .arg(reply.error().message());
                        qWarning() << "shell:" << reason;
                        emit failed(reason);
                    }
                    call->deleteLater();
                });
        qInfo() << "shell: log out via logind TerminateSession" << id;
        emit dispatched(m_lastMethod, m_lastSessionId);
        return true;
    }

    const QString systemctl = systemctlPath();
    if (systemctl.isEmpty()) {
        const QString reason = QStringLiteral(
            "logind is not available and systemctl could not be found.");
        qWarning() << "shell:" << reason;
        emit failed(reason);
        return false;
    }

    // No logind: stop our own user target. The session entry then sees the
    // compositor anchor leave and returns to the greeter. This is the only
    // fallback; the shell never signals the compositor or itself directly.
    m_lastMethod = QStringLiteral("systemctl --user stop %1")
                       .arg(QString::fromLatin1(kSessionTarget));
    const bool started = QProcess::startDetached(
        systemctl,
        {QStringLiteral("--user"), QStringLiteral("stop"),
         QString::fromLatin1(kSessionTarget)});
    if (!started) {
        const QString reason = QStringLiteral(
            "could not start systemctl to stop %1")
                                   .arg(QString::fromLatin1(kSessionTarget));
        qWarning() << "shell:" << reason;
        emit failed(reason);
        return false;
    }
    qInfo() << "shell: log out via" << m_lastMethod;
    emit dispatched(m_lastMethod, m_lastSessionId);
    return true;
}
