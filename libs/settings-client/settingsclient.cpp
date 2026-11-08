// SPDX-License-Identifier: MIT
#include "settingsclient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>
#include <QFileInfo>
#include <QMetaType>
#include <QTimer>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.Settings1");
const QString kPath = QStringLiteral("/org/dragonfruit/Settings1");
const QString kInterface = QStringLiteral("org.dragonfruit.Settings1");

// Unwrap the D-Bus variant wrapper Qt adds to `v` and `a{sv}` payloads.
QVariant unwrapVariant(const QVariant &value)
{
    if (value.userType() == qMetaTypeId<QDBusVariant>())
        return value.value<QDBusVariant>().variant();
    return value;
}

} // namespace

SettingsClient::SettingsClient(QObject *parent)
    : QObject(parent)
{
    // The schema defaults make the shell self-sufficient when no daemon is on
    // the bus (the dev tool does not start settingsd). A `GetAll` snapshot
    // overrides them once the daemon answers.
    m_values = settingsSchemaDefaults();
}

SettingsClient::~SettingsClient() = default;

QVariant SettingsClient::value(const QString &key, const QVariant &fallback) const
{
    const auto it = m_values.constFind(key);
    return it == m_values.constEnd() ? fallback : it.value();
}

bool SettingsClient::boolean(const QString &key, bool fallback) const
{
    const QVariant v = value(key, fallback);
    return v.canConvert<bool>() ? v.toBool() : fallback;
}

double SettingsClient::real(const QString &key, double fallback) const
{
    const QVariant v = value(key, fallback);
    return v.canConvert<double>() ? v.toDouble() : fallback;
}

qlonglong SettingsClient::integer(const QString &key, qlonglong fallback) const
{
    const QVariant v = value(key, fallback);
    return v.canConvert<qlonglong>() ? v.toLongLong() : fallback;
}

QString SettingsClient::string(const QString &key, const QString &fallback) const
{
    const QVariant v = value(key, fallback);
    return v.canConvert<QString>() ? v.toString() : fallback;
}

QStringList SettingsClient::stringList(const QString &key) const
{
    return value(key).toStringList();
}

QVariant coerceToSchema(const QString &key, const QVariant &value)
{
    // QML has one number type (double), so a control bound to an integer key
    // (`input.repeatRate`) writes a double. Coerce it back to the schema's
    // `x`/`d` type here, before the value is stored or sent over D-Bus, so the
    // daemon's type check accepts it and the reactive map keeps its type.
    static const QVariantMap defaults = settingsSchemaDefaults();
    const auto it = defaults.constFind(key);
    if (it == defaults.constEnd())
        return value;
    const QVariant &declared = it.value();
    if (declared.typeId() == QMetaType::LongLong
            && value.typeId() == QMetaType::Double)
        return QVariant::fromValue<qlonglong>(qRound64(value.toDouble()));
    if (declared.typeId() == QMetaType::Double
            && value.typeId() == QMetaType::LongLong)
        return value.toDouble();
    if (declared.typeId() == QMetaType::QStringList
            && value.typeId() == QMetaType::QVariantList) {
        // QML passes a JS array as QVariantList; the schema/D-Bus type is
        // `as`, so normalize every element to a string.
        QStringList list;
        const QVariantList items = value.toList();
        for (const QVariant &item : items)
            list.append(item.toString());
        return list;
    }
    return value;
}

void SettingsClient::applyValue(const QString &key, const QVariant &value)
{
    const QVariant coerced = coerceToSchema(key, value);
    const auto it = m_values.constFind(key);
    if (it != m_values.constEnd() && it.value() == coerced)
        return;
    m_values.insert(key, coerced);
    emit changed(key, coerced);
}

void SettingsClient::applyValues(const QVariantMap &values)
{
    for (auto it = values.constBegin(); it != values.constEnd(); ++it)
        applyValue(it.key(), it.value());
}

void SettingsClient::setAvailable(bool available)
{
    if (m_available == available)
        return;
    m_available = available;
    emit availableChanged(available);
}

