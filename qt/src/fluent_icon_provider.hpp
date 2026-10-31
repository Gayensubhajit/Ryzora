#pragma once

#include <QQuickImageProvider>
#include <QSvgRenderer>
#include <QPainter>
#include <QPixmap>
#include <QFile>
#include <QColor>
#include <QUrlQuery>
#include <QUrl>
#include <QRegularExpression>
#include <QHash>

class FluentIconProvider : public QQuickImageProvider {
public:
    FluentIconProvider() : QQuickImageProvider(QQuickImageProvider::Pixmap) {
        initAliases();
    }

    QPixmap requestPixmap(const QString &id, QSize *size, const QSize &requestedSize) override {
        // id format: "<raw_name>_<variant>?color=<hex>&size=<px>" or "<raw_name>?color=<hex>"
        QString iconSpec = id;
        QString queryStr;
        int queryIdx = id.indexOf('?');
        if (queryIdx >= 0) {
            iconSpec = id.left(queryIdx);
            queryStr = id.mid(queryIdx + 1);
        }

        QUrlQuery query(queryStr);
        QString colorHex = query.queryItemValue("color");
        if (colorHex.isEmpty()) {
            if (queryStr.startsWith('#')) colorHex = queryStr;
            else if (queryStr.contains('%')) colorHex = QUrl::fromPercentEncoding(queryStr.toUtf8());
            else colorHex = QStringLiteral("#ffffff");
        }
        if (!colorHex.startsWith('#')) colorHex.prepend('#');
        QColor targetColor(colorHex);
        if (!targetColor.isValid()) targetColor = Qt::white;

        bool isFilled = false;
        QString baseName = iconSpec.toLower().trimmed();
        if (baseName.endsWith("_filled")) {
            isFilled = true;
            baseName.chop(7);
        } else if (baseName.endsWith("_regular")) {
            isFilled = false;
            baseName.chop(8);
        }

        // Apply aliases
        QString canonical = m_aliases.value(baseName, baseName);
        canonical.replace('-', '_');

        QString variant = isFilled ? QStringLiteral("filled") : QStringLiteral("regular");
        QString resourcePath = QStringLiteral(":/resources/icons/fluent/%1_%2.svg").arg(canonical, variant);

        if (!QFile::exists(resourcePath)) {
            // Try fallback to regular if filled does not exist
            resourcePath = QStringLiteral(":/resources/icons/fluent/%1_regular.svg").arg(canonical);
        }

        int targetWidth = requestedSize.width() > 0 ? requestedSize.width() : 20;
        int targetHeight = requestedSize.height() > 0 ? requestedSize.height() : 20;

        QFile file(resourcePath);
        if (!file.open(QIODevice::ReadOnly)) {
            if (size) *size = QSize(targetWidth, targetHeight);
            QPixmap empty(targetWidth, targetHeight);
            empty.fill(Qt::transparent);
            return empty;
        }

        QByteArray svgData = file.readAll();
        file.close();

        // Dynamically tint the SVG fills
        QString colorHexStr = targetColor.name(QColor::HexRgb);
        svgData.replace("fill=\"#212121\"", QString("fill=\"%1\"").arg(colorHexStr).toUtf8());
        svgData.replace("fill=\"#000000\"", QString("fill=\"%1\"").arg(colorHexStr).toUtf8());
        svgData.replace("fill=\"currentColor\"", QString("fill=\"%1\"").arg(colorHexStr).toUtf8());

        QSvgRenderer renderer(svgData);
        if (size) *size = QSize(targetWidth, targetHeight);

        QImage img(targetWidth, targetHeight, QImage::Format_ARGB32_Premultiplied);
        img.fill(Qt::transparent);

        QPainter painter(&img);
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
        renderer.render(&painter);

        return QPixmap::fromImage(img);
    }

private:
    QHash<QString, QString> m_aliases;

