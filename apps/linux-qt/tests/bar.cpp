#include "linuxosintegration.h"

#include <QApplication>
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QProcess>
#include <QSignalSpy>
#include <QStandardPaths>
#include <QTest>

class BarTest final : public QObject {
    Q_OBJECT
private slots:
    void statusAndCommands() {
        ReviewRadar::LinuxOsIntegration os;
        QVERIFY(!os.barActive());
        auto competitor = QDBusConnection::connectToBus(QDBusConnection::SessionBus, "competing-app");
        QVERIFY(competitor.registerService("org.reviewradar.App"));
        os.configureBar(true);
        QVERIFY(!os.barActive());
        QVERIFY(competitor.unregisterService("org.reviewradar.App"));
        QDBusConnection::disconnectFromBus("competing-app");
        os.configureBar(true);
        QVERIFY(os.barActive());
        os.publishBarSnapshot({true, "action", 3, "2026-09-22T12:00:00Z", "syncing"});
        const QStringList base{"--user", "--auto-start=no", "--json=short", "call",
                               "org.reviewradar.App", "/org/reviewradar/Bar", "org.reviewradar.Bar1"};
        QProcess client;
        auto request = [&](const QString &method) { client.start("busctl", base + QStringList{method}); };
        request("GetSnapshot");
        QTRY_COMPARE(client.state(), QProcess::NotRunning);
        QCOMPARE(client.exitCode(), 0);
        const auto envelope = QJsonDocument::fromJson(client.readAllStandardOutput()).object();
        const auto payload = QJsonDocument::fromJson(envelope["data"].toArray().first().toString().toUtf8()).object();
        QCOMPARE(payload["version"].toInt(), 1);
        QCOMPARE(payload["attentionCount"].toInt(), 3);
        QCOMPARE(payload["workspace"].toString(), QString("action"));
        QCOMPARE(payload["syncState"].toString(), QString("syncing"));
        QCOMPARE(payload.size(), 6);
        QSignalSpy open(&os, &ReviewRadar::OsIntegration::showWorkspaceRequested);
        QSignalSpy preferences(&os, &ReviewRadar::OsIntegration::showPreferencesRequested);
        QSignalSpy refresh(&os, &ReviewRadar::OsIntegration::refreshRequested);
        request("OpenWorkspace");
        QTRY_COMPARE(open.count(), 1);
        QTRY_COMPARE(client.state(), QProcess::NotRunning);
        request("OpenPreferences");
        QTRY_COMPARE(preferences.count(), 1);
        QTRY_COMPARE(client.state(), QProcess::NotRunning);
        request("Refresh");
        QTRY_COMPARE(refresh.count(), 1);
        QTRY_COMPARE(client.state(), QProcess::NotRunning);
        if (!QStandardPaths::findExecutable("quickshell").isEmpty()) {
            // After the component refreshes, withdraw the interface and verify
            // that its cached count is replaced by an unavailable state.
            const auto disconnectAfterRefresh = connect(&os, &ReviewRadar::OsIntegration::refreshRequested,
                                                        &os, [&os] { os.configureBar(false); });
            QProcess quickshell;
            auto environment = QProcessEnvironment::systemEnvironment();
            environment.insert("QT_QUICK_CONTROLS_STYLE", "Basic");
            environment.remove("QT_STYLE_OVERRIDE");
            quickshell.setProcessEnvironment(environment);
            quickshell.start("quickshell", {"--path", QUICKSHELL_SMOKE_CONFIG, "--no-color"});
            QTRY_COMPARE_WITH_TIMEOUT(quickshell.state(), QProcess::NotRunning, 10000);
            QVERIFY2(quickshell.exitCode() == 0, qPrintable(QString::fromUtf8(quickshell.readAllStandardOutput()) + QString::fromUtf8(quickshell.readAllStandardError())));
            QCOMPARE(refresh.count(), 2);
            disconnect(disconnectAfterRefresh);
        }
        os.configureBar(false);
        QVERIFY(!os.barActive());
        request("GetSnapshot");
        QTRY_COMPARE(client.state(), QProcess::NotRunning);
        QVERIFY(client.exitCode() != 0);
        os.configureBar(true);
        QVERIFY(os.barActive());
    }
};

int main(int argc, char **argv) {
    QApplication app(argc, argv);
    BarTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "bar.moc"