QVariantMap settingsSchemaDefaults()
{
    // Mirrors services/settingsd/src/schema.rs (SCHEMA_VERSION 19). Values are
    // typed exactly as the schema declares: d, x, b, s, as.
    QVariantMap values;
    values.insert(QStringLiteral("dock.size"), 0.5);
    values.insert(QStringLiteral("dock.magnification"), 0.5);
    values.insert(QStringLiteral("dock.position"), QStringLiteral("bottom"));
    values.insert(QStringLiteral("dock.autohide"), false);
    values.insert(QStringLiteral("dock.animateOpening"), true);
    values.insert(QStringLiteral("dock.showIndicators"), true);
    values.insert(QStringLiteral("dock.minimizeIntoTileIcon"), false);
    values.insert(QStringLiteral("dock.minimizedAnimation"), QStringLiteral("scale"));
    values.insert(QStringLiteral("dock.titlebarDoubleClick"), QStringLiteral("zoom"));
    values.insert(QStringLiteral("dock.showRecentApps"), false);
    values.insert(QStringLiteral("dock.chooserOnHover"), false);
    values.insert(QStringLiteral("dock.minimizeReaction"), false);
    values.insert(QStringLiteral("dock.pinned"), QStringList());
    values.insert(QStringLiteral("workspaces.count"), qlonglong(3));
    values.insert(QStringLiteral("gestures.enabled"), true);
    values.insert(QStringLiteral("gestures.spaceSwitch"), true);
    values.insert(QStringLiteral("gestures.missionControl"), true);
    values.insert(QStringLiteral("appearance.colorScheme"), QStringLiteral("auto"));
    values.insert(QStringLiteral("appearance.accent"), QString());
    values.insert(QStringLiteral("appearance.transparency"), 0.5255);
    values.insert(QStringLiteral("wallpaper.source"), QString());
    values.insert(QStringLiteral("wallpaper.fit"), QStringLiteral("fill"));
    values.insert(QStringLiteral("wallpaper.showOnAllSpaces"), true);
    values.insert(QStringLiteral("wallpaper.provider"), QStringLiteral("wikimedia"));
    values.insert(QStringLiteral("wallpaper.providerAutoFetch"), true);
    values.insert(QStringLiteral("wallpaper.providerLastFetch"), qlonglong(0));
    values.insert(QStringLiteral("wallpaper.providerSource"), QString());
    values.insert(QStringLiteral("wallpaper.builtinDefault"), QString());
    values.insert(QStringLiteral("display.scale"), 1.0);
    values.insert(QStringLiteral("display.rotation"), QStringLiteral("normal"));
    values.insert(QStringLiteral("display.brightness"), 1.0);
    values.insert(QStringLiteral("accessibility.reduceMotion"), false);
    values.insert(QStringLiteral("accessibility.reduceTransparency"), false);
    values.insert(QStringLiteral("accessibility.increaseContrast"), false);
    values.insert(QStringLiteral("input.repeatDelay"), qlonglong(200));
    values.insert(QStringLiteral("input.repeatRate"), qlonglong(25));
    values.insert(QStringLiteral("input.pointerSpeed"), 0.0);
    values.insert(QStringLiteral("input.naturalScroll"), true);
    values.insert(QStringLiteral("input.tapToClick"), true);
    values.insert(QStringLiteral("input.leftHanded"), false);
    values.insert(QStringLiteral("input.scrollMethod"), QStringLiteral("two-finger"));
    values.insert(QStringLiteral("input.keyboardBrightness"), 0.5);
    values.insert(QStringLiteral("input.adjustBrightnessLowLight"), true);
    values.insert(QStringLiteral("input.backlightOffAfter"), qlonglong(0));
    values.insert(QStringLiteral("input.keyboardNavigation"), false);
    values.insert(QStringLiteral("input.emojiKeyAction"), QStringLiteral("emoji"));
    values.insert(QStringLiteral("idle.dim"), qlonglong(150));
    values.insert(QStringLiteral("idle.blank"), qlonglong(300));
    values.insert(QStringLiteral("idle.lock"), qlonglong(600));
    values.insert(QStringLiteral("idle.suspend"), qlonglong(0));
    values.insert(QStringLiteral("menu.global"), true);
    values.insert(QStringLiteral("menu.autoHide"), QStringLiteral("full-screen"));
    values.insert(QStringLiteral("menu.showBackground"), true);
    values.insert(QStringLiteral("menu.recentItems"), qlonglong(10));
    values.insert(QStringLiteral("menu.clock.showDate"), true);
    values.insert(QStringLiteral("menu.clock.showSeconds"), false);
    values.insert(QStringLiteral("menu.control.wifi"), true);
    values.insert(QStringLiteral("menu.control.bluetooth"), true);
    values.insert(QStringLiteral("menu.control.battery"), true);
    values.insert(QStringLiteral("menu.control.volume"), true);
    values.insert(QStringLiteral("menu.control.focus"), true);
    values.insert(QStringLiteral("menu.control.accessibility"), true);
    values.insert(QStringLiteral("sound.alertSound"), QStringLiteral("Chime"));
    values.insert(QStringLiteral("sound.playEffectsThrough"), QStringLiteral("output"));
    values.insert(QStringLiteral("sound.alertVolume"), 0.8);
    values.insert(QStringLiteral("sound.playOnStartup"), true);
    values.insert(QStringLiteral("sound.uiEffects"), true);
    values.insert(QStringLiteral("sound.volumeFeedback"), false);
    values.insert(QStringLiteral("sound.balance"), 0.5);
    values.insert(QStringLiteral("overview.hotCornerTopLeft"), QStringLiteral("mission-control"));
    values.insert(QStringLiteral("overview.hotCornerTopRight"), QStringLiteral("notification-center"));
    values.insert(QStringLiteral("overview.hotCornerBottomLeft"), QStringLiteral("desktop-reveal"));
    values.insert(QStringLiteral("overview.hotCornerBottomRight"), QStringLiteral("lock-screen"));
    values.insert(QStringLiteral("notifications.showPreviews"),
                  QStringLiteral("when-unlocked"));
    values.insert(QStringLiteral("notifications.showWhenSleeping"), false);
    values.insert(QStringLiteral("notifications.showWhenLocked"), true);
    values.insert(QStringLiteral("notifications.showWhenMirroring"), false);
    values.insert(QStringLiteral("lock.showUserNameAndPhoto"), true);
    values.insert(QStringLiteral("lock.showPasswordHints"), false);
    values.insert(QStringLiteral("lock.showMessageWhenLocked"), false);
    values.insert(QStringLiteral("lock.message"), QString());
    values.insert(QStringLiteral("lock.showPowerButtons"), true);
    // Revision-7/17 keys the Settings app also writes. Keep this table the
    // complete schema mirror; a missing key would make a valid write look
    // unknown to the client-side validator (T224).
    values.insert(QStringLiteral("dock.pinnedFolders"), QStringList());
    values.insert(QStringLiteral("printers.defaultPaperSize"),
                  QStringLiteral("us-letter"));
    // Revision 19 (T225): the Wi-Fi prompt policy.
    values.insert(QStringLiteral("network.wifi.askToJoin"),
                  QStringLiteral("notify"));
    values.insert(QStringLiteral("network.wifi.askToJoinHotspots"),
                  QStringLiteral("ask"));
    // Revision 20 (T226): the General group's durable policy.
    values.insert(QStringLiteral("clock.use24Hour"), false);
    values.insert(QStringLiteral("updates.auto"), false);
    values.insert(QStringLiteral("updates.autoInstall"), false);
    values.insert(QStringLiteral("locale.calendar"), QStringLiteral("gregorian"));
    values.insert(QStringLiteral("locale.temperature"), QStringLiteral("celsius"));
    values.insert(QStringLiteral("locale.measurement"), QStringLiteral("metric"));
    values.insert(QStringLiteral("locale.dateFormat"), QStringLiteral("medium"));
    values.insert(QStringLiteral("locale.numberFormat"), QStringLiteral("locale"));
    values.insert(QStringLiteral("locale.firstDay"), QStringLiteral("locale"));
    values.insert(QStringLiteral("backup.auto"), false);
    values.insert(QStringLiteral("backup.backend"), QStringLiteral("auto"));
    values.insert(QStringLiteral("backup.destination"), QString());
    values.insert(QStringLiteral("backup.schedule"), QStringLiteral("off"));
    return values;
}

