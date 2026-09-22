#pragma once

#include <QAbstractListModel>
#include <QHash>
#include <QJsonArray>
#include <QProcess>
#include <QTimer>
#include <QVariantList>

#include "osintegration.h"

class PullRequestModel final : public QAbstractListModel {
    Q_OBJECT

public:
    enum Role {
        IdRole = Qt::UserRole + 1, RepositoryRole, NumberRole, TitleRole, UrlRole,
        ActionLabelRole, AttentionRequiredRole, FingerprintRole, ExplanationHeadingRole,
        ReasonsRole, HealthRole, HealthSignalsRole, NextActionLabelRole, NextActionUrlRole, FrictionStatusRole,
        FrictionLevelRole, FrictionDetailRole, EventsRole,
        LifecycleRole, UpdatedAtRole,
    };

    explicit PullRequestModel(QObject *parent = nullptr);
    int rowCount(const QModelIndex &parent = {}) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;
    void replace(const QJsonArray &cards);
    Q_INVOKABLE QVariantMap get(int index) const;
    Q_INVOKABLE int matchingCount(const QString &search) const;

private:
    struct Card {
        QString id, repository, title, url, actionLabel, fingerprint, explanationHeading, health;
        QVariantList healthSignals;
        QString nextActionLabel, nextActionUrl, frictionStatus, frictionLevel, frictionDetail;
        int number = 0;
        bool attentionRequired = false;
        QStringList reasons;
        QVariantList events;
        QString lifecycle, updatedAt;
    };
    QList<Card> cards_;
};

class QueueController final : public QObject {
    Q_OBJECT
    Q_PROPERTY(PullRequestModel *pullRequests READ pullRequests CONSTANT)
    Q_PROPERTY(QString view READ view WRITE setView NOTIFY viewChanged)
    Q_PROPERTY(QString ranking READ ranking WRITE setRanking NOTIFY rankingChanged)
    Q_PROPERTY(QString status READ status NOTIFY statusChanged)
    Q_PROPERTY(bool loading READ loading NOTIFY loadingChanged)
    Q_PROPERTY(bool refreshing READ refreshing NOTIFY refreshingChanged)
    Q_PROPERTY(bool stale READ stale NOTIFY staleChanged)
    Q_PROPERTY(bool controlHeld READ controlHeld NOTIFY controlHeldChanged)
    Q_PROPERTY(int sourceCount READ sourceCount NOTIFY countsChanged)
    Q_PROPERTY(int suppressedCount READ suppressedCount NOTIFY countsChanged)
    Q_PROPERTY(bool notificationsEnabled READ notificationsEnabled NOTIFY preferencesChanged)
    Q_PROPERTY(bool trayEnabled READ trayEnabled NOTIFY preferencesChanged)
    Q_PROPERTY(bool closeToTray READ closeToTray NOTIFY preferencesChanged)
    Q_PROPERTY(bool trayAttentionDot READ trayAttentionDot NOTIFY preferencesChanged)
    Q_PROPERTY(bool trayAvailable READ trayAvailable NOTIFY trayAvailabilityChanged)
    Q_PROPERTY(bool barEnabled READ barEnabled NOTIFY preferencesChanged)
    Q_PROPERTY(bool barActive READ barActive NOTIFY preferencesChanged)

public:
    explicit QueueController(QObject *parent = nullptr);
    QueueController(ReviewRadar::OsIntegration *osIntegration, QObject *parent);
    PullRequestModel *pullRequests();
    QString view() const;
    void setView(const QString &view);
    QString ranking() const;
    void setRanking(const QString &ranking);
    QString status() const;
    bool loading() const;
    bool refreshing() const;
    bool stale() const;
    bool controlHeld() const;
    int sourceCount() const;
    int suppressedCount() const;
    bool notificationsEnabled() const { return notificationsEnabled_; }
    Q_INVOKABLE bool savePreferences(bool notificationsEnabled);
    Q_INVOKABLE bool saveDesktopPreferences(bool notificationsEnabled, bool trayEnabled, bool closeToTray, bool attentionDot);
    Q_INVOKABLE bool saveIntegrationPreferences(bool notificationsEnabled, bool trayEnabled, bool closeToTray, bool attentionDot, bool barEnabled);
    bool barEnabled() const { return barEnabled_; }
    bool barActive() const { return osIntegration_->barActive(); }
    bool trayEnabled() const { return trayEnabled_; }
    bool closeToTray() const { return closeToTray_; }
    bool trayAttentionDot() const { return trayAttentionDot_; }
    bool trayAvailable() const { return osIntegration_->trayAvailable(); }
    Q_INVOKABLE bool shouldCloseToTray() const { return trayEnabled_ && closeToTray_ && trayAvailable(); }
    Q_INVOKABLE bool testNotification();

    Q_INVOKABLE void refresh();
    Q_INVOKABLE void start();
    Q_INVOKABLE void openUrl(const QString &url);
    Q_INVOKABLE void copyText(const QString &text);
    Q_INVOKABLE void acknowledge(const QString &pullRequestId, const QString &fingerprint);
    Q_INVOKABLE void snooze(const QString &pullRequestId, const QString &fingerprint,
                            const QString &preset);

signals:
    void trayAvailabilityChanged();
    void showWorkspaceRequested();
    void showPreferencesRequested();
    void quitRequested();
    void preferencesChanged();
    void viewChanged();
    void rankingChanged();
    void statusChanged();
    void loadingChanged();
    void refreshingChanged();
    void staleChanged();
    void controlHeldChanged();
    void countsChanged();

private:
    QString applicationDataFile(const QString &name) const;
    QString captureDatabase() const;
    QString stateDatabase() const;
    QString commandFromEnvironment(const char *name, const QString &fallback) const;
    void initialize();
    void publishBarSnapshot();
    void loadProjection(bool collectAfter = false);
    void startCollection();
    void runStateCommand(const QStringList &arguments);
    void sendNotifications(const QJsonArray &cards, const QJsonArray &eligibleIds);
    void setStatus(const QString &status);

private:
    bool notificationsEnabled_ = true;
    bool trayEnabled_ = false;
    bool closeToTray_ = false;
    bool trayAttentionDot_ = true;
    bool barEnabled_ = false;
    bool barError_ = false;
    ReviewRadar::BarSnapshot barSnapshot_;
    QString requestedView_;
    PullRequestModel model_;
    QString view_ = QStringLiteral("tailored");
    QString ranking_ = QStringLiteral("tailored");
    QString status_ = QStringLiteral("Loading workspace…");
    bool loading_ = false;
    bool refreshing_ = false;
    bool stale_ = false;
    bool collectAfterProjection_ = false;
    int sourceCount_ = 0;
    int suppressedCount_ = 0;
    QProcess queueProcess_;
    QProcess collectorProcess_;
    QProcess stateProcess_;
    QTimer refreshTimer_;
    QTimer modifierTimer_;
    bool controlHeld_ = false;
    ReviewRadar::OsIntegration *osIntegration_;
};
