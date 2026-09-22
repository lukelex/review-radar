#include "queuecontroller.h"
#include "linuxosintegration.h"

#include <QGuiApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QFile>
#include <QSaveFile>
#include <QTime>
#include <QTimeZone>

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
    const auto review = health.value("reviewDecision").toString().toLower().replace('_', ' ');
    const auto checks = health.value("checks").toString().toLower().replace('-', ' ');
    if (!review.isEmpty()) healthSignals.append("Review: " + review);
    else if (health.value("isDraft").toBool()) healthSignals.append("Draft");
    if (!checks.isEmpty()) healthSignals.append("Checks: " + checks);
    const auto mergeable = health.value("mergeable").toString().toLower().replace('_', ' ');
    if (!mergeable.isEmpty()) healthSignals.append("Merge: " + mergeable);
    return healthSignals.join(" · ");
}

QVariantMap healthSignal(const QString &label, const QString &value, const QString &tone,
                         const QString &icon) {
    return {{"label", label}, {"value", value}, {"tone", tone}, {"icon", icon}};
}

QVariantList healthSignals(const QJsonObject &health) {
    QVariantList result;
    const auto review = health.value("reviewDecision").toString().toUpper();
    if (health.value("isDraft").toBool() && review.isEmpty()) {
        result.append(healthSignal("Review", "Draft", "neutral", "◆"));
    } else if (review == "APPROVED") {
        result.append(healthSignal("Review", "Approved", "positive", "✓"));
    } else if (review == "CHANGES_REQUESTED") {
        result.append(healthSignal("Review", "Changes requested", "caution", "↺"));
    } else if (review == "REVIEW_REQUIRED") {
        result.append(healthSignal("Review", "Required", "caution", "◷"));
    } else if (!review.isEmpty()) {
        result.append(healthSignal("Review", review.toLower().replace('_', ' '), "neutral", "?"));
    }

    const auto checks = health.value("checks").toString().toUpper();
    if (checks == "SUCCESS" || checks == "PASSING") {
        result.append(healthSignal("Checks", "Passing", "positive", "✓"));
    } else if (checks == "FAILURE" || checks == "FAILING" || checks == "ERROR") {
        result.append(healthSignal("Checks", "Failing", "negative", "!"));
    } else if (checks == "PENDING" || checks == "EXPECTED") {
        result.append(healthSignal("Checks", "Pending", "caution", "◷"));
    } else if (!checks.isEmpty()) {
        result.append(healthSignal("Checks", checks.toLower().replace('_', ' '), "neutral", "?"));
    }

    const auto mergeable = health.value("mergeable").toString().toUpper();
    const auto mergeState = health.value("mergeStateStatus").toString().toUpper();
    if (mergeable == "CONFLICTING" || mergeState == "DIRTY") {
        result.append(healthSignal("Merge", "Conflicting", "caution", "△"));
    } else if (mergeable == "MERGEABLE") {
        result.append(healthSignal("Merge", "Mergeable", "positive", "✓"));
    } else if (!mergeable.isEmpty()) {
        result.append(healthSignal("Merge", mergeable.toLower().replace('_', ' '), "neutral", "?"));
    }
    return result;
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
    case HealthSignalsRole: return card.healthSignals;
    case NextActionLabelRole: return card.nextActionLabel;
    case NextActionUrlRole: return card.nextActionUrl;
    case FrictionStatusRole: return card.frictionStatus;
    case FrictionLevelRole: return card.frictionLevel;
    case FrictionDetailRole: return card.frictionDetail;
    case EventsRole: return card.events;
    case LifecycleRole: return card.lifecycle;
    case UpdatedAtRole: return card.updatedAt;
    default: return {};
    }
}

