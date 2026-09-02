import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Controls.impl 2.15
import QtQuick.Layouts 1.15

MenuItem {
    id: control

    property color textColor: "#1f2329"
    property color disabledTextColor: "#6b7280"
    property color hoverColor: "#eaf1fe"

    TextMetrics {
        id: menuTextMetrics
        font: control.font
        text: control.text
    }

    implicitWidth: Math.max(
                       210,
                       menuTextMetrics.width + 24 + 10 + leftPadding + rightPadding)
    implicitHeight: 34
    leftPadding: 12
    rightPadding: 12

    contentItem: RowLayout {
        spacing: 10

        Item {
            Layout.preferredWidth: 24
            Layout.minimumWidth: 24
            Layout.maximumWidth: 24
            Layout.fillHeight: true

            ColorImage {
                anchors.centerIn: parent
                width: 22
                height: 22
                source: control.icon.source
                sourceSize.width: 22
                sourceSize.height: 22
                fillMode: Image.PreserveAspectFit
                defaultColor: "#374151"
                color: control.enabled ? control.textColor : control.disabledTextColor
                smooth: true
                visible: source !== ""
            }
        }

        Text {
            Layout.fillWidth: true
            text: control.text
            color: control.enabled ? control.textColor : control.disabledTextColor
            opacity: control.enabled ? 1 : 0.65
            font: control.font
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }
    }

    background: Rectangle {
        color: control.highlighted ? control.hoverColor : "transparent"
        radius: 5
    }
}
