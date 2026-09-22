#pragma once

#include <QDBusArgument>
#include <QMetaType>

namespace ReviewRadar {

// Freedesktop image-data: (iiibiiay), with byte-ordered, unpremultiplied RGBA.
struct NotificationImage {
    int width = 0;
    int height = 0;
    int rowStride = 0;
    bool hasAlpha = true;
    int bitsPerSample = 8;
    int channels = 4;
    QByteArray pixels;
};

inline QDBusArgument &operator<<(QDBusArgument &argument, const NotificationImage &image) {
    argument.beginStructure();
    argument << image.width << image.height << image.rowStride << image.hasAlpha
             << image.bitsPerSample << image.channels << image.pixels;
    argument.endStructure();
    return argument;
}

inline const QDBusArgument &operator>>(const QDBusArgument &argument, NotificationImage &image) {
    argument.beginStructure();
    argument >> image.width >> image.height >> image.rowStride >> image.hasAlpha
             >> image.bitsPerSample >> image.channels >> image.pixels;
    argument.endStructure();
    return argument;
}

} // namespace ReviewRadar

Q_DECLARE_METATYPE(ReviewRadar::NotificationImage)
