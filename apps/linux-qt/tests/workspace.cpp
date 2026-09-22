#include "queuecontroller.h"

#include <QApplication>
#include <QQuickStyle>
#include <QSignalSpy>
#include <QJsonDocument>
#include <QJsonObject>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickItem>
#include <QQuickWindow>
#include <QTest>
#include <QDir>
#include <QTemporaryDir>

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
    Q_PROPERTY(bool notificationsEnabled MEMBER notificationsEnabled NOTIFY preferencesChanged)
    Q_PROPERTY(bool trayEnabled MEMBER trayEnabled NOTIFY preferencesChanged)
    Q_PROPERTY(bool closeToTray MEMBER closeToTray NOTIFY preferencesChanged)
    Q_PROPERTY(bool trayAttentionDot MEMBER trayAttentionDot NOTIFY preferencesChanged)
    Q_PROPERTY(bool trayAvailable MEMBER trayAvailable CONSTANT)
    Q_PROPERTY(bool barEnabled MEMBER barEnabled NOTIFY preferencesChanged)
    Q_PROPERTY(bool barActive MEMBER barEnabled NOTIFY preferencesChanged)
public:
    PullRequestModel model;
    QString view = "tailored", ranking = "tailored", status = "Cached on this device · Updated just now";
    QString opened, acknowledged, fingerprint;
    bool loading = false, refreshing = false, stale = false;
    int sourceCount = 3, suppressedCount = 0;
    bool notificationsEnabled = true;
    bool trayEnabled = false, closeToTray = false, trayAttentionDot = true, trayAvailable = true;
    bool barEnabled = false;
    Q_INVOKABLE bool saveIntegrationPreferences(bool notifications, bool tray, bool background, bool dot, bool bar) {
        if (!saveSucceeds) return false;
        barEnabled = bar;
        return saveDesktopPreferences(notifications, tray, background, dot);
    }
    Q_INVOKABLE bool shouldCloseToTray() const { return trayEnabled && closeToTray && trayAvailable; }
    Q_INVOKABLE bool saveDesktopPreferences(bool notifications, bool tray, bool background, bool dot) {
        if (!saveSucceeds) return false;
        trayEnabled = tray; closeToTray = tray && background; trayAttentionDot = dot;
        return savePreferences(notifications);
    }
    bool saveSucceeds = true;
    bool notificationSucceeds = true;
    Q_INVOKABLE bool testNotification() { return notificationSucceeds; }
    Q_INVOKABLE bool savePreferences(bool enabled) {
        if (!saveSucceeds) return false;
        notificationsEnabled = enabled; emit preferencesChanged(); return true;
    }
    PullRequestModel *pullRequests() { return &model; }
    Q_INVOKABLE void refresh() {}
    Q_INVOKABLE void openUrl(const QString &url) { opened = url; }
    Q_INVOKABLE void acknowledge(const QString &id, const QString &value) { acknowledged = id; fingerprint = value; }
    Q_INVOKABLE void snooze(const QString &, const QString &, const QString &) {}
    Q_INVOKABLE void copyText(const QString &) {}
signals:
    void showWorkspaceRequested();
    void showPreferencesRequested();
    void quitRequested();
    void preferencesChanged();
    void viewChanged();
    void rankingChanged();
};

class RecordingOsIntegration final : public ReviewRadar::OsIntegration {
public:
    bool showNotification(const ReviewRadar::NotificationRequest &request) override {
        notification = request;
        return notificationSucceeds;
    }
    bool openUrl(const QUrl &value) override {
        opened = value;
        return true;
    }
    void copyText(const QString &value) override { copied = value; }
    QTemporaryDir directory;
    bool available = true, trayEnabled = false, dot = true;
    bool trayAvailable() const override { return available; }
    void configureTray(bool enabled, bool attentionDot) override { trayEnabled = enabled; dot = attentionDot; }
    void setTrayAttention(int) override {}
    bool bar = false;
    ReviewRadar::BarSnapshot snapshot;
    void configureBar(bool enabled) override { bar = enabled; }
    bool barActive() const override { return bar; }
    void publishBarSnapshot(const ReviewRadar::BarSnapshot &value) override { snapshot = value; }
    QString applicationDataFile(const QString &name) const override { return directory.filePath(name); }

    ReviewRadar::NotificationRequest notification;
    bool notificationSucceeds = true;
    QUrl opened;
    QString copied;
};