namespace {

// One constrained schema key. `allowed` is the exhaustive enum (empty for
// free-form text); `hasMin`/`hasMax` bound numeric keys. Mirrors
// services/settingsd/src/schema.rs; the Rust store remains the final owner,
// this table only lets the client reject a bad value before it leaves QML.
struct Constraint {
    const char *key;
    const char *const *allowed;
    int allowedCount;
    bool hasMin;
    double min;
    bool hasMax;
    double max;
};

const char *const kDockPosition[] = { "bottom", "left", "right" };
const char *const kDockAnimation[] = { "genie", "scale", "none" };
const char *const kTitlebarAction[] = { "zoom", "minimize", "none" };
const char *const kColorScheme[] = { "light", "dark", "auto" };
const char *const kWallpaperFit[] = { "fill", "fit", "stretch", "center" };
const char *const kRotation[] = { "normal", "90", "180", "270" };
const char *const kScrollMethod[] = { "two-finger", "edge", "button" };
const char *const kEmojiAction[] = { "emoji", "none" };
const char *const kHotCorner[] = { "none", "mission-control", "notification-center",
                                   "desktop-reveal", "lock-screen" };
const char *const kPreviews[] = { "always", "when-unlocked", "never" };
const char *const kMenuAutoHide[] = { "never", "always", "full-screen" };
const char *const kAlertSound[] = { "Chime", "Marimba", "Pulse", "Woodblock", "Breeze" };
const char *const kEffectsThrough[] = { "output", "alerts" };
const char *const kPaperSize[] = { "us-letter", "us-legal", "a3", "a4", "a5" };
const char *const kAskToJoin[] = { "off", "notify", "ask", "new" };
const char *const kAskToJoinHotspots[] = { "off", "ask", "automatic" };
const char *const kLocaleCalendar[] = { "gregorian", "iso", "buddhist", "japanese" };
const char *const kLocaleTemperature[] = { "celsius", "fahrenheit" };
const char *const kLocaleMeasurement[] = { "metric", "imperial" };
const char *const kLocaleDateFormat[] = { "short", "medium", "long" };
const char *const kLocaleNumberFormat[] = { "locale", "decimal", "comma" };
const char *const kLocaleFirstDay[] = { "locale", "sunday", "monday", "saturday" };
const char *const kBackupBackend[] = { "auto", "restic", "pika", "timeshift" };
const char *const kBackupSchedule[] = { "off", "hourly", "daily", "weekly", "monthly" };

const Constraint kConstraints[] = {
    { "dock.size", nullptr, 0, true, 0.0, true, 1.0 },
    { "dock.magnification", nullptr, 0, true, 0.0, true, 1.0 },
    { "dock.position", kDockPosition, 3, false, 0, false, 0 },
    { "dock.minimizedAnimation", kDockAnimation, 3, false, 0, false, 0 },
    { "dock.titlebarDoubleClick", kTitlebarAction, 3, false, 0, false, 0 },
    { "workspaces.count", nullptr, 0, true, 1.0, true, 16.0 },
    { "appearance.colorScheme", kColorScheme, 3, false, 0, false, 0 },
    { "appearance.transparency", nullptr, 0, true, 0.0, true, 1.0 },
    { "wallpaper.fit", kWallpaperFit, 4, false, 0, false, 0 },
    { "display.scale", nullptr, 0, true, 0.5, true, 2.0 },
    { "display.rotation", kRotation, 4, false, 0, false, 0 },
    { "display.brightness", nullptr, 0, true, 0.0, true, 1.0 },
    { "wallpaper.providerLastFetch", nullptr, 0, true, 0.0, false, 0.0 },
    { "input.repeatDelay", nullptr, 0, true, 0.0, true, 5000.0 },
    { "input.repeatRate", nullptr, 0, true, 0.0, true, 200.0 },
    { "input.pointerSpeed", nullptr, 0, true, -1.0, true, 1.0 },
    { "input.scrollMethod", kScrollMethod, 3, false, 0, false, 0 },
    { "input.keyboardBrightness", nullptr, 0, true, 0.0, true, 1.0 },
    { "input.backlightOffAfter", nullptr, 0, true, 0.0, true, 3600.0 },
    { "input.emojiKeyAction", kEmojiAction, 2, false, 0, false, 0 },
    { "idle.dim", nullptr, 0, true, 0.0, true, 86400.0 },
    { "idle.blank", nullptr, 0, true, 0.0, true, 86400.0 },
    { "idle.lock", nullptr, 0, true, 0.0, true, 86400.0 },
    { "idle.suspend", nullptr, 0, true, 0.0, true, 86400.0 },
    { "sound.alertSound", kAlertSound, 5, false, 0, false, 0 },
    { "sound.playEffectsThrough", kEffectsThrough, 2, false, 0, false, 0 },
    { "sound.alertVolume", nullptr, 0, true, 0.0, true, 1.0 },
    { "sound.balance", nullptr, 0, true, 0.0, true, 1.0 },
    { "overview.hotCornerTopLeft", kHotCorner, 5, false, 0, false, 0 },
    { "overview.hotCornerTopRight", kHotCorner, 5, false, 0, false, 0 },
    { "overview.hotCornerBottomLeft", kHotCorner, 5, false, 0, false, 0 },
    { "overview.hotCornerBottomRight", kHotCorner, 5, false, 0, false, 0 },
    { "notifications.showPreviews", kPreviews, 3, false, 0, false, 0 },
    { "menu.autoHide", kMenuAutoHide, 3, false, 0, false, 0 },
    { "menu.recentItems", nullptr, 0, true, 0.0, true, 50.0 },
    { "printers.defaultPaperSize", kPaperSize, 5, false, 0, false, 0 },
    { "network.wifi.askToJoin", kAskToJoin, 4, false, 0, false, 0 },
    { "network.wifi.askToJoinHotspots", kAskToJoinHotspots, 3, false, 0, false, 0 },
    { "locale.calendar", kLocaleCalendar, 4, false, 0, false, 0 },
    { "locale.temperature", kLocaleTemperature, 2, false, 0, false, 0 },
    { "locale.measurement", kLocaleMeasurement, 2, false, 0, false, 0 },
    { "locale.dateFormat", kLocaleDateFormat, 3, false, 0, false, 0 },
    { "locale.numberFormat", kLocaleNumberFormat, 3, false, 0, false, 0 },
    { "locale.firstDay", kLocaleFirstDay, 4, false, 0, false, 0 },
    { "backup.backend", kBackupBackend, 4, false, 0, false, 0 },
    { "backup.schedule", kBackupSchedule, 5, false, 0, false, 0 },
};

const Constraint *constraintFor(const QString &key)
{
    for (const Constraint &constraint : kConstraints) {
        if (key == QLatin1String(constraint.key))
            return &constraint;
    }
    return nullptr;
}

QString numberText(double value)
{
    return QString::number(value, 'g', 12);
}

QString schemaTypeName(const QVariant &declared)
{
    switch (declared.typeId()) {
    case QMetaType::Bool:
        return QStringLiteral("bool");
    case QMetaType::Double:
        return QStringLiteral("double");
    case QMetaType::LongLong:
        return QStringLiteral("int64");
    case QMetaType::QStringList:
        return QStringLiteral("string list");
    default:
        return QStringLiteral("string");
    }
}

bool isNumeric(const QVariant &value)
{
    switch (value.typeId()) {
    case QMetaType::Bool:
        return false;
    case QMetaType::Double:
    case QMetaType::Float:
    case QMetaType::Int:
    case QMetaType::UInt:
    case QMetaType::Long:
    case QMetaType::ULong:
    case QMetaType::LongLong:
    case QMetaType::ULongLong:
    case QMetaType::Short:
    case QMetaType::UShort:
        return true;
    default:
        return false;
    }
}

} // namespace

