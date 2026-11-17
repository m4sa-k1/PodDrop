#pragma once

#include <QFileInfo>
#include <QString>

#include "poddrop.h"

inline QString takeCString(char *p) {
    if (!p) return QString();
    QString s = QString::fromUtf8(p);
    poddrop_free_string(p);
    return s;
}

inline bool isSupportedAudioExt(const QString &path) {
    const QString ext = QFileInfo(path).suffix().toLower();
    return ext == "mp3" || ext == "m4a" || ext == "mp4" || ext == "flac" || ext == "ogg" ||
           ext == "opus" || ext == "wav";
}
