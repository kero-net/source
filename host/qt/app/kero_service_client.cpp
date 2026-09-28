#include "kero_service_client.h"

#include <QCoreApplication>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>

QString KeroServiceClient::executable() const {
    const QString sibling = QCoreApplication::applicationDirPath() + "/kero-host"
#ifdef Q_OS_WIN
        + ".exe"
#endif
        ;
    return QFileInfo(sibling).isFile() ? sibling : "kero-host";
}

KeroServiceClient::Reply KeroServiceClient::invoke(const QStringList& arguments) const {
    const auto run = [this](const QStringList& requestArguments) {
        QProcess process;
        QStringList request { "--json" };
        request += requestArguments;
        process.start(executable(), request);
        if (!process.waitForFinished(20000)) {
            process.kill();
            return Reply { false, {}, "The KERO service did not finish the request." };
        }
        const QByteArray output = process.readAllStandardOutput();
        QJsonParseError parseError;
        const QJsonDocument document = QJsonDocument::fromJson(output, &parseError);
        if (!document.isObject()) {
            const QString detail = QString::fromUtf8(process.readAllStandardError()).trimmed();
            return Reply { false, {}, detail.isEmpty() ? "KERO returned an invalid structured response." : detail };
        }
        const QJsonObject envelope = document.object();
        if (!envelope.value("ok").toBool()) return Reply { false, {}, envelope.value("error").toString("KERO rejected the request.") };
        return Reply { true, envelope.value("result"), {} };
    };
    Reply reply = run(arguments);
    if (!reply.ok && reply.error.contains("unrecognized subcommand")) {
        const Reply upgraded = run({ "service", "start" });
        if (upgraded.ok) reply = run(arguments);
    }
    return reply;
#if 0
    QProcess process;
    QStringList request { "--json" };
    request += arguments;
    process.start(executable(), request);
    if (!process.waitForFinished(20000)) {
        process.kill();
        return { false, {}, "The KERO service did not finish the request." };
    }
    const QByteArray output = process.readAllStandardOutput();
    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(output, &parseError);
    if (!document.isObject()) {
        const QString detail = QString::fromUtf8(process.readAllStandardError()).trimmed();
        return { false, {}, detail.isEmpty() ? "KERO returned an invalid structured response." : detail };
    }
    const QJsonObject envelope = document.object();
    if (!envelope.value("ok").toBool()) return { false, {}, envelope.value("error").toString("KERO rejected the request.") };
    return { true, envelope.value("result"), {} };
#endif
}
