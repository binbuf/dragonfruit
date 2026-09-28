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
    void audioDecodesOutputsInputsAndRouting();
    void outcomeOfReadsTheHostReport();

    void theWifiMenuModelExposesOneJoinRowPerNetwork();

    void batteryDecodesAndHidesWhenNotPresent();
    void batteryRefreshRaisesTheRequest();

    void bluetoothDecodesKnownAndNearbyDevices();
    void bluetoothHidesWhenTheDaemonHasNoController();
    void bluetoothRefreshRaisesTheRequest();

    void storageDecodesVolumesAndHidesWhenEmpty();
    void storageRefreshRaisesTheRequest();

    void inputDecodesTheInventoryAndHidesWhenEmpty();
    void inputRefreshRaisesTheRequest();

    void updatesDecodesTheHostStack();
    void updatesHidesOnAnAbsentHost();
    void updatesRefreshAndActionsRaiseTheRequests();

    void accountsDecodesTheUsersAndGroups();
    void accountsHidesOnAnAbsentHost();
    void accountsRefreshRaisesTheRequest();
    void printersDecodesTheQueuesAndScanners();
    void printersHidesOnAnAbsentHostOrAnEmptyHost();
    void printersRefreshRaisesTheRequest();
    void privacyDecodesTheCategories();
    void privacyHidesOnAnAbsentHostOrAnEmptyStore();
    void privacyRefreshRaisesTheRequest();
    void accessibilityDecodesTheBridgeState();
    void accessibilityHidesOnAnAbsentBusOrAnAllOffBus();
    void accessibilityRefreshRaisesTheRequest();
    void vpnDecodesTheConnections();
    void vpnHidesOnAnAbsentDaemonOrAnEmptyStore();
    void vpnRefreshRaisesTheRequest();

    void theAbsentDaemonMaskingMatrixHidesOnlyTheMaskedItem();
    void anUnreachedBridgeHostLeavesEveryItemHidden();
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

void TestStatusModel::audioDecodesOutputsInputsAndRouting()
{
    const QVariantMap view = SystemStatusModel::parseView(
        R"({"kind":"audio","state":"available","glyph":"volume","label":"60%",
            "volume":0.6,"percent":60,"muted":false,"defaultSink":"speakers",
            "defaultSource":"microphone","sinkCount":2,"sourceCount":1,
            "sinks":[{"id":7,"name":"speakers","description":"Built-in Speakers",
                      "volume":0.6,"percent":60,"muted":false,"default":true}],
            "sources":[{"id":20,"name":"microphone",
                        "description":"Built-in Microphone","volume":0.5,
                        "percent":50,"muted":false,"default":true}]})",
        QStringLiteral("audio"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("defaultSource")).toString(),
             QStringLiteral("microphone"));
    QCOMPARE(view.value(QStringLiteral("sourceCount")).toInt(), 1);
    QCOMPARE(view.value(QStringLiteral("sources")).toList().size(), 1);
    QCOMPARE(view.value(QStringLiteral("sources")).toList().at(0).toMap()
                 .value(QStringLiteral("description")).toString(),
             QStringLiteral("Built-in Microphone"));
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

// The battery (T-07.5b) is read-only; it decodes like the other views and
// hides on the second hidden case (`present: false`) as well as absence.
void TestStatusModel::batteryDecodesAndHidesWhenNotPresent()
{
    const QVariantMap view = SystemStatusModel::parseView(
        R"({"kind":"battery","state":"available","present":true,"percent":82,
            "level":0.82,"charging":true,"label":"82% charging"})",
        QStringLiteral("battery"));
    QCOMPARE(view.value(QStringLiteral("state")).toString(), QStringLiteral("available"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("percent")).toInt(), 82);
    QCOMPARE(view.value(QStringLiteral("charging")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("label")).toString(), QStringLiteral("82% charging"));

    // UPower present but no battery: available, yet the item hides.
    const QVariantMap absent = SystemStatusModel::parseView(
        R"({"kind":"battery","state":"available","present":false,"label":"No battery"})",
        QStringLiteral("battery"));
    QCOMPARE(absent.value(QStringLiteral("visible")).toBool(), false);

    // A fixture client's view round-trips through the instance API too.
    SystemStatusModel model;
    model.applyBatteryJson(
        R"({"kind":"battery","state":"available","present":true,"percent":50})");
    QVERIFY(model.batteryVisible());
    QCOMPARE(model.battery().value(QStringLiteral("percent")).toInt(), 50);
}

