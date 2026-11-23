// SPDX-License-Identifier: MIT
#include "brightnessbackend.h"

#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDebug>
#include <QFile>
#include <QFileInfo>
#include <QtGlobal>

namespace {

constexpr const char *kLogindService = "org.freedesktop.login1";
constexpr const char *kLogindSession = "org.freedesktop.login1.Session";
constexpr const char *kBacklightSubsystem = "backlight";
constexpr int kDefaultTimeoutMs = 1500;

} // namespace

BrightnessBackend::BrightnessBackend(QObject *parent)
    : QObject(parent)
{
}

BrightnessBackend::~BrightnessBackend()
{
    if (!m_connectionName.isEmpty())
        QDBusConnection::disconnectFromBus(m_connectionName);
}

void BrightnessBackend::setBusAddress(const QString &address)
{
    if (m_busAddress == address)
        return;
    if (!m_connectionName.isEmpty()) {
        QDBusConnection::disconnectFromBus(m_connectionName);
        m_connectionName.clear();
    }
    m_busAddress = address;
}

void BrightnessBackend::setSysfsRoot(const QString &root)
{
    if (m_sysfsRoot == root)
        return;
    m_sysfsRoot = root;
}

void BrightnessBackend::setSessionPath(const QString &path)
{
    m_sessionPath = path;
}

void BrightnessBackend::setTimeoutMs(int milliseconds)
{
    m_timeoutMs = qBound(1, milliseconds, 30'000);
}

void BrightnessBackend::setSysfsWriteEnabled(bool enabled)
{
    m_sysfsWriteEnabled = enabled;
}

QDBusConnection BrightnessBackend::bus()
{
    if (m_busAddress.isEmpty())
        return QDBusConnection::systemBus();
    if (m_connectionName.isEmpty()) {
        m_connectionName =
            QStringLiteral("dragonfruit-brightness-%1")
                .arg(reinterpret_cast<quintptr>(this), 0, 16);
    }
    return QDBusConnection::connectToBus(m_busAddress, m_connectionName);
}

bool BrightnessBackend::logindAvailable()
{
    const QDBusConnection connection = bus();
    if (!connection.isConnected())
        return false;
    QDBusConnectionInterface *interface = connection.interface();
    return interface
           && interface->isServiceRegistered(QString::fromLatin1(kLogindService));
}

bool BrightnessBackend::probe()
{
    m_device = resolveBacklightDevice(m_sysfsRoot);
    m_logind = false;
    m_sysfsWritable = false;
    m_absenceReason.clear();

    if (!m_device.isValid()) {
        m_absenceReason =
            QStringLiteral("no backlight device under %1; the brightness "
                           "level is stored but the panel cannot change")
                .arg(m_sysfsRoot);
        m_available = false;
        return false;
    }

    m_logind = logindAvailable();
    m_sysfsWritable = m_sysfsWriteEnabled
        && QFileInfo(m_device.path + QStringLiteral("/brightness")).isWritable();
    if (!m_logind && !m_sysfsWritable) {
        m_absenceReason =
            QStringLiteral("logind is not available and %1/brightness is not "
                           "writable; the brightness level is stored but the "
                           "panel cannot change")
                .arg(m_device.path);
        m_available = false;
        return false;
    }

    m_available = true;
    return true;
}

bool BrightnessBackend::writeSysfs(int value)
{
    QFile file(m_device.path + QStringLiteral("/brightness"));
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text))
        return false;
    const QByteArray payload = QByteArray::number(value) + '\n';
    const bool ok = file.write(payload) == payload.size();
    file.close();
    return ok;
}

bool BrightnessBackend::setLevel(double level)
{
    const double clamped = qBound(0.0, level, 1.0);
    if (!probe()) {
        qWarning() << "shell: brightness:" << m_absenceReason;
        emit failed(m_absenceReason);
        return false;
    }

    const int mapped = mapBrightnessLevel(clamped, m_device.maxBrightness);
    m_lastLevel = clamped;
    m_lastMapped = mapped;

    if (m_logind) {
        QDBusMessage call = QDBusMessage::createMethodCall(
            QString::fromLatin1(kLogindService), m_sessionPath,
            QString::fromLatin1(kLogindSession),
            QStringLiteral("SetBrightness"));
        call << QString::fromLatin1(kBacklightSubsystem) << m_device.name
             << static_cast<uint>(mapped);

        m_lastMethod = QStringLiteral("logind");
        const QString method = m_lastMethod;
        auto *watcher =
            new QDBusPendingCallWatcher(bus().asyncCall(call, m_timeoutMs), this);
        connect(watcher, &QDBusPendingCallWatcher::finished, this,
                [this, watcher, clamped, mapped, method]() {
                    const QDBusPendingReply<> reply = *watcher;
                    watcher->deleteLater();
                    if (reply.isError()) {
                        const QString reason =
                            QStringLiteral("logind SetBrightness failed: %1")
                                .arg(reply.error().message());
                        qWarning() << "shell: brightness:" << reason;
                        emit failed(reason);
                        return;
                    }
                    emit applied(clamped, mapped, method);
                });
        return true;
    }

    // logind is absent: only write sysfs when the file is actually writable.
    // The task's normal absent path is a logged no-op, not a silent claim.
    if (!m_sysfsWritable || !writeSysfs(mapped)) {
        const QString reason =
            QStringLiteral("could not write %1/brightness; the brightness "
                           "level is stored but the panel cannot change")
                .arg(m_device.path);
        qWarning() << "shell: brightness:" << reason;
        emit failed(reason);
        return false;
    }

    m_lastMethod = QStringLiteral("sysfs");
    emit applied(clamped, mapped, m_lastMethod);
    return true;
}
