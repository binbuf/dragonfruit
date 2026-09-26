// SPDX-License-Identifier: MIT
// The Dragonfruit polkit authentication agent (T-13.6).
//
// The shell registers this object with the host polkit authority as its
// session authentication agent. When a privileged action needs the user,
// polkit calls `BeginAuthentication` on the agent object; the shell renders
// the prompt in a design-system dialog, and the answer is relayed to the
// host's `polkit-agent-helper-1`, which owns the PAM conversation and reports
// the result directly to the authority. Dragonfruit never verifies a password
// and never escalates privilege itself.
//
// Contract: the agent object is served on the bus the authority lives on (the
// system bus in production, injectable for tests); it registers with the exact
// `(sa{sv})` subject of the shell's session (or a process subject), and it
// unregisters on destruction. If the authority is absent or another agent
// already serves the subject, `start()` reports failure and the desktop
// continues without burning down (a normal degradation).
//
// The class is Wayland-free; the shell controller feeds it key events and
// renders its properties.
#pragma once

#include <QDBusConnection>
#include <QDBusVirtualObject>
#include <QList>
#include <QObject>
#include <QString>
#include <QVariantList>
#include <QVariantMap>

class PolkitHelperSession;

// The D-Bus location of the polkit authority and of the agent object.
namespace dragonfruit {
constexpr const char *kPolkitAuthorityService = "org.freedesktop.PolicyKit1";
constexpr const char *kPolkitAuthorityPath = "/org/freedesktop/PolicyKit1/Authority";
constexpr const char *kPolkitAuthorityInterface = "org.freedesktop.PolicyKit1.Authority";
constexpr const char *kPolkitAgentInterface = "org.freedesktop.PolicyKit1.AuthenticationAgent";
constexpr const char *kPolkitAgentPath = "/org/dragonfruit/PolicyKit1/AuthenticationAgent";
} // namespace dragonfruit

class PolkitAgent : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool registered READ isRegistered NOTIFY changed)
    Q_PROPERTY(bool active READ isActive NOTIFY changed)
    Q_PROPERTY(QString actionId READ actionId NOTIFY changed)
    Q_PROPERTY(QString message READ message NOTIFY changed)
    Q_PROPERTY(QString iconName READ iconName NOTIFY changed)
    Q_PROPERTY(QString identityLabel READ identityLabel NOTIFY changed)
    Q_PROPERTY(QVariantList details READ details NOTIFY changed)
    Q_PROPERTY(QString prompt READ prompt NOTIFY changed)
    Q_PROPERTY(bool promptEcho READ promptEcho NOTIFY changed)
    Q_PROPERTY(QString info READ info NOTIFY changed)
    Q_PROPERTY(QString errorText READ errorText NOTIFY changed)
    Q_PROPERTY(bool busy READ isBusy NOTIFY changed)
    Q_PROPERTY(QString registrationError READ registrationError NOTIFY changed)

public:
    explicit PolkitAgent(QObject *parent = nullptr);
    ~PolkitAgent() override;

    // --- test/override seams -------------------------------------------------
    // Connect to `address` instead of the system bus (a private test bus).
    void setBusAddress(const QString &address);
    // Own this well-known name on the agent's bus (tests only; production
    // agents are addressed by their unique bus name).
    void setServiceName(const QString &name);
    // Register for this session id instead of the one resolved from the
    // environment.
    void setSubjectSessionId(const QString &sessionId);
    // Register for this process (pid + start-time) instead of a session.
    void setSubjectProcess(qint64 pid);
    // Override the helper binary (tests).
    void setHelperPath(const QString &path);

    // Connect, export the agent object, and register with the authority.
    // Idempotent. Returns true when the authority accepted the registration;
    // false is a normal degraded state (no authority, or another agent already
    // serves the subject) and `registrationError()` says why.
    bool start();

    // Present a synthetic request with no D-Bus or helper involved. This is
    // the `DF_POLKIT_FIXTURE` capture seam and a tests' view driver: `respond`
    // and `cancel` resolve it locally.
    void presentLocal(const QString &actionId, const QString &message,
                      const QString &identityLabel, const QVariantList &details);

    // Resolve the username for a unix-user identity. Public so the identity
    // parser can use it; it is a pure utility with no state.
    static QString usernameForUid(uint uid);

    bool isRegistered() const { return m_registered; }
    bool isActive() const { return m_active; }
    bool isBusy() const { return m_busy; }
    QString actionId() const { return m_actionId; }
    QString message() const { return m_message; }
    QString iconName() const { return m_iconName; }
    QString identityLabel() const { return m_identityLabel; }
    QVariantList details() const { return m_details; }
    QString prompt() const { return m_prompt; }
    bool promptEcho() const { return m_promptEcho; }
    QString info() const { return m_info; }
    QString errorText() const { return m_errorText; }
    QString registrationError() const { return m_registrationError; }

public slots:
    // Forward the typed answer to the helper.
    void respond(const QString &response);
    // Decline the active request (the polkit check fails closed).
    void cancel();

signals:
    // A request was accepted and is now presented; the shell maps its dialog.
    void started();
    // The active request resolved. `success` true means the authority gained
    // the authorization; false covers a denied, cancelled, or failed attempt.
    void finished(bool success);
    // Any view property changed.
    void changed();

private:
    friend class PolkitAgentObject;

    struct Request {
        QString actionId;
        QString message;
        QString iconName;
        QString cookie;
        QString username;
        QVariantList details;
    };

    // Handle one `BeginAuthentication` call. Always sends a reply; queues the
    // request when another is already on screen.
    void beginAuthentication(const QDBusMessage &message, const QDBusConnection &connection,
                             const QString &actionId, const QString &messageText,
                             const QString &iconName, const QVariantList &details,
                             const QString &cookie, const QStringList &usernames);
    void cancelAuthentication(const QString &cookie);
    // Present the head of the queue.
    void showNext();
    void onPrompt(const QString &text, bool echo);
    void onInfo(const QString &text);
    void onErrorMessage(const QString &text);
    void onHelperCompleted(bool success);
    // Clear the per-request state.
    void resetRequest();
    // The subject the agent registers for, as `(sa{sv})`.
    QVariant buildSubject() const;

    QDBusVirtualObject *m_object = nullptr;
    PolkitHelperSession *m_session = nullptr;
    QList<Request> m_queue;

    QString m_busAddress;
    QString m_serviceName;
    QString m_sessionId;
    qint64 m_processPid = -1;
    QString m_helperPath;
    QString m_connectionName;
    QString m_locale;
    QVariant m_subject;

    bool m_started = false;
    bool m_registered = false;
    QString m_registrationError;

    bool m_active = false;
    bool m_busy = false;
    bool m_localOnly = false;
    QString m_actionId;
    QString m_message;
    QString m_iconName;
    QString m_identityLabel;
    QVariantList m_details;
    QString m_prompt;
    bool m_promptEcho = false;
    QString m_info;
    QString m_errorText;
    QString m_cookie;
};