void TestStatusModel::batteryRefreshRaisesTheRequest()
{
    SystemStatusModel model;
    QSignalSpy spy(&model, &SystemStatusModel::refreshBatteryRequested);
    model.requestRefreshBattery();
    QCOMPARE(spy.count(), 1);
}

// Bluetooth (T-15.1b) decodes like the other views; the `present: false`
// case (a running daemon with no controller) hides the tile exactly as the
// battery hides with no battery.
void TestStatusModel::bluetoothDecodesKnownAndNearbyDevices()
{
    const QVariantMap view = SystemStatusModel::parseView(
        R"({"kind":"bluetooth","state":"available","present":true,"powered":true,
            "discovering":false,"glyph":"bluetooth","label":"Bluetooth connected one",
            "adapterName":"Workstation","connectedCount":1,
            "knownDevices":[{"address":"AA:BB:CC:DD:EE:FF","name":"WF-1000XM6",
                             "paired":true,"connected":true,"signal":64}],
            "nearbyDevices":[{"address":"11:22:33:44:55:66","name":"Speaker",
                              "paired":false,"connected":false,"signal":30}]})",
        QStringLiteral("bluetooth"));
    QCOMPARE(view.value(QStringLiteral("state")).toString(), QStringLiteral("available"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("enabled")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("powered")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("knownDevices")).toList().size(), 1);
    QCOMPARE(view.value(QStringLiteral("knownDevices")).toList().at(0).toMap()
                 .value(QStringLiteral("name")).toString(),
             QStringLiteral("WF-1000XM6"));

    SystemStatusModel model;
    model.applyBluetoothJson(
        R"({"kind":"bluetooth","state":"available","present":true,"powered":false})");
    QVERIFY(model.bluetoothVisible());
    QCOMPARE(model.bluetooth().value(QStringLiteral("powered")).toBool(), false);
}

void TestStatusModel::bluetoothHidesWhenTheDaemonHasNoController()
{
    // A running `bluetoothd` with no controller: available but not present.
    const QVariantMap noController = SystemStatusModel::parseView(
        R"({"kind":"bluetooth","state":"available","present":false,
            "label":"Bluetooth unavailable"})",
        QStringLiteral("bluetooth"));
    QCOMPARE(noController.value(QStringLiteral("state")).toString(),
             QStringLiteral("available"));
    QCOMPARE(noController.value(QStringLiteral("visible")).toBool(), false);

    // The daemon absent altogether.
    const QVariantMap absent = SystemStatusModel::parseView(
        R"({"kind":"bluetooth","state":"unavailable"})", QStringLiteral("bluetooth"));
    QCOMPARE(absent.value(QStringLiteral("visible")).toBool(), false);
}

void TestStatusModel::bluetoothRefreshRaisesTheRequest()
{
    SystemStatusModel model;
    QSignalSpy spy(&model, &SystemStatusModel::refreshBluetoothRequested);
    model.requestRefreshBluetooth();
    QCOMPARE(spy.count(), 1);
}

// Storage (T-15.2b) decodes like the other views; the `present: false` case
// (a running UDisks2 with no mountable volume) hides the tile exactly as the
// battery hides with no battery and Bluetooth with no controller.
void TestStatusModel::storageDecodesVolumesAndHidesWhenEmpty()
{
    const QVariantMap view = SystemStatusModel::parseView(
        R"({"kind":"storage","state":"available","present":true,
            "glyph":"storage","label":"Storage 1 mounted","mountedCount":1,
            "volumeCount":1,"removableCount":1,
            "drives":[{"path":"/drives/usb","name":"Flash Drive","removable":true,
                       "ejectable":true}],
            "volumes":[{"path":"/dev/sdb1","drivePath":"/drives/usb",
                        "name":"Photos","mounted":true,
                        "mountPoint":"/run/media/user/Photos","removable":true}]})",
        QStringLiteral("storage"));
    QCOMPARE(view.value(QStringLiteral("state")).toString(), QStringLiteral("available"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("enabled")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("mountedCount")).toInt(), 1);
    QCOMPARE(view.value(QStringLiteral("volumes")).toList().size(), 1);
    QCOMPARE(view.value(QStringLiteral("volumes")).toList().at(0).toMap()
                 .value(QStringLiteral("name")).toString(),
             QStringLiteral("Photos"));

    SystemStatusModel model;
    model.applyStorageJson(
        R"({"kind":"storage","state":"available","present":true,"mountedCount":1})");
    QVERIFY(model.storageVisible());

    // UDisks2 present but no mountable volume: available, yet the tile hides.
    model.applyStorageJson(
        R"({"kind":"storage","state":"available","present":false,
            "label":"Storage unavailable"})");
    QCOMPARE(model.storageVisible(), false);

    // The daemon absent altogether.
    model.applyStorageJson(R"({"kind":"storage","state":"unavailable"})");
    QCOMPARE(model.storageVisible(), false);
}

