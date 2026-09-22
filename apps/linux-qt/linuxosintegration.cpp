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
#include <QPainter>
#include <QJsonDocument>
#include <QJsonObject>

namespace ReviewRadar {

LinuxOsIntegration::LinuxOsIntegration(QObject *parent) : OsIntegration(parent) {
    connect(&barService_, &BarService::openRequested, this, &OsIntegration::showWorkspaceRequested);
    connect(&barService_, &BarService::preferencesRequested, this, &OsIntegration::showPreferencesRequested);
    connect(&barService_, &BarService::refreshRequested, this, &OsIntegration::refreshRequested);
    trayAvailable_ = QSystemTrayIcon::isSystemTrayAvailable();
    trayMenu_.addAction("Open Review Radar", this, &OsIntegration::showWorkspaceRequested);
    trayMenu_.addAction("Refresh", this, &OsIntegration::refreshRequested);
    trayMenu_.addAction("Preferences", this, &OsIntegration::showPreferencesRequested);
    trayMenu_.addSeparator();
    trayMenu_.addAction("Quit", this, &OsIntegration::quitRequested);
    tray_.setContextMenu(&trayMenu_);
    connect(&tray_, &QSystemTrayIcon::activated, this, [this](QSystemTrayIcon::ActivationReason reason) {
        if (reason == QSystemTrayIcon::Trigger || reason == QSystemTrayIcon::DoubleClick)
            emit showWorkspaceRequested();
    });
    trayMonitor_.setInterval(2000);
    connect(&trayMonitor_, &QTimer::timeout, this, [this] {
        const bool available = QSystemTrayIcon::isSystemTrayAvailable();
        if (available == trayAvailable_) return;
        trayAvailable_ = available;
        updateTray();
        emit trayAvailabilityChanged();
    });
    trayMonitor_.start();
    QDBusConnection::sessionBus().connect(
        "org.freedesktop.Notifications", "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications", "ActionInvoked", this,
        SLOT(notificationActionInvoked(uint,QString)));
}

bool LinuxOsIntegration::trayAvailable() const { return QSystemTrayIcon::isSystemTrayAvailable(); }

LinuxOsIntegration::~LinuxOsIntegration() { configureBar(false); }

void LinuxOsIntegration::configureBar(bool enabled) {
    auto bus = QDBusConnection::sessionBus();
    if (!enabled) {
        if (barActive_) {
            bus.unregisterObject("/org/reviewradar/Bar");
            bus.unregisterService("org.reviewradar.App");
        }
        barActive_ = false;
        return;
    }
    if (barActive_ || !bus.isConnected()) return;
    if (!bus.registerService("org.reviewradar.App")) return;
    if (!bus.registerObject("/org/reviewradar/Bar", &barService_, QDBusConnection::ExportAllSlots)) {
        bus.unregisterService("org.reviewradar.App");
        return;
    }
    barActive_ = true;
}

void LinuxOsIntegration::publishBarSnapshot(const BarSnapshot &snapshot) {
    barService_.snapshot = QString::fromUtf8(QJsonDocument(QJsonObject{
        {"version", 1}, {"available", snapshot.available}, {"workspace", snapshot.workspace},
        {"attentionCount", snapshot.attentionCount}, {"capturedAt", snapshot.capturedAt},
        {"syncState", snapshot.syncState}
    }).toJson(QJsonDocument::Compact));
}

void LinuxOsIntegration::configureTray(bool enabled, bool attentionDot) {
    trayEnabled_ = enabled;
    attentionDot_ = attentionDot;
    updateTray();
}

void LinuxOsIntegration::setTrayAttention(int count) {
    attentionCount_ = count;
    updateTray();
}

void LinuxOsIntegration::updateTray() {
    QPixmap icon = QIcon(":/assets/logo.svg").pixmap(32, 32);
    if (attentionDot_ && attentionCount_ > 0) {
        QPainter painter(&icon);
        painter.setRenderHint(QPainter::Antialiasing);
        painter.setPen(QPen(Qt::white, 2));
        painter.setBrush(QColor("#94631b"));
        painter.drawEllipse(QRectF(21, 21, 9, 9));
    }
    tray_.setIcon(QIcon(icon));
    tray_.setToolTip(QString("Review Radar — %1 needing attention in the current workspace").arg(attentionCount_));
    tray_.setVisible(trayEnabled_ && trayAvailable());
}

bool LinuxOsIntegration::showNotification(const NotificationRequest &request) {
    QDBusInterface notifications("org.freedesktop.Notifications", "/org/freedesktop/Notifications",
                                 "org.freedesktop.Notifications", QDBusConnection::sessionBus());
    if (!notifications.isValid()) return false;

    QStringList actions;
    for (const auto &action : request.actions) {
        if (!action.id.isEmpty() && !action.label.isEmpty() && !action.activationUrl.isEmpty()) {
            actions.append(action.id);
            actions.append(action.label);
        }
    }
    QVariantMap hints{{"desktop-entry", "review-radar-linux"}};
    QDBusReply<uint> reply = notifications.call(
        "Notify", "Review Radar", notificationIds_.value(request.id), "review-radar-linux", request.title,
        request.body, actions, hints, -1);
    if (!reply.isValid()) return false;

    notificationIds_.insert(request.id, reply.value());
    QHash<QString, QUrl> actionUrls;
    for (const auto &action : request.actions) {
        if (!action.id.isEmpty() && !action.activationUrl.isEmpty())
            actionUrls.insert(action.id, action.activationUrl);
    }
    notificationActions_.insert(reply.value(), actionUrls);
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
    const auto notification = notificationActions_.value(notificationId);
    if (!notification.contains(action)) return;
    emit notificationActivated(notification.value(action));
}

} // namespace ReviewRadar
