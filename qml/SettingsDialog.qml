pragma ComponentBehavior: Bound

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

Dialog {
    id: root

    property var themeBridge
    property var aiSettingsBridge
    property var speechSettingsBridge
    property int activePage: 0
    property color panelBg: "#ffffff"
    property color elevatedBg: "#fafbfc"
    property color borderColor: "#d0d7de"
    property color textPrimary: "#1f2329"
    property color textSecondary: "#6b7280"
    property color accent: "#2f6fed"
    property color accentBg: "#eaf1fe"

    title: "设置"
    modal: true
    width: 680
    height: 610
    closePolicy: Popup.CloseOnEscape

    onOpened: {
        aiSettingsPane.loadFromBridge()
        speechSettingsPane.loadFromBridge()
    }

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
            text: "设置"
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

    contentItem: RowLayout {
        spacing: 0

        Rectangle {
            Layout.preferredWidth: 130
            Layout.fillHeight: true
            color: root.elevatedBg

            Column {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: 10
                spacing: 6

                Repeater {
                    model: ["外观", "AI", "语音识别"]

                    delegate: Rectangle {
                        id: settingsPageOption
                        required property int index
                        required property string modelData
                        width: parent.width
                        height: 38
                        radius: 8
                        color: index === root.activePage
                               ? root.accentBg : "transparent"

                        Text {
                            anchors.centerIn: parent
                            text: settingsPageOption.modelData
                            color: settingsPageOption.index === root.activePage
                                   ? root.accent : root.textSecondary
                            font.pixelSize: 14
                            font.bold: settingsPageOption.index === root.activePage
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.activePage = settingsPageOption.index
                        }
                    }
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 20
            spacing: 12
            visible: root.activePage === 0

            Text {
                text: "主题"
                color: root.textPrimary
                font.pixelSize: 16
                font.bold: true
            }

            Text {
                text: "选择后立即生效，并在下次启动时继续使用。"
                color: root.textSecondary
                font.pixelSize: 12
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 2
                columnSpacing: 10
                rowSpacing: 10

                Repeater {
                    model: [
                        { id: "auto", name: "跟随系统", bg: "#e9edf2", panel: "#ffffff", accent: "#64748b" },
                        { id: "dark", name: "深邃暗色", bg: "#1e1f22", panel: "#2a2b2e", accent: "#5b8df6" },
                        { id: "midnight", name: "午夜蓝", bg: "#08121e", panel: "#0d1926", accent: "#38a7ff" },
                        { id: "aurora", name: "极光青", bg: "#071916", panel: "#0d211d", accent: "#2dd4bf" },
                        { id: "twilight", name: "暮光紫", bg: "#151020", panel: "#1d1729", accent: "#a78bfa" },
                        { id: "light", name: "柔和亮色", bg: "#f5f6f8", panel: "#ffffff", accent: "#2f6fed" },
                        { id: "paper", name: "暖纸色", bg: "#eee8dc", panel: "#faf6ec", accent: "#a85d2a" },
                        { id: "sky", name: "清新浅蓝", bg: "#eef4fa", panel: "#f8fbff", accent: "#2878c8" }
                    ]

                    delegate: Rectangle {
                        id: themeOption
                        required property var modelData

                        Layout.fillWidth: true
                        Layout.preferredHeight: 74
                        radius: 9
                        color: root.themeBridge
                               && root.themeBridge.themeMode === modelData.id
                               ? root.accentBg : root.elevatedBg
                        border.color: root.themeBridge
                                      && root.themeBridge.themeMode === modelData.id
                                      ? root.accent : root.borderColor
                        border.width: root.themeBridge
                                      && root.themeBridge.themeMode === modelData.id ? 2 : 1

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 10
                            spacing: 10

                            Rectangle {
                                Layout.preferredWidth: 54
                                Layout.preferredHeight: 42
                                radius: 6
                                color: themeOption.modelData.bg
                                border.color: root.borderColor

                                Rectangle {
                                    anchors.centerIn: parent
                                    width: 35
                                    height: 24
                                    radius: 4
                                    color: themeOption.modelData.panel

                                    Rectangle {
                                        anchors.left: parent.left
                                        anchors.bottom: parent.bottom
                                        anchors.margins: 4
                                        width: 18
                                        height: 4
                                        radius: 2
                                        color: themeOption.modelData.accent
                                    }
                                }
                            }

                            Text {
                                Layout.fillWidth: true
                                text: themeOption.modelData.name
                                color: root.textPrimary
                                font.pixelSize: 13
                                font.bold: root.themeBridge
                                           && root.themeBridge.themeMode
                                              === themeOption.modelData.id
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                if (root.themeBridge)
                                    root.themeBridge.applyThemeMode(
                                                themeOption.modelData.id)
                            }
                        }
                    }
                }
            }

            Item { Layout.fillHeight: true }
        }

        AiSettingsPane {
            id: aiSettingsPane
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 20
            visible: root.activePage === 1
            settingsBridge: root.aiSettingsBridge
            textPrimary: root.textPrimary
            textSecondary: root.textSecondary
            accent: root.accent
            panelBg: root.panelBg
            elevatedBg: root.elevatedBg
            borderColor: root.borderColor
            accentBg: root.accentBg
        }

        SpeechSettingsPane {
            id: speechSettingsPane
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 20
            visible: root.activePage === 2
            settingsBridge: root.speechSettingsBridge
            textPrimary: root.textPrimary
            textSecondary: root.textSecondary
            accent: root.accent
            panelBg: root.panelBg
            elevatedBg: root.elevatedBg
            borderColor: root.borderColor
            accentBg: root.accentBg
        }
    }
}