void TestStatusModel::storageRefreshRaisesTheRequest()
{
    SystemStatusModel model;
    QSignalSpy spy(&model, &SystemStatusModel::refreshStorageRequested);
    model.requestRefreshStorage();
    QCOMPARE(spy.count(), 1);
}

// Input (T-15.4b) is a read-only inventory; the `present: false` case (a
// running libinput with no recognized device) hides the tile exactly as the
// battery/Bluetooth/Storage second hide rules do.
void TestStatusModel::inputDecodesTheInventoryAndHidesWhenEmpty()
{
    const QVariantMap view = SystemStatusModel::parseView(
        R"({"kind":"input","state":"available","present":true,"glyph":"keyboard",
            "label":"1 keyboards, 2 pointing devices","deviceCount":3,
            "keyboardCount":1,"pointerCount":2,"mouseCount":1,"touchpadCount":1,
            "devices":[{"name":"AT keyboard","kernel":"/dev/input/event3",
                        "kind":"keyboard","kindLabel":"Keyboard"},
                       {"name":"TouchPad","kernel":"/dev/input/event7",
                        "kind":"touchpad","kindLabel":"Trackpad"}]})",
        QStringLiteral("input"));
    QCOMPARE(view.value(QStringLiteral("state")).toString(), QStringLiteral("available"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("enabled")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("touchpadCount")).toInt(), 1);
    QCOMPARE(view.value(QStringLiteral("devices")).toList().size(), 2);
    QCOMPARE(view.value(QStringLiteral("devices")).toList().at(1).toMap()
                 .value(QStringLiteral("kindLabel")).toString(),
             QStringLiteral("Trackpad"));

    SystemStatusModel model;
    model.applyInputJson(
        R"({"kind":"input","state":"available","present":true,"deviceCount":3})");
    QVERIFY(model.inputVisible());

    // libinput present but no recognized device: available, yet the tile hides.
    model.applyInputJson(
        R"({"kind":"input","state":"available","present":false,
            "label":"No input devices"})");
    QCOMPARE(model.inputVisible(), false);

    // libinput absent altogether.
    model.applyInputJson(R"({"kind":"input","state":"unavailable"})");
    QCOMPARE(model.inputVisible(), false);
}

void TestStatusModel::inputRefreshRaisesTheRequest()
{
    SystemStatusModel model;
    QSignalSpy spy(&model, &SystemStatusModel::refreshInputRequested);
    model.requestRefreshInput();
    QCOMPARE(spy.count(), 1);
}

// General/About/Updates (T-15.10b) decodes the host-stack view. The identity
// is always read when the host answers, so the item stays visible even with no
// update provider (`updatesAvailable: false`); only an absent host hides it.
void TestStatusModel::updatesDecodesTheHostStack()
{
    const QVariantMap view = SystemStatusModel::parseView(
        R"({"kind":"updates","state":"available","hostName":"dragon",
            "deviceName":"Dragonfruit Book","osLabel":"Dragonfruit Linux 44",
            "memoryLabel":"16 GB","updatesAvailable":true,"glyph":"software-update",
            "label":"1 Update Available","phase":"available","busy":false,
            "rebootRequired":false,"updateCount":1,"securityCount":1,
            "updates":[{"id":"glibc","name":"glibc","summary":"C library",
                        "currentVersion":"2.40","availableVersion":"2.41",
                        "severity":"security","severityLabel":"Security Update"}]})",
        QStringLiteral("updates"));
    QCOMPARE(view.value(QStringLiteral("state")).toString(), QStringLiteral("available"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("enabled")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("hostName")).toString(), QStringLiteral("dragon"));
    QCOMPARE(view.value(QStringLiteral("updatesAvailable")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("label")).toString(),
             QStringLiteral("1 Update Available"));
    QCOMPARE(view.value(QStringLiteral("updates")).toList().size(), 1);

    SystemStatusModel model;
    model.applyUpdatesJson(
        R"({"kind":"updates","state":"available","hostName":"dragon",
            "updatesAvailable":false,"label":"Software Update Unavailable"})");
    QVERIFY(model.updatesVisible());
    QCOMPARE(model.updates().value(QStringLiteral("updatesAvailable")).toBool(), false);
    QCOMPARE(model.updates().value(QStringLiteral("hostName")).toString(),
             QStringLiteral("dragon"));
}

