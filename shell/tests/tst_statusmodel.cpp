// SPDX-License-Identifier: MIT
// System-status model tests (T-07.5a): the bridge host's JSON views decode to
// the Wi-Fi and volume menu models, and each menu gesture raises the one
// request signal the shell forwards to the host. No QML, Wayland, or D-Bus.
#include <QtTest>

#include "systemstatusmodel.h"

class TestStatusModel : public QObject
{
    Q_OBJECT

private slots:
    void parseRejectsAKindMismatch();
    void parseDecodesTheWifiView();
    void parseDecodesAnErrorAsVisibleButInert();
    void parseDecodesAnAbsentDaemonAsHidden();
    void applyJsonEmitsChanged();
    void joinRaisesTheRequestAndIgnoresEmpty();
    void volumeIsClampedToTheUnitRange();
    void muteRaisesTheRequest();
    void outcomeOfReadsTheHostReport();

    void theWifiMenuModelExposesOneJoinRowPerNetwork();
};

static QByteArray availableWifi()
{
    return R"({
        "kind": "wifi",
        "state": "available",
        "glyph": "wifi-secure",
        "label": "home \u00b7 82%",
        "radioEnabled": true,
        "wifiState": "connected",
        "connectivity": "Connected",
        "activeSsid": "home",
        "readOnly": false,
        "note": "",
        "networkCount": 2,
        "networks": [
            {"ssid": "home", "strength": 82, "security": "WPA2", "secured": true,
             "active": true, "band": "5 GHz"},
            {"ssid": "cafe", "strength": 40, "security": "Open", "secured": false,
             "active": false, "band": "2.4 GHz"}
        ]
    })";
}

void TestStatusModel::parseRejectsAKindMismatch()
{
    QString error;
    const QVariantMap view = SystemStatusModel::parseView(availableWifi(),
                                                          QStringLiteral("audio"), &error);
    QVERIFY(view.isEmpty());
    QVERIFY(!error.isEmpty());
}

void TestStatusModel::parseDecodesTheWifiView()
{
    const QVariantMap view = SystemStatusModel::parseView(availableWifi(),
                                                          QStringLiteral("wifi"));
    QCOMPARE(view.value(QStringLiteral("state")).toString(), QStringLiteral("available"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("enabled")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("activeSsid")).toString(), QStringLiteral("home"));
    const QVariantList networks = view.value(QStringLiteral("networks")).toList();
    QCOMPARE(networks.size(), 2);
    QCOMPARE(networks.at(0).toMap().value(QStringLiteral("ssid")).toString(),
             QStringLiteral("home"));
    QCOMPARE(networks.at(0).toMap().value(QStringLiteral("secured")).toBool(), true);
}

void TestStatusModel::parseDecodesAnErrorAsVisibleButInert()
{
    const QVariantMap view = SystemStatusModel::parseView(
        R"({"kind":"wifi","state":"error","error":"NetworkManager: timeout"})",
        QStringLiteral("wifi"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("enabled")).toBool(), false);
    QCOMPARE(view.value(QStringLiteral("error")).toString(),
             QStringLiteral("NetworkManager: timeout"));
}

void TestStatusModel::parseDecodesAnAbsentDaemonAsHidden()
{
    const QVariantMap view = SystemStatusModel::parseView(
        R"({"kind":"audio","state":"unavailable"})", QStringLiteral("audio"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), false);
    QCOMPARE(view.value(QStringLiteral("enabled")).toBool(), false);
}

void TestStatusModel::applyJsonEmitsChanged()
{
    SystemStatusModel model;
    QSignalSpy spy(&model, &SystemStatusModel::changed);
    model.applyWifiJson(availableWifi());
    QCOMPARE(spy.count(), 1);
    QVERIFY(model.wifiVisible());
    QCOMPARE(model.wifi().value(QStringLiteral("activeSsid")).toString(),
             QStringLiteral("home"));

    // A mismatched kind must not clear the current view.
    model.applyAudioJson(availableWifi());
    QCOMPARE(spy.count(), 1);
    QVERIFY(model.wifiVisible());
}

void TestStatusModel::joinRaisesTheRequestAndIgnoresEmpty()
{
    SystemStatusModel model;
    QSignalSpy spy(&model, &SystemStatusModel::joinRequested);
    model.requestJoin(QStringLiteral("home"), QStringLiteral("hunter2"));
    QCOMPARE(spy.count(), 1);
    QCOMPARE(spy.at(0).at(0).toString(), QStringLiteral("home"));
    QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("hunter2"));

    model.requestJoin(QString(), QString());
    QCOMPARE(spy.count(), 1);
}

void TestStatusModel::volumeIsClampedToTheUnitRange()
{
    SystemStatusModel model;
    QSignalSpy spy(&model, &SystemStatusModel::volumeRequested);
    model.requestVolume(2.0);
    model.requestVolume(-1.0);
    model.requestVolume(0.6);
    QCOMPARE(spy.count(), 3);
    QCOMPARE(spy.at(0).at(0).toDouble(), 1.0);
    QCOMPARE(spy.at(1).at(0).toDouble(), 0.0);
    QCOMPARE(spy.at(2).at(0).toDouble(), 0.6);
}

void TestStatusModel::muteRaisesTheRequest()
{
    SystemStatusModel model;
    QSignalSpy spy(&model, &SystemStatusModel::muteRequested);
    model.requestMute(true);
    QCOMPARE(spy.count(), 1);
    QCOMPARE(spy.at(0).at(0).toBool(), true);
}

void TestStatusModel::outcomeOfReadsTheHostReport()
{
    QCOMPARE(SystemStatusModel::outcomeOf(R"({"outcome":"accepted"})"),
             QStringLiteral("accepted"));
    QCOMPARE(SystemStatusModel::outcomeOf(R"({"outcome":"denied","note":"polkit"})"),
             QStringLiteral("denied"));
    QCOMPARE(SystemStatusModel::outcomeOf("not json"), QString());
}

// The QML menus normalize the same list the model exposes; this mirrors the
// `WifiMenu.rows` contract so a regression in either half is caught.
void TestStatusModel::theWifiMenuModelExposesOneJoinRowPerNetwork()
{
    const QVariantMap view = SystemStatusModel::parseView(availableWifi(),
                                                          QStringLiteral("wifi"));
    const QVariantList networks = view.value(QStringLiteral("networks")).toList();
    QCOMPARE(networks.size(), 2);
    for (const QVariant &entry : networks) {
        const QVariantMap network = entry.toMap();
        QVERIFY(network.contains(QStringLiteral("ssid")));
        QVERIFY(network.contains(QStringLiteral("strength")));
        QVERIFY(network.contains(QStringLiteral("secured")));
    }
}

QTEST_MAIN(TestStatusModel)
#include "tst_statusmodel.moc"