QHash<int, QByteArray> PullRequestModel::roleNames() const {
    return {{IdRole, "pullRequestId"}, {RepositoryRole, "repository"}, {NumberRole, "number"},
            {TitleRole, "title"}, {UrlRole, "url"}, {ActionLabelRole, "actionLabel"},
            {AttentionRequiredRole, "attentionRequired"}, {FingerprintRole, "currentFingerprint"},
             {ExplanationHeadingRole, "explanationHeading"}, {ReasonsRole, "reasons"}, {HealthRole, "health"},
             {HealthSignalsRole, "healthSignals"},
            {NextActionLabelRole, "nextActionLabel"}, {NextActionUrlRole, "nextActionUrl"},
             {FrictionStatusRole, "frictionStatus"}, {FrictionLevelRole, "frictionLevel"},
             {FrictionDetailRole, "frictionDetail"},
            {EventsRole, "events"}, {LifecycleRole, "lifecycle"}, {UpdatedAtRole, "updatedAt"}};
}

QVariantMap PullRequestModel::get(int row) const {
    QVariantMap result;
    if (row < 0 || row >= cards_.size()) return result;
    const auto roles = roleNames();
    for (auto it = roles.cbegin(); it != roles.cend(); ++it)
        result.insert(QString::fromUtf8(it.value()), data(index(row), it.key()));
    return result;
}

int PullRequestModel::matchingCount(const QString &search) const {
    int count = 0;
    for (const auto &card : cards_) {
        if (card.title.contains(search, Qt::CaseInsensitive)
            || card.repository.contains(search, Qt::CaseInsensitive)
            || QString::number(card.number).contains(search)) ++count;
    }
    return count;
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
                        healthSignals(explanation.value("health").toObject()),
                         nextAction.value("label").toString(), nextAction.value("url").toString(),
                          friction.value("status").toString(), friction.value("level").toString(),
                         frictionSummary(friction), object.value("number").toInt(),
                         object.value("attentionRequired").toBool(), reasonList(reasons),
                          eventList(object.value("events").toArray()),
                          object.value("lifecycle").toString(), object.value("updatedAt").toString()});
    }
    endResetModel();
}

QueueController::QueueController(QObject *parent)
    : QObject(parent), model_(this), osIntegration_(new ReviewRadar::LinuxOsIntegration(this)) {
    initialize();
}

QueueController::QueueController(ReviewRadar::OsIntegration *osIntegration, QObject *parent)
    : QObject(parent), model_(this), osIntegration_(osIntegration) {
    initialize();
}

