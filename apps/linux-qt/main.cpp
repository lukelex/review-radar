#include <QGuiApplication>
#include <QCoreApplication>
#include <QIcon>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QTimer>

#include "queuecontroller.h"

int main(int argc, char *argv[]) {
    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("Review Radar"));
    app.setOrganizationName(QStringLiteral("Review Radar"));
    app.setWindowIcon(QIcon(QStringLiteral(":/assets/logo.svg")));

    QueueController queue;
    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty(QStringLiteral("queue"), &queue);
    engine.load(QUrl(QStringLiteral("qrc:/ReviewRadar/qml/Main.qml")));
    if (engine.rootObjects().isEmpty()) {
        return 1;
    }
    if (QCoreApplication::arguments().contains(QStringLiteral("--smoke-test"))) {
        QTimer::singleShot(0, &app, [&app] { app.quit(); });
    } else {
        queue.start();
    }
    return app.exec();
}