void TestStatusModel::updatesHidesOnAnAbsentHost()
{
    SystemStatusModel model;
    model.applyUpdatesJson(R"({"kind":"updates","state":"unavailable"})");
    QCOMPARE(model.updatesVisible(), false);
    QCOMPARE(model.updates().value(QStringLiteral("state")).toString(),
             QStringLiteral("unavailable"));

    // A mismatched kind is rejected and leaves the last view in place, never
    // clearing it into a visible error.
    model.applyUpdatesJson(R"({"kind":"storage","state":"available"})");
    QCOMPARE(model.updatesVisible(), false);
    QCOMPARE(model.updates().value(QStringLiteral("state")).toString(),
             QStringLiteral("unavailable"));
}

void TestStatusModel::updatesRefreshAndActionsRaiseTheRequests()
{
    SystemStatusModel model;
    QSignalSpy refreshSpy(&model, &SystemStatusModel::refreshUpdatesRequested);
    QSignalSpy checkSpy(&model, &SystemStatusModel::checkUpdatesRequested);
    QSignalSpy installSpy(&model, &SystemStatusModel::installUpdatesRequested);
    QSignalSpy rebootSpy(&model, &SystemStatusModel::rebootUpdatesRequested);
    model.requestRefreshUpdates();
    model.requestCheckUpdates();
    model.requestInstallUpdates();
    model.requestRebootUpdates();
    QCOMPARE(refreshSpy.count(), 1);
    QCOMPARE(checkSpy.count(), 1);
    QCOMPARE(installSpy.count(), 1);
    QCOMPARE(rebootSpy.count(), 1);
}

// Users and Groups (T-15.11b) decodes the host-stack view. The user list is
// always read when AccountsService answers, so the item stays visible even with
// no group provider (`groupsAvailable: false`); only an absent host hides it.
void TestStatusModel::accountsDecodesTheUsersAndGroups()
{
    const QVariantMap view = SystemStatusModel::parseView(
        R"({"kind":"accounts","state":"available","glyph":"users",
            "label":"2 Users","present":true,"humanCount":2,"adminCount":1,
            "lockedCount":1,"groupsAvailable":true,"groupCount":1,
            "automaticLogin":"Dan Doe","automaticLoginUser":"dan",
            "automaticLoginUid":1000,
            "users":[{"uid":1000,"userName":"dan","displayName":"Dan Doe",
                      "initial":"D","accountType":"administrator",
                      "accountTypeLabel":"Admin","locked":false,"system":false,
                      "automaticLogin":true},
                     {"uid":1001,"userName":"sam","displayName":"Sam Smith",
                      "initial":"S","accountType":"standard",
                      "accountTypeLabel":"Standard","locked":true,"system":false}],
            "groups":[{"name":"wheel","gid":10,"memberCount":1,
                       "members":["dan"],"system":false}]})",
        QStringLiteral("accounts"));
    QCOMPARE(view.value(QStringLiteral("state")).toString(), QStringLiteral("available"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("enabled")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("glyph")).toString(), QStringLiteral("users"));
    QCOMPARE(view.value(QStringLiteral("label")).toString(), QStringLiteral("2 Users"));
    QCOMPARE(view.value(QStringLiteral("humanCount")).toInt(), 2);
    QCOMPARE(view.value(QStringLiteral("automaticLoginUser")).toString(),
             QStringLiteral("dan"));
    QCOMPARE(view.value(QStringLiteral("users")).toList().size(), 2);
    QCOMPARE(view.value(QStringLiteral("groups")).toList().size(), 1);

    SystemStatusModel model;
    // A host with no group provider stays visible: only the Settings pane's
    // group controls disable.
    model.applyAccountsJson(
        R"({"kind":"accounts","state":"available","label":"1 User",
            "groupsAvailable":false,"humanCount":1})");
    QVERIFY(model.accountsVisible());
    QCOMPARE(model.accounts().value(QStringLiteral("groupsAvailable")).toBool(), false);
    QCOMPARE(model.accounts().value(QStringLiteral("label")).toString(),
             QStringLiteral("1 User"));
}