class WorkspaceTest final : public QObject {
    Q_OBJECT
private slots:
    void osIntegrationBoundary();
    void workspace();
};

QQuickItem *findItem(QQuickItem *parent, const QString &name) {
    if (parent->objectName() == name) return parent;
    for (auto *child : parent->childItems())
        if (auto *found = findItem(child, name)) return found;
    return nullptr;
}

void WorkspaceTest::osIntegrationBoundary() {
    RecordingOsIntegration osIntegration;
    QueueController controller(&osIntegration, nullptr);

    controller.openUrl("https://github.com/example/repository/pull/1");
    controller.copyText("https://github.com/example/repository/pull/1");

    QCOMPARE(osIntegration.opened,
             QUrl("https://github.com/example/repository/pull/1"));
    QCOMPARE(osIntegration.copied, QString("https://github.com/example/repository/pull/1"));
    QVERIFY(controller.notificationsEnabled());
    QVERIFY(controller.savePreferences(false));
    QueueController restarted(&osIntegration, nullptr);
    QVERIFY(!restarted.notificationsEnabled());
    QVERIFY(!restarted.trayEnabled());
    QVERIFY(restarted.saveDesktopPreferences(false, true, true, false));
    QVERIFY(restarted.shouldCloseToTray());
    QVERIFY(osIntegration.trayEnabled);
    QVERIFY(!osIntegration.dot);
    QueueController trayRestored(&osIntegration, nullptr);
    QVERIFY(trayRestored.trayEnabled());
    QVERIFY(trayRestored.closeToTray());
    QVERIFY(!trayRestored.trayAttentionDot());
    QSignalSpy restore(&trayRestored, &QueueController::showWorkspaceRequested);
    osIntegration.available = false;
    emit osIntegration.trayAvailabilityChanged();
    QVERIFY(!trayRestored.shouldCloseToTray());
    QCOMPARE(restore.count(), 1);
    QVERIFY(trayRestored.saveDesktopPreferences(false, false, true, true));
    QVERIFY(!trayRestored.closeToTray());
    QVERIFY(!osIntegration.trayEnabled);
    QVERIFY(!trayRestored.barEnabled());
    QVERIFY(trayRestored.saveIntegrationPreferences(false, false, false, true, true));
    QVERIFY(osIntegration.bar);
    QVERIFY(!osIntegration.snapshot.available);
    QCOMPARE(osIntegration.snapshot.syncState, QString("unavailable"));
    QueueController barRestored(&osIntegration, nullptr);
    QVERIFY(barRestored.barEnabled());
    QVERIFY(barRestored.barActive());
    QVERIFY(barRestored.savePreferences(true));
    QVERIFY(barRestored.barEnabled()); // Notification-only saves retain the integration.
    QVERIFY(barRestored.saveIntegrationPreferences(false, false, false, true, false));
    QVERIFY(!osIntegration.bar);
    QVERIFY(restarted.testNotification());
    QCOMPARE(osIntegration.notification.id, QString("preferences-test"));
    QVERIFY(osIntegration.notification.activationUrl.isEmpty());
    QVERIFY(osIntegration.notification.actions.isEmpty());
    QVERIFY(!restarted.notificationsEnabled());
    osIntegration.notificationSucceeds = false;
    QVERIFY(!restarted.testNotification());
    osIntegration.directory.remove();
    QVERIFY(!restarted.savePreferences(true));
    QVERIFY(!restarted.notificationsEnabled());
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
              "health":{"reviewDecision":"REVIEW_REQUIRED","checks":"passing","mergeable":"MERGEABLE"}},
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
        const auto healthSignals = queue.model.get(0).value("healthSignals").toList();
        QCOMPARE(healthSignals.size(), 3);
        QCOMPARE(healthSignals.at(0).toMap().value("label").toString(), QString("Review"));
        QCOMPARE(healthSignals.at(0).toMap().value("value").toString(), QString("Required"));
        QCOMPARE(healthSignals.at(0).toMap().value("tone").toString(), QString("caution"));
        QCOMPARE(healthSignals.at(1).toMap().value("value").toString(), QString("Passing"));
        const auto followUpSignals = queue.model.get(1).value("healthSignals").toList();
        QCOMPARE(followUpSignals.at(0).toMap().value("tone").toString(), QString("caution"));
        QCOMPARE(followUpSignals.at(1).toMap().value("tone").toString(), QString("negative"));
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

        search->setProperty("text", "WEB");
        QMetaObject::invokeMethod(search, "forceActiveFocus");
        QVERIFY(search->property("activeFocus").toBool());
        QTest::keyClick(window, Qt::Key_Escape);
        QTRY_COMPARE(search->property("text").toString(), QString());
        QVERIFY(!search->property("activeFocus").toBool());

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
        QTest::qWait(50); // Settle the list layout before hit testing.
        QTest::mouseClick(window, Qt::LeftButton, Qt::NoModifier,
                           card->mapToScene(QPointF(30, 30)).toPoint());
        QTRY_VERIFY(window->findChild<QObject *>("detail-panel"));
        auto *nextCard = findItem(window->contentItem(), "card-1");
        QVERIFY(nextCard);
        QTRY_COMPARE(card->property("selected").toBool(), true);
        QTest::keyClick(window, Qt::Key_J);
        QTRY_COMPARE(nextCard->property("selected").toBool(), true);
        QCOMPARE(card->property("selected").toBool(), false);
        QCOMPARE(card->property("keyboardActive").toBool(), false);
        auto *open = window->findChild<QQuickItem *>("detail-open");
        QVERIFY(open);
        QTest::qWait(50); // Allow the new detail layout to settle before hit testing.
        QTest::mouseClick(window, Qt::LeftButton, Qt::NoModifier,
                          open->mapToScene(QPointF(open->width() / 2, open->height() / 2)).toPoint());
        QCOMPARE(queue.opened, QString("https://github.com/example/web/pull/87"));
        QTest::keyClick(window, Qt::Key_O);
        QCOMPARE(queue.opened, QString("https://github.com/example/web/pull/87"));
        auto *read = window->findChild<QQuickItem *>("detail-read");
        QVERIFY(read);
        QTest::mouseClick(window, Qt::LeftButton, Qt::NoModifier,
                          read->mapToScene(QPointF(read->width() / 2, read->height() / 2)).toPoint());
        QCOMPARE(queue.acknowledged, QString("pr-87"));
        QCOMPARE(queue.fingerprint, QString("event-87"));
        screenshot("detail");
        window->resize(860, 640);
        QTRY_VERIFY(window->property("narrow").toBool());
        screenshot("narrow-detail");
        auto *shortcuts = window->findChild<QObject *>("shortcuts-dialog");
        QVERIFY(shortcuts);
        QVERIFY(QMetaObject::invokeMethod(shortcuts, "open"));
        QTRY_VERIFY(shortcuts->property("visible").toBool());
        QTest::qWait(50);
        QVERIFY(shortcuts->property("height").toReal() <= window->height() - 40);
        screenshot("shortcuts-modal");
        QVERIFY(QMetaObject::invokeMethod(shortcuts, "close"));
        auto *confirmation = window->findChild<QObject *>("acknowledge-dialog");
        QVERIFY(confirmation);
        QVERIFY(QMetaObject::invokeMethod(confirmation, "open"));
        QTRY_VERIFY(confirmation->property("visible").toBool());
        screenshot("confirmation-modal");
        const auto previousAcknowledgement = queue.acknowledged;
        QVERIFY(QMetaObject::invokeMethod(confirmation, "reject"));
        QCOMPARE(queue.acknowledged, previousAcknowledgement);
        auto *preferences = window->findChild<QObject *>("preferences-dialog");
        QVERIFY(preferences);
        QVERIFY(QMetaObject::invokeMethod(preferences, "open"));
        QTRY_VERIFY(preferences->property("opened").toBool());
        screenshot("preferences-narrow");
        window->resize(1440, 1000);
        QTest::qWait(50);
        screenshot("preferences");
        QTest::keyClick(window, Qt::Key_Space);
        QTRY_VERIFY(!preferences->property("draftNotifications").toBool());
        QVERIFY(queue.notificationsEnabled); // Draft has no effect until Save.
        auto *testNotification = window->findChild<QObject *>("test-notification");
        QVERIFY(testNotification);
        QVERIFY(QMetaObject::invokeMethod(testNotification, "clicked"));
        QVERIFY(preferences->property("testSucceeded").toBool());
        QVERIFY(!preferences->property("testMessage").toString().isEmpty());
        queue.notificationSucceeds = false;
        QVERIFY(QMetaObject::invokeMethod(testNotification, "clicked"));
        QVERIFY(!preferences->property("testSucceeded").toBool());
        QVERIFY(preferences->property("dirty").toBool());
        QVERIFY(queue.notificationsEnabled);
        preferences->setProperty("section", "Desktop integration");
        screenshot("preferences-integrations");
        QVERIFY(preferences->property("dirty").toBool());
        preferences->setProperty("section", "Notifications");
        QVERIFY(!preferences->property("draftNotifications").toBool());
        auto *save = window->findChild<QObject *>("save-preferences");
        QVERIFY(save);
        queue.saveSucceeds = false;
        QVERIFY(QMetaObject::invokeMethod(save, "clicked"));
        QVERIFY(preferences->property("visible").toBool());
        QVERIFY(!preferences->property("errorMessage").toString().isEmpty());
        QVERIFY(queue.notificationsEnabled);
        queue.saveSucceeds = true;
        QVERIFY(QMetaObject::invokeMethod(save, "clicked"));
        QTRY_VERIFY(!preferences->property("visible").toBool());
        QVERIFY(!queue.notificationsEnabled);
        const auto notificationAction = ReviewRadar::NotificationAction{
            "next-action", "Start review", QUrl("https://github.com/example/api/pull/142/files")};
        QCOMPARE(notificationAction.label, QString("Start review"));
        QVERIFY(!notificationAction.activationUrl.isEmpty());
        QVERIFY(QMetaObject::invokeMethod(preferences, "open"));
        QTRY_VERIFY(preferences->property("opened").toBool());
        QVERIFY(!preferences->property("draftNotifications").toBool());
        QTest::keyClick(window, Qt::Key_Escape);
        QTRY_VERIFY(!preferences->property("visible").toBool());
        QVERIFY(QMetaObject::invokeMethod(preferences, "open"));
        QTRY_VERIFY(preferences->property("opened").toBool());
        QTest::keyClick(window, Qt::Key_Space);
        QTest::keyClick(window, Qt::Key_Escape);
        auto *discard = window->findChild<QObject *>("discard-preferences");
        QVERIFY(discard);
        QTRY_VERIFY(discard->property("visible").toBool());
        QVERIFY(QMetaObject::invokeMethod(discard, "accept"));
        QTRY_VERIFY(!preferences->property("visible").toBool());
        QVERIFY(!queue.notificationsEnabled);
        // A cache update retains selection by identity and refreshes its data.
        queue.trayEnabled = true;
        queue.closeToTray = true;
        window->close();
        QTRY_VERIFY(!window->isVisible());
        emit queue.showWorkspaceRequested();
        QTRY_VERIFY(window->isVisible());
        emit queue.showPreferencesRequested();
        QTRY_VERIFY(preferences->property("opened").toBool());
        preferences->setProperty("section", "Desktop integration");
        QVERIFY(preferences->property("draftTray").toBool());
        QVERIFY(preferences->property("draftCloseToTray").toBool());
        screenshot("preferences-integrations");
        auto *traySetting = window->findChild<QObject *>("tray-setting");
        QVERIFY(traySetting);
        QVERIFY(QMetaObject::invokeMethod(traySetting, "changed", Q_ARG(bool, false)));
        QVERIFY(!preferences->property("draftTray").toBool());
        QVERIFY(!preferences->property("draftCloseToTray").toBool());
        auto *barSetting = window->findChild<QObject *>("bar-setting");
        QVERIFY(barSetting);
        QVERIFY(QMetaObject::invokeMethod(barSetting, "changed", Q_ARG(bool, true)));
        QVERIFY(preferences->property("draftBar").toBool());
        QVERIFY(!queue.barEnabled);
        QVERIFY(QMetaObject::invokeMethod(save, "clicked"));
        QVERIFY(queue.barEnabled);
        QTRY_VERIFY(!preferences->property("visible").toBool());
        QVERIFY(!queue.shouldCloseToTray());
        auto replacement = fixture.array();
        auto selected = replacement.at(1).toObject();
        selected["title"] = "Updated keyboard navigation title";
        replacement[1] = selected;
        queue.model.replace(replacement);
        QTRY_COMPARE(window->findChild<QObject *>("detail-panel")->property("entry").toMap().value("title").toString(), QString("Updated keyboard navigation title"));
        queue.model.replace({});
        QTRY_COMPARE(window->property("matchingCount").toInt(), 0);
        QTRY_VERIFY(!window->findChild<QObject *>("detail-panel"));
        QVERIFY2(warnings.isEmpty(), qPrintable(warnings.join('\n')));
}

int main(int argc, char **argv) {
    QApplication app(argc, argv);
    QQuickStyle::setStyle("Basic");
    WorkspaceTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "workspace.moc"
