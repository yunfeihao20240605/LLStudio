import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import QtQuick.Window 2.15

import "theme" as ThemeComponents
import com.yfhao.els.bridge 1.0

ApplicationWindow {
    id: root

    property bool videoFullScreen: false
    property int visibilityBeforeVideoFullScreen: Window.Windowed

    function setVideoFullScreen(enabled) {
        if (videoFullScreen === enabled)
            return
        if (enabled) {
            visibilityBeforeVideoFullScreen = visibility
            videoFullScreen = true
            visibility = Window.FullScreen
        } else {
            videoFullScreen = false
            visibility = visibilityBeforeVideoFullScreen === Window.FullScreen
                    ? Window.Windowed : visibilityBeforeVideoFullScreen
        }
    }

    function startOrContinueTraining(repeatCount, intervalSeconds) {
        if (trainingController.hasActiveSession) {
            if (mediaBridge.isPlaying) {
                if (trainingController.isTraining)
                    trainingController.pauseTraining()
                else {
                    mediaBridge.pause()
                    videoPlaybackPane.syncPlaybackDependentPanels()
                }
            } else if (!trainingController.resumeTraining()) {
                mediaBridge.play()
                videoPlaybackPane.syncPlaybackDependentPanels()
            }
            return
        }
        if (segmentBridge.saveCurrentSelection(
                    waveformBridge.selectionStart,
                    waveformBridge.selectionEnd,
                    repeatCount,
                    intervalSeconds)) {
            trainingController.startTraining(repeatCount, intervalSeconds)
        }
    }

    function startLabelTraining(index) {
        trainingController.stopTraining()
        if (!segmentBridge.buildLabelPlaybackPlan(index))
            return false

        var ranges = []
        for (var rangeIndex = 0;
             rangeIndex < segmentBridge.labelPlaybackRangeCount;
             ++rangeIndex) {
            ranges.push({
                start: segmentBridge.labelPlaybackRangeStartAt(rangeIndex),
                end: segmentBridge.labelPlaybackRangeEndAt(rangeIndex)
            })
        }

        return trainingController.startRangeSequence(
                    ranges,
                    controlPanel.repeatCount,
                    controlPanel.intervalSeconds,
                    segmentBridge.labelPlaybackLabel)
    }

    function toggleTrainingPlayback() {
        if (!trainingController.mediaAvailable)
            return

        if (trainingController.hasActiveSession) {
            startOrContinueTraining(controlPanel.repeatCount,
                                    controlPanel.intervalSeconds)
            return
        }

        if (!trainingController.selectionAvailable) {
            toggleNormalPlayback()
            return
        }

        startOrContinueTraining(controlPanel.repeatCount, controlPanel.intervalSeconds)
    }

    function toggleNormalPlayback() {
        if (!trainingController.mediaAvailable)
            return

        var wasPlaying = mediaBridge.isPlaying
        if (trainingController.hasActiveSession)
            trainingController.cancelTrainingSession()

        if (wasPlaying) {
            mediaBridge.pause()
        } else {
            mediaBridge.play()
        }
        videoPlaybackPane.syncPlaybackDependentPanels()
    }

    function seekPlaybackManually(positionSecs) {
        if (trainingController.hasActiveSession && mediaBridge.isPlaying)
            trainingController.pauseTraining()
        return videoPlaybackPane.seekToPosition(positionSecs)
    }

    function textInputHasFocus() {
        var item = activeFocusItem
        return item && item.text !== undefined
                && item.cursorPosition !== undefined
    }

    function beginNextSegment() {
        if (segmentBridge.activeIndex < 0)
            return false

        var nextStart = segmentBridge.activeEnd
        if (nextStart < 0 || nextStart >= waveformBridge.durationSecs)
            return false

        trainingController.stopTraining()
        if (!waveformBridge.startNewSelectionAt(nextStart))
            return false
        segmentBridge.deactivateSegment()
        videoPlaybackPane.seekToPosition(nextStart)
        return true
    }

    function activateLearningSegment(index) {
        trainingController.stopTraining()
        if (segmentBridge.activateSegment(index)) {
            waveformBridge.setSelectionRange(segmentBridge.activeStart,
                                             segmentBridge.activeEnd)
            controlPanel.applyTrainingSettings(segmentBridge.activeRepeatCount,
                                               segmentBridge.activeIntervalSeconds)
            videoPlaybackPane.seekToPosition(segmentBridge.activeStart)
        }
    }

    function deleteLearningSegment(index) {
        var deletingActive = index === segmentBridge.activeIndex
        trainingController.stopTraining()
        if (segmentBridge.deleteSegment(index) && deletingActive)
            waveformBridge.clearSelection()
    }

    function createNote(startSecs, endSecs, hasRange) {
        if (!noteBridge.hasVideo)
            return false
        var created = hasRange
                ? noteBridge.createForRange(startSecs, endSecs)
                : noteBridge.createAtPosition(startSecs)
        if (created) {
            noteBridge.syncPlaybackPosition(mediaBridge.currentPosition)
            subtitleView.showNoteEditor()
        }
        return created
    }

    function navigateToNote(startSecs, endSecs, hasRange) {
        trainingController.stopTraining()
        if (hasRange) {
            segmentBridge.deactivateSegment()
            waveformBridge.setSelectionRange(startSecs, endSecs)
        }
        videoPlaybackPane.seekToPosition(startSecs)
    }

    width: 1440
    height: 920
    visible: true
    minimumWidth: 1220
    minimumHeight: 760
    title: "Language Learning Studio"

    Shortcut {
        sequence: "Space"
        context: Qt.ApplicationShortcut
        autoRepeat: false
        onActivated: root.toggleTrainingPlayback()
    }

    Shortcut {
        sequence: "Shift+Space"
        context: Qt.ApplicationShortcut
        autoRepeat: false
        onActivated: root.toggleNormalPlayback()
    }

    Shortcut {
        sequence: "N"
        context: Qt.ApplicationShortcut
        autoRepeat: false
        enabled: segmentBridge.activeIndex >= 0 && !root.textInputHasFocus()
        onActivated: root.beginNextSegment()
    }

    Shortcut {
        sequence: "Escape"
        context: Qt.ApplicationShortcut
        enabled: root.videoFullScreen
        onActivated: root.setVideoFullScreen(false)
    }

    AppBootstrap {
        id: bootstrap
    }

    ThemeBridge {
        id: themeBridge
    }

    ThemeComponents.Theme {
        id: theme
        mode: themeBridge.themeMode
    }

    MediaBridge {
        id: mediaBridge
    }

    LibraryBridge {
        id: libraryBridge
    }

    SubtitleBridge {
        id: subtitleBridge
    }

    NoteBridge {
        id: noteBridge
    }

    RecordingBridge {
        id: recordingBridge
    }

    WaveformBridge {
        id: waveformBridge
    }

    SegmentBridge {
        id: segmentBridge
    }

    Connections {
        target: mediaBridge
        function onCurrentPositionChanged() {
            noteBridge.syncPlaybackPosition(mediaBridge.currentPosition)
        }
    }

    Connections {
        target: waveformBridge
        function onSelectionRevisionChanged() {
            var hasRange = waveformBridge.hasSelectionStart
                    && waveformBridge.hasSelectionEnd
                    && waveformBridge.selectionEnd > waveformBridge.selectionStart
            recordingBridge.syncTargetRange(waveformBridge.selectionStart,
                                            waveformBridge.selectionEnd,
                                            hasRange)
        }
    }

    Connections {
        target: waveformBridge
        function onHasErrorChanged() {
            if (waveformBridge.hasError)
                themeBridge.reportError(waveformBridge.statusMessage)
        }
    }

    SelectionTrainingController {
        id: trainingController
        mediaAvailable: mediaBridge.loadedPath.length > 0 && mediaBridge.duration > 0
        playbackPosition: mediaBridge.currentPosition
        selectionAvailable: waveformBridge.hasSelectionStart
                            && waveformBridge.hasSelectionEnd
                            && waveformBridge.selectionEnd > waveformBridge.selectionStart
        selectionStart: waveformBridge.selectionStart
        selectionEnd: waveformBridge.selectionEnd

        onLoopCompleted: {
            if (trainingController.isLabelSequence)
                segmentBridge.recordLabelPlaybackLoop()
            else if (segmentBridge.activeIndex >= 0)
                segmentBridge.incrementCompletedLoops()
        }

        onSeekAndPlayRequested: function(positionSecs) {
            if (videoPlaybackPane.seekToPosition(positionSecs)) {
                mediaBridge.play()
                videoPlaybackPane.syncPlaybackDependentPanels()
            }
        }

        onPauseRequested: {
            mediaBridge.pause()
            videoPlaybackPane.syncPlaybackDependentPanels()
        }

        onPauseAtPositionRequested: function(positionSecs) {
            mediaBridge.pause()
            videoPlaybackPane.seekToPosition(positionSecs)
        }

        onResumePlaybackRequested: {
            mediaBridge.play()
            videoPlaybackPane.syncPlaybackDependentPanels()
        }
    }

    color: theme.windowBg

    menuBar: MenuBar {
        visible: !root.videoFullScreen
        background: Rectangle {
            color: theme.panelBg
            border.color: theme.border
        }

        delegate: MenuBarItem {
            contentItem: Text {
                text: parent.text
                color: parent.highlighted ? theme.accent : theme.textPrimary
                font.pixelSize: 14
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }

            background: Rectangle {
                color: parent.highlighted ? theme.accentBg : "transparent"
                radius: 8
            }
        }

        Menu {
            title: "文件"
            Action {
                text: "打开视频"
                onTriggered: videoPlaybackPane.openVideo()
            }
            Action { text: "导入字幕" }
        }
        Menu {
            title: "编辑"
            Action { text: "查找" }
        }
        Menu {
            title: "播放"
            Action { text: "播放/暂停" }
        }
        Menu {
            title: "学习"
            Action { text: "开始训练" }
        }
        Menu {
            title: "工具"
            Action { text: "主题切换" }
            Action {
                text: "波形状态…"
                onTriggered: waveformStatusDialog.open()
            }
        }
        Menu {
            title: "帮助"
            Action { text: "关于" }
        }
    }

    ColumnLayout {
        id: mainContent
        anchors.fill: parent
        anchors.margins: root.videoFullScreen ? 0 : 12
        spacing: root.videoFullScreen ? 0 : 10

        TapHandler {
            acceptedButtons: Qt.LeftButton
            gesturePolicy: TapHandler.DragThreshold
            onTapped: function(eventPoint) {
                subtitleView.clearEditorFocusIfOutside(mainContent,
                                                       eventPoint.position.x,
                                                       eventPoint.position.y)
            }
        }

        SplitView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: Qt.Horizontal

            LibrarySidebar {
                id: librarySidebar
                visible: !root.videoFullScreen
                SplitView.minimumWidth: 240
                SplitView.preferredWidth: 290
                SplitView.maximumWidth: 360
                Layout.fillHeight: true
                segmentBridge: segmentBridge
                libraryBridge: libraryBridge
                selectionAvailable: waveformBridge.hasSelectionStart
                                    && waveformBridge.hasSelectionEnd
                                    && waveformBridge.selectionEnd > waveformBridge.selectionStart
                selectionStart: waveformBridge.selectionStart
                selectionEnd: waveformBridge.selectionEnd
                onVideoOpenRequested: function(path) {
                    videoPlaybackPane.loadVideoAndRelatedAssets(path)
                }
                panelBg: theme.panelBg
                elevatedBg: theme.elevatedBg
                borderColor: theme.border
                textPrimary: theme.textPrimary
                textSecondary: theme.textSecondary
                accent: theme.accent
                accentBg: theme.accentBg
            }

            SplitView {
                id: playbackWorkspace
                SplitView.fillWidth: true
                SplitView.minimumWidth: 620
                orientation: Qt.Vertical

                VideoPlaybackPane {
                    id: videoPlaybackPane
                    SplitView.minimumHeight: 340
                    SplitView.preferredHeight: 430
                    SplitView.fillHeight: root.videoFullScreen
                    fullScreenMode: root.videoFullScreen
                    mediaBridge: mediaBridge
                    subtitleBridge: subtitleBridge
                    waveformBridge: waveformBridge
                    onManualSeekRequested: function(positionSecs) {
                        root.seekPlaybackManually(positionSecs)
                    }
                    onNormalPlaybackToggleRequested: root.toggleNormalPlayback()
                    onVideoLoaded: function(path, durationSecs) {
                        trainingController.stopTraining()
                        if (libraryBridge.recordOpenedVideo(path, durationSecs))
                            librarySidebar.revealLearningVideos()
                        segmentBridge.loadForVideoPath(path, durationSecs)
                        noteBridge.loadForVideoPath(path, durationSecs)
                        noteBridge.syncPlaybackPosition(mediaBridge.currentPosition)
                        recordingBridge.loadForVideoPath(path, durationSecs)
                        recordingBridge.syncTargetRange(
                                    waveformBridge.selectionStart,
                                    waveformBridge.selectionEnd,
                                    waveformBridge.hasSelectionStart
                                    && waveformBridge.hasSelectionEnd
                                    && waveformBridge.selectionEnd
                                       > waveformBridge.selectionStart)
                    }
                    onFullScreenToggleRequested: {
                        root.setVideoFullScreen(!root.videoFullScreen)
                    }
                    panelBg: theme.panelBg
                    elevatedBg: theme.elevatedBg
                    borderColor: theme.border
                    textPrimary: theme.textPrimary
                    textSecondary: theme.textSecondary
                    accent: theme.accent
                    accentBg: theme.accentBg
                }

                WaveformView {
                    visible: !root.videoFullScreen
                    SplitView.minimumHeight: 240
                    SplitView.preferredHeight: recordingBridge.hasRecording ? 380 : 290
                    subtitleBridge: subtitleBridge
                    waveformBridge: waveformBridge
                    recordingBridge: recordingBridge
                    canBeginNextSegment: segmentBridge.activeIndex >= 0
                                         && segmentBridge.activeEnd
                                            < waveformBridge.durationSecs
                    onPlaybackPositionRequested: function(positionSecs) {
                        root.seekPlaybackManually(positionSecs)
                    }
                    onSelectionCleared: segmentBridge.deactivateSegment()
                    onNextSegmentRequested: root.beginNextSegment()
                    onNoteCreationRequested: function(startSecs, endSecs, hasRange) {
                        root.createNote(startSecs, endSecs, hasRange)
                    }
                    onRecordingStartRequested: {
                        trainingController.stopTraining()
                        mediaBridge.pause()
                        videoPlaybackPane.syncPlaybackDependentPanels()
                        recordingBridge.startRecording()
                    }
                    onRecordingStopRequested: recordingBridge.stopRecording()
                    onRecordingDeleteRequested: recordingBridge.deleteRecording()
                    panelBg: theme.panelBg
                    elevatedBg: theme.elevatedBg
                    borderColor: theme.border
                    textPrimary: theme.textPrimary
                    textSecondary: theme.textSecondary
                    accent: theme.accent
                    accentBg: theme.accentBg
                }

                ControlPanel {
                    id: controlPanel
                    visible: !root.videoFullScreen
                    SplitView.minimumHeight: 110
                    SplitView.preferredHeight: 130
                    canStartTraining: trainingController.mediaAvailable
                                      && (trainingController.selectionAvailable
                                          || trainingController.hasActiveSession)
                    isTraining: trainingController.isTraining
                    hasStartedTraining: trainingController.hasActiveSession
                    isPlaybackPlaying: mediaBridge.isPlaying
                    completedLoops: trainingController.completedLoops
                    totalLoops: trainingController.totalLoops
                    selectionStart: waveformBridge.selectionStart
                    selectionEnd: waveformBridge.selectionEnd
                    trainingStatus: trainingController.statusMessage
                    onStartTrainingRequested: function(repeatCount, intervalSeconds) {
                        root.startOrContinueTraining(repeatCount, intervalSeconds)
                    }
                    panelBg: theme.panelBg
                    elevatedBg: theme.elevatedBg
                    borderColor: theme.border
                    textPrimary: theme.textPrimary
                    textSecondary: theme.textSecondary
                    accent: theme.accent
                    accentBg: theme.accentBg
                }
            }

            SplitView {
                id: learningDetailsPanel
                visible: !root.videoFullScreen
                SplitView.minimumWidth: 280
                SplitView.preferredWidth: 330
                SplitView.maximumWidth: 420
                orientation: Qt.Vertical

                SubtitleView {
                    id: subtitleView
                    SplitView.minimumHeight: 360
                    SplitView.preferredHeight: 520
                    subtitleBridge: subtitleBridge
                    noteBridge: noteBridge
                    waveformBridge: waveformBridge
                    onNoteNavigationRequested: function(startSecs, endSecs, hasRange) {
                        root.navigateToNote(startSecs, endSecs, hasRange)
                    }
                    panelBg: theme.panelBg
                    elevatedBg: theme.elevatedBg
                    borderColor: theme.border
                    textPrimary: theme.textPrimary
                    textSecondary: theme.textSecondary
                    accent: theme.accent
                    accentBg: theme.accentBg
                }

                SegmentListView {
                    SplitView.minimumHeight: 220
                    SplitView.fillHeight: true
                    segmentBridge: segmentBridge
                    onSegmentActivated: function(index) {
                        root.activateLearningSegment(index)
                    }
                    onSegmentDeleteRequested: function(index) {
                        root.deleteLearningSegment(index)
                    }
                    onLabelPlaybackRequested: function(index) {
                        root.startLabelTraining(index)
                    }
                    onNoteCreationRequested: function(startSecs, endSecs) {
                        root.createNote(startSecs, endSecs, true)
                    }
                    panelBg: theme.panelBg
                    elevatedBg: theme.elevatedBg
                    borderColor: theme.border
                    textPrimary: theme.textPrimary
                    textSecondary: theme.textSecondary
                    accent: theme.accent
                    accentBg: theme.accentBg
                }
            }
        }

        StatusBar {
            visible: !root.videoFullScreen
            Layout.fillWidth: true
            Layout.preferredHeight: 34
            panelBg: theme.panelBg
            borderColor: theme.border
            textPrimary: theme.textPrimary
            textSecondary: theme.textSecondary
        }

        Text {
            Layout.fillWidth: true
            visible: themeBridge.lastError.length > 0
            text: themeBridge.lastError
            color: "#c03d3d"
            wrapMode: Text.Wrap
            font.pixelSize: 13
        }
    }

    Dialog {
        id: waveformStatusDialog
        title: "波形状态"
        modal: false
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        x: (parent.width - width) / 2
        y: (parent.height - height) / 2
        width: 520
        height: 220

        background: Rectangle {
            color: theme.panelBg
            border.color: theme.border
            border.width: 1
            radius: 8
        }

        header: Label {
            text: "波形状态"
            color: theme.textPrimary
            font.pixelSize: 15
            padding: 12
            background: Rectangle {
                color: theme.elevatedBg
                radius: 8
            }
        }

        contentItem: ScrollView {
            clip: true
            TextArea {
                readOnly: true
                wrapMode: Text.Wrap
                text: waveformBridge ? waveformBridge.statusMessage : ""
                color: theme.textPrimary
                font.pixelSize: 13
                background: null
                padding: 12
            }
        }

        footer: DialogButtonBox {
            Button {
                text: "关闭"
                onClicked: waveformStatusDialog.close()
            }
        }
    }
}
