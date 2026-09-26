// SPDX-License-Identifier: MIT
// Clipboard-history store tests (T-13.5b): classification, dedup, caps,
// pinning, secret-hint refusal, clear-on-lock, search, restore, persistence.
// Pure Qt — no Wayland, D-Bus, or QML.
#include <QtTest>

#include "clipboardhistory.h"

namespace {

ClipboardHistory::EntryMap textPayload(const QByteArray &text)
{
    return { { QStringLiteral("text/plain;charset=utf-8"), text } };
}

} // namespace

class TestClipboardHistory : public QObject
{
    Q_OBJECT

private slots:
    void observesText()
    {
        ClipboardHistory history;
        QSignalSpy spy(&history, &ClipboardHistory::changed);
        const QStringList mimes{ QStringLiteral("text/plain;charset=utf-8") };
        QVERIFY(history.observe(mimes, textPayload("hello clipboard")));
        QCOMPARE(history.count(), 1);
        QCOMPARE(spy.count(), 1);
        const QVariantMap entry = history.entries().constFirst().toMap();
        QCOMPARE(entry.value(QStringLiteral("kind")).toString(), QStringLiteral("text"));
        QCOMPARE(entry.value(QStringLiteral("preview")).toString(),
                 QStringLiteral("hello clipboard"));
    }

    void deduplicatesAndPromotes()
    {
        ClipboardHistory history;
        history.observe({ QStringLiteral("text/plain") }, textPayload("first"));
        history.observe({ QStringLiteral("text/plain") }, textPayload("second"));
        history.observe({ QStringLiteral("text/plain") }, textPayload("first"));
        QCOMPARE(history.count(), 2);
        QCOMPARE(history.entries().constFirst().toMap().value(QStringLiteral("preview")).toString(),
                 QStringLiteral("first"));
    }

    void classifiesKinds()
    {
        ClipboardHistory history;
        history.observe({ QStringLiteral("text/plain") }, textPayload("text"));
        history.observe({ QStringLiteral("image/png") },
                        { { QStringLiteral("image/png"), QByteArray("not-an-image") } });
        history.observe({ QStringLiteral("text/uri-list") },
                        { { QStringLiteral("text/uri-list"),
                            QByteArray("file:///home/u/a.txt\nfile:///home/u/b.txt\n") } });
        const QVariantList entries = history.entries();
        QCOMPARE(entries.at(0).toMap().value(QStringLiteral("kind")).toString(),
                 QStringLiteral("files"));
        QCOMPARE(entries.at(0).toMap().value(QStringLiteral("preview")).toString(),
                 QStringLiteral("a.txt, b.txt"));
        QCOMPARE(entries.at(1).toMap().value(QStringLiteral("kind")).toString(),
                 QStringLiteral("image"));
        QCOMPARE(entries.at(2).toMap().value(QStringLiteral("kind")).toString(),
                 QStringLiteral("text"));
    }

    void refusesSecrets()
    {
        ClipboardHistory history;
        const QStringList mimes{ QStringLiteral("text/plain;charset=utf-8"),
                                 QStringLiteral("x-kde-passwordManagerHint") }; // df-allow-desktop-name
        QVERIFY(!history.observe(mimes,
                                 { { QStringLiteral("text/plain;charset=utf-8"),
                                     QByteArray("hunter2") },
                                   { QStringLiteral("x-kde-passwordManagerHint"), // df-allow-desktop-name
                                     QByteArray("secret") } }));
        QCOMPARE(history.count(), 0);
    }

    void capsEntries()
    {
        ClipboardHistory history;
        for (int i = 0; i < ClipboardHistory::kMaxEntries + 10; ++i)
            history.observe({ QStringLiteral("text/plain") },
                            textPayload(QByteArray::number(i)));
        QCOMPARE(history.count(), ClipboardHistory::kMaxEntries);
        // The newest survives; the oldest were evicted.
        QCOMPARE(history.entries().constFirst().toMap().value(QStringLiteral("preview")).toString(),
                 QString::number(ClipboardHistory::kMaxEntries + 9));
    }

    void pinnedSurvivesCapsAndLock()
    {
        ClipboardHistory history;
        history.observe({ QStringLiteral("text/plain") }, textPayload("keep me"));
        QVERIFY(history.setPinned(0, true));
        history.observe({ QStringLiteral("text/plain") }, textPayload("drop me"));
        history.observe({ QStringLiteral("text/plain") }, textPayload("keep me"));
        // Dedup promoted the pinned entry; still one with that preview.
        QCOMPARE(history.entries().constFirst().toMap().value(QStringLiteral("pinned")).toBool(), true);
        QVERIFY(history.clearOnLock());
        history.clearUnpinned();
        QCOMPARE(history.count(), 1);
        QCOMPARE(history.entries().constFirst().toMap().value(QStringLiteral("preview")).toString(),
                 QStringLiteral("keep me"));
        history.clear();
        QCOMPARE(history.count(), 0);
    }

    void searchFilters()
    {
        ClipboardHistory history;
        history.observe({ QStringLiteral("text/plain") }, textPayload("apple pie"));
        history.observe({ QStringLiteral("text/plain") }, textPayload("banana bread"));
        QCOMPARE(history.search(QStringLiteral("apple")).size(), 1);
        QCOMPARE(history.search(QStringLiteral("APPLE")).size(), 1);
        QCOMPARE(history.search(QStringLiteral("zzz")).size(), 0);
        QCOMPARE(history.search(QString()).size(), 2);
    }

    void restoresPayloads()
    {
        ClipboardHistory history;
        history.observe({ QStringLiteral("text/plain") }, textPayload("restore me"));
        QStringList mimes;
        ClipboardHistory::EntryMap payloads;
        QVERIFY(history.restore(0, &mimes, &payloads));
        QCOMPARE(payloads.value(QStringLiteral("text/plain;charset=utf-8")),
                 QByteArray("restore me"));
        QVERIFY(!history.restore(5, &mimes, &payloads));
    }

    void rejectsOversizedEntry()
    {
        ClipboardHistory history;
        QCOMPARE(history.observe({ QStringLiteral("image/png") },
                                 { { QStringLiteral("image/png"),
                                     QByteArray(ClipboardHistory::kMaxEntryBytes + 1, 'x') } }),
                 false);
        QCOMPARE(history.count(), 0);
    }

    void persistenceRoundTrip()
    {
        ClipboardHistory history;
        history.observe({ QStringLiteral("text/plain") }, textPayload("alpha"));
        history.observe({ QStringLiteral("text/uri-list") },
                        { { QStringLiteral("text/uri-list"), QByteArray("file:///tmp/x\n") } });
        history.setPinned(1, true);
        const QByteArray json = history.toJson();

        ClipboardHistory restored;
        QVERIFY(restored.fromJson(json));
        QCOMPARE(restored.count(), 2);
        QCOMPARE(restored.entries().constFirst().toMap().value(QStringLiteral("kind")).toString(),
                 QStringLiteral("files"));
        QCOMPARE(restored.entries().constLast().toMap().value(QStringLiteral("pinned")).toBool(),
                 true);
    }

    void malformedPersistenceIsTolerated()
    {
        ClipboardHistory history;
        QVERIFY(history.observe({ QStringLiteral("text/plain") }, textPayload("keep")));
        QVERIFY(!history.fromJson(QByteArray("not json")));
        // A failed load leaves the existing history untouched.
        QCOMPARE(history.count(), 1);
    }
};

QTEST_MAIN(TestClipboardHistory)
#include "tst_clipboardhistory.moc"