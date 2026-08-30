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

    title: "关于 LLStudio"
    modal: true
    width: 540
    height: 430
    closePolicy: Popup.CloseOnEscape

    background: Rectangle {
        color: root.panelBg
        border.color: root.borderColor
        border.width: 1
        radius: 12
    }

    contentItem: ColumnLayout {
        spacing: 12

        Image {
            Layout.fillWidth: true
            Layout.preferredHeight: 270
            source: root.darkTheme
                    ? "qrc:/qt/qml/com/yfhao/els/app/llstudio-brand.png"
                    : "qrc:/qt/qml/com/yfhao/els/app/llstudio-brand-light.png"
            fillMode: Image.PreserveAspectFit
            smooth: true
        }

        Label {
            Layout.fillWidth: true
            text: "LLStudio"
            color: root.textPrimary
            font.pixelSize: 21
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
        }

        Label {
            Layout.fillWidth: true
            text: "语言学习工作台"
            color: root.textSecondary
            font.pixelSize: 13
            horizontalAlignment: Text.AlignHCenter
        }

        Label {
            Layout.fillWidth: true
            text: "版本 " + (Qt.application.version || "0.1.3")
            color: root.textSecondary
            font.pixelSize: 12
            horizontalAlignment: Text.AlignHCenter
        }
    }

    footer: DialogButtonBox {
        alignment: Qt.AlignRight

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