void TestStatusModel::accountsHidesOnAnAbsentHost()
{
    SystemStatusModel model;
    model.applyAccountsJson(R"({"kind":"accounts","state":"unavailable"})");
    QCOMPARE(model.accountsVisible(), false);
    QCOMPARE(model.accounts().value(QStringLiteral("state")).toString(),
             QStringLiteral("unavailable"));

    // A mismatched kind is rejected and leaves the last view in place.
    model.applyAccountsJson(R"({"kind":"updates","state":"available"})");
    QCOMPARE(model.accountsVisible(), false);
    QCOMPARE(model.accounts().value(QStringLiteral("state")).toString(),
             QStringLiteral("unavailable"));
}

void TestStatusModel::accountsRefreshRaisesTheRequest()
{
    SystemStatusModel model;
    QSignalSpy refreshSpy(&model, &SystemStatusModel::refreshAccountsRequested);
    model.requestRefreshAccounts();
    QCOMPARE(refreshSpy.count(), 1);
}

// Printers and Scanners (T-15.12b) decodes the bridge host's CUPS/SANE view.
// The item hides when the host answers with no queue and no scanner
// (`present: false`); only a present queue or scanner keeps it visible.
void TestStatusModel::printersDecodesTheQueuesAndScanners()
{
    const QVariantMap view = SystemStatusModel::parseView(
        R"({"kind":"printers","state":"available","glyph":"printer",
            "label":"2 Printers, 1 Scanner","present":true,
            "printersAvailable":true,"scannersAvailable":true,
            "printerCount":2,"scannerCount":1,"queuedJobCount":1,
            "defaultPrinter":"Canon_MF230",
            "printers":[{"name":"Canon_MF230","displayName":"Canon MF230",
                         "state":"idle","stateLabel":"Idle",
                         "stateMessage":"Idle, Last Used","isDefault":true,
                         "acceptingJobs":true,"jobCount":0,"jobs":[]}],
            "scanners":[{"device":"epson2:net:192.168.0.7",
                         "description":"Epson GT-1500 flatbed scanner",
                         "kind":"flatbed","kindLabel":"Flatbed"}]})",
        QStringLiteral("printers"));
    QCOMPARE(view.value(QStringLiteral("state")).toString(), QStringLiteral("available"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("enabled")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("glyph")).toString(), QStringLiteral("printer"));
    QCOMPARE(view.value(QStringLiteral("label")).toString(),
             QStringLiteral("2 Printers, 1 Scanner"));
    QCOMPARE(view.value(QStringLiteral("defaultPrinter")).toString(),
             QStringLiteral("Canon_MF230"));
    QCOMPARE(view.value(QStringLiteral("printers")).toList().size(), 1);
    QCOMPARE(view.value(QStringLiteral("scanners")).toList().size(), 1);

    SystemStatusModel model;
    // A host with one half absent keeps the other half live and stays visible.
    model.applyPrintersJson(
        R"({"kind":"printers","state":"available","printersAvailable":true,
            "scannersAvailable":false,"present":true,"printerCount":1,
            "scannerCount":0})");
    QVERIFY(model.printersVisible());
    QCOMPARE(model.printers().value(QStringLiteral("scannersAvailable")).toBool(), false);
}

void TestStatusModel::printersHidesOnAnAbsentHostOrAnEmptyHost()
{
    SystemStatusModel model;
    model.applyPrintersJson(R"({"kind":"printers","state":"unavailable"})");
    QCOMPARE(model.printersVisible(), false);
    QCOMPARE(model.printers().value(QStringLiteral("state")).toString(),
             QStringLiteral("unavailable"));

    // A running host with no queue and no scanner is `available` with
    // `present: false`; the second hide rule hides the tile.
    model.applyPrintersJson(
        R"({"kind":"printers","state":"available","present":false,
            "label":"No Printers or Scanners","printerCount":0,"scannerCount":0})");
    QCOMPARE(model.printersVisible(), false);

    // A mismatched kind is rejected and leaves the last view in place.
    model.applyPrintersJson(R"({"kind":"accounts","state":"available"})");
    QCOMPARE(model.printers().value(QStringLiteral("state")).toString(),
             QStringLiteral("available"));
}

void TestStatusModel::printersRefreshRaisesTheRequest()
{
    SystemStatusModel model;
    QSignalSpy refreshSpy(&model, &SystemStatusModel::refreshPrintersRequested);
    model.requestRefreshPrinters();
    QCOMPARE(refreshSpy.count(), 1);
}

