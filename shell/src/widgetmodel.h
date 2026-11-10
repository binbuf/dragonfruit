// SPDX-License-Identifier: MIT
// The Notification Center widget stack (T234).
//
// Widgets are optional: the model starts with the measured live Calendar card
// (164 pt in the panel) and can add/remove/resize entries from the separate
// gallery. It is deliberately Wayland-free and provider-free, so the shell can
// unit-test the catalogue and the stack without a compositor. Live data is
// pushed in by the shell (`setWidgetData`); a widget with no provider keeps
// `available == false` and the QML shows its explicit empty state instead of
// invented values.
#pragma once

#include <QObject>
#include <QStringList>
#include <QVariantList>
#include <QVariantMap>

class WidgetModel : public QObject
{
    Q_OBJECT
    Q_PROPERTY(QVariantList widgets READ widgets NOTIFY widgetsChanged)
    Q_PROPERTY(QVariantList catalog READ catalog CONSTANT)
    Q_PROPERTY(QStringList categories READ categories CONSTANT)

public:
    explicit WidgetModel(QObject *parent = nullptr);

    // The widgets in the panel, in stack order. Each entry carries `id`,
    // `kind`, `size`, `title`, `available` and a `data` map.
    QVariantList widgets() const { return m_widgets; }
    // The gallery catalogue (`kind`, `title`, `category`, `description`,
    // `sizes`).
    QVariantList catalog() const { return m_catalog; }
    // The measured 19 gallery categories.
    QStringList categories() const { return m_categories; }

    // The pure catalogue, public so tests and the gallery fixture agree.
    static QVariantList defaultCatalog();
    static QStringList defaultCategories();

    // Accepted live-panel sizes (the gallery preview size is never stored).
    static bool isKnownSize(const QString &size);

public slots:
    // Add `kind` at `size` (default medium). A second instance of the same
    // kind is rejected: the gallery toggles one entry per kind.
    bool addWidget(const QString &kind, const QString &size = QStringLiteral("medium"));
    bool removeWidget(const QString &id);
    bool resizeWidget(const QString &id, const QString &size);
    // Merge provider data into one widget and mark it live. An empty map is
    // the explicit "provider absent" state.
    bool setWidgetData(const QString &id, const QVariantMap &data);
    bool setWidgetAvailable(const QString &id, bool available);
    // Restore the measured default stack (Calendar only).
    void reset();

signals:
    void widgetsChanged();

private:
    QVariantMap entryForKind(const QString &kind) const;
    int indexOf(const QString &id) const;

    QVariantList m_widgets;
    QVariantList m_catalog;
    QStringList m_categories;
    int m_nextId = 1;
};
