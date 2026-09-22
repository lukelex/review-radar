#pragma once

#include <QObject>
#include <QString>

namespace ReviewRadar {

// The exported surface deliberately contains no card identities or database paths.
class BarService final : public QObject {
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.reviewradar.Bar1")
public:
    using QObject::QObject;
    QString snapshot = QStringLiteral("{\"version\":1,\"available\":false}");
public slots:
    QString GetSnapshot() const { return snapshot; }
    void OpenWorkspace() { emit openRequested(); }
    void OpenPreferences() { emit preferencesRequested(); }
    void Refresh() { emit refreshRequested(); }
signals:
    void openRequested();
    void preferencesRequested();
    void refreshRequested();
};

} // namespace ReviewRadar
