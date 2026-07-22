import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

Rectangle {
    id: root

    property color panelBg: "#ffffff"
    property color elevatedBg: "#fafbfc"
    property color borderColor: "#d0d7de"
    property color textPrimary: "#1f2329"
    property color textSecondary: "#6b7280"
    property color accent: "#2f6fed"
    property color accentBg: "#eaf1fe"
    property var subtitleBridge
    property var waveformBridge
    property int activeTab: 0
    property var subtitleEntries: []

    function selectionIsValid() {
        return waveformBridge
                && waveformBridge.hasSelectionStart
                && waveformBridge.hasSelectionEnd
                && waveformBridge.selectionEnd > waveformBridge.selectionStart
    }

    function formatTimestamp(totalSeconds) {
        var totalMillis = Math.max(0, Math.round((totalSeconds || 0) * 1000))
        var hours = Math.floor(totalMillis / 3600000)
        var minutes = Math.floor((totalMillis % 3600000) / 60000)
        var seconds = Math.floor((totalMillis % 60000) / 1000)
        var millis = totalMillis % 1000
        return (hours < 10 ? "0" + hours : hours) + ":"
                + (minutes < 10 ? "0" + minutes : minutes) + ":"
                + (seconds < 10 ? "0" + seconds : seconds) + ","
                + (millis < 10 ? "00" + millis : (millis < 100 ? "0" + millis : millis))
    }

    function updateSubtitleEntries() {
        if (!subtitleBridge || subtitleBridge.entriesJson.length === 0) {
            subtitleEntries = []
            return
        }
        subtitleEntries = JSON.parse(subtitleBridge.entriesJson)
    }

    function syncSelectionEditor() {
        if (!subtitleBridge)
            return
        if (!selectionIsValid()) {
            subtitleBridge.syncSelectionRange(0, 0)
            return
        }
        subtitleBridge.syncSelectionRange(waveformBridge.selectionStart,
                                          waveformBridge.selectionEnd)
    }

    function saveEditorText() {
        if (!subtitleBridge || !selectionIsValid()
                || subtitleEditor.text.trim().length === 0)
            return
        if (subtitleBridge.saveCueForRange(waveformBridge.selectionStart,
                                           waveformBridge.selectionEnd,
                                           subtitleEditor.text))
            clearEditorFocus()
    }

    function clearEditorFocus() {
        if (!subtitleEditor.activeFocus)
            return
        subtitleEditor.focus = false
        root.forceActiveFocus(Qt.MouseFocusReason)
    }

    function clearEditorFocusIfOutside(sourceItem, sourceX, sourceY) {
        if (!subtitleEditor.activeFocus || !sourceItem)
            return
        var point = subtitleEditor.mapFromItem(sourceItem, sourceX, sourceY)
        if (point.x < 0 || point.y < 0
                || point.x > subtitleEditor.width
                || point.y > subtitleEditor.height)
            clearEditorFocus()
    }

    Component.onCompleted: {
        updateSubtitleEntries()
        syncSelectionEditor()
        subtitleEditor.text = subtitleBridge ? subtitleBridge.editingText : ""
    }

    radius: 16
    color: panelBg
    border.color: borderColor
    border.width: 1

    Connections {
        target: subtitleBridge
        ignoreUnknownSignals: true

        function onEntriesJsonChanged() {
            root.updateSubtitleEntries()
        }

        function onEditingTextChanged() {
            subtitleEditor.text = root.subtitleBridge ? root.subtitleBridge.editingText : ""
        }
    }

    Shortcut {
        sequence: "Escape"
        context: Qt.ApplicationShortcut
        enabled: subtitleEditor.activeFocus
        onActivated: root.clearEditorFocus()
    }

    Connections {
        target: waveformBridge
        ignoreUnknownSignals: true

        function onSelectionRevisionChanged() { root.syncSelectionEditor() }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 10

        RowLayout {
            Layout.fillWidth: true

            Text {
                text: "字幕与笔记"
                color: root.textPrimary
                font.pixelSize: 16
                font.bold: true
            }

            Item { Layout.fillWidth: true }

            Text {
                visible: root.activeTab === 0
                text: root.subtitleEntries.length + " 条字幕"
                color: root.textSecondary
                font.pixelSize: 12
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 0

            Repeater {
                model: ["字幕", "笔记", "单词"]

                delegate: Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 34
                    radius: 8
                    color: index === root.activeTab ? root.accentBg : "transparent"
                    border.color: index === root.activeTab ? root.accent : root.borderColor
                    border.width: index === root.activeTab ? 1 : 0

                    Text {
                        anchors.centerIn: parent
                        text: modelData
                        color: index === root.activeTab ? root.accent : root.textSecondary
                        font.pixelSize: 13
                        font.bold: index === root.activeTab
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: root.activeTab = index
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 128
            visible: root.activeTab === 0
            radius: 9
            color: root.elevatedBg
            border.color: root.subtitleBridge && root.subtitleBridge.editingCueIndex >= 0
                          ? root.accent : root.borderColor

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 8
                spacing: 6

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        Layout.fillWidth: true
                        text: !root.subtitleBridge || !root.subtitleBridge.hasVideo
                              ? "请先加载视频"
                              : (!root.selectionIsValid()
                                 ? "请先设置有效的 A～B 字幕时间"
                                 : root.formatTimestamp(root.waveformBridge.selectionStart)
                                   + " --> "
                                   + root.formatTimestamp(root.waveformBridge.selectionEnd))
                        color: root.textSecondary
                        font.pixelSize: 12
                        elide: Text.ElideRight
                    }

                    Button {
                        text: root.subtitleBridge && root.subtitleBridge.editingCueIndex >= 0
                              ? "更新字幕" : "添加字幕"
                        enabled: root.subtitleBridge
                                 && root.subtitleBridge.hasVideo
                                 && root.selectionIsValid()
                                 && subtitleEditor.text.trim().length > 0
                        onClicked: root.saveEditorText()
                    }
                }

                TextArea {
                    id: subtitleEditor
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    enabled: root.subtitleBridge
                             && root.subtitleBridge.hasVideo
                             && root.selectionIsValid()
                    placeholderText: enabled ? "输入在当前选区听到的文字……"
                                             : "设置 A、B 后可添加字幕"
                    wrapMode: TextEdit.Wrap
                    selectByMouse: true

                    Keys.onPressed: function(event) {
                        if ((event.modifiers & Qt.ControlModifier)
                                && (event.key === Qt.Key_Return || event.key === Qt.Key_Enter)) {
                            root.saveEditorText()
                            event.accepted = true
                        }
                    }
                }
            }
        }

        ListView {
            id: subtitleList
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: root.activeTab === 0
            clip: true
            spacing: 6
            model: root.subtitleEntries

            delegate: Rectangle {
                width: ListView.view.width
                height: subtitleContent.implicitHeight + 18
                radius: 9
                color: root.subtitleBridge
                       && root.subtitleBridge.editingCueIndex === modelData.index
                       ? root.accentBg
                       : (root.subtitleBridge
                          && root.subtitleBridge.activeCueIndex === modelData.index
                          ? root.elevatedBg : "transparent")
                border.color: root.subtitleBridge
                              && root.subtitleBridge.editingCueIndex === modelData.index
                              ? root.accent : root.borderColor
                border.width: 1

                Column {
                    id: subtitleContent
                    width: parent.width - 20
                    anchors.left: parent.left
                    anchors.leftMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 5

                    Text {
                        width: parent.width
                        text: modelData.startTime + " --> " + modelData.endTime
                        color: root.textSecondary
                        font.pixelSize: 11
                    }

                    Text {
                        width: parent.width
                        text: modelData.text
                        color: root.textPrimary
                        wrapMode: Text.Wrap
                        font.pixelSize: 13
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: {
                        if (root.subtitleBridge)
                            root.subtitleBridge.selectCue(modelData.index)
                        if (root.waveformBridge)
                            root.waveformBridge.setSelectionRange(modelData.start, modelData.end)
                    }
                }
            }
        }

        Text {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: root.activeTab !== 0
            text: root.activeTab === 1 ? "笔记功能尚未实现" : "单词功能尚未实现"
            color: root.textSecondary
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }

        Text {
            Layout.fillWidth: true
            visible: root.subtitleBridge && root.subtitleBridge.statusMessage.length > 0
            text: root.subtitleBridge ? root.subtitleBridge.statusMessage : ""
            color: root.textSecondary
            wrapMode: Text.Wrap
            font.pixelSize: 11
            elide: Text.ElideRight
        }
    }
}
