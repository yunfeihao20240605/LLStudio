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
    property var noteBridge
    property var waveformBridge
    property int activeTab: 0
    property var subtitleEntries: []

    signal noteNavigationRequested(real startSecs, real endSecs, bool hasRange)

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

    function ensureActiveCueVisible() {
        if (activeTab !== 0 || !subtitleBridge)
            return
        var cueIndex = subtitleBridge.activeCueIndex
        if (cueIndex < 0 || cueIndex >= subtitleList.count)
            return

        Qt.callLater(function() {
            if (root.activeTab !== 0 || !root.subtitleBridge
                    || root.subtitleBridge.activeCueIndex !== cueIndex)
                return
            var item = subtitleList.itemAtIndex(cueIndex)
            if (!item) {
                subtitleList.positionViewAtIndex(cueIndex, ListView.Contain)
                Qt.callLater(function() { root.positionCueNearTop(cueIndex) })
                return
            }

            var safeTop = subtitleList.contentY + subtitleList.height * 0.18
            var safeBottom = subtitleList.contentY + subtitleList.height * 0.72
            if (item.y < safeTop || item.y + item.height > safeBottom)
                root.positionCueNearTop(cueIndex)
        })
    }

    function positionCueNearTop(cueIndex) {
        if (!subtitleBridge || subtitleBridge.activeCueIndex !== cueIndex)
            return
        var item = subtitleList.itemAtIndex(cueIndex)
        if (!item)
            return
        var target = item.y + item.height / 2 - subtitleList.height * 0.35
        var maximum = Math.max(0, subtitleList.contentHeight - subtitleList.height)
        subtitleList.contentY = Math.max(0, Math.min(maximum, target))
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
        if (subtitleEditor.activeFocus) {
            subtitleEditor.focus = false
            root.forceActiveFocus(Qt.MouseFocusReason)
        }
        noteView.clearEditorFocus()
    }

    function clearEditorFocusIfOutside(sourceItem, sourceX, sourceY) {
        if (!sourceItem)
            return
        if (subtitleEditor.activeFocus) {
            var point = subtitleEditor.mapFromItem(sourceItem, sourceX, sourceY)
            if (point.x < 0 || point.y < 0
                    || point.x > subtitleEditor.width
                    || point.y > subtitleEditor.height) {
                subtitleEditor.focus = false
                root.forceActiveFocus(Qt.MouseFocusReason)
            }
        }
        noteView.clearEditorFocusIfOutside(sourceItem, sourceX, sourceY)
    }

    function showNoteEditor() {
        activeTab = 1
        Qt.callLater(function() { noteView.focusEditor() })
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
            root.ensureActiveCueVisible()
        }

        function onActiveCueIndexChanged() { root.ensureActiveCueVisible() }

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

    onActiveTabChanged: {
        if (activeTab === 0)
            ensureActiveCueVisible()
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
                visible: root.activeTab !== 2
                text: root.activeTab === 0
                      ? root.subtitleEntries.length + " 条字幕"
                      : (root.noteBridge ? root.noteBridge.noteCount : 0) + " 条笔记"
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

        ListView {
            id: subtitleList
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: root.activeTab === 0
            clip: true
            spacing: 6
            model: root.subtitleEntries

            delegate: Rectangle {
                id: subtitleDelegate
                readonly property bool isPlayingCue: root.subtitleBridge
                                                     && root.subtitleBridge.activeCueIndex
                                                        === modelData.index
                width: ListView.view.width
                height: subtitleContent.implicitHeight + 18
                radius: 9
                color: isPlayingCue ? root.accentBg : "transparent"
                border.color: isPlayingCue ? root.accent : root.borderColor
                border.width: isPlayingCue ? 2 : 1

                Rectangle {
                    visible: parent.isPlayingCue
                    width: 4
                    height: Math.max(12, parent.height - 16)
                    anchors.left: parent.left
                    anchors.leftMargin: 3
                    anchors.verticalCenter: parent.verticalCenter
                    radius: 2
                    color: root.accent
                }

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
                        color: subtitleDelegate.isPlayingCue ? root.accent : root.textPrimary
                        wrapMode: Text.Wrap
                        font.pixelSize: 13
                        font.bold: subtitleDelegate.isPlayingCue
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

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 128
            visible: root.activeTab === 0
            radius: 9
            color: root.elevatedBg
            border.color: root.borderColor

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

        NoteView {
            id: noteView
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: root.activeTab === 1
            noteBridge: root.noteBridge
            elevatedBg: root.elevatedBg
            borderColor: root.borderColor
            textPrimary: root.textPrimary
            textSecondary: root.textSecondary
            accent: root.accent
            accentBg: root.accentBg
            onNavigationRequested: function(startSecs, endSecs, hasRange) {
                root.noteNavigationRequested(startSecs, endSecs, hasRange)
            }
        }

        Text {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: root.activeTab === 2
            text: "单词功能尚未实现"
            color: root.textSecondary
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }
}
