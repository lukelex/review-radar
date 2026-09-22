#pragma once

#include "osintegration.h"

#include <QHash>
#include <QSystemTrayIcon>
#include <QMenu>
#include <QTimer>

namespace ReviewRadar {

class LinuxOsIntegration final : public OsIntegration {
    Q_OBJECT

public:
    explicit LinuxOsIntegration(QObject *parent = nullptr);

    bool showNotification(const NotificationRequest &request) override;
    bool openUrl(const QUrl &url) override;
    void copyText(const QString &text) override;
    QString applicationDataFile(const QString &name) const override;
    bool trayAvailable() const override;
    void configureTray(bool enabled, bool attentionDot) override;
    void setTrayAttention(int count) override;

private slots:
    void notificationActionInvoked(uint notificationId, const QString &action);

private:
    void updateTray();
    QSystemTrayIcon tray_;
    QMenu trayMenu_;
    QTimer trayMonitor_;
    bool trayAvailable_ = false;
    bool trayEnabled_ = false;
    bool attentionDot_ = true;
    int attentionCount_ = 0;
    QHash<QString, uint> notificationIds_;
    QHash<uint, QHash<QString, QUrl>> notificationActions_;
};

} // namespace ReviewRadar