// Privacy and Security (T-15.13b) decodes the bridge host's portal
// PermissionStore view. The item hides when the host answers with no
// application permission (`present: false`).
void TestStatusModel::privacyDecodesTheCategories()
{
    const QVariantMap view = SystemStatusModel::parseView(
        R"({"kind":"privacy","state":"available","glyph":"privacy",
            "label":"3 Apps","present":true,"appCount":3,
            "grantedCount":1,"deniedCount":1,"categoryCount":14,
            "categories":[{"id":"devices","label":"Camera","summary":"2 apps",
                           "appCount":2,"grantedCount":1,
                           "resources":[{"id":"camera","appCount":2,
                             "apps":[{"app":"org.example.Snapshot","state":"denied",
                                      "stateLabel":"Denied","permissions":["no"]},
                                     {"app":"org.mozilla.firefox","state":"allowed",
                                      "stateLabel":"Allowed","permissions":["yes"]}]}]}]})",
        QStringLiteral("privacy"));
    QCOMPARE(view.value(QStringLiteral("state")).toString(), QStringLiteral("available"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("enabled")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("glyph")).toString(), QStringLiteral("privacy"));
    QCOMPARE(view.value(QStringLiteral("label")).toString(), QStringLiteral("3 Apps"));
    QCOMPARE(view.value(QStringLiteral("appCount")).toInt(), 3);
    QCOMPARE(view.value(QStringLiteral("categories")).toList().size(), 1);

    SystemStatusModel model;
    model.applyPrivacyJson(
        R"({"kind":"privacy","state":"available","present":true,"appCount":2,
            "categories":[{"id":"devices","label":"Camera","summary":"2 apps"}]})");
    QVERIFY(model.privacyVisible());
    QCOMPARE(model.privacy().value(QStringLiteral("appCount")).toInt(), 2);
}

void TestStatusModel::privacyHidesOnAnAbsentHostOrAnEmptyStore()
{
    SystemStatusModel model;
    model.applyPrivacyJson(R"({"kind":"privacy","state":"unavailable"})");
    QCOMPARE(model.privacyVisible(), false);
    QCOMPARE(model.privacy().value(QStringLiteral("state")).toString(),
             QStringLiteral("unavailable"));

    // A running store with no application permission is `available` with
    // `present: false`; the second hide rule hides the tile.
    model.applyPrivacyJson(
        R"({"kind":"privacy","state":"available","present":false,
            "label":"No App Permissions","appCount":0,"categoryCount":14})");
    QCOMPARE(model.privacyVisible(), false);

    // A mismatched kind is rejected and leaves the last view in place.
    model.applyPrivacyJson(R"({"kind":"printers","state":"available"})");
    QCOMPARE(model.privacy().value(QStringLiteral("state")).toString(),
             QStringLiteral("available"));
}

void TestStatusModel::privacyRefreshRaisesTheRequest()
{
    SystemStatusModel model;
    QSignalSpy refreshSpy(&model, &SystemStatusModel::refreshPrivacyRequested);
    model.requestRefreshPrivacy();
    QCOMPARE(refreshSpy.count(), 1);
}

// Accessibility (T-15.14b) decodes the bridge host's live AT-SPI view. The item
// hides when the bus answers with every feature off (`present: false`).
void TestStatusModel::accessibilityDecodesTheBridgeState()
{
    const QVariantMap view = SystemStatusModel::parseView(
        R"({"kind":"accessibility","state":"available","glyph":"accessibility",
            "label":"Screen Reader On","present":true,"enabled":true,
            "enabledLabel":"On","screenReader":true,"screenReaderLabel":"On"})",
        QStringLiteral("accessibility"));
    QCOMPARE(view.value(QStringLiteral("state")).toString(), QStringLiteral("available"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("enabled")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("glyph")).toString(),
             QStringLiteral("accessibility"));
    QCOMPARE(view.value(QStringLiteral("label")).toString(),
             QStringLiteral("Screen Reader On"));
    QCOMPARE(view.value(QStringLiteral("screenReader")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("screenReaderLabel")).toString(),
             QStringLiteral("On"));

    SystemStatusModel model;
    model.applyAccessibilityJson(
        R"({"kind":"accessibility","state":"available","present":true,
            "enabled":true,"screenReader":false,"label":"On"})");
    QVERIFY(model.accessibilityVisible());
    QCOMPARE(model.accessibility().value(QStringLiteral("label")).toString(),
             QStringLiteral("On"));
}

