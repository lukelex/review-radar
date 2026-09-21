#pragma once

#include <QAbstractListModel>
#include <QHash>
#include <QJsonArray>
#include <QProcess>
#include <QTimer>
#include <QVariantList>

class PullRequestModel final : public QAbstractListModel {
    Q_OBJECT

public:
    enum Role {
        IdRole = Qt::UserRole + 1, RepositoryRole, NumberRole, TitleRole, UrlRole,
        ActionLabelRole, AttentionRequiredRole, FingerprintRole, ExplanationHeadingRole,
        ReasonRole, NextActionLabelRole, NextActionUrlRole, FrictionStatusRole,
        FrictionLevelRole, EventsRole,
    };

    explicit PullRequestModel(QObject *parent = nullptr);
    int rowCount(const QModelIndex &parent = {}) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;
    void replace(const QJsonArray &cards);

private:
    struct Card {
        QString id, repository, title, url, actionLabel, fingerprint, explanationHeading, reason;
        QString nextActionLabel, nextActionUrl, frictionStatus, frictionLevel;
        int number = 0;
        bool attentionRequired = false;
        QVariantList events;
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
    Q_PROPERTY(int sourceCount READ sourceCount NOTIFY countsChanged)
    Q_PROPERTY(int suppressedCount READ suppressedCount NOTIFY countsChanged)

public:
    explicit QueueController(QObject *parent = nullptr);
    PullRequestModel *pullRequests();
    QString view() const;
    void setView(const QString &view);
    QString ranking() const;
    void setRanking(const QString &ranking);
    QString status() const;
    bool loading() const;
    int sourceCount() const;
    int suppressedCount() const;

    Q_INVOKABLE void refresh();
    Q_INVOKABLE void openUrl(const QString &url);
    Q_INVOKABLE void copyText(const QString &text);
    Q_INVOKABLE void acknowledge(const QString &pullRequestId, const QString &fingerprint);
    Q_INVOKABLE void snooze(const QString &pullRequestId, const QString &fingerprint,
                            const QString &preset);

signals:
    void viewChanged();
    void rankingChanged();
    void statusChanged();
    void loadingChanged();
    void countsChanged();

private:
    QString applicationDataFile(const QString &name) const;
    QString captureDatabase() const;
    QString commandFromEnvironment(const char *name, const QString &fallback) const;
    void loadProjection();
    void runStateCommand(const QStringList &arguments);
    void sendNotifications(const QJsonArray &cards, const QJsonArray &eligibleIds);
    void setStatus(const QString &status);

private slots:
    void notificationActionInvoked(uint notificationId, const QString &action);

private:
    PullRequestModel model_;
    QString view_ = QStringLiteral("tailored");
    QString ranking_ = QStringLiteral("tailored");
    QString status_ = QStringLiteral("Loading workspace…");
    bool loading_ = false;
    int sourceCount_ = 0;
    int suppressedCount_ = 0;
    QProcess queueProcess_;
    QProcess collectorProcess_;
    QProcess stateProcess_;
    QTimer refreshTimer_;
    QHash<uint, QString> notificationUrls_;
};
