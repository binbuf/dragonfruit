// SPDX-License-Identifier: MIT
#include "polkitsession.h"

#include <QFileInfo>
#include <QLocalSocket>
#include <QProcess>

namespace {

// Reverse of GLib's `g_strescape`, which the helper uses to encode each PAM
// line so a message can never contain a literal newline. Handles the common
// escapes and up to three octal digits (`\NNN`). A trailing lone backslash is
// dropped, matching `g_strcompress`.
QString unescapeCString(const QString &input)
{
    QString output;
    output.reserve(input.size());
    for (int i = 0; i < input.size(); ++i) {
        const QChar ch = input.at(i);
        if (ch != QLatin1Char('\\')) {
            output.append(ch);
            continue;
        }
        if (++i >= input.size())
            break;
        const QChar escaped = input.at(i);
        switch (escaped.unicode()) {
        case 'n':
            output.append(QLatin1Char('\n'));
            break;
        case 't':
            output.append(QLatin1Char('\t'));
            break;
        case 'r':
            output.append(QLatin1Char('\r'));
            break;
        case 'b':
            output.append(QLatin1Char('\b'));
            break;
        case 'f':
            output.append(QLatin1Char('\f'));
            break;
        case 'v':
            output.append(QLatin1Char('\v'));
            break;
        case '\\':
            output.append(QLatin1Char('\\'));
            break;
        case '"':
            output.append(QLatin1Char('"'));
            break;
        case '\'':
            output.append(QLatin1Char('\''));
            break;
        default:
            if (escaped >= QLatin1Char('0') && escaped <= QLatin1Char('7')) {
                int value = escaped.digitValue();
                for (int digits = 0; digits < 2 && i + 1 < input.size(); ++digits) {
                    const QChar next = input.at(i + 1);
                    if (next < QLatin1Char('0') || next > QLatin1Char('7'))
                        break;
                    value = value * 8 + next.digitValue();
                    ++i;
                }
                output.append(QChar(value & 0xff));
            } else {
                output.append(escaped);
            }
            break;
        }
    }
    return output;
}

} // namespace

PolkitHelperSession::PolkitHelperSession(QObject *parent)
    : QObject(parent)
{
}

PolkitHelperSession::~PolkitHelperSession()
{
    cleanup();
}

QString PolkitHelperSession::resolveHelperPath()
{
    const QByteArray override = qgetenv("DF_POLKIT_HELPER");
    if (!override.isEmpty()) {
        const QString candidate = QString::fromLocal8Bit(override);
        if (QFileInfo::exists(candidate))
            return QFileInfo(candidate).absoluteFilePath();
    }
    const QStringList candidates = {
        QStringLiteral("/usr/lib/polkit-1/polkit-agent-helper-1"),
        QStringLiteral("/usr/libexec/polkit-agent-helper-1"),
    };
    for (const QString &candidate : candidates) {
        if (QFileInfo::exists(candidate))
            return candidate;
    }
    return QString();
}

QString PolkitHelperSession::resolveHelperSocketPath()
{
    const QByteArray override = qgetenv("DF_POLKIT_HELPER_SOCKET");
    if (!override.isEmpty())
        return QString::fromLocal8Bit(override);
    const QString path = QStringLiteral("/run/polkit/agent-helper.socket");
    return QFileInfo::exists(path) ? path : QString();
}

void PolkitHelperSession::setHelperPath(const QString &path)
{
    m_helperPath = path;
}

void PolkitHelperSession::setSocketPath(const QString &path)
{
    m_socketPath = path;
}

