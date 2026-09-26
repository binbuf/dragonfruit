// SPDX-License-Identifier: MIT
// Lock-auth seam tests (T-12.3b): the shell's side of the helper process
// boundary. No PAM, Wayland, or QML: a fake helper script returns the
// success/denied/error exit statuses the real helper uses, and the class must
// map them to `succeeded` / `failed` and never run two attempts at once.
#include <QtTest>

#include <QFileInfo>
#include <QSignalSpy>
#include <QTemporaryDir>

#include "lockauth.h"

class TestLockAuth : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase();
    void helperPathOverrideWins();
    void successSignalsOnZero();
    void deniedSignalsIncorrectPassword();
    void unavailableSignalsOnOtherExit();
    void busyRunRefusesASecondAttempt();
    void missingHelperFailsToStart();

private:
    QString makeHelper(const QString &name, const QString &body);
    QTemporaryDir m_dir;
};

void TestLockAuth::initTestCase()
{
    QVERIFY2(m_dir.isValid(), "temporary directory for the fake helpers");
}

QString TestLockAuth::makeHelper(const QString &name, const QString &body)
{
    const QString path = m_dir.filePath(name);
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text))
        return QString();
    file.write(body.toUtf8());
    file.close();
    QFile::setPermissions(path, QFileDevice::ReadOwner | QFileDevice::WriteOwner
                                    | QFileDevice::ExeOwner | QFileDevice::ReadGroup
                                    | QFileDevice::ExeGroup | QFileDevice::ReadOther
                                    | QFileDevice::ExeOther);
    return path;
}

void TestLockAuth::helperPathOverrideWins()
{
    const QString path = makeHelper(QStringLiteral("override.sh"),
                                    QStringLiteral("#!/bin/sh\nexit 0\n"));
    QVERIFY(!path.isEmpty());
    qputenv("DF_PAM_HELPER", path.toLocal8Bit());
    QCOMPARE(LockAuthenticator::resolveHelperPath(), QFileInfo(path).absoluteFilePath());
    qunsetenv("DF_PAM_HELPER");
}

void TestLockAuth::successSignalsOnZero()
{
    const QString path = makeHelper(
        QStringLiteral("auth.sh"),
        QStringLiteral("#!/bin/sh\n"
                       "read password\n"
                       "case \"$password\" in\n"
                       "  good) exit 0 ;;\n"
                       "  bad) exit 1 ;;\n"
                       "  *) exit 2 ;;\n"
                       "esac\n"));
    QVERIFY(!path.isEmpty());

    LockAuthenticator authenticator;
    authenticator.setHelperPath(path);
    QSignalSpy succeeded(&authenticator, &LockAuthenticator::succeeded);
    QSignalSpy failed(&authenticator, &LockAuthenticator::failed);
    QVERIFY(authenticator.authenticate(QStringLiteral("alice"), QStringLiteral("good")));
    QTRY_COMPARE(succeeded.count(), 1);
    QCOMPARE(failed.count(), 0);
}

void TestLockAuth::deniedSignalsIncorrectPassword()
{
    const QString path = makeHelper(
        QStringLiteral("deny.sh"),
        QStringLiteral("#!/bin/sh\nread password\nexit 1\n"));
    QVERIFY(!path.isEmpty());

    LockAuthenticator authenticator;
    authenticator.setHelperPath(path);
    QSignalSpy failed(&authenticator, &LockAuthenticator::failed);
    QVERIFY(authenticator.authenticate(QStringLiteral("alice"), QStringLiteral("wrong")));
    QTRY_COMPARE(failed.count(), 1);
    QCOMPARE(failed.takeFirst().at(0).toString(), QStringLiteral("Incorrect password"));
}

void TestLockAuth::unavailableSignalsOnOtherExit()
{
    const QString path = makeHelper(
        QStringLiteral("error.sh"),
        QStringLiteral("#!/bin/sh\nread password\nexit 7\n"));
    QVERIFY(!path.isEmpty());

    LockAuthenticator authenticator;
    authenticator.setHelperPath(path);
    QSignalSpy failed(&authenticator, &LockAuthenticator::failed);
    QVERIFY(authenticator.authenticate(QStringLiteral("alice"), QStringLiteral("x")));
    QTRY_COMPARE(failed.count(), 1);
    QCOMPARE(failed.takeFirst().at(0).toString(), QStringLiteral("Authentication unavailable"));
}

void TestLockAuth::busyRunRefusesASecondAttempt()
{
    const QString path = makeHelper(
        QStringLiteral("slow.sh"),
        QStringLiteral("#!/bin/sh\nread password\nsleep 1\nexit 0\n"));
    QVERIFY(!path.isEmpty());

    LockAuthenticator authenticator;
    authenticator.setHelperPath(path);
    QVERIFY(authenticator.authenticate(QStringLiteral("alice"), QStringLiteral("any")));
    QVERIFY(authenticator.isBusy());
    // One attempt at a time: the second call is refused, not queued.
    QVERIFY(!authenticator.authenticate(QStringLiteral("alice"), QStringLiteral("any")));
    QTRY_VERIFY_WITH_TIMEOUT(!authenticator.isBusy(), 5000);
}

void TestLockAuth::missingHelperFailsToStart()
{
    LockAuthenticator authenticator;
    authenticator.setHelperPath(m_dir.filePath(QStringLiteral("does-not-exist")));
    QSignalSpy failed(&authenticator, &LockAuthenticator::failed);
    QVERIFY(!authenticator.authenticate(QStringLiteral("alice"), QStringLiteral("x")));
    QTRY_COMPARE(failed.count(), 1);
    QCOMPARE(failed.takeFirst().at(0).toString(), QStringLiteral("Authentication unavailable"));
}

QTEST_GUILESS_MAIN(TestLockAuth)
#include "tst_lockauth.moc"