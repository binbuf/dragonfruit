// SPDX-License-Identifier: MIT
#include "lockauth.h"

#include <QCoreApplication>
#include <QFileInfo>
#include <QProcess>
#include <QStandardPaths>

LockAuthenticator::LockAuthenticator(QObject *parent)
    : QObject(parent)
{
}

LockAuthenticator::~LockAuthenticator()
{
    if (m_process && m_process->state() != QProcess::NotRunning) {
        m_process->kill();
        m_process->waitForFinished(500);
    }
}

QString LockAuthenticator::resolveHelperPath()
{
    const QByteArray override = qgetenv("DF_PAM_HELPER");
    if (!override.isEmpty()) {
        const QString candidate = QString::fromLocal8Bit(override);
        if (QFileInfo::exists(candidate))
            return QFileInfo(candidate).absoluteFilePath();
    }

    const QString onPath =
        QStandardPaths::findExecutable(QStringLiteral("dragonfruit-pam-helper"));
    if (!onPath.isEmpty())
        return onPath;

    // The dev tree keeps the helper in the cargo target directory; a packaged
    // install puts it on PATH. Check the shell's own neighbours so an
    // uninstalled `make demo` works without exporting anything.
    const QString appDir = QCoreApplication::applicationDirPath();
    const QStringList candidates = {
        appDir + QStringLiteral("/dragonfruit-pam-helper"),
        appDir + QStringLiteral("/../../../target/debug/dragonfruit-pam-helper"),
        appDir + QStringLiteral("/../../../target/release/dragonfruit-pam-helper"),
        QStringLiteral("target/debug/dragonfruit-pam-helper"),
        QStringLiteral("target/release/dragonfruit-pam-helper"),
    };
    for (const QString &candidate : candidates) {
        if (QFileInfo::exists(candidate))
            return QFileInfo(candidate).absoluteFilePath();
    }
    return QString();
}

void LockAuthenticator::setHelperPath(const QString &path)
{
    m_helperPath = path;
}

void LockAuthenticator::setService(const QString &service)
{
    m_service = service;
}

void LockAuthenticator::clearProcess()
{
    m_busy = false;
    if (!m_process)
        return;
    m_process->deleteLater();
    m_process = nullptr;
}

bool LockAuthenticator::authenticate(const QString &user, QString password)
{
    if (m_busy)
        return false;

    const QString helper = m_helperPath.isEmpty() ? resolveHelperPath() : m_helperPath;
    if (helper.isEmpty())
        return false;

    m_process = new QProcess(this);
    m_process->setProcessChannelMode(QProcess::SeparateChannels);
    connect(m_process, &QProcess::finished, this,
            [this](int exitCode, QProcess::ExitStatus) { onFinished(exitCode); });
    connect(m_process, &QProcess::errorOccurred, this,
            [this](QProcess::ProcessError error) {
                if (error == QProcess::FailedToStart && m_process) {
                    clearProcess();
                    emit failed(QStringLiteral("Authentication unavailable"));
                }
            });

    QStringList arguments;
    arguments << QStringLiteral("--user") << user;
    if (!m_service.isEmpty())
        arguments << QStringLiteral("--service") << m_service;

    m_busy = true;
    m_process->start(helper, arguments);
    if (!m_process->waitForStarted(2000)) {
        // `errorOccurred` already reported FailedToStart; a timeout is the
        // remaining start failure and gets the same message.
        const bool alreadyReported = m_process == nullptr;
        clearProcess();
        if (!alreadyReported)
            emit failed(QStringLiteral("Authentication unavailable"));
        return false;
    }

    QByteArray input = password.toUtf8();
    // Never keep the password in this process once it is in the pipe.
    password.fill(QChar(0));
    input.append('\n');
    m_process->write(input);
    input.fill('\0');
    m_process->closeWriteChannel();
    return true;
}

void LockAuthenticator::onFinished(int exitCode)
{
    clearProcess();
    switch (exitCode) {
    case 0:
        emit succeeded();
        break;
    case 1:
        emit failed(QStringLiteral("Incorrect password"));
        break;
    default:
        emit failed(QStringLiteral("Authentication unavailable"));
        break;
    }
}

void LockAuthenticator::cancel()
{
    if (m_process && m_process->state() != QProcess::NotRunning) {
        m_process->kill();
        m_process->waitForFinished(500);
    }
    clearProcess();
}