import QtQuick 2.15
import QtQuick.Controls 2.15

Slider {
    id: root

    property color panelColor: "#ffffff"
    property color trackColor: "#d0d7de"
    property color accentColor: "#2f6fed"

    background: Rectangle {
        x: root.leftPadding
        y: root.topPadding + root.availableHeight / 2 - height / 2
        width: root.availableWidth
        height: 4
        radius: 2
        color: root.trackColor
        opacity: root.enabled ? 1 : 0.55

        Rectangle {
            width: root.visualPosition * parent.width
            height: parent.height
            radius: parent.radius
            color: root.accentColor
            opacity: root.enabled ? 1 : 0.55
        }
    }

    handle: Rectangle {
        x: root.leftPadding + root.visualPosition * (root.availableWidth - width)
        y: root.topPadding + root.availableHeight / 2 - height / 2
        implicitWidth: 18
        implicitHeight: 18
        radius: 9
        color: root.pressed ? root.accentColor : root.panelColor
        border.color: root.enabled ? root.accentColor : root.trackColor
        border.width: 2
        opacity: root.enabled ? 1 : 0.7
    }
}
