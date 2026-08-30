import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

Dialog {
    id: root

    property color panelBg: "#ffffff"
    property color elevatedBg: "#fafbfc"
    property color borderColor: "#d0d7de"
    property color textPrimary: "#1f2329"
    property color textSecondary: "#6b7280"
    property color accent: "#2f6fed"
    property color accentBg: "#eaf1fe"
    property bool darkTheme: false

    modal: true
    width: 540
    height: 460
    closePolicy: Popup.CloseOnEscape

    background: Rectangle {
        color: root.panelBg
        border.color: root.borderColor
        border.width: 1
        radius: 12
    }

    header: Rectangle {
        implicitHeight: 52
        color: root.panelBg
        border.color: root.borderColor
        border.width: 1

        Label {
            anchors.fill: parent
            anchors.leftMargin: 22
            text: "LLStudio"
            color: root.textPrimary
            font.pixelSize: 20
            verticalAlignment: Text.AlignVCenter
        }
    }

    contentItem: Rectangle {
        color: root.panelBg

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 12

            Image {
                Layout.fillWidth: true
                Layout.preferredHeight: 230
                source: root.darkTheme
                        ? "qrc:/qt/qml/com/yfhao/els/app/llstudio-brand.png"
                        : "qrc:/qt/qml/com/yfhao/els/app/llstudio-brand-light.png"
                fillMode: Image.PreserveAspectFit
                smooth: true
            }

            Label {
                Layout.fillWidth: true
                text: "Version " + (Qt.application.version || "0.3.0")
                color: root.textSecondary
                font.pixelSize: 14
                font.bold: true
                horizontalAlignment: Text.AlignHCenter
            }

            Label {
                Layout.fillWidth: true
                Layout.preferredHeight: implicitHeight
                text: "以兴趣为引，以 AI 为伴，随时随地享受个性化学习"
                color: root.textSecondary
                font.pixelSize: 13
                wrapMode: Text.NoWrap
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }

        }
    }

    footer: DialogButtonBox {
        alignment: Qt.AlignRight

        background: Rectangle {
            color: root.elevatedBg
            border.color: root.borderColor
            border.width: 1
        }

        ThemedToolButton {
            text: "关闭"
            DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
            panelColor: root.panelBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            disabledTextColor: root.textSecondary
            accentColor: root.accent
            accentBackgroundColor: root.accentBg
            onClicked: root.close()
        }
    }
}