QVariantMap settingsSchemaInfo(const QString &key)
{
    const QVariantMap defaults = settingsSchemaDefaults();
    const auto it = defaults.constFind(key);
    if (it == defaults.constEnd())
        return {};

    QVariantMap out;
    out.insert(QStringLiteral("key"), key);
    out.insert(QStringLiteral("default"), it.value());
    out.insert(QStringLiteral("type"), schemaTypeName(it.value()));
    const Constraint *constraint = constraintFor(key);
    QStringList allowed;
    bool hasMin = false;
    bool hasMax = false;
    double min = 0.0;
    double max = 0.0;
    if (constraint) {
        for (int i = 0; i < constraint->allowedCount; ++i)
            allowed.append(QString::fromLatin1(constraint->allowed[i]));
        hasMin = constraint->hasMin;
        hasMax = constraint->hasMax;
        min = constraint->min;
        max = constraint->max;
    }
    out.insert(QStringLiteral("allowed"), allowed);
    out.insert(QStringLiteral("hasMin"), hasMin);
    out.insert(QStringLiteral("min"), min);
    out.insert(QStringLiteral("hasMax"), hasMax);
    out.insert(QStringLiteral("max"), max);
    return out;
}

QString SettingsClient::validate(const QString &key, const QVariant &value) const
{
    const QVariantMap defaults = settingsSchemaDefaults();
    const auto it = defaults.constFind(key);
    if (it == defaults.constEnd())
        return QStringLiteral("unknown settings key \"%1\"").arg(key);

    const QVariant declared = it.value();
    const QVariant coerced = coerceToSchema(key, value);
    const QString expected = schemaTypeName(declared);

    switch (declared.typeId()) {
    case QMetaType::Bool:
        if (coerced.typeId() != QMetaType::Bool)
            return QStringLiteral("%1: expected %2, got %3")
                .arg(key, expected, schemaTypeName(coerced));
        break;
    case QMetaType::Double:
        if (!isNumeric(coerced))
            return QStringLiteral("%1: expected %2, got %3")
                .arg(key, expected, schemaTypeName(coerced));
        break;
    case QMetaType::LongLong:
        if (!isNumeric(coerced))
            return QStringLiteral("%1: expected %2, got %3")
                .arg(key, expected, schemaTypeName(coerced));
        break;
    case QMetaType::QStringList:
        if (coerced.typeId() != QMetaType::QStringList
                && coerced.typeId() != QMetaType::QVariantList)
            return QStringLiteral("%1: expected %2, got %3")
                .arg(key, expected, schemaTypeName(coerced));
        break;
    default:
        if (coerced.typeId() != QMetaType::QString)
            return QStringLiteral("%1: expected %2, got %3")
                .arg(key, expected, schemaTypeName(coerced));
        break;
    }

    const Constraint *constraint = constraintFor(key);
    if (!constraint)
        return QString();

    if (constraint->allowedCount > 0) {
        const QString text = coerced.toString();
        for (int i = 0; i < constraint->allowedCount; ++i) {
            if (text == QLatin1String(constraint->allowed[i]))
                return QString();
        }
        QStringList allowed;
        for (int i = 0; i < constraint->allowedCount; ++i)
            allowed.append(QString::fromLatin1(constraint->allowed[i]));
        return QStringLiteral("%1: \"%2\" is not one of [%3]")
            .arg(key, text, allowed.join(QStringLiteral(", ")));
    }

    if (isNumeric(coerced)) {
        const double number = coerced.toDouble();
        const bool below = constraint->hasMin && number < constraint->min;
        const bool above = constraint->hasMax && number > constraint->max;
        if (below || above) {
            if (constraint->hasMin && constraint->hasMax) {
                return QStringLiteral("%1: %2 is outside %3..=%4")
                    .arg(key, numberText(number), numberText(constraint->min),
                         numberText(constraint->max));
            }
            if (below) {
                return QStringLiteral("%1: %2 is below %3")
                    .arg(key, numberText(number), numberText(constraint->min));
            }
            return QStringLiteral("%1: %2 is above %3")
                .arg(key, numberText(number), numberText(constraint->max));
        }
    }
    return QString();
}