void QueueController::initialize() {
    QFile preferences(applicationDataFile("preferences.json"));
    if (preferences.open(QIODevice::ReadOnly)) {
        const auto object = QJsonDocument::fromJson(preferences.readAll()).object();
        notificationsEnabled_ = object.value("notificationsEnabled").toBool(true);
        trayEnabled_ = object.value("trayEnabled").toBool(false);
        closeToTray_ = trayEnabled_ && object.value("closeToTray").toBool(false);
        trayAttentionDot_ = object.value("trayAttentionDot").toBool(true);
        barEnabled_ = object.value("barEnabled").toBool(false);
        notifyReviewRequests_ = object.value("notifyReviewRequests").toBool(true);
        notifyFeedback_ = object.value("notifyFeedback").toBool(true);
        notifyChecks_ = object.value("notifyChecks").toBool(true);
        notifyConflicts_ = object.value("notifyConflicts").toBool(true);
    }
    osIntegration_->configureBar(barEnabled_);
    connect(this, &QueueController::loadingChanged, this, &QueueController::publishBarSnapshot);
    connect(this, &QueueController::refreshingChanged, this, &QueueController::publishBarSnapshot);
    connect(this, &QueueController::staleChanged, this, &QueueController::publishBarSnapshot);
    publishBarSnapshot();
    osIntegration_->configureTray(trayEnabled_, trayAttentionDot_);
    connect(osIntegration_, &ReviewRadar::OsIntegration::showWorkspaceRequested, this, &QueueController::showWorkspaceRequested);
    connect(osIntegration_, &ReviewRadar::OsIntegration::showPreferencesRequested, this, &QueueController::showPreferencesRequested);
    connect(osIntegration_, &ReviewRadar::OsIntegration::refreshRequested, this, &QueueController::refresh);
    connect(osIntegration_, &ReviewRadar::OsIntegration::quitRequested, this, &QueueController::quitRequested);
    connect(osIntegration_, &ReviewRadar::OsIntegration::trayAvailabilityChanged, this, [this] {
        emit trayAvailabilityChanged();
        if (!trayAvailable()) emit showWorkspaceRequested();
    });
    refreshTimer_.setInterval(5 * 60 * 1000);
    connect(&refreshTimer_, &QTimer::timeout, this, &QueueController::refresh);
    refreshTimer_.start();
    modifierTimer_.setInterval(50);
    connect(&modifierTimer_, &QTimer::timeout, this, [this]() {
        const bool held = QGuiApplication::queryKeyboardModifiers().testFlag(Qt::ControlModifier);
        if (controlHeld_ == held) return;
        controlHeld_ = held;
        emit controlHeldChanged();
    });
    modifierTimer_.start();
    connect(osIntegration_, &ReviewRadar::OsIntegration::notificationActivated, this,
            [this](const QUrl &url) { openUrl(url.toString()); });
     connect(&collectorProcess_, &QProcess::finished, this,
              [this](int exitCode, QProcess::ExitStatus exitStatus) {
          refreshing_ = false;
          emit refreshingChanged();
           if (exitStatus != QProcess::NormalExit || exitCode != 0) {
               barError_ = true;
               publishBarSnapshot();
               reportRequestFailure("Could not refresh GitHub",
                                    QStringLiteral("Exit code: %1\n\n%2")
                                        .arg(exitCode)
                                        .arg(QString::fromUtf8(collectorProcess_.readAllStandardError()).trimmed()));
              return;
          }
          loadProjection();
      });
     connect(&collectorProcess_, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
         if (error != QProcess::FailedToStart) return;
         refreshing_ = false;
         emit refreshingChanged();
         barError_ = true;
         publishBarSnapshot();
         reportRequestFailure("Could not refresh GitHub", collectorProcess_.errorString());
     });
     connect(&collectorProcess_, &QProcess::readyReadStandardOutput, this, [this]() {
         const auto output = QString::fromUtf8(collectorProcess_.readAllStandardOutput());
         const auto lines = output.split('\n', Qt::SkipEmptyParts);
         for (const auto &line : lines) {
             if (line.startsWith("Progress: ")) setStatus(line);
         }
     });
     connect(&queueProcess_, &QProcess::finished, this, [this](int exitCode, QProcess::ExitStatus exitStatus) {
         loading_ = false;
         emit loadingChanged();
         const bool shouldCollect = collectAfterProjection_;
         collectAfterProjection_ = false;
           if (exitStatus != QProcess::NormalExit || exitCode != 0) {
               barError_ = true;
               publishBarSnapshot();
               reportRequestFailure("Could not load workspace",
                                    QStringLiteral("Exit code: %1\n\n%2")
                                        .arg(exitCode)
                                        .arg(QString::fromUtf8(queueProcess_.readAllStandardError()).trimmed()));
              if (shouldCollect) startCollection();
              return;
         }
        QJsonParseError error;
        const auto response = QJsonDocument::fromJson(queueProcess_.readAllStandardOutput(), &error);
          if (error.error != QJsonParseError::NoError || !response.isObject()) {
               barError_ = true;
               publishBarSnapshot();
              reportRequestFailure("Could not read queue response", error.errorString());
              if (shouldCollect) startCollection();
              return;
        }
        const auto result = response.object();
        const auto cards = result.value("pullRequests").toArray();
          model_.replace(cards);
          hasProjection_ = true;
         int attentionCount = 0;
         for (const auto &card : cards)
             if (card.toObject().value("attentionRequired").toBool()) ++attentionCount;
         osIntegration_->setTrayAttention(attentionCount);
         barSnapshot_.available = true;
         barSnapshot_.workspace = requestedView_;
         barSnapshot_.attentionCount = attentionCount;
         barSnapshot_.capturedAt = result.value("capturedAt").toString();
         barError_ = false;
        if (stale_) {
            stale_ = false;
            emit staleChanged();
        }
        sourceCount_ = result.value("sourceCount").toInt();
        suppressedCount_ = result.value("suppressedCount").toInt();
        emit countsChanged();
         sendNotifications(cards, result.value("notificationEligibleIds").toArray());
          setStatus("Updated " + result.value("capturedAt").toString());
          publishBarSnapshot();
         if (shouldCollect) startCollection();
     });
     connect(&queueProcess_, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
         if (error != QProcess::FailedToStart) return;
         loading_ = false;
         emit loadingChanged();
         const bool shouldCollect = collectAfterProjection_;
         collectAfterProjection_ = false;
         barError_ = true;
         publishBarSnapshot();
         reportRequestFailure("Could not load workspace", queueProcess_.errorString());
         if (shouldCollect) startCollection();
     });
     connect(&stateProcess_, &QProcess::finished, this, [this](int exitCode, QProcess::ExitStatus exitStatus) {
          if (exitStatus != QProcess::NormalExit || exitCode != 0) {
             reportRequestFailure("Could not update local state",
                                  QStringLiteral("Exit code: %1\n\n%2")
                                      .arg(exitCode)
                                      .arg(QString::fromUtf8(stateProcess_.readAllStandardError()).trimmed()));
             return;
         }
          loadProjection();
     });
     connect(&stateProcess_, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
         if (error == QProcess::FailedToStart)
             reportRequestFailure("Could not update local state", stateProcess_.errorString());
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
bool QueueController::stale() const { return stale_; }
bool QueueController::controlHeld() const { return controlHeld_; }
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
    barError_ = false;
    emit refreshingChanged();
    if (!stale_) {
        stale_ = true;
        emit staleChanged();
    }
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
    requestedView_ = view_;
    queueProcess_.setArguments({"--database", captureDatabase(),
                                "--state-database", stateDatabase(),
                                "--view", view_, "--ranking", ranking_,
                                "--record-attention", "true"});
    queueProcess_.start();
}