    void initAliases() {
        // Semantic navigation aliases
        m_aliases["compass"] = "compass";
        m_aliases["discover"] = "compass";
        m_aliases["store"] = "compass";
        
        m_aliases["apps"] = "apps";
        m_aliases["grid"] = "apps";
        m_aliases["layout-grid"] = "apps";
        m_aliases["layout_grid"] = "apps";
        m_aliases["grid-2x2"] = "apps";

        m_aliases["library"] = "library";
        m_aliases["books"] = "library";

        m_aliases["updates"] = "arrow_sync";
        m_aliases["arrow-sync"] = "arrow_sync";
        m_aliases["arrow_sync"] = "arrow_sync";
        m_aliases["refresh-cw"] = "arrow_sync";
        m_aliases["refresh_cw"] = "arrow_sync";
        m_aliases["arrow-up-circle"] = "arrow_sync";
        m_aliases["package-check"] = "arrow_sync";

        m_aliases["studio"] = "sliders";
        m_aliases["sliders"] = "sliders";
        m_aliases["sliders-horizontal"] = "sliders";
        m_aliases["sliders_horizontal"] = "sliders";
        m_aliases["options"] = "sliders";
        m_aliases["palette"] = "color";
        m_aliases["color"] = "color";

        m_aliases["backups"] = "archive";
        m_aliases["archive"] = "archive";
        m_aliases["archive-restore"] = "archive";
        m_aliases["archive_restore"] = "archive";
        m_aliases["backup"] = "archive";
        m_aliases["database-backup"] = "archive";

        m_aliases["diagnostics"] = "pulse";
        m_aliases["pulse"] = "pulse";
        m_aliases["activity"] = "pulse";

        m_aliases["settings"] = "settings";
        m_aliases["gear"] = "settings";
        m_aliases["cog"] = "settings";

        // Appearance
        m_aliases["weather-sunny"] = "weather_sunny";
        m_aliases["weather_sunny"] = "weather_sunny";
        m_aliases["sun"] = "weather_sunny";
        m_aliases["light"] = "weather_sunny";

        m_aliases["weather-moon"] = "weather_moon";
        m_aliases["weather_moon"] = "weather_moon";
        m_aliases["moon"] = "weather_moon";
        m_aliases["dark"] = "weather_moon";

        m_aliases["desktop"] = "desktop";
        m_aliases["monitor"] = "desktop";
        m_aliases["display"] = "desktop";
        m_aliases["system"] = "desktop";

        m_aliases["info"] = "info";
        m_aliases["about"] = "info";

        m_aliases["download"] = "arrow_download";
        m_aliases["arrow-download"] = "arrow_download";
        m_aliases["arrow_download"] = "arrow_download";
        m_aliases["arrow-down"] = "arrow_download";
        m_aliases["get"] = "arrow_download";

        m_aliases["add"] = "add";
        m_aliases["plus"] = "add";
        m_aliases["install"] = "add";

        m_aliases["delete"] = "delete";
        m_aliases["trash"] = "delete";
        m_aliases["trash-2"] = "delete";
        m_aliases["uninstall"] = "delete";

        m_aliases["arrow-undo"] = "arrow_undo";
        m_aliases["arrow_undo"] = "arrow_undo";
        m_aliases["undo"] = "arrow_undo";
        m_aliases["restore"] = "arrow_undo";
        m_aliases["history"] = "arrow_undo";

        m_aliases["search"] = "search";
        m_aliases["arrow-clockwise"] = "arrow_clockwise";
        m_aliases["arrow_clockwise"] = "arrow_clockwise";
        m_aliases["refresh"] = "arrow_clockwise";
        m_aliases["spinner"] = "arrow_clockwise";

        m_aliases["checkmark"] = "checkmark";
        m_aliases["check"] = "checkmark";
        m_aliases["checkmark-circle"] = "checkmark_circle";
        m_aliases["checkmark_circle"] = "checkmark_circle";
        m_aliases["check-circle"] = "checkmark_circle";

        m_aliases["dismiss"] = "dismiss";
        m_aliases["close"] = "dismiss";
        m_aliases["x"] = "dismiss";

        m_aliases["play"] = "play";
        m_aliases["open"] = "play";

        m_aliases["more-horizontal"] = "more_horizontal";
        m_aliases["more_horizontal"] = "more_horizontal";
        m_aliases["more-vertical"] = "more_vertical";
        m_aliases["more_vertical"] = "more_vertical";

        m_aliases["chevron-left"] = "chevron_left";
        m_aliases["chevron_left"] = "chevron_left";
        m_aliases["chevron-right"] = "chevron_right";
        m_aliases["chevron_right"] = "chevron_right";
        m_aliases["chevron-down"] = "chevron_down";
        m_aliases["chevron_down"] = "chevron_down";
        m_aliases["chevron-up"] = "chevron_up";
        m_aliases["chevron_up"] = "chevron_up";

        m_aliases["warning"] = "warning";
        m_aliases["alert"] = "warning";

        m_aliases["lock"] = "lock_closed";
        m_aliases["lock-closed"] = "lock_closed";
        m_aliases["lock_closed"] = "lock_closed";

        m_aliases["folder"] = "folder";
        m_aliases["filter"] = "filter";
        m_aliases["code"] = "code";
        m_aliases["terminal"] = "code";
        m_aliases["shield"] = "shield";
        m_aliases["wrench"] = "wrench";
    }
};
