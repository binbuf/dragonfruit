// SPDX-License-Identifier: MIT
// Settings privileged-action authorizer tests (T224).
//
// A fake polkit authority on the private session bus records the
// `CheckAuthorization` call and answers allow/deny/cancel/dismissed. This
// proves the Settings app asks the system authority with
// `AllowUserInteraction` (the flag that raises the shell's agent) and never
// handles a password itself.
#include "SettingsAuthorizer.h"

#include <QDBusArgument>
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusMetaType>
#include <QDBusVirtualObject>
#include <QMap>
#include <QMetaType>
#include <QSignalSpy>
#include <QTest>
#include <QVariantMap>

// The reply's `a{ss}` details map. QtDBus has no built-in `QMap<QString,
// QString>` codec, so the test writes the same shape polkit does explicitly.
struct StringMap {
    QMap<QString, QString> values;
};

QDBusArgument &operator<<(QDBusArgument &argument, const StringMap &map)
{
    argument.beginMap(QMetaType::QString, QMetaType::QString);
    for (auto it = map.values.constBegin(); it != map.values.constEnd(); ++it) {
        argument.beginMapEntry();
        argument << it.key() << it.value();
        argument.endMapEntry();
    }
    argument.endMap();
    return argument;
}

const QDBusArgument &operator>>(const QDBusArgument &argument, StringMap &map)
{
    argument.beginMap();
    while (!argument.atEnd()) {
        argument.beginMapEntry();
        QString key;
        QString value;
        argument >> key >> value;
        argument.endMapEntry();
        map.values.insert(key, value);
    }
    argument.endMap();
    return argument;
}

Q_DECLARE_METATYPE(StringMap)

namespace {

const QString kAuthorityService = QStringLiteral("org.freedesktop.PolicyKit1");
const QString kAuthorityPath = QStringLiteral("/org/freedesktop/PolicyKit1/Authority");
const QString kAuthorityIface = QStringLiteral("org.freedesktop.PolicyKit1.Authority");

struct Subject {
    QString kind;
    QVariantMap attributes;
};

QDBusArgument &operator<<(QDBusArgument &argument, const Subject &subject)
{
    argument.beginStructure();
    argument << subject.kind << subject.attributes;
    argument.endStructure();
    return argument;
}

const QDBusArgument &operator>>(const QDBusArgument &argument, Subject &subject)
{
    argument.beginStructure();
    argument >> subject.kind >> subject.attributes;
    argument.endStructure();
    return argument;
}

class FakeAuthority : public QDBusVirtualObject
{
public:
    enum Mode { Allow, Deny, Cancel, Dismissed, DismissedEncoded, Error };

    Mode mode = Allow;
    QString actionId;
    QString subjectKind;
    QVariantMap subjectAttributes;
    uint flags = 0;
    int calls = 0;

    QString introspect(const QString &) const override { return QString(); }

    bool handleMessage(const QDBusMessage &message, const QDBusConnection &connection) override
    {
        if (message.interface() != kAuthorityIface
                || message.member() != QLatin1String("CheckAuthorization"))
            return false;

        ++calls;
        const QList<QVariant> args = message.arguments();
        if (args.size() >= 1 && args.at(0).canConvert<QDBusArgument>()) {
            QDBusArgument argument = args.at(0).value<QDBusArgument>();
            Subject subject;
            argument >> subject;
            subjectKind = subject.kind;
            subjectAttributes = subject.attributes;
        }
        if (args.size() >= 2)
            actionId = args.at(1).toString();
        if (args.size() >= 4)
            flags = args.at(3).toUInt();

        if (mode == Cancel) {
            connection.send(message.createErrorReply(
                QStringLiteral("org.freedesktop.PolicyKit1.Error.Cancelled"),
                QStringLiteral("Authentication cancelled")));
            return true;
        }
        if (mode == Error) {
            connection.send(message.createErrorReply(
                QStringLiteral("org.freedesktop.PolicyKit1.Error.Failed"),
                QStringLiteral("Authority unavailable")));
            return true;
        }

        if (mode == DismissedEncoded) {
            // The real authority returns the `a{ss}` details as a wire map, not
            // a decoded QVariantMap; send that shape so the authorizer's
            // QDBusArgument decode path is exercised.
            StringMap encoded;
            encoded.values.insert(QStringLiteral("polkit.dismissed"),
                                  QStringLiteral("true"));
            connection.send(message.createReply(
                QVariantList{ false, false, QVariant::fromValue(encoded) }));
            return true;
        }

        QVariantMap details;
        if (mode == Dismissed)
            details.insert(QStringLiteral("polkit.dismissed"), QStringLiteral("true"));
        const bool authorized = mode == Allow;
        connection.send(message.createReply(QVariantList{ authorized, false, details }));
        return true;
    }
};

QString sessionBusAddress()
{
    return QString::fromLocal8Bit(qgetenv("DBUS_SESSION_BUS_ADDRESS"));
}

} // namespace

Q_DECLARE_METATYPE(Subject)

