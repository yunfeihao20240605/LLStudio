import QtQuick 2.15
import QtQuick.Controls 2.15

Button {
    id: root

    property color panelColor: "#ffffff"
    property color borderColor: "#d0d7de"
    property color textColor: "#1f2329"
    property color disabledTextColor: "#6b7280"
    property color accentColor: "#2f6fed"
    property color accentBackgroundColor: "#eaf1fe"

    implicitHeight: 34
    leftPadding: 12
    rightPadding: 12

    contentItem: Text {
        text: root.text
        color: root.enabled
               ? (root.hovered || root.down ? root.accentColor : root.textColor)
               : root.disabledTextColor
        opacity: root.enabled ? 1 : 0.58
        font: root.font
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        radius: 7
        color: !root.enabled ? root.panelColor
                             : (root.down
                                ? Qt.darker(root.accentBackgroundColor, 1.05)
                                : (root.hovered
                                   ? root.accentBackgroundColor
                                   : root.panelColor))
        border.color: root.enabled && (root.hovered || root.down)
                      ? root.accentColor : root.borderColor
        border.width: 1
    }
}
