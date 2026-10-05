#pragma once

#include <QObject>
#include <QString>

class QTcpServer;
class QTcpSocket;
class QWidget;

/**
 * Provides opt-in, loopback-only inspection and navigation for a KERO window.
 *
 * The server exists for local UI testing and agent tooling. It is created only
 * when the user explicitly supplies an automation port and a non-empty token.
 */
class KeroAutomationServer final : public QObject {
    Q_OBJECT

public:
    explicit KeroAutomationServer(QWidget* window, QObject* parent = nullptr);

    /**
     * Starts the local automation endpoint.
     *
     * @param port loopback TCP port to listen on; zero chooses an available port
     * @param token required bearer token for every request
     * @return true when the endpoint is listening
     */
    bool listen(quint16 port, const QString& token);

    /** Returns the selected loopback port after a successful listen call. */
    quint16 port() const;

private:
    void handleConnection();
    void handleRequest(QTcpSocket* socket, const QByteArray& request);

    QWidget* window_;
    QTcpServer* server_;
    QString token_;
};