QVariantMap SettingsClient::writeErrors() const
{
    QVariantMap out;
    for (auto it = m_writeErrors.constBegin(); it != m_writeErrors.constEnd(); ++it)
        out.insert(it.key(), it.value());
    return out;
}

QVariantMap SettingsClient::writeStates() const
{
    QVariantMap out;
    for (auto it = m_writeStates.constBegin(); it != m_writeStates.constEnd(); ++it)
        out.insert(it.key(), it.value());
    return out;
}

QString SettingsClient::writeError(const QString &key) const
{
    return m_writeErrors.value(key);
}

QString SettingsClient::writeState(const QString &key) const
{
    return m_writeStates.value(key, QStringLiteral("idle"));
}

void SettingsClient::setWriteState(const QString &key, const QString &state)
{
    // Emit even when the state name is unchanged: the error message for an
    // `error` state can change without the state changing, and rows bind
    // `writeErrors[key]` to this one notification.
    m_writeStates.insert(key, state);
    emit writeStateChanged(key);
}

void SettingsClient::clearWriteError(const QString &key)
{
    if (!m_writeErrors.contains(key))
        return;
    m_writeErrors.remove(key);
    setWriteState(key, QStringLiteral("idle"));
}

bool SettingsClient::beginWrite(const QString &key, const QVariant &value,
                                quint64 *generation)
{
    const QString problem = validate(key, value);
    if (!problem.isEmpty()) {
        m_writeErrors.insert(key, problem);
        setWriteState(key, QStringLiteral("error"));
        // The control has already moved itself (a Toggle flips before it
        // emits), so nudge every `Settings.values` binding to re-read even
        // though the map did not change.
        emit writeFinished(key, value, false, problem);
        return false;
    }

    const QVariant coerced = coerceToSchema(key, value);
    PendingWrite pending;
    pending.previous = this->value(key);
    pending.value = coerced;
    pending.generation = ++m_generations[key];
    m_writes.insert(key, pending);
    if (generation)
        *generation = pending.generation;
    m_writeErrors.remove(key);
    setWriteState(key, QStringLiteral("pending"));
    emit writeStarted(key, coerced);
    applyValue(key, coerced);
    return true;
}

