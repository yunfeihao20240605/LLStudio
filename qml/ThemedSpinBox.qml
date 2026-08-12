import QtQuick 2.15
import QtQuick.Controls 2.15

SpinBox {
    id: control

    property color panelColor: "#ffffff"
    property color elevatedColor: "#fafbfc"
    property color borderColor: "#d0d7de"
    property color textColor: "#1f2329"
    property color disabledTextColor: "#6b7280"
    property color accentColor: "#2f6fed"
    property color accentBackgroundColor: "#eaf1fe"

    implicitHeight: 40
    leftPadding: 34
    rightPadding: 34
    editable: false

    contentItem: TextInput {
        z: 2
        text: control.textFromValue(control.value, control.locale)
        color: control.enabled ? control.textColor : control.disabledTextColor
        opacity: control.enabled ? 1 : 0.65
        font: control.font
        readOnly: true
        selectByMouse: false
        horizontalAlignment: Qt.AlignHCenter
        verticalAlignment: Qt.AlignVCenter
        inputMethodHints: Qt.ImhFormattedNumbersOnly
    }

    down.indicator: Rectangle {
        x: 1
        y: 1
        width: 32
        height: control.height - 2
        radius: 6
        color: !control.enabled ? control.elevatedColor
                               : (control.down.pressed
                                  ? control.accentBackgroundColor
                                  : (control.down.hovered
                                     ? control.accentBackgroundColor
                                     : control.elevatedColor))

        Text {
            anchors.centerIn: parent
            text: "−"
            color: control.enabled ? control.textColor
                                   : control.disabledTextColor
            opacity: control.enabled ? 1 : 0.6
            font.pixelSize: 20
            font.bold: true
        }
    }

    up.indicator: Rectangle {
        x: control.width - width - 1
        y: 1
        width: 32
        height: control.height - 2
        radius: 6
        color: !control.enabled ? control.elevatedColor
                               : (control.up.pressed
                                  ? control.accentBackgroundColor
                                  : (control.up.hovered
                                     ? control.accentBackgroundColor
                                     : control.elevatedColor))

        Text {
            anchors.centerIn: parent
            text: "+"
            color: control.enabled ? control.textColor
                                   : control.disabledTextColor
            opacity: control.enabled ? 1 : 0.6
            font.pixelSize: 20
            font.bold: true
        }
    }

    background: Rectangle {
        radius: 7
        color: control.panelColor
        border.color: control.activeFocus ? control.accentColor
                                          : control.borderColor
        border.width: control.activeFocus ? 2 : 1
    }
}
