import QtQuick 2.15
import QtQuick.Controls 2.15

TextField {
    id: control

    property color panelColor: "#ffffff"
    property color borderColor: "#d0d7de"
    property color textColor: "#1f2329"
    property color placeholderColor: "#6b7280"
    property color accentColor: "#2f6fed"

    implicitHeight: 40
    leftPadding: 12
    rightPadding: 12
    color: control.textColor
    placeholderTextColor: control.placeholderColor
    selectionColor: control.accentColor
    selectedTextColor: "#ffffff"

    background: Rectangle {
        radius: 7
        color: control.panelColor
        border.color: control.activeFocus ? control.accentColor
                                          : control.borderColor
        border.width: control.activeFocus ? 2 : 1
    }
}