void QueueController::openUrl(const QString &url) {
    osIntegration_->openUrl(QUrl(url));
}
void QueueController::copyText(const QString &text) { osIntegration_->copyText(text); }
void QueueController::acknowledge(const QString &id, const QString &fingerprint) {
    runStateCommand({"acknowledge", "--pull-request-id", id, "--fingerprint", fingerprint});
}
void QueueController::snooze(const QString &id, const QString &fingerprint, const QString &preset) {
    runStateCommand({"snooze", "--pull-request-id", id, "--fingerprint", fingerprint,
                     "--until", snoozeUntil(preset).toString(Qt::ISODateWithMs)});
}

QString QueueController::applicationDataFile(const QString &name) const {
    return osIntegration_->applicationDataFile(name);
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
    // Observation/deduplication already happened in the shared state projection.
    if (!notificationsEnabled_) return;
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
        bool categoryEnabled = false;
        for (const auto &value : reasons) {
            const auto code = value.toObject().value("code").toString();
            categoryEnabled = categoryEnabled
                || (code == "reviewRequested" && notifyReviewRequests_)
                || ((code == "changesRequested" || code == "newFeedback") && notifyFeedback_)
                || (code == "checksFailing" && notifyChecks_)
                || (code == "mergeConflict" && notifyConflicts_);
        }
        if (!categoryEnabled) continue;
        const auto reason = reasons.isEmpty() ? QString{} : reasons.first().toObject().value("summary").toString();
        const auto title = card.value("actionLabel").toString();
        const auto body = QStringLiteral("%1 #%2\n%3")
                              .arg(card.value("repository").toString())
                              .arg(card.value("number").toInt())
                              .arg(reason);
        const auto pullRequestUrl = QUrl(card.value("url").toString());
        const auto nextAction = reasons.isEmpty() ? QJsonObject{}
                                                  : reasons.first().toObject().value("nextAction").toObject();
        const auto nextActionUrl = QUrl(nextAction.value("url").toString());
        QList<ReviewRadar::NotificationAction> actions;
        if (!nextActionUrl.isEmpty()) {
            actions.append({"next-action", nextAction.value("label").toString("Open next action"),
                            nextActionUrl});
        }
        if (!pullRequestUrl.isEmpty() && pullRequestUrl != nextActionUrl)
            actions.append({"open-pull-request", "Open pull request", pullRequestUrl});
        osIntegration_->showNotification({card.value("currentFingerprint").toString(), title, body,
                                           pullRequestUrl, actions});
    }
}
void QueueController::reportRequestFailure(const QString &title, const QString &details) {
    if (hasProjection_ && !stale_) {
        stale_ = true;
        emit staleChanged();
    }
    const auto message = details.trimmed();
    setStatus(title + (message.isEmpty() ? QString{} : QStringLiteral(": ") + message));
    emit requestFailed(title, message.isEmpty() ? QStringLiteral("No additional diagnostic output was provided.")
                                                  : message);
}
void QueueController::setStatus(const QString &status) { if (status_ != status) { status_ = status; emit statusChanged(); } }

