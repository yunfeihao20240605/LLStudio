pragma ComponentBehavior: Bound

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

    title: "快捷键"
    modal: true
    width: 510
    height: 520
    closePolicy: Popup.CloseOnEscape

    background: Rectangle {
        color: root.panelBg
        border.color: root.borderColor
        border.width: 1
        radius: 12
    }

    header: Rectangle {
        implicitHeight: 54
        color: root.elevatedBg
        radius: 12

        Text {
            anchors.left: parent.left
            anchors.leftMargin: 18
            anchors.verticalCenter: parent.verticalCenter
            text: "快捷键"
            color: root.textPrimary
            font.pixelSize: 17
            font.bold: true
        }
    }

    footer: Rectangle {
        implicitHeight: 58
        color: root.elevatedBg
        border.color: root.borderColor
        border.width: 1

        ThemedToolButton {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.rightMargin: 16
            width: 112
            text: "关闭"
            panelColor: root.panelBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            disabledTextColor: root.textSecondary
            accentColor: root.accent
            accentBackgroundColor: root.accentBg
            onClicked: root.close()
        }
    }

    // 修饰键按平台显示：macOS 为 ⌘，Windows / Linux 为 Ctrl
    readonly property bool isMac: Qt.platform.os === "osx"
    readonly property string modKey: isMac ? "⌘" : "Ctrl"
    readonly property string modName: isMac ? "⌘（Command）" : "Ctrl"

    contentItem: ListView {
        id: shortcutList
        clip: true
        spacing: 4
        model: [
            { keys: root.modKey + " O", action: "打开视频" },
            { keys: root.modKey + " ,", action: "打开设置" },
            { keys: root.modKey + " F", action: "查找" },
            { keys: "空格", action: "开始、暂停或继续训练；无选区时控制普通播放" },
            { keys: "Shift + 空格", action: "播放、暂停或继续原视频" },
            { keys: "N", action: "以上一片段终点开始新片段" },
            { keys: "Delete", action: "删除学习库中当前选中的视频" },
            { keys: "Esc", action: "退出全屏、取消多选或退出文本编辑焦点" }
        ]

        delegate: Rectangle {
            id: shortcutRow
            required property int index
            required property var modelData

            width: shortcutList.width
            height: 46
            radius: 8
            color: shortcutRow.index % 2 === 0 ? root.elevatedBg : "transparent"

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 16

                Label {
                    Layout.preferredWidth: 105
                    text: shortcutRow.modelData.keys
                    color: root.accent
                    font.bold: true
                    font.pixelSize: 13
                }

                Label {
                    Layout.fillWidth: true
                    text: shortcutRow.modelData.action
                    color: root.textPrimary
                    wrapMode: Text.Wrap
                    font.pixelSize: 13
                }
            }
        }
    }
}