void TestStatusModel::accessibilityHidesOnAnAbsentBusOrAnAllOffBus()
{
    SystemStatusModel model;
    model.applyAccessibilityJson(R"({"kind":"accessibility","state":"unavailable"})");
    QCOMPARE(model.accessibilityVisible(), false);
    QCOMPARE(model.accessibility().value(QStringLiteral("state")).toString(),
             QStringLiteral("unavailable"));

    // A bus that answers with every feature off is `available` with
    // `present: false`; the second hide rule hides the tile.
    model.applyAccessibilityJson(
        R"({"kind":"accessibility","state":"available","present":false,
            "label":"Off","enabled":false,"screenReader":false})");
    QCOMPARE(model.accessibilityVisible(), false);

    // A mismatched kind is rejected and leaves the last view in place.
    model.applyAccessibilityJson(R"({"kind":"input","state":"available"})");
    QCOMPARE(model.accessibility().value(QStringLiteral("state")).toString(),
             QStringLiteral("available"));
}

void TestStatusModel::accessibilityRefreshRaisesTheRequest()
{
    SystemStatusModel model;
    QSignalSpy refreshSpy(&model, &SystemStatusModel::refreshAccessibilityRequested);
    model.requestRefreshAccessibility();
    QCOMPARE(refreshSpy.count(), 1);
}

// Network advanced (VPN) (T-15.15b) decodes the bridge host's NetworkManager
// view. The item hides when the daemon answers with no VPN configured
// (`present: false`).
void TestStatusModel::vpnDecodesTheConnections()
{
    const QVariantMap view = SystemStatusModel::parseView(
        R"({"kind":"vpn","state":"available","glyph":"vpn","label":"Work VPN",
            "present":true,"connectionCount":2,"connectedCount":1,
            "activeUuid":"aaa","activeName":"Work VPN",
            "connections":[
              {"id":"Work VPN","uuid":"aaa","kind":"openvpn",
               "state":"connected","connected":true,"label":"OpenVPN \u00b7 Connected"},
              {"id":"Home","uuid":"bbb","kind":"wireguard",
               "state":"disconnected","connected":false,"label":"WireGuard \u00b7 Disconnected"}]})",
        QStringLiteral("vpn"));
    QCOMPARE(view.value(QStringLiteral("state")).toString(), QStringLiteral("available"));
    QCOMPARE(view.value(QStringLiteral("visible")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("enabled")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("glyph")).toString(), QStringLiteral("vpn"));
    QCOMPARE(view.value(QStringLiteral("label")).toString(), QStringLiteral("Work VPN"));
    QCOMPARE(view.value(QStringLiteral("connectionCount")).toInt(), 2);
    QCOMPARE(view.value(QStringLiteral("connectedCount")).toInt(), 1);

    SystemStatusModel model;
    model.applyVpnJson(
        R"({"kind":"vpn","state":"available","present":true,"glyph":"vpn",
            "label":"Work VPN","connectedCount":1})");
    QVERIFY(model.vpnVisible());
    QCOMPARE(model.vpn().value(QStringLiteral("label")).toString(),
             QStringLiteral("Work VPN"));
}

void TestStatusModel::vpnHidesOnAnAbsentDaemonOrAnEmptyStore()
{
    SystemStatusModel model;
    model.applyVpnJson(R"({"kind":"vpn","state":"unavailable"})");
    QCOMPARE(model.vpnVisible(), false);
    QCOMPARE(model.vpn().value(QStringLiteral("state")).toString(),
             QStringLiteral("unavailable"));

    // A daemon that answers with no VPN configured is `available` with
    // `present: false`; the second hide rule hides the tile.
    model.applyVpnJson(
        R"({"kind":"vpn","state":"available","present":false,"label":"No VPN",
            "connectionCount":0,"connectedCount":0})");
    QCOMPARE(model.vpnVisible(), false);

    // A mismatched kind is rejected and leaves the last view in place.
    model.applyVpnJson(R"({"kind":"input","state":"available"})");
    QCOMPARE(model.vpn().value(QStringLiteral("state")).toString(),
             QStringLiteral("available"));
}

void TestStatusModel::vpnRefreshRaisesTheRequest()
{
    SystemStatusModel model;
    QSignalSpy refreshSpy(&model, &SystemStatusModel::refreshVpnRequested);
    model.requestRefreshVpn();
    QCOMPARE(refreshSpy.count(), 1);
}