bool PolkitHelperSession::start(const QString &username, const QString &cookie)
{
    if (m_running)
        return false;

    m_completed = false;
    m_buffer.clear();

    // Prefer the non-setuid helper socket when present (polkit 127+); fall
    // back to the setuid helper process. Tests point the socket at an absent
    // path to exercise the spawn path with a fake helper.
    QString socket = m_socketPath;
    if (socket.isEmpty())
        socket = resolveHelperSocketPath();
    m_useSocket = !socket.isEmpty() && QFileInfo::exists(socket);

    if (m_useSocket) {
        m_socket = new QLocalSocket(this);
        connect(m_socket, &QLocalSocket::readyRead, this, &PolkitHelperSession::onReadyRead);
        connect(m_socket, &QLocalSocket::errorOccurred, this,
                &PolkitHelperSession::onSocketError);
        m_running = true;
        m_socket->connectToServer(socket);
        if (!m_socket->waitForConnected(2000)) {
            cleanup();
            return false;
        }
        QByteArray handshake = username.toUtf8();
        handshake.append('\n');
        m_socket->write(handshake);
        QByteArray cookieLine = cookie.toUtf8();
        cookieLine.append('\n');
        m_socket->write(cookieLine);
        m_socket->flush();
        return true;
    }

    const QString helper = m_helperPath.isEmpty() ? resolveHelperPath() : m_helperPath;
    if (helper.isEmpty())
        return false;

    m_process = new QProcess(this);
    m_process->setProcessChannelMode(QProcess::SeparateChannels);
    connect(m_process, &QProcess::readyReadStandardOutput, this, &PolkitHelperSession::onReadyRead);
    connect(m_process, &QProcess::finished, this,
            [this](int exitCode, QProcess::ExitStatus) { onProcessFinished(exitCode); });
    connect(m_process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart && m_process) {
            finish(false);
        }
    });

    m_running = true;
    // The helper takes the username as its one argument and the cookie on
    // standard input (so it is never visible in the process list).
    m_process->start(helper, QStringList{ username });
    if (!m_process->waitForStarted(2000)) {
        finish(false);
        return false;
    }
    QByteArray cookieLine = cookie.toUtf8();
    cookieLine.append('\n');
    m_process->write(cookieLine);
    return true;
}

bool PolkitHelperSession::respond(const QString &response)
{
    if (!m_running)
        return false;
    QByteArray data = response.toUtf8();
    if (data.isEmpty() || !data.endsWith('\n'))
        data.append('\n');
    bool ok = false;
    if (m_useSocket && m_socket) {
        ok = m_socket->write(data) == data.size();
        m_socket->flush();
    } else if (m_process) {
        ok = m_process->write(data) == data.size();
    }
    data.fill('\0');
    return ok;
}

void PolkitHelperSession::cancel()
{
    if (m_running)
        finish(false);
}

void PolkitHelperSession::cleanup()
{
    // Mark not-running first and detach the signals, so a `finished` emitted
    // while we kill/reap cannot re-enter `finish` -> `cleanup`.
    m_running = false;
    if (m_process) {
        m_process->disconnect(this);
        if (m_process->state() != QProcess::NotRunning) {
            m_process->kill();
            m_process->waitForFinished(500);
        }
        m_process->deleteLater();
        m_process = nullptr;
    }
    if (m_socket) {
        m_socket->disconnect(this);
        m_socket->abort();
        m_socket->deleteLater();
        m_socket = nullptr;
    }
}

void PolkitHelperSession::finish(bool success)
{
    if (m_completed) {
        cleanup();
        return;
    }
    m_completed = true;
    cleanup();
    emit completed(success);
}

void PolkitHelperSession::onReadyRead()
{
    if (m_useSocket && m_socket)
        m_buffer.append(m_socket->readAll());
    else if (m_process)
        m_buffer.append(m_process->readAllStandardOutput());

    int newline = -1;
    while ((newline = m_buffer.indexOf('\n')) >= 0) {
        const QByteArray raw = m_buffer.left(newline);
        m_buffer.remove(0, newline + 1);
        handleLine(unescapeCString(QString::fromUtf8(raw)));
        if (m_completed)
            return;
    }
}

void PolkitHelperSession::onProcessFinished(int exitCode)
{
    Q_UNUSED(exitCode);
    // The helper reports SUCCESS/FAILURE on stdout; reaching EOF first is a
    // failed conversation (a crash or a closed pipe). Either way it is closed.
    if (m_running)
        finish(false);
}

void PolkitHelperSession::onSocketError()
{
    if (m_running)
        finish(false);
}

void PolkitHelperSession::handleLine(const QString &line)
{
    if (line.startsWith(QLatin1String("PAM_PROMPT_ECHO_OFF "))) {
        emit prompt(line.mid(sizeof("PAM_PROMPT_ECHO_OFF ") - 1), false);
    } else if (line.startsWith(QLatin1String("PAM_PROMPT_ECHO_ON "))) {
        emit prompt(line.mid(sizeof("PAM_PROMPT_ECHO_ON ") - 1), true);
    } else if (line.startsWith(QLatin1String("PAM_ERROR_MSG "))) {
        emit errorMessage(line.mid(sizeof("PAM_ERROR_MSG ") - 1));
    } else if (line.startsWith(QLatin1String("PAM_TEXT_INFO "))) {
        emit info(line.mid(sizeof("PAM_TEXT_INFO ") - 1));
    } else if (line.startsWith(QLatin1String("SUCCESS"))) {
        finish(true);
    } else if (line.startsWith(QLatin1String("FAILURE"))) {
        finish(false);
    }
    // Unknown lines are ignored: a newer helper may add lines a older agent
    // does not know, and the conversation still ends with SUCCESS/FAILURE.
}