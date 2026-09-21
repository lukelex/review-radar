#include "queuecontroller.h"

#include <QGuiApplication>
#include <QJsonDocument>
#include <QJsonObject>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickItem>
#include <QQuickWindow>
#include <QTest>
#include <QDir>

// Presentation contract only: no network, SQLite, notifications, or host actions.
class PreviewQueue final : public QObject {
    Q_OBJECT
    Q_PROPERTY(PullRequestModel *pullRequests READ pullRequests CONSTANT)
    Q_PROPERTY(QString view MEMBER view NOTIFY viewChanged)
    Q_PROPERTY(QString ranking MEMBER ranking NOTIFY rankingChanged)
    Q_PROPERTY(QString status MEMBER status CONSTANT)
    Q_PROPERTY(bool loading MEMBER loading CONSTANT)
    Q_PROPERTY(bool refreshing MEMBER refreshing CONSTANT)
    Q_PROPERTY(bool stale MEMBER stale CONSTANT)
    Q_PROPERTY(int sourceCount MEMBER sourceCount CONSTANT)
    Q_PROPERTY(int suppressedCount MEMBER suppressedCount CONSTANT)
public:
    PullRequestModel model;
    QString view = "tailored", ranking = "tailored", status = "Cached on this device · Updated just now";
    QString opened, acknowledged, fingerprint;
    bool loading = false, refreshing = false, stale = false;
    int sourceCount = 3, suppressedCount = 0;
    PullRequestModel *pullRequests() { return &model; }
    Q_INVOKABLE void refresh() {}
    Q_INVOKABLE void openUrl(const QString &url) { opened = url; }
    Q_INVOKABLE void acknowledge(const QString &id, const QString &value) { acknowledged = id; fingerprint = value; }
    Q_INVOKABLE void snooze(const QString &, const QString &, const QString &) {}
    Q_INVOKABLE void copyText(const QString &) {}
signals:
    void viewChanged();
    void rankingChanged();
};

class WorkspaceTest final : public QObject {
    Q_OBJECT
private slots:
    void workspace();
};

QQuickItem *findItem(QQuickItem *parent, const QString &name) {
    if (parent->objectName() == name) return parent;
    for (auto *child : parent->childItems())
        if (auto *found = findItem(child, name)) return found;
    return nullptr;
}

