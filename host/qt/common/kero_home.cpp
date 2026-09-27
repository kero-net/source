#include "kero_home.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>

QString defaultKeroHomeLocation() {
    return QDir::cleanPath(QDir::home().filePath(".kero"));
}

QString configuredKeroHome() {
    const QString environmentHome = qEnvironmentVariable("KERO_HOME");
    if (!environmentHome.isEmpty()) return QDir::cleanPath(environmentHome);

    return defaultKeroHomeLocation();
}

KeroHomeResult ensureKeroHome() {
    KeroHomeResult result;
    result.path = configuredKeroHome();
    const QDir home(result.path);
    const QFileInfo homeInfo(result.path);
    if (homeInfo.exists() && !homeInfo.isDir()) {
        result.error = "The configured KERO home is not a directory.";
        return result;
    }

    if (!QDir().mkpath(home.filePath("data")) || !QDir().mkpath(home.filePath("mnt"))) {
        result.error = "KERO could not create its data and mount roots.";
        return result;
    }

    QFile config(home.filePath("config"));
    if (!config.exists() && !config.open(QIODevice::WriteOnly | QIODevice::Text)) {
        result.error = "KERO could not create its global configuration.";
        return result;
    }

    result.ready = true;
    return result;
}