class TestSettingsAuth : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase()
    {
        qDBusRegisterMetaType<Subject>();
        qDBusRegisterMetaType<StringMap>();
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        // Point the live authorizer at the private bus and give it a stable
        // session subject; the fake authority does not need a real logind.
        qputenv("DF_SETTINGS_POLKIT_BUS", sessionBusAddress().toLocal8Bit());
        qputenv("DF_SETTINGS_POLKIT_SUBJECT_SESSION", "test-session");
        m_bus = bus;
        QVERIFY(m_bus.registerService(kAuthorityService));
        QVERIFY(m_bus.registerVirtualObject(kAuthorityPath, &m_authority,
                                            QDBusConnection::SubPath));
    }

    void cleanupTestCase()
    {
        m_bus.unregisterObject(kAuthorityPath);
        m_bus.unregisterService(kAuthorityService);
    }

    void allowCallsTheAuthorityWithUserInteraction()
    {
        m_authority.mode = FakeAuthority::Allow;
        SettingsAuthorizer authorizer;
        QSignalSpy finished(&authorizer, &SettingsAuthorizer::finished);
        authorizer.check(QStringLiteral("req-1"),
                         QStringLiteral("org.freedesktop.accounts.user-administration"),
                         QStringLiteral("Authentication is required."));
        QTRY_COMPARE_WITH_TIMEOUT(finished.count(), 1, 5000);
        QCOMPARE(finished.at(0).at(1).toBool(), true);
        QCOMPARE(finished.at(0).at(2).toString(), QString());
        QCOMPARE(m_authority.actionId,
                 QStringLiteral("org.freedesktop.accounts.user-administration"));
        QCOMPARE(m_authority.subjectKind, QStringLiteral("unix-session"));
        QCOMPARE(m_authority.subjectAttributes.value("session-id").toString(),
                 QStringLiteral("test-session"));
        QCOMPARE(m_authority.flags, 1u); // AllowUserInteraction
    }

    void denyIsReportedAsADenial()
    {
        m_authority.mode = FakeAuthority::Deny;
        SettingsAuthorizer authorizer;
        QSignalSpy finished(&authorizer, &SettingsAuthorizer::finished);
        authorizer.check(QStringLiteral("req-2"), QStringLiteral("org.example.admin"),
                         QString());
        QTRY_COMPARE_WITH_TIMEOUT(finished.count(), 1, 5000);
        QCOMPARE(finished.at(0).at(1).toBool(), false);
        QVERIFY(finished.at(0).at(2).toString().contains("denied"));
    }

    void cancelIsReportedAsACancellation()
    {
        m_authority.mode = FakeAuthority::Cancel;
        SettingsAuthorizer authorizer;
        QSignalSpy finished(&authorizer, &SettingsAuthorizer::finished);
        authorizer.check(QStringLiteral("req-3"), QStringLiteral("org.example.admin"),
                         QString());
        QTRY_COMPARE_WITH_TIMEOUT(finished.count(), 1, 5000);
        QCOMPARE(finished.at(0).at(1).toBool(), false);
        QVERIFY(finished.at(0).at(2).toString().contains("cancelled"));
    }

    void aDismissedPromptFailsClosed()
    {
        m_authority.mode = FakeAuthority::Dismissed;
        SettingsAuthorizer authorizer;
        QSignalSpy finished(&authorizer, &SettingsAuthorizer::finished);
        authorizer.check(QStringLiteral("req-4"), QStringLiteral("org.example.admin"),
                         QString());
        QTRY_COMPARE_WITH_TIMEOUT(finished.count(), 1, 5000);
        QCOMPARE(finished.at(0).at(1).toBool(), false);
        QVERIFY(!finished.at(0).at(2).toString().isEmpty());
    }

    // The real authority's `a{ss}` reply is a wire map; the authorizer must
    // decode it to classify a closed prompt as cancelled.
    void anEncodedDismissedDetailsMapIsACancellation()
    {
        m_authority.mode = FakeAuthority::DismissedEncoded;
        SettingsAuthorizer authorizer;
        QSignalSpy finished(&authorizer, &SettingsAuthorizer::finished);
        authorizer.check(QStringLiteral("req-4b"), QStringLiteral("org.example.admin"),
                         QString());
        QTRY_COMPARE_WITH_TIMEOUT(finished.count(), 1, 5000);
        QCOMPARE(finished.at(0).at(1).toBool(), false);
        QVERIFY(finished.at(0).at(2).toString().contains("cancelled"));
    }

    void anAuthorityErrorFailsClosed()
    {
        m_authority.mode = FakeAuthority::Error;
        SettingsAuthorizer authorizer;
        QSignalSpy finished(&authorizer, &SettingsAuthorizer::finished);
        authorizer.check(QStringLiteral("req-5"), QStringLiteral("org.example.admin"),
                         QString());
        QTRY_COMPARE_WITH_TIMEOUT(finished.count(), 1, 5000);
        QCOMPARE(finished.at(0).at(1).toBool(), false);
        QVERIFY(finished.at(0).at(2).toString().contains("Authority unavailable"));
    }

    // The fixture seam covers the wrong-password result without a PAM helper:
    // the pane must revert and explain, never open a password field.
    void wrongPasswordFixtureFailsClosed()
    {
        SettingsAuthorizer authorizer;
        authorizer.setFixture(QStringLiteral("wrong-password"));
        QSignalSpy finished(&authorizer, &SettingsAuthorizer::finished);
        authorizer.check(QStringLiteral("req-6"), QStringLiteral("org.example.admin"),
                         QString());
        QTRY_COMPARE_WITH_TIMEOUT(finished.count(), 1, 5000);
        QCOMPARE(finished.at(0).at(1).toBool(), false);
        QVERIFY(finished.at(0).at(2).toString().contains("incorrect"));
    }

private:
    QDBusConnection m_bus = QDBusConnection(QStringLiteral("none"));
    FakeAuthority m_authority;
};

QTEST_GUILESS_MAIN(TestSettingsAuth)
#include "tst_settings_auth.moc"
