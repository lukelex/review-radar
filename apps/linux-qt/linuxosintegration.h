#pragma once

#include "osintegration.h"

#include <QHash>

namespace ReviewRadar {

class LinuxOsIntegration final : public OsIntegration {
    Q_OBJECT

public:
    explicit LinuxOsIntegration(QObject *parent = nullptr);

    bool showNotification(const NotificationRequest &request) override;
    bool openUrl(const QUrl &url) override;
    void copyText(const QString &text) override;
    QString applicationDataFile(const QString &name) const override;

private slots:
    void notificationActionInvoked(uint notificationId, const QString &action);

private:
    QHash<QString, uint> notificationIds_;
    QHash<uint, QHash<QString, QUrl>> notificationActions_;
};

} // namespace ReviewRadar
