import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

Rectangle {
    id: root

    property color panelBg: "#ffffff"
    property color borderColor: "#d0d7de"
    property color textPrimary: "#1f2329"
    property color textSecondary: "#6b7280"

    radius: 10
    color: panelBg
    border.color: borderColor
    border.width: 1

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 12
        anchors.rightMargin: 12

        Text {
            text: "就绪"
            color: textSecondary
            font.pixelSize: 12
        }

        Item {
            Layout.fillWidth: true
        }

        Text {
            text: "视频：1920x1080   30fps"
            color: textSecondary
            font.pixelSize: 12
        }

        Text {
            text: "音频：48kHz   立体声"
            color: textSecondary
            font.pixelSize: 12
        }

        Rectangle {
            Layout.preferredWidth: 116
            Layout.preferredHeight: 24
            radius: 8
            color: "transparent"
            border.color: borderColor

            Row {
                anchors.centerIn: parent
                spacing: 6

                Text {
                    text: "⛁"
                    color: textSecondary
                    font.pixelSize: 12
                }

                Text {
                    text: "数据库已连接"
                    color: textSecondary
                    font.pixelSize: 12
                }
            }
        }
    }
}
