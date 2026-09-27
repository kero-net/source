#include "kero_automation_server.h"

#include <QAbstractButton>
#include <QApplication>
#include <QBuffer>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QListWidget>
#include <QTcpServer>
#include <QTcpSocket>
#include <QWidget>
#include <QWizard>

namespace {
QJsonObject describeWidget(const QWidget* widget) {
    QJsonObject result;
    result.insert("objectName", widget->objectName());
    result.insert("className", widget->metaObject()->className());
    result.insert("visible", widget->isVisible());
    result.insert("enabled", widget->isEnabled());
    result.insert("text", widget->property("text").toString());

    const QRect geometry = widget->geometry();
    result.insert("x", geometry.x());
    result.insert("y", geometry.y());
    result.insert("width", geometry.width());
    result.insert("height", geometry.height());
    return result;
}

QJsonArray inspectWidgets(const QObject* parent) {
    QJsonArray widgets;
    for (const QObject* child : parent->children()) {
        if (const auto* widget = qobject_cast<const QWidget*>(child)) {
            QJsonObject item = describeWidget(widget);
            item.insert("children", inspectWidgets(widget));
            widgets.append(item);
        } else {
            const QJsonArray descendants = inspectWidgets(child);
            for (const QJsonValue& descendant : descendants) widgets.append(descendant);
        }
    }
    return widgets;
}

QByteArray screenshot(QWidget* widget) {
    QByteArray bytes;
    QBuffer buffer(&bytes);
    buffer.open(QIODevice::WriteOnly);
    widget->grab().save(&buffer, "PNG");
    return bytes.toBase64();
}

void reply(QTcpSocket* socket, const QJsonObject& response) {
    socket->write(QJsonDocument(response).toJson(QJsonDocument::Compact));
    socket->write("\n");
    socket->disconnectFromHost();
}

QJsonObject failure(const QString& message) {
    return QJsonObject{{"ok", false}, {"error", message}};
}
} // namespace

KeroAutomationServer::KeroAutomationServer(QWidget* window, QObject* parent)
    : QObject(parent), window_(window), server_(new QTcpServer(this)) {
    connect(server_, &QTcpServer::newConnection, this, &KeroAutomationServer::handleConnection);
}

bool KeroAutomationServer::listen(quint16 port, const QString& token) {
    if (token.isEmpty()) return false;
    token_ = token;
    return server_->listen(QHostAddress::LocalHost, port);
}

quint16 KeroAutomationServer::port() const {
    return server_->serverPort();
}

void KeroAutomationServer::handleConnection() {
    QTcpSocket* socket = server_->nextPendingConnection();
    connect(socket, &QTcpSocket::readyRead, this, [this, socket] {
        const QByteArray request = socket->readAll().trimmed();
        if (!request.isEmpty()) handleRequest(socket, request);
    });
    connect(socket, &QTcpSocket::disconnected, socket, &QObject::deleteLater);
}

void KeroAutomationServer::handleRequest(QTcpSocket* socket, const QByteArray& request) {
    QJsonParseError error;
    const QJsonDocument document = QJsonDocument::fromJson(request, &error);
    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        reply(socket, failure("Request must be a JSON object."));
        return;
    }

    const QJsonObject command = document.object();
    if (command.value("token").toString() != token_) {
        reply(socket, failure("Authentication failed."));
        return;
    }

    const QString method = command.value("method").toString();
    if (method == "inspect") {
        reply(socket, QJsonObject{{"ok", true}, {"window", describeWidget(window_)},
                                  {"widgets", inspectWidgets(window_)}});
        return;
    }
    if (method == "screenshot") {
        reply(socket, QJsonObject{{"ok", true}, {"mimeType", "image/png"},
                                  {"data", QString::fromLatin1(screenshot(window_))}});
        return;
    }

    const QString target = command.value("target").toString();
    if (target.isEmpty()) {
        reply(socket, failure("An objectName target is required."));
        return;
    }
    QWidget* widget = window_->findChild<QWidget*>(target);
    if (!widget) {
        reply(socket, failure("No widget has that objectName."));
        return;
    }
    if (!widget->isEnabled()) {
        reply(socket, failure("The target widget is disabled."));
        return;
    }

    if (method == "click") {
        auto* button = qobject_cast<QAbstractButton*>(widget);
        if (!button) {
            reply(socket, failure("The target is not a button."));
            return;
        }
        button->click();
    } else if (method == "setChecked") {
        auto* button = qobject_cast<QAbstractButton*>(widget);
        if (!button || !button->isCheckable()) {
            reply(socket, failure("The target is not a checkable button."));
            return;
        }
        button->setChecked(command.value("checked").toBool());
    } else if (method == "selectRow") {
        auto* list = qobject_cast<QListWidget*>(widget);
        const int row = command.value("row").toInt(-1);
        if (!list || row < 0 || row >= list->count()) {
            reply(socket, failure("The target row does not exist."));
            return;
        }
        list->setCurrentRow(row);
    } else {
        reply(socket, failure("Unsupported method."));
        return;
    }

    QApplication::processEvents();
    reply(socket, QJsonObject{{"ok", true}});
}
