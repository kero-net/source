#pragma once

#include <QLabel>
#include <QPixmap>

class AspectFillLabel final : public QLabel {
public:
    using QLabel::QLabel;

    // Qt Designer/uic calls setPixmap() for the pixmap property. Keep the
    // authored image and rescale from that source on every resize so repeated
    // resizes never compound interpolation loss.
    void setPixmap(const QPixmap& pixmap);

protected:
    void resizeEvent(QResizeEvent* event) override;

private:
    void updateScaledPixmap();

    QPixmap sourcePixmap_;
};
