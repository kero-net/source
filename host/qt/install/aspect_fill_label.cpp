#include "aspect_fill_label.h"

#include <QResizeEvent>

void AspectFillLabel::setPixmap(const QPixmap& pixmap) {
    sourcePixmap_ = pixmap;
    updateScaledPixmap();
}

void AspectFillLabel::resizeEvent(QResizeEvent* event) {
    QLabel::resizeEvent(event);
    updateScaledPixmap();
}

void AspectFillLabel::updateScaledPixmap() {
    if (sourcePixmap_.isNull() || size().isEmpty()) {
        QLabel::clear();
        return;
    }

    QLabel::setPixmap(sourcePixmap_.scaled(
        size(),
        Qt::KeepAspectRatioByExpanding,
        Qt::SmoothTransformation));
}
