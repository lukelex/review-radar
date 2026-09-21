#include <QGuiApplication>
#include <QCoreApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QTimer>

#include "queuecontroller.h"

int main(int argc, char *argv[]) {
    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("Review Radar"));
    app.setOrganizationName(QStringLiteral("Review Radar"));

    QueueController queue;
    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty(QStringLiteral("queue"), &queue);
    engine.loadFromModule("ReviewRadar", "Main");
    if (engine.rootObjects().isEmpty()) {
        return 1;
    }
    if (QCoreApplication::arguments().contains(QStringLiteral("--smoke-test"))) {
        QTimer::singleShot(0, &app, &QCoreApplication::quit);
    } else {
        queue.refresh();
    }
    return app.exec();
}
