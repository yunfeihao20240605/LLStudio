import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import Qt.labs.platform 1.1 as Platform

Item {
    id: root

    property var noteBridge
    property string videoPath: ""
    property bool canCreateNote: false
    property color panelBg: "#ffffff"
    property color elevatedBg: "#fafbfc"
    property color borderColor: "#d0d7de"
    property color textPrimary: "#1f2329"
    property color textSecondary: "#6b7280"
    property color accent: "#2f6fed"
    property color accentBg: "#eaf1fe"
    property bool loadingEditor: false
    property bool editorDirty: false
    property bool previewMode: false

    signal navigationRequested(real startSecs, real endSecs, bool hasRange)
    signal newNoteRequested()
    signal noteExportSucceeded(string message)
    signal noteExportFailed(string message)

    function formatTimestamp(totalSeconds) {
        var totalMillis = Math.max(0, Math.round((totalSeconds || 0) * 1000))
        var hours = Math.floor(totalMillis / 3600000)
        var minutes = Math.floor((totalMillis % 3600000) / 60000)
        var seconds = Math.floor((totalMillis % 60000) / 1000)
        var millis = totalMillis % 1000
        return (hours < 10 ? "0" + hours : hours) + ":"
                + (minutes < 10 ? "0" + minutes : minutes) + ":"
                + (seconds < 10 ? "0" + seconds : seconds) + ","
                + (millis < 10 ? "00" + millis
                               : (millis < 100 ? "0" + millis : millis))
    }

    function noteTimeLabel(index) {
        if (!noteBridge || index < 0)
            return ""
        var start = formatTimestamp(noteBridge.noteStartAt(index))
        return noteBridge.noteHasRangeAt(index)
                ? start + " - " + formatTimestamp(noteBridge.noteEndAt(index))
                : start
    }

    function syncEditorFromBridge() {
        loadingEditor = true
        noteEditor.text = noteBridge ? noteBridge.editingNoteContent : ""
        editorDirty = false
        saveTimer.stop()
        loadingEditor = false
    }

    function flushPendingSave() {
        saveTimer.stop()
        if (!editorDirty || !noteBridge || noteBridge.editingNoteIndex < 0)
            return true
        if (!noteBridge.updateActiveNote(noteEditor.text))
            return false
        editorDirty = false
        return true
    }

    function activateNote(index) {
        if (!noteBridge || index < 0 || index >= noteBridge.noteCount)
            return false
        if (!flushPendingSave() || !noteBridge.selectNote(index))
            return false
        navigationRequested(noteBridge.noteStartAt(index),
                            noteBridge.noteEndAt(index),
                            noteBridge.noteHasRangeAt(index))
        return true
    }

    function deleteNote(index) {
        if (!noteBridge || !flushPendingSave())
            return false
        return noteBridge.deleteNote(index)
    }

    function defaultExportFileName() {
        var path = root.videoPath || ""
        var slash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"))
        var name = slash >= 0 ? path.substring(slash + 1) : path
        var dot = name.lastIndexOf(".")
        var stem = dot > 0 ? name.substring(0, dot) : name
        return (stem.length > 0 ? stem : "学习笔记") + ".md"
    }

    function exportNotes() {
        if (!noteBridge) {
            root.noteExportFailed("笔记服务尚未就绪")
            return false
        }
        if (!noteBridge.hasVideo) {
            root.noteExportFailed("请先加载视频")
            return false
        }
        if (noteBridge.noteCount <= 0) {
            root.noteExportFailed("暂无可导出的笔记")
            return false
        }
        if (!flushPendingSave()) {
            root.noteExportFailed(noteBridge.statusMessage)
            return false
        }
        exportFileDialog.currentFile = defaultExportFileName()
        exportFileDialog.open()
        return true
    }

    function exportNotesToPath(path) {
        if (!path || path.length === 0) {
            root.noteExportFailed("请选择 Markdown 文件保存位置")
            return false
        }
        if (noteBridge.exportMarkdown(path)) {
            root.noteExportSucceeded(noteBridge.statusMessage)
            return true
        }
        root.noteExportFailed(noteBridge.statusMessage)
        return false
    }

    function focusEditor() {
        if (!noteBridge || noteBridge.editingNoteIndex < 0)
            return
        previewMode = false
        noteEditor.forceActiveFocus(Qt.OtherFocusReason)
    }

    function setPreviewMode(enabled) {
        if (previewMode === enabled)
            return true
        if (enabled && !flushPendingSave())
            return false
        previewMode = enabled
        if (enabled) {
            noteEditor.focus = false
            root.forceActiveFocus(Qt.MouseFocusReason)
        } else {
            Qt.callLater(function() { root.focusEditor() })
        }
        return true
    }

    function clearEditorFocus() {
        if (!noteEditor.activeFocus)
            return
        flushPendingSave()
        noteEditor.focus = false
        root.forceActiveFocus(Qt.MouseFocusReason)
    }

    function clearEditorFocusIfOutside(sourceItem, sourceX, sourceY) {
        if (!noteEditor.activeFocus || !sourceItem)
            return
        var point = noteEditor.mapFromItem(sourceItem, sourceX, sourceY)
        if (point.x < 0 || point.y < 0
                || point.x > noteEditor.width || point.y > noteEditor.height)
            clearEditorFocus()
    }

    function ensurePlaybackNoteVisible() {
        if (!noteBridge || noteBridge.playbackNoteIndex < 0
                || noteBridge.playbackNoteIndex >= noteList.count)
            return
        noteList.positionViewAtIndex(noteBridge.playbackNoteIndex, ListView.Contain)
    }

    Component.onCompleted: syncEditorFromBridge()

    Connections {
        target: root.noteBridge
        ignoreUnknownSignals: true

        function onEditingNoteContentChanged() { root.syncEditorFromBridge() }
        function onPlaybackNoteIndexChanged() { root.ensurePlaybackNoteVisible() }
    }

    Timer {
        id: saveTimer
        interval: 500
        repeat: false
        onTriggered: root.flushPendingSave()
    }

    Platform.FileDialog {
        id: exportFileDialog
        title: "导出笔记"
        fileMode: Platform.FileDialog.SaveFile
        nameFilters: ["Markdown 文件 (*.md)", "所有文件 (*)"]
        defaultSuffix: "md"

        onAccepted: {
            var selected = file
            var path = selected && selected.toLocalFile
                       ? selected.toLocalFile()
                       : (selected ? selected.toString() : "")
            root.exportNotesToPath(path)
        }
    }

    Shortcut {
        sequence: "Escape"
        context: Qt.ApplicationShortcut
        enabled: noteEditor.activeFocus
        onActivated: root.clearEditorFocus()
    }

    SplitView {
        anchors.fill: parent
        orientation: Qt.Vertical
        handle: ThemedSplitHandle {
            splitOrientation: Qt.Vertical
            gapColor: root.panelBg
            dividerColor: root.borderColor
            accentColor: root.accent
        }

        Item {
            SplitView.minimumHeight: 100
            SplitView.preferredHeight: 230
            SplitView.fillHeight: true

            ListView {
                id: noteList
                anchors.fill: parent
                clip: true
                spacing: 6
                model: root.noteBridge ? root.noteBridge.noteCount : 0

                delegate: Rectangle {
                    id: noteDelegate
                    required property int index
                    readonly property int bridgeRevision: root.noteBridge
                                                                  ? root.noteBridge.revision : 0
                    readonly property bool isPlaybackNote: root.noteBridge
                                                            && root.noteBridge.playbackNoteIndex
                                                               === index
                    readonly property bool isEditingNote: root.noteBridge
                                                           && root.noteBridge.editingNoteIndex
                                                              === index

                    width: ListView.view.width
                    height: 68
                    radius: 8
                    color: isPlaybackNote ? root.accentBg : "transparent"
                    border.color: isPlaybackNote ? root.accent
                                                 : (isEditingNote
                                                    ? root.textSecondary
                                                    : root.borderColor)
                    border.width: isPlaybackNote ? 2 : 1

                    Rectangle {
                        visible: noteDelegate.isPlaybackNote
                        anchors.left: parent.left
                        anchors.leftMargin: 3
                        anchors.verticalCenter: parent.verticalCenter
                        width: 4
                        height: parent.height - 16
                        radius: 2
                        color: root.accent
                    }

                    Column {
                        anchors.fill: parent
                        anchors.leftMargin: 11
                        anchors.rightMargin: 9
                        anchors.topMargin: 7
                        anchors.bottomMargin: 7
                        spacing: 4

                        Text {
                            width: parent.width
                            text: {
                                var revision = noteDelegate.bridgeRevision
                                return root.noteTimeLabel(noteDelegate.index)
                            }
                            color: root.textSecondary
                            font.pixelSize: 11
                            elide: Text.ElideRight
                        }

                        Text {
                            width: parent.width
                            text: {
                                var revision = noteDelegate.bridgeRevision
                                var preview = root.noteBridge
                                        ? root.noteBridge.notePreviewAt(noteDelegate.index) : ""
                                return preview.length > 0 ? preview : "新笔记"
                            }
                            color: noteDelegate.isPlaybackNote ? root.accent
                                                               : root.textPrimary
                            font.pixelSize: 13
                            font.bold: noteDelegate.isPlaybackNote
                            wrapMode: Text.Wrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        onClicked: function(mouse) {
                            if (mouse.button === Qt.RightButton)
                                noteContextMenu.popup()
                            else
                                root.activateNote(noteDelegate.index)
                        }
                    }

                    Menu {
                        id: noteContextMenu
                        MenuItem {
                            text: "删除笔记"
                            onTriggered: root.deleteNote(noteDelegate.index)
                        }
                    }
                }
            }

            Text {
                anchors.centerIn: parent
                visible: !root.noteBridge || root.noteBridge.noteCount === 0
                text: root.noteBridge && root.noteBridge.hasVideo
                      ? "暂无笔记" : "请先加载视频"
                color: root.textSecondary
                font.pixelSize: 13
            }
        }

        Rectangle {
            SplitView.minimumHeight: 130
            SplitView.preferredHeight: 180
            SplitView.maximumHeight: 320
            color: root.elevatedBg
            border.color: root.borderColor
            radius: 8

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 8
                spacing: 6

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8

                    Text {
                        Layout.fillWidth: true
                        text: root.noteBridge && root.noteBridge.editingNoteIndex >= 0
                              ? root.noteTimeLabel(root.noteBridge.editingNoteIndex)
                              : "选择一条笔记后编辑"
                        color: root.textSecondary
                        font.pixelSize: 12
                        elide: Text.ElideRight
                    }

                    RowLayout {
                        Layout.preferredHeight: 30
                        spacing: 4

                        ThemedToolButton {
                            id: newNoteButton
                            Layout.preferredWidth: 30
                            Layout.preferredHeight: 30
                            text: "+"
                            font.pixelSize: 16
                            enabled: root.canCreateNote
                            panelColor: root.elevatedBg
                            borderColor: root.borderColor
                            textColor: root.textPrimary
                            disabledTextColor: root.textSecondary
                            accentColor: root.accent
                            accentBackgroundColor: root.accentBg
                            onClicked: root.newNoteRequested()
                            ToolTip.visible: hovered
                            ToolTip.text: enabled ? "新建当前片段笔记" : "请先选择一个片段"
                        }

                        ThemedToolButton {
                            id: editButton
                            Layout.preferredWidth: 30
                            Layout.preferredHeight: 30
                            text: "✎"
                            font.pixelSize: 16
                            enabled: root.noteBridge
                                     && root.noteBridge.editingNoteIndex >= 0
                            panelColor: !root.previewMode
                                         ? root.accentBg : root.elevatedBg
                            borderColor: !root.previewMode
                                          ? root.accent : root.borderColor
                            textColor: !root.previewMode
                                       ? root.accent : root.textSecondary
                            disabledTextColor: root.textSecondary
                            accentColor: root.accent
                            accentBackgroundColor: root.accentBg
                            onClicked: root.setPreviewMode(false)
                            ToolTip.visible: hovered
                            ToolTip.text: "编辑笔记"
                        }

                        ThemedToolButton {
                            id: previewButton
                            Layout.preferredWidth: 30
                            Layout.preferredHeight: 30
                            text: "◉"
                            font.pixelSize: 16
                            enabled: root.noteBridge
                                     && root.noteBridge.editingNoteIndex >= 0
                            panelColor: root.previewMode
                                         ? root.accentBg : root.elevatedBg
                            borderColor: root.previewMode
                                          ? root.accent : root.borderColor
                            textColor: root.previewMode
                                       ? root.accent : root.textSecondary
                            disabledTextColor: root.textSecondary
                            accentColor: root.accent
                            accentBackgroundColor: root.accentBg
                            onClicked: root.setPreviewMode(true)
                            ToolTip.visible: hovered
                            ToolTip.text: "预览笔记"
                        }

                        ThemedToolButton {
                            id: exportButton
                            Layout.preferredWidth: 30
                            Layout.preferredHeight: 30
                            text: "⇩"
                            font.pixelSize: 16
                            enabled: root.noteBridge
                                     && root.noteBridge.hasVideo
                                     && root.noteBridge.noteCount > 0
                            panelColor: root.elevatedBg
                            borderColor: root.borderColor
                            textColor: root.textPrimary
                            disabledTextColor: root.textSecondary
                            accentColor: root.accent
                            accentBackgroundColor: root.accentBg
                            onClicked: root.exportNotes()
                            ToolTip.visible: hovered
                            ToolTip.text: !root.noteBridge || !root.noteBridge.hasVideo
                                          ? "请先加载视频"
                                          : (root.noteBridge.noteCount > 0
                                             ? "导出笔记" : "暂无可导出的笔记")
                        }
                    }
                }

                StackLayout {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    currentIndex: root.previewMode ? 1 : 0

                    ScrollView {
                        clip: true

                        TextArea {
                            id: noteEditor
                            enabled: root.noteBridge
                                     && root.noteBridge.editingNoteIndex >= 0
                            placeholderText: enabled ? "使用 Markdown 输入学习笔记……"
                                                     : "选择一条笔记"
                            wrapMode: TextEdit.Wrap
                            selectByMouse: true
                            color: enabled ? root.textPrimary : root.textSecondary
                            placeholderTextColor: root.textSecondary
                            selectionColor: root.accent
                            selectedTextColor: "#ffffff"
                            background: null

                            onTextChanged: {
                                if (root.loadingEditor || !enabled)
                                    return
                                root.editorDirty = true
                                saveTimer.restart()
                            }
                            onActiveFocusChanged: {
                                if (!activeFocus)
                                    root.flushPendingSave()
                            }
                        }
                    }

                    ScrollView {
                        id: previewScroll
                        clip: true

                        Text {
                            width: previewScroll.availableWidth
                            text: noteEditor.text.trim().length > 0
                                  ? noteEditor.text : "*暂无内容*"
                            textFormat: Text.MarkdownText
                            wrapMode: Text.Wrap
                            color: noteEditor.text.trim().length > 0
                                   ? root.textPrimary : root.textSecondary
                            font.pixelSize: 13
                            lineHeight: 1.25
                        }
                    }
                }
            }
        }
    }
}