bool QueueController::savePreferences(bool notificationsEnabled) {
    return saveDesktopPreferences(notificationsEnabled, trayEnabled_, closeToTray_, trayAttentionDot_);
}

bool QueueController::saveDesktopPreferences(bool notificationsEnabled, bool trayEnabled,
                                             bool closeToTray, bool attentionDot) {
    return saveIntegrationPreferences(notificationsEnabled, trayEnabled, closeToTray, attentionDot, barEnabled_);
}

bool QueueController::saveIntegrationPreferences(bool notificationsEnabled, bool trayEnabled,
                                                bool closeToTray, bool attentionDot, bool barEnabled) {
    return saveAllPreferences(notificationsEnabled, trayEnabled, closeToTray, attentionDot, barEnabled,
                              notifyReviewRequests_, notifyFeedback_, notifyChecks_, notifyConflicts_);
}

bool QueueController::saveAllPreferences(bool notificationsEnabled, bool trayEnabled,
                                         bool closeToTray, bool attentionDot, bool barEnabled,
                                         bool reviewRequests, bool feedback, bool checks, bool conflicts) {
    QSaveFile file(applicationDataFile("preferences.json"));
    const auto bytes = QJsonDocument(QJsonObject{{"notificationsEnabled", notificationsEnabled},
        {"trayEnabled", trayEnabled}, {"closeToTray", trayEnabled && closeToTray},
        {"trayAttentionDot", attentionDot}, {"barEnabled", barEnabled},
        {"notifyReviewRequests", reviewRequests}, {"notifyFeedback", feedback},
        {"notifyChecks", checks}, {"notifyConflicts", conflicts}}).toJson();
    if (!file.open(QIODevice::WriteOnly) || file.write(bytes) != bytes.size() || !file.commit())
        return false;
    notificationsEnabled_ = notificationsEnabled;
    trayEnabled_ = trayEnabled;
    closeToTray_ = trayEnabled && closeToTray;
    trayAttentionDot_ = attentionDot;
    barEnabled_ = barEnabled;
    notifyReviewRequests_ = reviewRequests;
    notifyFeedback_ = feedback;
    notifyChecks_ = checks;
    notifyConflicts_ = conflicts;
    osIntegration_->configureBar(barEnabled_);
    publishBarSnapshot();
    osIntegration_->configureTray(trayEnabled_, trayAttentionDot_);
    if (!shouldCloseToTray()) emit showWorkspaceRequested();
    emit preferencesChanged();
    return true;
}

bool QueueController::testNotification() {
    return osIntegration_->showNotification({
        "preferences-test", "Review Radar test notification",
        "Desktop notifications are working. Your preferences have not changed.", {}, {}});
}

void QueueController::publishBarSnapshot() {
    barSnapshot_.syncState = barError_ ? "error" : refreshing_ ? "syncing"
        : loading_ ? "loading" : stale_ ? "stale" : barSnapshot_.available ? "ready" : "unavailable";
    osIntegration_->publishBarSnapshot(barSnapshot_);
}
