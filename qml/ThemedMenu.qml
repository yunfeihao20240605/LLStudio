import QtQuick 2.15
import QtQuick.Controls 2.15

Menu {
    id: control

    property color panelColor: "#ffffff"
    property color borderColor: "#d0d7de"

    topPadding: 5
    bottomPadding: 5
    leftPadding: 1
    rightPadding: 1

    background: Rectangle {
        implicitWidth: 210
        color: control.panelColor
        border.color: control.borderColor
        border.width: 1
        radius: 7
    }
}
