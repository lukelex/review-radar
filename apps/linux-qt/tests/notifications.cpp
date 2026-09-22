#include "linuxosintegration.h"
#include "notificationimage.h"

#include <QApplication>
#include <QCryptographicHash>
#include <QDBusConnection>
#include <QDBusMetaType>
#include <QIcon>
#include <QImage>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QTest>
#include <cstdio>

class NotificationReceiver final : public QObject {
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.freedesktop.Notifications")
public slots:
    uint Notify(const QString &, uint, const QString &icon, const QString &, const QString &,
                const QStringList &, const QVariantMap &hints, int) {
        const auto argument = hints.value("image-data").value<QDBusArgument>();
        const auto signature = argument.currentSignature();
        const auto image = qdbus_cast<ReviewRadar::NotificationImage>(argument);
        const auto result = QJsonDocument(QJsonObject{
            {"signature", signature}, {"icon", icon},
            {"width", image.width}, {"height", image.height}, {"stride", image.rowStride},
            {"alpha", image.hasAlpha}, {"bits", image.bitsPerSample}, {"channels", image.channels},
            {"bytes", image.pixels.size()},
            {"hash", QString::fromLatin1(QCryptographicHash::hash(image.pixels, QCryptographicHash::Sha256).toHex())}
        }).toJson(QJsonDocument::Compact);
        std::puts(result.constData());
        std::fflush(stdout);
        return 1;
    }
};

class NotificationTest final : public QObject {
    Q_OBJECT
private slots:
    void embeddedLogoCrossesDbus() {
        QProcess receiver;
        // Prevent accidental success via a host-installed icon or desktop file.
        auto environment = QProcessEnvironment::systemEnvironment();
        environment.insert("XDG_DATA_DIRS", "/nonexistent");
        environment.insert("XDG_DATA_HOME", "/nonexistent");
        receiver.setProcessEnvironment(environment);
        receiver.start(QCoreApplication::applicationFilePath(), {"--receiver"});
        QByteArray output;
        QTRY_VERIFY((output += receiver.readAllStandardOutput()).contains("READY\n"));
        output.clear();
        ReviewRadar::LinuxOsIntegration os;
        QVERIFY(os.showNotification({"test-icon", "Test notification", "Icon transport test", {}, {}}));
        QTRY_VERIFY(!(output += receiver.readAllStandardOutput()).isEmpty());
        receiver.terminate();
        QVERIFY(receiver.waitForFinished());
        const auto received = QJsonDocument::fromJson(output.trimmed()).object();
        QCOMPARE(received["signature"].toString(), QString("(iiibiiay)"));
        QVERIFY(received["alpha"].toBool());
        QCOMPARE(received["bits"].toInt(), 8);
        QCOMPARE(received["channels"].toInt(), 4);
        const auto expected = QIcon(":/assets/logo.svg").pixmap(64, 64).toImage().convertToFormat(QImage::Format_RGBA8888);
        QVERIFY(!expected.isNull());
        QCOMPARE(received["width"].toInt(), expected.width());
        QCOMPARE(received["height"].toInt(), expected.height());
        QCOMPARE(received["stride"].toInt(), expected.bytesPerLine());
        QCOMPARE(received["bytes"].toInt(), expected.sizeInBytes());
        const QByteArray pixels(reinterpret_cast<const char *>(expected.constBits()), expected.sizeInBytes());
        QCOMPARE(received["hash"].toString(), QString::fromLatin1(QCryptographicHash::hash(pixels, QCryptographicHash::Sha256).toHex()));
    }
};

int main(int argc, char **argv) {
    QApplication app(argc, argv);
    if (app.arguments().contains("--receiver")) {
        qDBusRegisterMetaType<ReviewRadar::NotificationImage>();
        NotificationReceiver receiver;
        auto bus = QDBusConnection::sessionBus();
        if (!bus.registerService("org.freedesktop.Notifications")
            || !bus.registerObject("/org/freedesktop/Notifications", &receiver, QDBusConnection::ExportAllSlots)) return 1;
        std::puts("READY");
        std::fflush(stdout);
        return app.exec();
    }
    NotificationTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "notifications.moc"
