import QtQuick 2.15
import QtQuick.Controls 2.15

MenuSeparator {
    id: control

    property color separatorColor: "#d0d7de"

    implicitHeight: visible ? 9 : 0
    contentItem: Rectangle {
        implicitHeight: 1
        color: control.separatorColor
    }
}
