import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

ScrollView {
    id: root

    property var settingsBridge
    property color textPrimary: "#1f2329"
    property color textSecondary: "#6b7280"
    property color accent: "#2f6fed"
    property color panelBg: "#ffffff"
    property color elevatedBg: "#fafbfc"
    property color borderColor: "#d0d7de"
    property color accentBg: "#eaf1fe"
    property string saveStatus: ""

    clip: true
    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

    function loadFromBridge() {
        if (!settingsBridge)
            return
        baseUrl.text = settingsBridge.baseUrl
        apiKey.text = settingsBridge.apiKey
        modelName.text = settingsBridge.model
        prompt.text = settingsBridge.systemPrompt
        saveStatus = ""
    }

    function save() {
        if (!settingsBridge)
            return false
        settingsBridge.baseUrl = baseUrl.text.trim()
        settingsBridge.apiKey = apiKey.text.trim()
        settingsBridge.model = modelName.text.trim()
        settingsBridge.systemPrompt = prompt.text.trim()
        var saved = settingsBridge.saveConfig()
        saveStatus = saved ? "AI 配置已保存" : settingsBridge.errorMessage
        return saved
    }

    ColumnLayout {
        width: root.availableWidth
        spacing: 9

        Text {
            text: "AI"
            color: root.textPrimary
            font.pixelSize: 16
            font.bold: true
        }

        Text {
            text: "配置字幕学习对话使用的模型服务。"
            color: root.textSecondary
            font.pixelSize: 12
        }

        Label { text: "协议"; color: root.textPrimary }
        ThemedComboBox {
            Layout.fillWidth: true
            model: ["OpenAI 兼容"]
            enabled: false
            panelColor: root.panelBg
            elevatedColor: root.elevatedBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            disabledTextColor: root.textSecondary
            accentColor: root.accent
            accentBackgroundColor: root.accentBg
        }

        Label { text: "Base URL"; color: root.textPrimary }
        ThemedTextField {
            id: baseUrl
            Layout.fillWidth: true
            placeholderText: "例如：https://api.example.com/v1"
            panelColor: root.panelBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            placeholderColor: root.textSecondary
            accentColor: root.accent
        }

        Label { text: "API Key"; color: root.textPrimary }
        ThemedTextField {
            id: apiKey
            Layout.fillWidth: true
            placeholderText: "输入 API Key"
            echoMode: TextInput.Password
            panelColor: root.panelBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            placeholderColor: root.textSecondary
            accentColor: root.accent
        }

        Label { text: "模型"; color: root.textPrimary }
        ThemedTextField {
            id: modelName
            Layout.fillWidth: true
            placeholderText: "输入模型名称"
            panelColor: root.panelBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            placeholderColor: root.textSecondary
            accentColor: root.accent
        }

        RowLayout {
            Layout.fillWidth: true

            Label {
                Layout.fillWidth: true
                text: "系统提示词"
                color: root.textPrimary
            }

            ThemedToolButton {
                text: "恢复默认"
                panelColor: root.panelBg
                borderColor: root.borderColor
                textColor: root.textPrimary
                disabledTextColor: root.textSecondary
                accentColor: root.accent
                accentBackgroundColor: root.accentBg
                onClicked: {
                    if (!root.settingsBridge)
                        return
                    root.settingsBridge.restoreDefaultPrompt()
                    prompt.text = root.settingsBridge.systemPrompt
                    root.saveStatus = ""
                }
            }
        }

        ThemedTextArea {
            id: prompt
            Layout.fillWidth: true
            Layout.preferredHeight: 155
            placeholderText: "输入系统提示词"
            wrapMode: TextEdit.Wrap
            panelColor: root.panelBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            placeholderColor: root.textSecondary
            accentColor: root.accent
        }

        RowLayout {
            Layout.fillWidth: true

            Label {
                Layout.fillWidth: true
                text: root.saveStatus
                color: root.saveStatus === "AI 配置已保存"
                       ? root.accent : "#c03d3d"
                wrapMode: Text.Wrap
                visible: text.length > 0
            }

            ThemedToolButton {
                text: "保存 AI 设置"
                panelColor: root.panelBg
                borderColor: root.borderColor
                textColor: root.textPrimary
                disabledTextColor: root.textSecondary
                accentColor: root.accent
                accentBackgroundColor: root.accentBg
                onClicked: root.save()
            }
        }
    }
}
