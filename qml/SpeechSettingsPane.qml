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
    property var engineModelLabels: [
        "中文通用",
        "中英粤",
        "中文医疗",
        "英语",
        "粤语",
        "日语",
        "韩语",
        "越南语",
        "马来语",
        "印度尼西亚语",
        "菲律宾语",
        "泰语",
        "葡萄牙语",
        "土耳其语",
        "阿拉伯语",
        "西班牙语",
        "印地语",
        "法语",
        "德语",
        "多方言"
    ]
    property var engineModelValues: [
        "16k_zh",
        "16k_zh-PY",
        "16k_zh_medical",
        "16k_en",
        "16k_yue",
        "16k_ja",
        "16k_ko",
        "16k_vi",
        "16k_ms",
        "16k_id",
        "16k_fil",
        "16k_th",
        "16k_pt",
        "16k_tr",
        "16k_ar",
        "16k_es",
        "16k_hi",
        "16k_fr",
        "16k_de",
        "16k_zh_dialect"
    ]

    clip: true
    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

    function loadFromBridge() {
        if (!settingsBridge)
            return
        recognitionMode.currentIndex = settingsBridge.recognitionMode
                === "sentenceRecognition" ? 0
                : (settingsBridge.recognitionMode === "recordingFile" ? 2 : 1)
        appId.text = settingsBridge.appId
        realtimeEndpoint.text = settingsBridge.realtimeEndpoint
        endpoint.text = settingsBridge.endpoint
        secretId.text = settingsBridge.secretId
        secretKey.text = settingsBridge.secretKey
        region.text = settingsBridge.region
        var configuredEngineModel = settingsBridge.engineModel
        if (configuredEngineModel === "16k_zh_en")
            configuredEngineModel = "16k_zh-PY"
        var configuredIndex = root.engineModelValues.indexOf(configuredEngineModel)
        engineModel.currentIndex = configuredIndex >= 0
                ? configuredIndex
                : root.engineModelValues.indexOf("16k_en")
    }

    function save() {
        if (!settingsBridge)
            return false
        settingsBridge.providerKind = "tencent-asr"
        settingsBridge.recognitionMode = recognitionMode.currentIndex === 0
                ? "sentenceRecognition"
                : (recognitionMode.currentIndex === 2 ? "recordingFile" : "realtime")
        settingsBridge.appId = appId.text.trim()
        settingsBridge.realtimeEndpoint = realtimeEndpoint.text.trim()
        settingsBridge.endpoint = endpoint.text.trim()
        settingsBridge.secretId = secretId.text.trim()
        settingsBridge.secretKey = secretKey.text.trim()
        settingsBridge.region = region.text.trim()
        var selectedEngineModel = root.engineModelValues[engineModel.currentIndex]
        settingsBridge.engineModel = selectedEngineModel || "16k_en"
        return settingsBridge.saveConfig()
    }

    ColumnLayout {
        width: root.availableWidth
        spacing: 8

        Text {
            text: "语音识别"
            color: root.textPrimary
            font.pixelSize: 16
            font.bold: true
        }

        Text {
            text: "配置当前片段字幕识别使用的提供商和识别方式。"
            color: root.textSecondary
            font.pixelSize: 12
        }

        Label { text: "提供商"; color: root.textPrimary }
        ThemedComboBox {
            Layout.fillWidth: true
            model: ["腾讯云"]
            enabled: false
            panelColor: root.panelBg
            elevatedColor: root.elevatedBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            disabledTextColor: root.textSecondary
            accentColor: root.accent
            accentBackgroundColor: root.accentBg
        }

        Label { text: "识别模式"; color: root.textPrimary }
        ThemedComboBox {
            id: recognitionMode
            Layout.fillWidth: true
            model: ["一句话识别（推荐）", "实时语音识别", "录音文件识别"]
            panelColor: root.panelBg
            elevatedColor: root.elevatedBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            disabledTextColor: root.textSecondary
            accentColor: root.accent
            accentBackgroundColor: root.accentBg
        }

        Label {
            text: "AppID"
            color: root.textPrimary
            visible: recognitionMode.currentIndex === 1
        }
        ThemedTextField {
            id: appId
            Layout.fillWidth: true
            visible: recognitionMode.currentIndex === 1
            placeholderText: "腾讯云账号的数字 AppID"
            inputMethodHints: Qt.ImhDigitsOnly
            panelColor: root.panelBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            placeholderColor: root.textSecondary
            accentColor: root.accent
        }

        Label {
            text: "实时 Endpoint"
            color: root.textPrimary
            visible: recognitionMode.currentIndex === 1
        }
        ThemedTextField {
            id: realtimeEndpoint
            Layout.fillWidth: true
            visible: recognitionMode.currentIndex === 1
            placeholderText: "wss://asr.cloud.tencent.com"
            panelColor: root.panelBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            placeholderColor: root.textSecondary
            accentColor: root.accent
        }

        Label {
            text: recognitionMode.currentIndex === 0
                  ? "一句话识别 Endpoint" : "录音文件 Endpoint"
            color: root.textPrimary
            visible: recognitionMode.currentIndex !== 1
        }
        ThemedTextField {
            id: endpoint
            Layout.fillWidth: true
            visible: recognitionMode.currentIndex !== 1
            placeholderText: "例如：https://asr.tencentcloudapi.com"
            panelColor: root.panelBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            placeholderColor: root.textSecondary
            accentColor: root.accent
        }

        Label { text: "SecretId"; color: root.textPrimary }
        ThemedTextField {
            id: secretId
            Layout.fillWidth: true
            panelColor: root.panelBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            placeholderColor: root.textSecondary
            accentColor: root.accent
        }

        Label { text: "SecretKey"; color: root.textPrimary }
        ThemedTextField {
            id: secretKey
            Layout.fillWidth: true
            echoMode: TextInput.Password
            panelColor: root.panelBg
            borderColor: root.borderColor
            textColor: root.textPrimary
            placeholderColor: root.textSecondary
            accentColor: root.accent
        }

        RowLayout {
            Layout.fillWidth: true
            ColumnLayout {
                Layout.fillWidth: true
                visible: recognitionMode.currentIndex === 2
                Label { text: "地域"; color: root.textPrimary }
                ThemedTextField {
                    id: region
                    Layout.fillWidth: true
                    placeholderText: "ap-shanghai"
                    panelColor: root.panelBg
                    borderColor: root.borderColor
                    textColor: root.textPrimary
                    placeholderColor: root.textSecondary
                    accentColor: root.accent
                }
            }
            ColumnLayout {
                Layout.fillWidth: true
                Label { text: "语言模型"; color: root.textPrimary }
                ThemedComboBox {
                    id: engineModel
                    Layout.fillWidth: true
                    model: root.engineModelLabels
                    panelColor: root.panelBg
                    elevatedColor: root.elevatedBg
                    borderColor: root.borderColor
                    textColor: root.textPrimary
                    disabledTextColor: root.textSecondary
                    accentColor: root.accent
                    accentBackgroundColor: root.accentBg
                }
            }
        }

        Label {
            Layout.fillWidth: true
            visible: recognitionMode.currentIndex === 0
            text: "一句话识别适合 60 秒以内的学习片段，使用独立的一句话识别调用额度。"
            color: root.textSecondary
            wrapMode: Text.Wrap
        }

        Label {
            Layout.fillWidth: true
            visible: recognitionMode.currentIndex === 1
            text: "实时模式使用 AppID 对应的实时 ASR 额度，不依赖录音文件识别资源包。"
            color: root.textSecondary
            wrapMode: Text.Wrap
        }

        RowLayout {
            Layout.fillWidth: true

            Label {
                Layout.fillWidth: true
                text: root.settingsBridge ? root.settingsBridge.statusMessage : ""
                color: root.textSecondary
                wrapMode: Text.Wrap
                visible: text.length > 0
            }

            ThemedToolButton {
                text: "保存语音识别设置"
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