void SettingsClient::completeWrite(const QString &key, quint64 generation, bool ok,
                                   const QString &message, bool deferred)
{
    auto it = m_writes.find(key);
    if (it == m_writes.end() || it->generation != generation)
        return; // a newer write owns this key now

    const QVariant attempted = it->value;
    const QVariant previous = it->previous;
    m_writes.erase(it);

    if (ok) {
        m_writeErrors.remove(key);
        setWriteState(key, deferred ? QStringLiteral("deferred")
                                    : QStringLiteral("idle"));
        emit writeFinished(key, attempted, true, QString());
        return;
    }

    // Revert to the value the owner last confirmed. `applyValue` emits
    // `changed`, so every bound control snaps back without polling.
    applyValue(key, previous);
    m_writeErrors.insert(key, message);
    setWriteState(key, QStringLiteral("error"));
    emit writeFinished(key, attempted, false, message);
}

void SettingsClient::noteOwnerValue(const QString &key, const QVariant &value)
{
    Q_UNUSED(value);
    // The owner has spoken: a pending local error is no longer the truth. An
    // echo that races our own in-flight write is left to `completeWrite`.
    if (m_writes.contains(key))
        return;
    clearWriteError(key);
}

void SettingsClient::clearOwnerWriteStates()
{
    const QStringList keys = m_writeStates.keys();
    for (const QString &key : keys) {
        if (m_writes.contains(key))
            continue; // an in-flight write owns this key until it resolves
        m_writeStates.remove(key);
        m_writeErrors.remove(key);
        emit writeStateChanged(key);
    }
}

