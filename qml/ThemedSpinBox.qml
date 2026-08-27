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
    property int decimals: 0
    readonly property int scaleFactor: Math.pow(10, decimals)

    implicitHeight: 40
    leftPadding: 34
    rightPadding: 34
    editable: true

    contentItem: TextInput {
        id: input
        z: 2
        text: control.displayTextFromValue(control.value)
        color: control.enabled ? control.textColor : control.disabledTextColor
        opacity: control.enabled ? 1 : 0.65
        font: control.font
        readOnly: !control.editable
        selectByMouse: true
        horizontalAlignment: Qt.AlignHCenter
        verticalAlignment: Qt.AlignVCenter
        inputMethodHints: Qt.ImhFormattedNumbersOnly
        validator: DoubleValidator {
            bottom: control.from / control.scaleFactor
            top: control.to / control.scaleFactor
            decimals: control.decimals
        }
        onEditingFinished: control.value = control.parseValueFromText(text)
    }

    Connections {
        target: control
        function onValueChanged() {
            input.text = control.displayTextFromValue(control.value)
        }
    }

    function displayTextFromValue(value) {
        return (value / scaleFactor).toFixed(decimals)
    }

    function parseValueFromText(text) {
        var parsed = Number(String(text).replace(",", "."))
        return isFinite(parsed) ? Math.round(parsed * scaleFactor) : control.value
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
