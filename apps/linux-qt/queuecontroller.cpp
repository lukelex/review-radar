#include "queuecontroller.h"

#include <QClipboard>
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusReply>
#include <QDesktopServices>
#include <QDir>
#include <QGuiApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QStandardPaths>
#include <QTime>
#include <QTimeZone>
#include <QUrl>

namespace {
QVariantList eventList(const QJsonArray &source) {
    QVariantList result;
    for (const auto &event : source) result.append(event.toObject().toVariantMap());
    return result;
}

QStringList reasonList(const QJsonArray &source) {
    QStringList result;
    for (const auto &reason : source) result.append(reason.toObject().value("summary").toString());
    return result;
}

QString healthSummary(const QJsonObject &health) {
    QStringList healthSignals;
    const auto review = health.value("reviewDecision").toString();
    const auto checks = health.value("checks").toString();
    if (!review.isEmpty()) healthSignals.append("Review: " + review);
    else if (health.value("isDraft").toBool()) healthSignals.append("Draft");
    if (!checks.isEmpty()) healthSignals.append("Checks: " + checks);
    const auto mergeable = health.value("mergeable").toString();
    if (!mergeable.isEmpty()) healthSignals.append("Merge: " + mergeable);
    return healthSignals.join(" · ");
}

QString frictionSummary(const QJsonObject &friction) {
    QStringList facts;
    for (const auto &contributor : friction.value("contributors").toArray()) {
        const auto object = contributor.toObject();
        facts.append(object.value("signal").toString() + ": "
                     + QString::number(object.value("value").toVariant().toLongLong()) + " "
                     + object.value("unit").toString());
    }
    for (const auto &limitation : friction.value("limitations").toArray())
        facts.append(limitation.toString().replace('-', ' '));
    return facts.join(" · ");
}

QDateTime snoozeUntil(const QString &preset) {
    const auto now = QDateTime::currentDateTimeUtc();
    if (preset == "tomorrow") return QDateTime(now.date().addDays(1), QTime(9, 0), QTimeZone::utc());
    if (preset == "next-week") return QDateTime(now.date().addDays(7), QTime(9, 0), QTimeZone::utc());
    return now.addSecs(4 * 60 * 60);
}
} // namespace

PullRequestModel::PullRequestModel(QObject *parent) : QAbstractListModel(parent) {}

int PullRequestModel::rowCount(const QModelIndex &parent) const {
    return parent.isValid() ? 0 : cards_.size();
}

QVariant PullRequestModel::data(const QModelIndex &index, int role) const {
    if (!index.isValid() || index.row() < 0 || index.row() >= cards_.size()) return {};
    const auto &card = cards_.at(index.row());
    switch (role) {
    case IdRole: return card.id;
    case RepositoryRole: return card.repository;
    case NumberRole: return card.number;
    case TitleRole: return card.title;
    case UrlRole: return card.url;
    case ActionLabelRole: return card.actionLabel;
    case AttentionRequiredRole: return card.attentionRequired;
    case FingerprintRole: return card.fingerprint;
    case ExplanationHeadingRole: return card.explanationHeading;
    case ReasonsRole: return card.reasons;
    case HealthRole: return card.health;
    case NextActionLabelRole: return card.nextActionLabel;
    case NextActionUrlRole: return card.nextActionUrl;
    case FrictionStatusRole: return card.frictionStatus;
    case FrictionLevelRole: return card.frictionLevel;
    case FrictionDetailRole: return card.frictionDetail;
    case EventsRole: return card.events;
    default: return {};
    }
}

QHash<int, QByteArray> PullRequestModel::roleNames() const {
    return {{IdRole, "pullRequestId"}, {RepositoryRole, "repository"}, {NumberRole, "number"},
            {TitleRole, "title"}, {UrlRole, "url"}, {ActionLabelRole, "actionLabel"},
            {AttentionRequiredRole, "attentionRequired"}, {FingerprintRole, "currentFingerprint"},
             {ExplanationHeadingRole, "explanationHeading"}, {ReasonsRole, "reasons"}, {HealthRole, "health"},
            {NextActionLabelRole, "nextActionLabel"}, {NextActionUrlRole, "nextActionUrl"},
             {FrictionStatusRole, "frictionStatus"}, {FrictionLevelRole, "frictionLevel"},
             {FrictionDetailRole, "frictionDetail"},
            {EventsRole, "events"}};
}

