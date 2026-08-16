import QtQuick 2.15
import QtQuick.Controls 2.15

MenuItem {
    id: control

    property color textColor: "#1f2329"
    property color disabledTextColor: "#6b7280"
    property color hoverColor: "#eaf1fe"

    implicitWidth: Math.max(210, contentItem.implicitWidth + leftPadding + rightPadding)
    implicitHeight: 34
    leftPadding: 12
    rightPadding: 12

    contentItem: Text {
        text: control.text
        color: control.enabled ? control.textColor : control.disabledTextColor
        opacity: control.enabled ? 1 : 0.65
        font: control.font
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        color: control.highlighted ? control.hoverColor : "transparent"
        radius: 5
    }
}
