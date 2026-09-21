#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>

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
    queue.refresh();
    return app.exec();
}