// T-07.6a: the absent-daemon masking matrix at the decode seam. Masking one
// daemon must hide only its own item; the other two stay live. Mask all three
// and every item hides without an error.
void TestStatusModel::theAbsentDaemonMaskingMatrixHidesOnlyTheMaskedItem()
{
    SystemStatusModel model;
    const QByteArray wifi = availableWifi();
    const QByteArray audio =
        R"({"kind":"audio","state":"available","glyph":"volume","volume":0.5,"percent":50,"muted":false})";
    const QByteArray battery =
        R"({"kind":"battery","state":"available","present":true,"percent":82,"level":0.82,"charging":true,"label":"82% charging"})";
    const QByteArray maskedWifi = R"({"kind":"wifi","state":"unavailable"})";
    const QByteArray maskedAudio = R"({"kind":"audio","state":"unavailable"})";
    const QByteArray maskedBattery = R"({"kind":"battery","state":"unavailable"})";

    model.applyWifiJson(wifi);
    model.applyAudioJson(audio);
    model.applyBatteryJson(battery);
    QVERIFY(model.wifiVisible());
    QVERIFY(model.audioVisible());
    QVERIFY(model.batteryVisible());

    // Mask Wi-Fi alone: the volume and battery slots stay live.
    model.applyWifiJson(maskedWifi);
    QCOMPARE(model.wifiVisible(), false);
    QCOMPARE(model.audioVisible(), true);
    QCOMPARE(model.batteryVisible(), true);
    QCOMPARE(model.wifi().value(QStringLiteral("enabled")).toBool(), false);
    model.applyWifiJson(wifi);

    // Mask audio alone: the Wi-Fi and battery slots stay live.
    model.applyAudioJson(maskedAudio);
    QCOMPARE(model.wifiVisible(), true);
    QCOMPARE(model.audioVisible(), false);
    QCOMPARE(model.batteryVisible(), true);
    model.applyAudioJson(audio);

    // Mask power alone: the Wi-Fi and volume slots stay live.
    model.applyBatteryJson(maskedBattery);
    QCOMPARE(model.wifiVisible(), true);
    QCOMPARE(model.audioVisible(), true);
    QCOMPARE(model.batteryVisible(), false);
    model.applyBatteryJson(battery);

    // Mask all three: every item hides, and none of them errors.
    model.applyWifiJson(maskedWifi);
    model.applyAudioJson(maskedAudio);
    model.applyBatteryJson(maskedBattery);
    QCOMPARE(model.wifiVisible(), false);
    QCOMPARE(model.audioVisible(), false);
    QCOMPARE(model.batteryVisible(), false);
    QCOMPARE(model.wifi().value(QStringLiteral("state")).toString(),
             QStringLiteral("unavailable"));
    QCOMPARE(model.audio().value(QStringLiteral("state")).toString(),
             QStringLiteral("unavailable"));
    QCOMPARE(model.battery().value(QStringLiteral("state")).toString(),
             QStringLiteral("unavailable"));
    QVERIFY(!model.wifi().contains(QStringLiteral("error")));
    QVERIFY(!model.audio().contains(QStringLiteral("error")));
    QVERIFY(!model.battery().contains(QStringLiteral("error")));
}

// When the bridge host is not on the bus (or never answers), the model has no
// views at all: every item is hidden and the session carries on. A malformed
// or empty payload must not clear a slot into a visible error state.
void TestStatusModel::anUnreachedBridgeHostLeavesEveryItemHidden()
{
    SystemStatusModel model;
    QCOMPARE(model.wifiVisible(), false);
    QCOMPARE(model.audioVisible(), false);
    QCOMPARE(model.batteryVisible(), false);

    // An empty or malformed payload is ignored: the item stays hidden and the
    // view stays empty rather than becoming a visible "error".
    model.applyWifiJson(QByteArray());
    model.applyAudioJson(QByteArray("not json"));
    model.applyBatteryJson(QByteArray("{}"));
    QCOMPARE(model.wifiVisible(), false);
    QCOMPARE(model.audioVisible(), false);
    QCOMPARE(model.batteryVisible(), false);
    QVERIFY(model.wifi().isEmpty());
    QVERIFY(model.audio().isEmpty());
    QVERIFY(model.battery().isEmpty());
}

QTEST_MAIN(TestStatusModel)
#include "tst_statusmodel.moc"