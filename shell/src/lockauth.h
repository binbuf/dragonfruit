// SPDX-License-Identifier: MIT
// Lock-screen PAM authentication (T-12.3b).
//
// The lock UI never verifies a password itself and keeps no credential store:
// this class is the shell's thin side of the process boundary to
// `dragonfruit-pam-helper`, which runs the host PAM stack. The password is
// written to the helper's standard input and then cleared from this process;
// only the helper's exit status comes back.
//
// Kept Wayland-free so `tst_lockauth` can drive it with a fake helper script.
#pragma once

#include <QObject>
#include <QString>

class QProcess;

class LockAuthenticator : public QObject
{
    Q_OBJECT

public:
    explicit LockAuthenticator(QObject *parent = nullptr);
    ~LockAuthenticator() override;

    // Locate `dragonfruit-pam-helper`: `DF_PAM_HELPER`, then `PATH`, then a
    // sibling of the shell binary / the dev target directory relative to the
    // working directory. Empty when not found.
    static QString resolveHelperPath();

    // Override the helper binary (tests, packaged layouts). Empty restores
    // `resolveHelperPath()`.
    void setHelperPath(const QString &path);
    QString helperPath() const { return m_helperPath; }

    // Override the PAM service. Empty lets the helper use its default.
    void setService(const QString &service);
    QString service() const { return m_service; }

    bool isBusy() const { return m_busy; }

    // Verify `user`'s `password`. The value is cleared from this process after
    // it is written to the helper. Returns false when a run is already in
    // flight or the helper cannot be started; on completion `succeeded()` or
    // `failed(message)` is emitted.
    bool authenticate(const QString &user, QString password);

    // Abort an in-flight authentication. No signal is emitted.
    void cancel();

signals:
    void succeeded();
    void failed(const QString &message);

private:
    void onFinished(int exitCode);
    void clearProcess();

    QProcess *m_process = nullptr;
    QString m_helperPath;
    QString m_service;
    bool m_busy = false;
};