QString shippedDefaultWallpaperPath()
{
    // Mirrors services/wallpaperd/src/defaults.rs exactly so shell and
    // provider never disagree about the out-of-box asset (ADR 0094).
    const QString override = qEnvironmentVariable("DF_DEFAULT_WALLPAPER");
    if (!override.isEmpty() && QFileInfo::exists(override))
        return override;

    const QString dataDirs = qEnvironmentVariable("XDG_DATA_DIRS");
    const QStringList dirs = dataDirs.isEmpty()
        ? QStringList{ QStringLiteral("/usr/local/share"), QStringLiteral("/usr/share") }
        : dataDirs.split(QLatin1Char(':'), Qt::SkipEmptyParts);
    for (const QString &dir : dirs) {
        const QString path = dir + QStringLiteral("/dragonfruit/wallpapers/Default.jpg");
        if (QFileInfo::exists(path))
            return path;
    }

#ifdef DF_IN_TREE_DEFAULT_WALLPAPER
    const QString inTree = QStringLiteral(DF_IN_TREE_DEFAULT_WALLPAPER);
    if (QFileInfo::exists(inTree))
        return inTree;
#endif
    return QString();
}

// --- DbusSettingsClient ----------------------------------------------------

DbusSettingsClient::DbusSettingsClient(QObject *parent)
    : SettingsClient(parent)
    , m_service(kService)
    , m_path(kPath)
    , m_interface(kInterface)
{
    // Watch the well-known name itself. `QDBusConnectionInterface`'s
    // serviceRegistered/serviceUnregistered only fire for a connection's
    // unique name, so a settingsd owned by another process never triggered
    // them: the shell resynced once via the constructor's GetAll and then
    // never after a restart (T-08.3 found this live). A QDBusServiceWatcher
    // follows the well-known name across owners.
    m_watcher = new QDBusServiceWatcher(m_service, QDBusConnection::sessionBus(),
                                        QDBusServiceWatcher::WatchForRegistration
                                            | QDBusServiceWatcher::WatchForUnregistration,
                                        this);
    connect(m_watcher, &QDBusServiceWatcher::serviceRegistered, this,
            &DbusSettingsClient::onServiceRegistered);
    connect(m_watcher, &QDBusServiceWatcher::serviceUnregistered, this,
            &DbusSettingsClient::onServiceUnregistered);
    subscribeToChanges();
    setAvailable(isAvailable());
    if (m_available)
        refresh();
}

bool DbusSettingsClient::isAvailable() const
{
    QDBusConnectionInterface *iface = QDBusConnection::sessionBus().interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusSettingsClient::subscribeToChanges()
{
    // The `Changed` signal is the only notification (no polling). Registered
    // once; it follows the well-known name across daemon restarts.
    QDBusConnection::sessionBus().connect(m_service, m_path, m_interface,
                                          QStringLiteral("Changed"), this,
                                          SLOT(onRemoteChanged(QString,QDBusVariant)));
}

void DbusSettingsClient::onRemoteChanged(const QString &key, const QDBusVariant &value)
{
    const QVariant unwrapped = unwrapVariant(QVariant::fromValue(value));
    noteOwnerValue(key, unwrapped);
    applyValue(key, unwrapped);
}

void DbusSettingsClient::onServiceRegistered(const QString &name)
{
    if (name != m_service)
        return;
    setAvailable(true);
    // Re-sync: the daemon may have a persisted value the shell has not seen.
    refresh();
}

void DbusSettingsClient::onServiceUnregistered(const QString &name)
{
    if (name != m_service)
        return;
    // The daemon restart resets its owner; keep the last known values so the
    // desktop does not lose a setting visually (T-08 restart behavior).
    setAvailable(false);
}

void DbusSettingsClient::refresh()
{
    if (!isAvailable()) {
        emit refreshed();
        return;
    }
    auto *call = new QDBusInterface(m_service, m_path, m_interface,
                                    QDBusConnection::sessionBus(), this);
    auto *watcher = new QDBusPendingCallWatcher(call->asyncCall(QStringLiteral("GetAll")), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, watcher, call]() {
                const QDBusPendingReply<QVariantMap> reply = *watcher;
                if (reply.isValid()) {
                    QVariantMap values;
                    const QVariantMap raw = reply.value();
                    for (auto it = raw.constBegin(); it != raw.constEnd(); ++it)
                        values.insert(it.key(), unwrapVariant(it.value()));
                    applyValues(values);
                    // The owner's snapshot is authoritative (ADR 0032). A
                    // write made while the daemon was away survives in memory
                    // for the session only, so its `deferred` state is
                    // reconciled away here rather than claiming persistence.
                    clearOwnerWriteStates();
                } else {
                    qWarning() << "dragonfruit-shell: settings GetAll failed:"
                               << watcher->reply().errorMessage();
                }
                emit refreshed();
                call->deleteLater();
                watcher->deleteLater();
            });
}

