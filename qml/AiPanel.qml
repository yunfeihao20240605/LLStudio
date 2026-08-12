import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

Rectangle {
    id: root
    property var aiBridge
    property color panelBg: "#ffffff"
    property color elevatedBg: "#fafbfc"
    property color borderColor: "#d0d7de"
    property color textPrimary: "#1f2329"
    property color textSecondary: "#6b7280"
    property color accent: "#2f6fed"
    property color accentBg: "#eaf1fe"
    property bool showQuickQuestions: false
    property var filteredQuickQuestions: []
    readonly property var quickQuestions: [
        "这句话怎么发音？",
        "请分析连读和弱读。",
        "重音应该放在哪里？"
    ]
    color: panelBg
    border.color: borderColor
    radius: 10

    function clearInputFocus() {
        if (!input.activeFocus)
            return
        input.focus = false
        root.forceActiveFocus(Qt.MouseFocusReason)
    }

    function clearInputFocusIfOutside(sourceItem, sourceX, sourceY) {
        if (!input.activeFocus || !sourceItem)
            return
        var point = input.mapFromItem(sourceItem, sourceX, sourceY)
        if (point.x < 0 || point.y < 0
                || point.x > input.width || point.y > input.height)
            clearInputFocus()
    }

    function updateQuickQuestions() {
        var value = input.text
        var trigger = value.length > 0 ? value.charAt(0) : ""
        if (trigger !== "/" && trigger !== "、") {
            showQuickQuestions = false
            filteredQuickQuestions = []
            return
        }
        var query = value.slice(1).trim().toLowerCase()
        var matches = []
        for (var i = 0; i < quickQuestions.length; ++i) {
            if (query.length === 0 || quickQuestions[i].toLowerCase().indexOf(query) >= 0)
                matches.push(quickQuestions[i])
        }
        filteredQuickQuestions = matches
        showQuickQuestions = matches.length > 0
    }

    function chooseQuickQuestion(question) {
        showQuickQuestions = false
        input.text = question
        input.forceActiveFocus()
        send()
    }

    Timer { interval: 100; repeat: true; running: root.aiBridge && root.aiBridge.isSending; onTriggered: root.aiBridge.poll() }

    Shortcut {
        sequence: "Escape"
        context: Qt.ApplicationShortcut
        enabled: input.activeFocus
        onActivated: root.clearInputFocus()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 6
        RowLayout {
            Layout.fillWidth: true
            Item { Layout.fillWidth: true }
            ToolButton {
                id: clearButton
                Layout.preferredWidth: 30
                Layout.preferredHeight: 30
                text: "⌫"
                enabled: root.aiBridge
                onClicked: root.aiBridge.clearConversation()
                background: Item {}
                contentItem: Text {
                    text: clearButton.text
                    color: clearButton.down ? root.accent
                                            : clearButton.hovered ? root.textPrimary : root.textSecondary
                    font.pixelSize: 18
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                ToolTip.visible: hovered
                ToolTip.text: "清空对话"
            }
        }
        ScrollView {
            Layout.fillWidth: true; Layout.fillHeight: true; clip: true
            Column {
                width: parent.width
                spacing: 10

        Text {
                    width: parent.width
                    text: root.aiBridge && root.aiBridge.currentOriginal.length > 0
                          ? root.aiBridge.currentOriginal : "尚未选择字幕"
                    color: root.textPrimary
                    wrapMode: Text.Wrap
                    textFormat: Text.PlainText
                }

                Text {
                    width: parent.width
                    visible: root.aiBridge && root.aiBridge.currentTranslated.length > 0
                    text: root.aiBridge ? root.aiBridge.currentTranslated : ""
                    color: root.textSecondary
                    wrapMode: Text.Wrap
                    textFormat: Text.PlainText
                }

                Rectangle {
                    width: parent.width
                    height: 1
                    color: root.borderColor
                }

                Text {
                    id: messages
                    width: parent.width
                    text: root.aiBridge ? formatMessages(root.aiBridge.messagesJson) : ""
                    color: root.textPrimary
                    wrapMode: Text.Wrap
                    textFormat: Text.RichText
                }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            TextField {
                id: input
                Layout.fillWidth: true
                placeholderText: "输入问题，或输入 / 选择快捷问题…"
                onTextChanged: root.updateQuickQuestions()
                onAccepted: {
                    if (root.showQuickQuestions && quickList.count > 0)
                        root.chooseQuickQuestion(
                                    root.filteredQuickQuestions[
                                        Math.max(0, quickList.currentIndex)])
                    else
                        root.send()
                }
                Keys.onPressed: function(event) {
                    if (!root.showQuickQuestions)
                        return
                    if (event.key === Qt.Key_Down) {
                        quickList.currentIndex = Math.min(quickList.count - 1,
                                                          quickList.currentIndex + 1)
                        event.accepted = true
                    } else if (event.key === Qt.Key_Up) {
                        quickList.currentIndex = Math.max(0,
                                                          quickList.currentIndex - 1)
                        event.accepted = true
                    } else if (event.key === Qt.Key_Escape) {
                        root.showQuickQuestions = false
                        event.accepted = true
                    }
                }
            }
            Button { text: root.aiBridge && root.aiBridge.isSending ? "请求中" : "发送"; enabled: root.aiBridge && !root.aiBridge.isSending; onClicked: send() }
        }
        Text { Layout.fillWidth: true; text: root.aiBridge ? root.aiBridge.errorMessage : ""; color: "#c03d3d"; wrapMode: Text.Wrap; visible: text.length > 0 }
    }

    Popup {
        id: quickQuestionsPopup
        parent: root
        x: input.mapToItem(root, 0, 0).x
        y: input.mapToItem(root, 0, 0).y - height - 4
        width: input.width
        height: Math.min(quickList.contentHeight + 8, 150)
        padding: 4
        visible: root.showQuickQuestions
        closePolicy: Popup.CloseOnPressOutside
        onClosed: root.showQuickQuestions = false

        background: Rectangle {
            color: root.panelBg
            border.color: root.borderColor
            radius: 6
        }

        ListView {
            id: quickList
            anchors.fill: parent
            clip: true
            model: root.filteredQuickQuestions
            currentIndex: 0
            delegate: Rectangle {
                width: quickList.width
                height: 32
                radius: 4
                color: index === quickList.currentIndex ? root.accentBg : "transparent"

                Text {
                    anchors.fill: parent
                    anchors.leftMargin: 8
                    text: modelData
                    color: root.textPrimary
                    verticalAlignment: Text.AlignVCenter
                }

                MouseArea {
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: root.chooseQuickQuestion(modelData)
                }
            }
        }
    }

    function send() {
        if (input.text.trim().length > 0 && root.aiBridge && root.aiBridge.sendMessage(input.text)) {
            input.clear()
            showQuickQuestions = false
        }
    }
    function formatMessages(json) {
        try {
            var items = JSON.parse(json || "[]")
            var output = []
            for (var i = 0; i < items.length; ++i) {
                var label = items[i].role === "User" ? "我" : "AI"
                var rendered = items[i].renderedContent || ""
                output.push("<div><b>" + label + "：</b>" + rendered + "</div>")
            }
            return output.join("<br/>")
        } catch (error) { return "" }
    }

}
