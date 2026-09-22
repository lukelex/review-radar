#include "linuxosintegration.h"

#include <QClipboard>
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusMessage>
#include <QDBusReply>
#include <QDesktopServices>
#include <QDir>
#include <QGuiApplication>
#include <QStandardPaths>
#include <QVariant>

namespace ReviewRadar {

LinuxOsIntegration::LinuxOsIntegration(QObject *parent) : OsIntegration(parent) {
    QDBusConnection::sessionBus().connect(
        "org.freedesktop.Notifications", "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications", "ActionInvoked", this,
        SLOT(notificationActionInvoked(uint,QString)));
}

bool LinuxOsIntegration::showNotification(const NotificationRequest &request) {
    QDBusInterface notifications("org.freedesktop.Notifications", "/org/freedesktop/Notifications",
                                 "org.freedesktop.Notifications", QDBusConnection::sessionBus());
    if (!notifications.isValid()) return false;

    const QStringList actions{"open", "Open pull request"};
    QVariantMap hints{{"desktop-entry", "review-radar-linux"}};
    QDBusReply<uint> reply = notifications.call(
        "Notify", "Review Radar", notificationIds_.value(request.id), QString(), request.title,
        request.body, actions, hints, -1);
    if (!reply.isValid()) return false;

    notificationIds_.insert(request.id, reply.value());
    notificationUrls_.insert(reply.value(), request.activationUrl);
    return true;
}

bool LinuxOsIntegration::openUrl(const QUrl &url) {
    const auto bus = QDBusConnection::sessionBus();
    if (bus.isConnected()) {
        QDBusMessage request = QDBusMessage::createMethodCall(
            "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.OpenURI", "OpenURI");
        request << QString() << url.toString() << QVariant::fromValue(QVariantMap{});
        if (bus.call(request, QDBus::AutoDetect, 3000).type() == QDBusMessage::ReplyMessage) return true;
    }
    return QDesktopServices::openUrl(url);
}

void LinuxOsIntegration::copyText(const QString &text) {
    QGuiApplication::clipboard()->setText(text);
}

QString LinuxOsIntegration::applicationDataFile(const QString &name) const {
    const auto dataRoot = QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation);
    const auto directory = QDir(dataRoot).filePath(QStringLiteral("review-radar"));
    QDir().mkpath(directory);
    return QDir(directory).filePath(name);
}

void LinuxOsIntegration::notificationActionInvoked(uint notificationId, const QString &action) {
    if (action != "open" || !notificationUrls_.contains(notificationId)) return;
    emit notificationActivated(notificationUrls_.take(notificationId));
}

} // namespace ReviewRadar
