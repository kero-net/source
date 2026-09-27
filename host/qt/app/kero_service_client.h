#pragma once

#include <QJsonValue>
#include <QString>
#include <QStringList>

class KeroServiceClient final {
public:
    struct Reply { bool ok = false; QJsonValue result; QString error; };

    Reply invoke(const QStringList& arguments) const;
    QString executable() const;
};