bool DbusSettingsClient::set(const QString &key, const QVariant &value)
{
    // T224: one apply path. Validate first, apply optimistically, then let the
    // owner confirm. A rejection or a daemon error reverts to the previous
    // value and leaves a message for the row.
    quint64 generation = 0;
    if (!beginWrite(key, value, &generation))
        return false;

    if (!isAvailable()) {
        // No owner: keep the write in memory for this session only. The next
        // `GetAll` (daemon restart resync) makes the owner's persisted value
        // authoritative again, so this is reported as `deferred`, never as a
        // durable success.
        completeWrite(key, generation, true, QString(), true);
        return true;
    }

    const QVariant coerced = this->value(key);
    auto *call = new QDBusInterface(m_service, m_path, m_interface,
                                    QDBusConnection::sessionBus(), this);
    auto *watcher = new QDBusPendingCallWatcher(
        call->asyncCall(QStringLiteral("Set"), key,
                        QVariant::fromValue(QDBusVariant(coerced))),
        this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, watcher, call, key, generation]() {
                const QDBusMessage reply = watcher->reply();
                if (reply.type() == QDBusMessage::ErrorMessage) {
                    qWarning() << "dragonfruit-shell: settings Set failed:"
                               << reply.errorMessage();
                    completeWrite(key, generation, false, reply.errorMessage());
                } else {
                    completeWrite(key, generation, true, QString());
                }
                call->deleteLater();
                watcher->deleteLater();
            });
    return true;
}

// --- MockSettingsClient ----------------------------------------------------

MockSettingsClient::MockSettingsClient(QObject *parent)
    : SettingsClient(parent)
{
    m_available = true;
}

void DbusSettingsClient::reset()
{
    if (!isAvailable())
        return;
    auto *call = new QDBusInterface(m_service, m_path, m_interface,
                                    QDBusConnection::sessionBus(), this);
    auto *watcher = new QDBusPendingCallWatcher(
        call->asyncCall(QStringLiteral("Reset")), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, watcher, call]() {
                const QDBusPendingReply<> reply = *watcher;
                call->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid()) {
                    qWarning() << "dragonfruit-shell: settings Reset failed:"
                               << watcher->reply().errorMessage();
                    return;
                }
                // The daemon emits Changed per key; the explicit resync also
                // covers a key that already held its default.
                refresh();
            });
}

void MockSettingsClient::refresh()
{
    // A mock resync is authoritative, like the daemon's `GetAll`.
    clearOwnerWriteStates();
    emit refreshed();
}

bool MockSettingsClient::set(const QString &key, const QVariant &value)
{
    quint64 generation = 0;
    if (!beginWrite(key, value, &generation))
        return false;

    if (!m_available) {
        completeWrite(key, generation, true, QString(), true);
        return true;
    }

    const QString failure = m_failures.take(key);
    if (!failure.isEmpty()) {
        // Apply optimistically now and fail on the next event-loop turn, so a
        // test can observe the optimistic value before the revert.
        QTimer::singleShot(0, this, [this, key, generation, failure]() {
            completeWrite(key, generation, false, failure);
        });
        return true;
    }
    completeWrite(key, generation, true, QString());
    return true;
}

void MockSettingsClient::reset()
{
    applyValues(settingsSchemaDefaults());
    clearOwnerWriteStates();
    emit refreshed();
}

void MockSettingsClient::failNextWrite(const QString &key, const QString &message)
{
    m_failures.insert(key, message);
}

void MockSettingsClient::setAvailableFixture(bool available)
{
    if (m_available == available)
        return;
    setAvailable(available);
    if (available)
        refresh();
}