void PullRequestModel::replace(const QJsonArray &cards) {
    beginResetModel();
    cards_.clear();
    for (const auto &value : cards) {
        const auto object = value.toObject();
        const auto explanation = object.value("explanation").toObject();
        const auto reasons = explanation.value("reasons").toArray();
        const auto reason = reasons.isEmpty() ? QJsonObject{} : reasons.first().toObject();
        const auto nextAction = reason.value("nextAction").toObject();
        const auto friction = object.value("reviewFriction").toObject();
        cards_.append({object.value("id").toString(), object.value("repository").toString(),
                       object.value("title").toString(), object.value("url").toString(),
                       object.value("actionLabel").toString(), object.value("currentFingerprint").toString(),
                        explanation.value("heading").toString(), healthSummary(explanation.value("health").toObject()),
                         nextAction.value("label").toString(), nextAction.value("url").toString(),
                         friction.value("status").toString(), friction.value("level").toString(),
                         frictionSummary(friction), object.value("number").toInt(),
                         object.value("attentionRequired").toBool(), reasonList(reasons),
                         eventList(object.value("events").toArray())});
    }
    endResetModel();
}

QueueController::QueueController(QObject *parent) : QObject(parent), model_(this) {
    refreshTimer_.setInterval(5 * 60 * 1000);
    connect(&refreshTimer_, &QTimer::timeout, this, &QueueController::refresh);
    refreshTimer_.start();
    QDBusConnection::sessionBus().connect(
        "org.freedesktop.Notifications", "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications", "ActionInvoked", this,
        SLOT(notificationActionInvoked(uint,QString)));
    connect(&collectorProcess_, &QProcess::finished, this,
             [this](int exitCode, QProcess::ExitStatus exitStatus) {
         refreshing_ = false;
         emit refreshingChanged();
         if (exitStatus != QProcess::NormalExit || exitCode != 0) {
             setStatus("Could not refresh GitHub: " + QString::fromUtf8(collectorProcess_.readAllStandardError()).trimmed());
             return;
         }
        loadProjection();
     });
     connect(&queueProcess_, &QProcess::finished, this, [this](int exitCode, QProcess::ExitStatus exitStatus) {
         loading_ = false;
         emit loadingChanged();
         const bool shouldCollect = collectAfterProjection_;
         collectAfterProjection_ = false;
         if (exitStatus != QProcess::NormalExit || exitCode != 0) {
             setStatus("Could not load workspace: " + QString::fromUtf8(queueProcess_.readAllStandardError()).trimmed());
             if (shouldCollect) startCollection();
             return;
        }
        QJsonParseError error;
        const auto response = QJsonDocument::fromJson(queueProcess_.readAllStandardOutput(), &error);
         if (error.error != QJsonParseError::NoError || !response.isObject()) {
             setStatus("Could not read queue response: " + error.errorString());
             if (shouldCollect) startCollection();
             return;
        }
        const auto result = response.object();
        const auto cards = result.value("pullRequests").toArray();
        model_.replace(cards);
        sourceCount_ = result.value("sourceCount").toInt();
        suppressedCount_ = result.value("suppressedCount").toInt();
        emit countsChanged();
         sendNotifications(cards, result.value("notificationEligibleIds").toArray());
         setStatus("Updated " + result.value("capturedAt").toString());
         if (shouldCollect) startCollection();
    });
    connect(&stateProcess_, &QProcess::finished, this, [this](int exitCode, QProcess::ExitStatus exitStatus) {
         if (exitStatus != QProcess::NormalExit || exitCode != 0) {
            setStatus("Could not update local state: " + QString::fromUtf8(stateProcess_.readAllStandardError()).trimmed());
            return;
        }
         loadProjection();
    });
}

PullRequestModel *QueueController::pullRequests() { return &model_; }
QString QueueController::view() const { return view_; }
void QueueController::setView(const QString &view) {
    if (view_ != view) {
        view_ = view;
        emit viewChanged();
        loadProjection();
    }
}
QString QueueController::ranking() const { return ranking_; }
void QueueController::setRanking(const QString &ranking) {
    if (ranking_ != ranking) {
        ranking_ = ranking;
        emit rankingChanged();
        loadProjection();
    }
}
QString QueueController::status() const { return status_; }
bool QueueController::loading() const { return loading_; }
bool QueueController::refreshing() const { return refreshing_; }
int QueueController::sourceCount() const { return sourceCount_; }
int QueueController::suppressedCount() const { return suppressedCount_; }

void QueueController::refresh() {
    if (refreshing_) {
        setStatus("Refresh already in progress…");
        return;
    }
    loadProjection(true);
}

void QueueController::start() { loadProjection(true); }

void QueueController::startCollection() {
    if (refreshing_ || qEnvironmentVariableIsSet("REVIEW_RADAR_SKIP_COLLECTION")) return;
    refreshing_ = true;
    emit refreshingChanged();
    setStatus("Refreshing GitHub in the background…");
    collectorProcess_.setProgram(commandFromEnvironment("REVIEW_RADAR_COLLECTOR_COMMAND", "review-radar-github"));
    collectorProcess_.setArguments({"--database", captureDatabase()});
    collectorProcess_.start();
}