void WorkspaceTest::workspace() {
        PreviewQueue queue;
        const auto fixture = QJsonDocument::fromJson(R"([
          {"id":"pr-142","repository":"example/api","number":142,
           "title":"Add pagination to activity endpoint","url":"https://github.com/example/api/pull/142",
           "lifecycle":"open","updatedAt":"2026-09-21T12:00:00Z","attentionRequired":true,
           "actionLabel":"Review requested","currentFingerprint":"event-142",
           "explanation":{"heading":"Your review is outstanding.",
             "reasons":[{"summary":"user-002 requested your review.","nextAction":{"label":"Start review","url":"https://github.com/example/api/pull/142/files"}}],
             "health":{"reviewDecision":"REVIEW_REQUIRED","checks":"passing"}},
           "reviewFriction":{"status":"assessed","level":"low","contributors":[{"signal":"review rounds","value":1,"unit":"round"}],"limitations":[]},
           "events":[{"kind":"review-requested","actor":"user-002","occurredAt":"2026-09-21T12:00:00Z"}]},
          {"id":"pr-87","repository":"example/web","number":87,
           "title":"Improve keyboard navigation","url":"https://github.com/example/web/pull/87",
           "lifecycle":"open","updatedAt":"2026-09-21T11:00:00Z","attentionRequired":true,
           "actionLabel":"Changes requested","currentFingerprint":"event-87",
           "explanation":{"heading":"Feedback is ready for you.",
             "reasons":[{"summary":"user-003 requested changes on your PR.","nextAction":{"label":"View feedback"}}],
             "health":{"reviewDecision":"CHANGES_REQUESTED","checks":"failing"}},
           "reviewFriction":{"status":"assessed","level":"high","contributors":[],"limitations":["Limited history"]},"events":[]},
          {"id":"pr-139","repository":"example/api","number":139,
           "title":"Document response fields","url":"https://github.com/example/api/pull/139",
           "lifecycle":"open","updatedAt":"2026-09-21T10:00:00Z","attentionRequired":false,
           "actionLabel":"Awaiting review","currentFingerprint":"event-139",
           "explanation":{"heading":"Over to your reviewers.","reasons":[{"summary":"Your PR is awaiting review. No action needed right now."}],"health":{"checks":"passing"}},
           "reviewFriction":{"status":"limited-history","contributors":[],"limitations":["Review history is incomplete"]},"events":[]}
        ])");
        QVERIFY(fixture.isArray());
        queue.model.replace(fixture.array());
        QQmlApplicationEngine engine;
        QStringList warnings;
        connect(&engine, &QQmlEngine::warnings, this, [&warnings](const QList<QQmlError> &errors) {
            for (const auto &error : errors) warnings.append(error.toString());
        });
        engine.rootContext()->setContextProperty("queue", &queue);
        engine.load(QUrl("qrc:/ReviewRadarTest/qml/Main.qml"));
        QVERIFY(!engine.rootObjects().isEmpty());
        auto *window = qobject_cast<QQuickWindow *>(engine.rootObjects().first());
        QVERIFY(window);
        QVERIFY(QTest::qWaitForWindowExposed(window));
        QTRY_COMPARE(window->property("matchingCount").toInt(), 3);
        auto *search = window->findChild<QObject *>("search");
        QVERIFY(search);
        search->setProperty("text", "WEB");
        QTRY_COMPARE(window->property("matchingCount").toInt(), 1);
        search->setProperty("text", "no matches");
        QTRY_COMPARE(window->property("matchingCount").toInt(), 0);
        search->setProperty("text", "");
        QTRY_COMPARE(window->property("matchingCount").toInt(), 3);

        QTest::keyClick(window, Qt::Key_3, Qt::ControlModifier);
        QTRY_COMPARE(queue.view, QString("my-prs"));
        QTest::keyClick(window, Qt::Key_1, Qt::ControlModifier);
        QTRY_COMPARE(queue.view, QString("tailored"));

        auto *list = window->findChild<QQuickItem *>("pull-request-list");
        QVERIFY(list);
        const auto initialContentY = list->property("contentY").toReal();
        QTest::keyClick(window, Qt::Key_D, Qt::ControlModifier);
        QTRY_VERIFY(list->property("contentY").toReal() > initialContentY);
        const auto pagedContentY = list->property("contentY").toReal();
        QTest::keyClick(window, Qt::Key_U, Qt::ControlModifier);
        QTRY_VERIFY(list->property("contentY").toReal() < pagedContentY);

        // Basic Vim navigation follows the filtered list without stealing
        // keystrokes from the search field.
        QCOMPARE(window->property("navigationIndex").toInt(), -1);
        QTest::keyClick(window, Qt::Key_J);
        QTRY_COMPARE(window->property("navigationIndex").toInt(), 0);
        QTest::keyClick(window, Qt::Key_J);
        QTRY_COMPARE(window->property("navigationIndex").toInt(), 1);
        QTest::keyClick(window, Qt::Key_K);
        QTRY_COMPARE(window->property("navigationIndex").toInt(), 0);
        QTest::keyClick(window, Qt::Key_L);
        QTRY_VERIFY(window->findChild<QObject *>("detail-panel"));
        QCOMPARE(window->findChild<QObject *>("detail-panel")
                     ->property("entry").toMap().value("pullRequestId").toString(),
                 QString("pr-142"));
        QTest::keyClick(window, Qt::Key_J);
        QTRY_COMPARE(window->findChild<QObject *>("detail-panel")
                         ->property("entry").toMap().value("pullRequestId").toString(),
                     QString("pr-87"));
        QTest::keyClick(window, Qt::Key_H);
        QTRY_VERIFY(!window->findChild<QObject *>("detail-panel"));

        search->setProperty("text", "WEB");
        QMetaObject::invokeMethod(search, "forceActiveFocus");
        QTest::keyClick(window, Qt::Key_J);
        QCOMPARE(window->property("navigationIndex").toInt(), 1);
        search->setProperty("text", "");

        auto screenshot = [window](const QString &name) {
            const auto directory = qEnvironmentVariable("UI_SCREENSHOT_DIR");
            if (directory.isEmpty()) return;
            QTest::qWait(150);
            QVERIFY(window->grabWindow().save(QDir(directory).filePath(name + ".png")));
        };
        screenshot("workspace");
        // Hit-test the actual card and buttons, not just their signal handlers.
        QTRY_VERIFY(findItem(window->contentItem(), "card-0"));
        auto *card = findItem(window->contentItem(), "card-0");
        QVERIFY(card);
        QTest::mouseClick(window, Qt::LeftButton, Qt::NoModifier,
                          card->mapToScene(QPointF(30, 30)).toPoint());
        QTRY_VERIFY(window->findChild<QObject *>("detail-panel"));
        auto *open = window->findChild<QQuickItem *>("detail-open");
        QVERIFY(open);
        QTest::mouseClick(window, Qt::LeftButton, Qt::NoModifier,
                          open->mapToScene(QPointF(open->width() / 2, open->height() / 2)).toPoint());
        QCOMPARE(queue.opened, QString("https://github.com/example/api/pull/142/files"));
        QTest::keyClick(window, Qt::Key_O);
        QCOMPARE(queue.opened, QString("https://github.com/example/api/pull/142"));
        auto *read = window->findChild<QQuickItem *>("detail-read");
        QVERIFY(read);
        QTest::mouseClick(window, Qt::LeftButton, Qt::NoModifier,
                          read->mapToScene(QPointF(read->width() / 2, read->height() / 2)).toPoint());
        QCOMPARE(queue.acknowledged, QString("pr-142"));
        QCOMPARE(queue.fingerprint, QString("event-142"));
        screenshot("detail");
        window->resize(860, 640);
        QTRY_VERIFY(window->property("narrow").toBool());
        screenshot("narrow-detail");
        // A cache update retains selection by identity and refreshes its data.
        auto replacement = fixture.array();
        auto first = replacement.first().toObject();
        first["title"] = "Updated pagination title";
        replacement[0] = first;
        queue.model.replace(replacement);
        QTRY_COMPARE(window->findChild<QObject *>("detail-panel")->property("entry").toMap().value("title").toString(), QString("Updated pagination title"));
        queue.model.replace({});
        QTRY_COMPARE(window->property("matchingCount").toInt(), 0);
        QTRY_VERIFY(!window->findChild<QObject *>("detail-panel"));
        QVERIFY2(warnings.isEmpty(), qPrintable(warnings.join('\n')));
}

int main(int argc, char **argv) {
    QGuiApplication app(argc, argv);
    WorkspaceTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "workspace.moc"
