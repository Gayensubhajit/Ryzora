import QtQuick

Canvas {
    id: root

    property string name: "lock"
    property color color: theme ? theme.text : "#ffffff"
    property real size: 16
    property real strokeWidth: 1.8

    implicitWidth: size
    implicitHeight: size

    onColorChanged: requestPaint()
    onSizeChanged: requestPaint()
    onNameChanged: requestPaint()

    function roundedRect(ctx, x, y, w, h, r) {
        ctx.moveTo(x + r, y);
        ctx.arcTo(x + w, y, x + w, y + h, r);
        ctx.arcTo(x + w, y + h, x, y + h, r);
        ctx.arcTo(x, y + h, x, y, r);
        ctx.arcTo(x, y, x + w, y, r);
        ctx.closePath();
    }

    onPaint: {
        var ctx = getContext("2d");
        ctx.reset();
        var s = root.size / 24.0;
        ctx.scale(s, s);
        ctx.strokeStyle = root.color;
        ctx.fillStyle = root.color;
        ctx.lineWidth = root.strokeWidth;
        ctx.lineCap = "round";
        ctx.lineJoin = "round";

        var n = root.name;

        if (n === "lock") {
            ctx.beginPath();
            ctx.arc(12, 7, 5, Math.PI, 0, false);
            ctx.stroke();
            ctx.beginPath();
            roundedRect(ctx, 5, 11, 14, 10, 2);
            ctx.stroke();
            ctx.beginPath();
            ctx.arc(12, 15, 1.2, 0, 2 * Math.PI);
            ctx.fill();
        } else if (n === "unlock") {
            ctx.beginPath();
            ctx.arc(12, 7, 5, Math.PI, 0.2, false);
            ctx.stroke();
            ctx.beginPath();
            roundedRect(ctx, 5, 11, 14, 10, 2);
            ctx.stroke();
        } else if (n === "shield") {
            ctx.beginPath();
            ctx.moveTo(12, 22);
            ctx.bezierCurveTo(17, 19, 20, 14, 20, 10);
            ctx.lineTo(20, 4);
            ctx.lineTo(12, 2);
            ctx.lineTo(4, 4);
            ctx.lineTo(4, 10);
            ctx.bezierCurveTo(4, 14, 7, 19, 12, 22);
            ctx.closePath();
            ctx.stroke();
        } else if (n === "shield-check") {
            ctx.beginPath();
            ctx.moveTo(12, 22);
            ctx.bezierCurveTo(17, 19, 20, 14, 20, 10);
            ctx.lineTo(20, 4);
            ctx.lineTo(12, 2);
            ctx.lineTo(4, 4);
            ctx.lineTo(4, 10);
            ctx.bezierCurveTo(4, 14, 7, 19, 12, 22);
            ctx.closePath();
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(9, 12);
            ctx.lineTo(11, 14);
            ctx.lineTo(15, 10);
            ctx.stroke();
        } else if (n === "check") {
            ctx.beginPath();
            ctx.moveTo(4, 12);
            ctx.lineTo(9, 17);
            ctx.lineTo(20, 6);
            ctx.stroke();
        } else if (n === "chevron-right") {
            ctx.beginPath();
            ctx.moveTo(9, 18);
            ctx.lineTo(15, 12);
            ctx.lineTo(9, 6);
            ctx.stroke();
        } else if (n === "chevron-left") {
            ctx.beginPath();
            ctx.moveTo(15, 18);
            ctx.lineTo(9, 12);
            ctx.lineTo(15, 6);
            ctx.stroke();
        } else if (n === "chevron-down") {
            ctx.beginPath();
            ctx.moveTo(6, 9);
            ctx.lineTo(12, 15);
            ctx.lineTo(18, 9);
            ctx.stroke();
        } else if (n === "chevron-up") {
            ctx.beginPath();
            ctx.moveTo(18, 15);
            ctx.lineTo(12, 9);
            ctx.lineTo(6, 15);
            ctx.stroke();
        } else if (n === "eye-off") {
            ctx.beginPath();
            ctx.moveTo(2, 12);
            ctx.bezierCurveTo(6, 6, 18, 6, 22, 12);
            ctx.bezierCurveTo(18, 18, 6, 18, 2, 12);
            ctx.closePath();
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(3, 3);
            ctx.lineTo(21, 21);
            ctx.stroke();
        } else if (n === "filter") {
            ctx.beginPath();
            ctx.moveTo(4, 5); ctx.lineTo(20, 5);
            ctx.lineTo(14, 13); ctx.lineTo(14, 19);
            ctx.lineTo(10, 21); ctx.lineTo(10, 13);
            ctx.closePath();
            ctx.stroke();
        } else if (n === "arrow-right") {
            ctx.beginPath();
            ctx.moveTo(5, 12);
            ctx.lineTo(19, 12);
            ctx.moveTo(13, 6);
            ctx.lineTo(19, 12);
            ctx.lineTo(13, 18);
            ctx.stroke();
        } else if (n === "refresh") {
            ctx.beginPath();
            ctx.arc(12, 12, 8, -Math.PI / 2, Math.PI, false);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(12, 2);
            ctx.lineTo(12, 6);
            ctx.lineTo(16, 4);
            ctx.closePath();
            ctx.fill();
        } else if (n === "store") {
            ctx.beginPath();
            roundedRect(ctx, 4, 6, 16, 15, 2);
            ctx.stroke();
            ctx.beginPath();
            ctx.arc(12, 7, 4, Math.PI, 0, false);
            ctx.stroke();
        } else if (n === "terminal") {
            ctx.beginPath();
            ctx.moveTo(4, 17);
            ctx.lineTo(10, 12);
            ctx.lineTo(4, 7);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(12, 19);
            ctx.lineTo(20, 19);
            ctx.stroke();
        } else if (n === "image") {
            ctx.beginPath();
            roundedRect(ctx, 3, 3, 18, 18, 2);
            ctx.stroke();
            ctx.beginPath();
            ctx.arc(8.5, 8.5, 1.5, 0, 2 * Math.PI);
            ctx.fill();
            ctx.beginPath();
            ctx.moveTo(21, 15);
            ctx.lineTo(16, 10);
            ctx.lineTo(5, 21);
            ctx.stroke();
        } else if (n === "palette") {
            ctx.beginPath();
            ctx.arc(12, 12, 9, 0, 2 * Math.PI);
            ctx.stroke();
            ctx.beginPath(); ctx.arc(8, 10, 1, 0, 2*Math.PI); ctx.fill();
            ctx.beginPath(); ctx.arc(12, 7, 1, 0, 2*Math.PI); ctx.fill();
            ctx.beginPath(); ctx.arc(16, 10, 1, 0, 2*Math.PI); ctx.fill();
        } else if (n === "sliders" || n === "overview") {
            ctx.beginPath();
            ctx.moveTo(4, 21); ctx.lineTo(4, 14);
            ctx.moveTo(4, 10); ctx.lineTo(4, 3);
            ctx.moveTo(12, 21); ctx.lineTo(12, 12);
            ctx.moveTo(12, 8); ctx.lineTo(12, 3);
            ctx.moveTo(20, 21); ctx.lineTo(20, 16);
            ctx.moveTo(20, 12); ctx.lineTo(20, 3);
            ctx.stroke();
            ctx.beginPath(); ctx.arc(4, 12, 2, 0, 2 * Math.PI); ctx.stroke();
            ctx.beginPath(); ctx.arc(12, 10, 2, 0, 2 * Math.PI); ctx.stroke();
            ctx.beginPath(); ctx.arc(20, 14, 2, 0, 2 * Math.PI); ctx.stroke();
        } else if (n === "sliders-horizontal") {
            ctx.beginPath();
            ctx.moveTo(3, 8); ctx.lineTo(21, 8);
            ctx.moveTo(3, 16); ctx.lineTo(21, 16);
            ctx.moveTo(8, 5); ctx.lineTo(8, 11);
            ctx.moveTo(16, 13); ctx.lineTo(16, 19);
            ctx.stroke();
        } else if (n === "play") {
            ctx.beginPath();
            ctx.moveTo(7, 5);
            ctx.lineTo(19, 12);
            ctx.lineTo(7, 19);
            ctx.closePath();
            ctx.stroke();
        } else if (n === "x") {
            ctx.beginPath();
            ctx.moveTo(18, 6); ctx.lineTo(6, 18);
            ctx.moveTo(6, 6); ctx.lineTo(18, 18);
            ctx.stroke();
        } else if (n === "info") {
            ctx.beginPath();
            ctx.arc(12, 12, 9, 0, 2 * Math.PI);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(12, 16); ctx.lineTo(12, 12);
            ctx.moveTo(12, 8); ctx.lineTo(12.01, 8);
            ctx.stroke();
        } else if (n === "desktop") {
            ctx.beginPath();
            roundedRect(ctx, 3, 4, 18, 12, 2);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(8, 20); ctx.lineTo(16, 20);
            ctx.moveTo(12, 16); ctx.lineTo(12, 20);
            ctx.stroke();
        } else if (n === "key") {
            ctx.beginPath();
            ctx.arc(8, 12, 4, 0, 2 * Math.PI);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(12, 12); ctx.lineTo(20, 12);
            ctx.moveTo(17, 12); ctx.lineTo(17, 15);
            ctx.moveTo(19, 12); ctx.lineTo(19, 14);
            ctx.stroke();
        } else if (n === "trash") {
            ctx.beginPath();
            ctx.moveTo(4, 6); ctx.lineTo(20, 6);
            ctx.moveTo(9, 6); ctx.lineTo(9, 3); ctx.lineTo(15, 3); ctx.lineTo(15, 6);
            roundedRect(ctx, 6, 6, 12, 15, 2);
            ctx.moveTo(9, 10); ctx.lineTo(9, 17);
            ctx.moveTo(12, 10); ctx.lineTo(12, 17);
            ctx.moveTo(15, 10); ctx.lineTo(15, 17);
            ctx.stroke();
        } else if (n === "search") {
            ctx.beginPath();
            ctx.arc(10.5, 10.5, 6, 0, 2 * Math.PI);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(15, 15); ctx.lineTo(20, 20);
            ctx.stroke();
        } else if (n === "compass") {
            ctx.beginPath();
            ctx.arc(12, 12, 9, 0, 2 * Math.PI);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(15, 9); ctx.lineTo(13.5, 13.5); ctx.lineTo(9, 15); ctx.lineTo(10.5, 10.5);
            ctx.closePath();
            ctx.stroke();
        } else if (n === "library" || n === "library-big") {
            ctx.beginPath();
            roundedRect(ctx, 4, 4, 4, 16, 1);
            roundedRect(ctx, 9, 4, 4, 16, 1);
            ctx.moveTo(15, 4.5); ctx.lineTo(18.5, 5.5); ctx.lineTo(15.5, 20); ctx.lineTo(12, 19);
            ctx.closePath();
            ctx.moveTo(2, 20); ctx.lineTo(22, 20);
            ctx.stroke();
        } else if (n === "book") {
            ctx.beginPath();
            ctx.moveTo(4, 19.5); ctx.bezierCurveTo(4, 18, 5.5, 17, 7.5, 17); ctx.lineTo(20, 17);
            ctx.moveTo(4, 4.5); ctx.bezierCurveTo(4, 6, 5.5, 7, 7.5, 7); ctx.lineTo(20, 7);
            ctx.moveTo(4, 4.5); ctx.lineTo(4, 19.5);
            ctx.moveTo(20, 7); ctx.lineTo(20, 20); ctx.lineTo(7.5, 20);
            ctx.stroke();
        } else if (n === "package" || n === "box") {
            ctx.beginPath();
            ctx.moveTo(12, 2.5); ctx.lineTo(21, 7.5); ctx.lineTo(12, 12.5); ctx.lineTo(3, 7.5);
            ctx.closePath();
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(3, 7.5); ctx.lineTo(3, 16.5); ctx.lineTo(12, 21.5); ctx.lineTo(12, 12.5);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(21, 7.5); ctx.lineTo(21, 16.5); ctx.lineTo(12, 21.5);
            ctx.stroke();
        } else if (n === "layout-grid" || n === "grid" || n === "apps") {
            ctx.beginPath();
            roundedRect(ctx, 3, 3, 7, 7, 1.5);
            ctx.stroke();
            ctx.beginPath();
            roundedRect(ctx, 14, 3, 7, 7, 1.5);
            ctx.stroke();
            ctx.beginPath();
            roundedRect(ctx, 14, 14, 7, 7, 1.5);
            ctx.stroke();
            ctx.beginPath();
            roundedRect(ctx, 3, 14, 7, 7, 1.5);
            ctx.stroke();
        } else if (n === "arrow-up-circle") {
            ctx.beginPath();
            ctx.arc(12, 12, 9, 0, 2 * Math.PI);
            ctx.moveTo(12, 16); ctx.lineTo(12, 8);
            ctx.moveTo(8.5, 11.5); ctx.lineTo(12, 8); ctx.lineTo(15.5, 11.5);
            ctx.stroke();
        } else if (n === "arrow-up" || n === "updates") {
            ctx.beginPath();
            ctx.moveTo(12, 19); ctx.lineTo(12, 5);
            ctx.moveTo(6, 11); ctx.lineTo(12, 5); ctx.lineTo(18, 11);
            ctx.stroke();
        } else if (n === "download" || n === "arrow-down") {
            ctx.beginPath();
            ctx.moveTo(12, 4); ctx.lineTo(12, 16);
            ctx.moveTo(6, 10); ctx.lineTo(12, 16); ctx.lineTo(18, 10);
            ctx.moveTo(4, 20); ctx.lineTo(20, 20);
            ctx.stroke();
        } else if (n === "settings" || n === "gear") {
            ctx.beginPath();
            ctx.arc(12, 12, 3.5, 0, 2 * Math.PI);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(12, 2); ctx.lineTo(12, 5);
            ctx.moveTo(12, 19); ctx.lineTo(12, 22);
            ctx.moveTo(2, 12); ctx.lineTo(5, 12);
            ctx.moveTo(19, 12); ctx.lineTo(22, 12);
            ctx.moveTo(4.93, 4.93); ctx.lineTo(7.05, 7.05);
            ctx.moveTo(16.95, 16.95); ctx.lineTo(19.07, 19.07);
            ctx.moveTo(4.93, 19.07); ctx.lineTo(7.05, 16.95);
            ctx.moveTo(16.95, 7.05); ctx.lineTo(19.07, 4.93);
            ctx.stroke();
        } else if (n === "clock") {
            ctx.beginPath();
            ctx.arc(12, 12, 9, 0, 2 * Math.PI);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(12, 7); ctx.lineTo(12, 12); ctx.lineTo(15.5, 14);
            ctx.stroke();
        } else if (n === "eye") {
            ctx.beginPath();
            ctx.moveTo(2, 12);
            ctx.bezierCurveTo(6, 6, 18, 6, 22, 12);
            ctx.bezierCurveTo(18, 18, 6, 18, 2, 12);
            ctx.closePath();
            ctx.stroke();
            ctx.beginPath();
            ctx.arc(12, 12, 3, 0, 2 * Math.PI);
            ctx.stroke();
        } else if (n === "alert" || n === "warning") {
            ctx.beginPath();
            ctx.moveTo(12, 3); ctx.lineTo(21, 19); ctx.lineTo(3, 19);
            ctx.closePath();
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(12, 8); ctx.lineTo(12, 13);
            ctx.stroke();
            ctx.beginPath();
            ctx.arc(12, 16, 0.9, 0, 2 * Math.PI);
            ctx.fill();
        } else if (n === "stop" || n === "square") {
            ctx.beginPath();
            roundedRect(ctx, 6, 6, 12, 12, 2);
            ctx.stroke();
        } else if (n === "sun") {
            ctx.beginPath();
            ctx.arc(12, 12, 4.5, 0, 2 * Math.PI);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(12, 2); ctx.lineTo(12, 4.5);
            ctx.moveTo(12, 19.5); ctx.lineTo(12, 22);
            ctx.moveTo(2, 12); ctx.lineTo(4.5, 12);
            ctx.moveTo(19.5, 12); ctx.lineTo(22, 12);
            ctx.moveTo(4.93, 4.93); ctx.lineTo(6.7, 6.7);
            ctx.moveTo(17.3, 17.3); ctx.lineTo(19.07, 19.07);
            ctx.moveTo(4.93, 19.07); ctx.lineTo(6.7, 17.3);
            ctx.moveTo(17.3, 6.7); ctx.lineTo(19.07, 4.93);
            ctx.stroke();
        } else if (n === "moon") {
            ctx.beginPath();
            ctx.moveTo(12, 3);
            ctx.bezierCurveTo(7.03, 3, 3, 7.03, 3, 12);
            ctx.bezierCurveTo(3, 16.97, 7.03, 21, 12, 21);
            ctx.bezierCurveTo(16.5, 21, 20.2, 18, 21, 14);
            ctx.bezierCurveTo(15.5, 14.5, 10.5, 9.5, 11, 4);
            ctx.bezierCurveTo(11.3, 3.3, 11.6, 3.1, 12, 3);
            ctx.closePath();
            ctx.stroke();
        } else if (n === "zap" || n === "lightning") {
            ctx.beginPath();
            ctx.moveTo(13, 2); ctx.lineTo(5, 13); ctx.lineTo(12, 13); ctx.lineTo(11, 22); ctx.lineTo(19, 11); ctx.lineTo(12, 11);
            ctx.closePath();
            ctx.stroke();
        } else if (n === "plus") {
            ctx.beginPath();
            ctx.moveTo(12, 5); ctx.lineTo(12, 19);
            ctx.moveTo(5, 12); ctx.lineTo(19, 12);
            ctx.stroke();
        } else if (n === "save") {
            ctx.beginPath();
            ctx.moveTo(19, 21); ctx.lineTo(5, 21);
            ctx.bezierCurveTo(3.9, 21, 3, 20.1, 3, 19);
            ctx.lineTo(3, 5);
            ctx.bezierCurveTo(3, 3.9, 3.9, 3, 5, 3);
            ctx.lineTo(16, 3); ctx.lineTo(21, 8); ctx.lineTo(21, 19);
            ctx.bezierCurveTo(21, 20.1, 20.1, 21, 19, 21);
            ctx.closePath();
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(17, 21); ctx.lineTo(17, 13); ctx.lineTo(7, 13); ctx.lineTo(7, 21);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(7, 3); ctx.lineTo(7, 8); ctx.lineTo(15, 8);
            ctx.stroke();
        } else if (n === "upload") {
            ctx.beginPath();
            ctx.moveTo(4, 17); ctx.lineTo(4, 19);
            ctx.bezierCurveTo(4, 20.1, 4.9, 21, 6, 21);
            ctx.lineTo(18, 21);
            ctx.bezierCurveTo(19.1, 21, 20, 20.1, 20, 19);
            ctx.lineTo(20, 17);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(12, 15); ctx.lineTo(12, 3);
            ctx.moveTo(7, 8); ctx.lineTo(12, 3); ctx.lineTo(17, 8);
            ctx.stroke();
        } else if (n === "rotate-ccw") {
            ctx.beginPath();
            ctx.arc(12, 12, 8, -Math.PI / 2, Math.PI, true);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(1, 4); ctx.lineTo(4, 4); ctx.lineTo(4, 1);
            ctx.stroke();
        } else if (n === "file-text") {
            ctx.beginPath();
            ctx.moveTo(14, 2); ctx.lineTo(6, 2);
            ctx.bezierCurveTo(4.9, 2, 4, 2.9, 4, 4);
            ctx.lineTo(4, 20);
            ctx.bezierCurveTo(4, 21.1, 4.9, 22, 6, 22);
            ctx.lineTo(18, 22);
            ctx.bezierCurveTo(19.1, 22, 20, 21.1, 20, 20);
            ctx.lineTo(20, 8); ctx.lineTo(14, 2);
            ctx.closePath();
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(14, 2); ctx.lineTo(14, 8); ctx.lineTo(20, 8);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(8, 13); ctx.lineTo(16, 13);
            ctx.moveTo(8, 17); ctx.lineTo(14, 17);
            ctx.stroke();
        } else if (n === "archive") {
            ctx.beginPath();
            roundedRect(ctx, 3, 4, 18, 4, 1);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(4, 8); ctx.lineTo(4, 19);
            ctx.arcTo(4, 20, 5, 20, 1);
            ctx.lineTo(19, 20);
            ctx.arcTo(20, 20, 20, 19, 1);
            ctx.lineTo(20, 8);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(10, 12); ctx.lineTo(14, 12);
            ctx.stroke();
        } else if (n === "activity") {
            ctx.beginPath();
            ctx.moveTo(2, 12);
            ctx.lineTo(6, 12);
            ctx.lineTo(9, 4);
            ctx.lineTo(15, 20);
            ctx.lineTo(18, 12);
            ctx.lineTo(22, 12);
            ctx.stroke();
        }
    }
}