void QueueController::loadProjection(bool collectAfter) {
    if (queueProcess_.state() != QProcess::NotRunning) {
        collectAfterProjection_ = collectAfterProjection_ || collectAfter;
        return;
    }
    collectAfterProjection_ = collectAfterProjection_ || collectAfter;
    loading_ = true;
    emit loadingChanged();
    setStatus("Loading workspace…");
    queueProcess_.setProgram(commandFromEnvironment("REVIEW_RADAR_QUEUE_COMMAND", "review-radar-queue"));
    queueProcess_.setArguments({"--database", captureDatabase(),
                                "--state-database", stateDatabase(),
                                "--view", view_, "--ranking", ranking_,
                                "--record-attention", "true"});
    queueProcess_.start();
}

void QueueController::openUrl(const QString &url) { QDesktopServices::openUrl(QUrl(url)); }
void QueueController::copyText(const QString &text) { QGuiApplication::clipboard()->setText(text); }
void QueueController::acknowledge(const QString &id, const QString &fingerprint) {
    runStateCommand({"acknowledge", "--pull-request-id", id, "--fingerprint", fingerprint});
}
void QueueController::snooze(const QString &id, const QString &fingerprint, const QString &preset) {
    runStateCommand({"snooze", "--pull-request-id", id, "--fingerprint", fingerprint,
                     "--until", snoozeUntil(preset).toString(Qt::ISODateWithMs)});
}

QString QueueController::applicationDataFile(const QString &name) const {
    const auto directory = QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation);
    QDir().mkpath(directory);
    return QDir(directory).filePath(name);
}
QString QueueController::captureDatabase() const {
    const auto configured = qEnvironmentVariable("REVIEW_RADAR_CAPTURE_DATABASE");
    return configured.isEmpty() ? applicationDataFile("review-radar.sqlite3") : configured;
}
QString QueueController::stateDatabase() const {
    const auto configured = qEnvironmentVariable("REVIEW_RADAR_STATE_DATABASE");
    return configured.isEmpty() ? applicationDataFile("review-radar-state.sqlite3") : configured;
}
QString QueueController::commandFromEnvironment(const char *name, const QString &fallback) const {
    const auto configured = qEnvironmentVariable(name);
    return configured.isEmpty() ? fallback : configured;
}
void QueueController::runStateCommand(const QStringList &arguments) {
    if (stateProcess_.state() != QProcess::NotRunning) return;
    QStringList full{"--database", stateDatabase()};
    full.append(arguments);
    stateProcess_.setProgram(commandFromEnvironment("REVIEW_RADAR_STATE_COMMAND", "review-radar-state"));
    stateProcess_.setArguments(full);
    stateProcess_.start();
}
void QueueController::sendNotifications(const QJsonArray &cards, const QJsonArray &eligibleIds) {
    QDBusInterface notifications("org.freedesktop.Notifications", "/org/freedesktop/Notifications",
                                 "org.freedesktop.Notifications", QDBusConnection::sessionBus());
    if (!notifications.isValid()) return;
    for (const auto &eligibleId : eligibleIds) {
        const auto id = eligibleId.toString();
        QJsonObject card;
        for (const auto &candidate : cards) {
            if (candidate.toObject().value("id").toString() == id) {
                card = candidate.toObject();
                break;
            }
        }
        if (card.isEmpty()) continue;
        const auto reasons = card.value("explanation").toObject().value("reasons").toArray();
        const auto reason = reasons.isEmpty() ? QString{} : reasons.first().toObject().value("summary").toString();
        const auto title = card.value("actionLabel").toString();
        const auto body = QStringLiteral("%1 #%2\n%3")
                              .arg(card.value("repository").toString())
                              .arg(card.value("number").toInt())
                              .arg(reason);
        const QStringList actions{"open", "Open pull request"};
        QVariantMap hints{{"desktop-entry", "review-radar-linux"}};
        QDBusReply<uint> reply = notifications.call("Notify", "Review Radar", 0U, QString(),
                                                     title, body, actions, hints, -1);
        if (reply.isValid()) notificationUrls_.insert(reply.value(), card.value("url").toString());
    }
}
void QueueController::notificationActionInvoked(uint notificationId, const QString &action) {
    if (action == "open" && notificationUrls_.contains(notificationId)) {
        openUrl(notificationUrls_.take(notificationId));
    }
}
void QueueController::setStatus(const QString &status) { if (status_ != status) { status_ = status; emit statusChanged(); } }
