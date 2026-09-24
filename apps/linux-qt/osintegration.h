#pragma once

#include <QObject>
#include <QList>
#include <QString>
#include <QUrl>

namespace ReviewRadar {

struct NotificationAction {
    QString id;
    QString label;
    QUrl activationUrl;
};

struct NotificationRequest {
    QString id;
    QString title;
    QString body;
    QUrl activationUrl;
    QList<NotificationAction> actions;
};

struct BarSnapshot {
    bool available = false;
    QString workspace;
    int attentionCount = 0;
    QString capturedAt;
    QString syncState = "unavailable";
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
    virtual bool trayAvailable() const = 0;
    virtual void configureTray(bool enabled, bool attentionDot) = 0;
    virtual void setTrayAttention(int count) = 0;
    virtual void configureBar(bool enabled) = 0;
    virtual bool barActive() const = 0;
    virtual void publishBarSnapshot(const BarSnapshot &snapshot) = 0;
    virtual QString githubToken() const { return {}; }
    virtual bool hasGithubToken() const { return false; }
    virtual bool saveGithubToken(const QString &) { return false; }
    virtual bool clearGithubToken() { return false; }

signals:
    void notificationActivated(const QUrl &url);
    void trayAvailabilityChanged();
    void showWorkspaceRequested();
    void showPreferencesRequested();
    void refreshRequested();
    void quitRequested();
};

} // namespace ReviewRadar
