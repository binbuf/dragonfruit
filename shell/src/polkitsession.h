// SPDX-License-Identifier: MIT
// The polkit authentication-helper conversation (T-13.6).
//
// A polkit authentication agent never verifies a password itself. It spawns
// the host's setuid helper (`polkit-agent-helper-1`), which owns the PAM
// conversation and reports the result back to the polkit authority. This class
// is the shell's thin side of that process boundary: it starts the helper,
// writes the authority's cookie, relays the helper's PAM lines
// (`PAM_PROMPT_ECHO_OFF` / `PAM_PROMPT_ECHO_ON` / `PAM_TEXT_INFO` /
// `PAM_ERROR_MSG`), forwards the user's response, and reports
// `SUCCESS`/`FAILURE`.
//
// Newer polkit drops the setuid helper and serves
// `/run/polkit/agent-helper.socket` instead; when that socket exists this class
// connects to it (a `QLocalSocket`) and uses the same line protocol, preceded
// by the username. Both transports share one `handleLine` path.
//
// Kept Wayland-free and credential-free: the response is only ever written to
// the helper, never logged or persisted.
#pragma once

#include <QByteArray>
#include <QObject>
#include <QString>

class QLocalSocket;
class QProcess;

class PolkitHelperSession : public QObject
{
    Q_OBJECT

public:
    explicit PolkitHelperSession(QObject *parent = nullptr);
    ~PolkitHelperSession() override;

    // Locate the helper: `DF_POLKIT_HELPER`, then the distribution paths
    // (`/usr/lib/polkit-1/polkit-agent-helper-1`,
    // `/usr/libexec/polkit-agent-helper-1`). Empty when not found.
    static QString resolveHelperPath();
    // The non-setuid helper socket polkit 127+ serves. Empty when absent.
    static QString resolveHelperSocketPath();

    // Override the helper binary (tests, packaged layouts). Empty restores
    // `resolveHelperPath()`.
    void setHelperPath(const QString &path);
    QString helperPath() const { return m_helperPath; }
    // Override the helper socket path. Empty restores
    // `resolveHelperSocketPath()`; a non-empty path that does not exist forces
    // the spawn path (tests use this to keep the fake helper in play).
    void setSocketPath(const QString &path);
    QString socketPath() const { return m_socketPath; }

    bool isRunning() const { return m_running; }

    // Begin the conversation for `username` with the authority's `cookie`.
    // Neither value is retained beyond the call. Returns false when a run is
    // already in flight or the helper cannot be started; on completion
    // `completed(success)` is emitted exactly once.
    bool start(const QString &username, const QString &cookie);

    // Forward one PAM response (typically the password). Cleared after it is
    // written. Returns false when no conversation is in flight.
    bool respond(const QString &response);

    // Abort the conversation. Emits `completed(false)` if one was running.
    void cancel();

signals:
    // A PAM question; `echo` false means the answer must be masked.
    void prompt(const QString &text, bool echo);
    // PAM informational/error text to show beside the prompt.
    void info(const QString &text);
    void errorMessage(const QString &text);
    void completed(bool success);

private:
    void onReadyRead();
    void onProcessFinished(int exitCode);
    void onSocketError();
    void handleLine(const QString &line);
    void finish(bool success);
    void cleanup();

    QProcess *m_process = nullptr;
    QLocalSocket *m_socket = nullptr;
    QByteArray m_buffer;
    QString m_helperPath;
    QString m_socketPath;
    bool m_useSocket = false;
    bool m_running = false;
    bool m_completed = false;
};