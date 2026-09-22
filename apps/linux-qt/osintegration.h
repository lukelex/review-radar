#pragma once

#include <QObject>
#include <QString>
#include <QUrl>

namespace ReviewRadar {

struct NotificationRequest {
    QString id;
    QString title;
    QString body;
    QUrl activationUrl;
};

// Boundary for operating-system effects used by a native shell. The controller
// depends only on this contract; platform and toolkit integrations live behind it.
class OsIntegration : public QObject {
    Q_OBJECT

public:
    using QObject::QObject;
    ~OsIntegration() override = default;

    virtual bool showNotification(const NotificationRequest &request) = 0;
    virtual bool openUrl(const QUrl &url) = 0;
    virtual void copyText(const QString &text) = 0;
    virtual QString applicationDataFile(const QString &name) const = 0;

signals:
    void notificationActivated(const QUrl &url);
};

} // namespace ReviewRadar
