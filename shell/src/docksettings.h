// SPDX-License-Identifier: MIT
// Interim desktop-settings model for the Dock's `dock.*` keys (T-10 section
// 19). settingsd (T-15) owns persistence; until it lands the shell reads and
// writes the same keys under `$XDG_CONFIG_HOME/dragonfruit/settings.json` in
// the eventual `org.dragonfruit.Settings1` named-key shape, so T-15 adopts
// the file without a migration. Delete this class when T-15's settings API is
// available.
//
// `dock.pinned` is owned by DockPins; this class owns every other key and
// merges on save (it re-reads the file first) so the two writers cannot
// clobber each other.
#pragma once

#include <QJsonObject>
#include <QString>

class DockSettings
{
public:
    explicit DockSettings(const QString &filePath = defaultFilePath());

    static QString defaultFilePath();

    // Load the keys from disk. A missing file leaves the defaults. Returns
    // false on a malformed file; `lastError()` explains and the in-memory
    // values stay at their defaults.
    bool load();
    bool save();
    bool fileExists() const;

    // `dock.size`: 0..1, mapped by the shell onto the icon-size range.
    double size() const { return m_size; }
    void setSize(double value);

    // `dock.magnification`: 0..1, 0 = off.
    double magnification() const { return m_magnification; }
    void setMagnification(double value);

    // `dock.position`: bottom | left | right.
    QString position() const { return m_position; }
    void setPosition(const QString &value);

    // `dock.autohide`: reserve nothing and translate off the edge.
    bool autohide() const { return m_autohide; }
    void setAutohide(bool value) { m_autohide = value; }

    // `dock.animateOpening`: the launch bounce.
    bool animateOpening() const { return m_animateOpening; }
    void setAnimateOpening(bool value) { m_animateOpening = value; }

    // `dock.showIndicators`: the running dot.
    bool showIndicators() const { return m_showIndicators; }
    void setShowIndicators(bool value) { m_showIndicators = value; }

    // `dock.minimizeIntoTileIcon`: no separate minimized entries.
    bool minimizeIntoTileIcon() const { return m_minimizeIntoTileIcon; }
    void setMinimizeIntoTileIcon(bool value) { m_minimizeIntoTileIcon = value; }

    // `dock.minimizedAnimation`: genie | scale | none.
    QString minimizedAnimation() const { return m_minimizedAnimation; }
    void setMinimizedAnimation(const QString &value);

    // `dock.titlebarDoubleClick`: zoom | minimize | none (consumed by the
    // compositor/SSD; the Dock only owns the key).
    QString titlebarDoubleClick() const { return m_titlebarDoubleClick; }
    void setTitlebarDoubleClick(const QString &value);

    // `dock.showRecentApps`: recent/suggested entries (deferred, T-10 s.17).
    bool showRecentApps() const { return m_showRecentApps; }
    void setShowRecentApps(bool value) { m_showRecentApps = value; }

    // `accessibility.reduceMotion`: the global animation policy the Dock
    // binds onto the design-system `Theme.reducedMotion` (T-10 section 20).
    bool reduceMotion() const { return m_reduceMotion; }
    void setReduceMotion(bool value) { m_reduceMotion = value; }

    QString lastError() const { return m_error; }

    // True when every key matches `other`. The shell uses this to ignore the
    // file-watch notification for its own writes.
    bool equals(const DockSettings &other) const;

private:
    QString m_filePath;
    QString m_error;

    double m_size = 0.5;
    double m_magnification = 0.5;
    QString m_position = QStringLiteral("bottom");
    bool m_autohide = false;
    bool m_animateOpening = true;
    bool m_showIndicators = true;
    bool m_minimizeIntoTileIcon = false;
    QString m_minimizedAnimation = QStringLiteral("scale");
    QString m_titlebarDoubleClick = QStringLiteral("zoom");
    bool m_showRecentApps = false;
    bool m_reduceMotion = false;
};
