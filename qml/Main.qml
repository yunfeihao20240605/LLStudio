import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import QtQuick.Window 2.15
import Qt.labs.platform 1.1 as Platform

import "theme" as ThemeComponents
import com.yfhao.els.bridge 1.0

ApplicationWindow {
    id: root
    property string successMessage: ""
    property int pendingSegmentIndex: -1
    property bool pendingSegmentTraining: false
    property bool pendingSegmentRecordingTraining: false
    property int pendingRecordingTrainingIndex: -1

    property bool videoFullScreen: false
    property int visibilityBeforeVideoFullScreen: Window.Windowed
    property bool deferRecordingTargetSync: false
    property int segmentActivationRevision: 0
    property int pendingReadySegmentIndex: -1
    property bool recordingOperationActive: false
    property bool selectionAdjustmentActive: false
    property int selectionAdjustmentCueIndex: -1
    property real selectionAdjustmentOriginalStart: 0
    property real selectionAdjustmentOriginalEnd: 0

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
            var sourceIsPlaying = trainingController.isRecordingSession
                    ? recordingPlaybackBridge.isPlaying : mediaBridge.isPlaying
            if (sourceIsPlaying) {
                if (trainingController.isTraining)
                    trainingController.pauseTraining()
                else {
                    if (trainingController.isRecordingSession)
                        recordingPlaybackBridge.pause()
                    else {
                        mediaBridge.pause()
                        videoPlaybackPane.syncPlaybackDependentPanels()
                    }
                }
            } else if (!trainingController.resumeTraining()) {
                if (trainingController.isRecordingSession)
                    recordingPlaybackBridge.play()
                else {
                    mediaBridge.play()
                    videoPlaybackPane.syncPlaybackDependentPanels()
                }
            }
            return
        }
        recordingPlaybackBridge.pause()
        if (segmentBridge.saveCurrentSelection(
                    waveformBridge.selectionStart,
                    waveformBridge.selectionEnd,
                    repeatCount,
                    intervalSeconds)) {
            trainingController.startTraining(repeatCount, intervalSeconds)
        }
    }

    function toggleRecordingTraining() {
        if (!recordingBridge.hasRecording
                || !recordingPlaybackBridge.hasRecording)
            return false
        if (!recordingPlaybackBridge.hasPlayableOverlap) {
            themeBridge.reportError("录音与当前选区没有重叠")
            return false
        }
        themeBridge.reportError("")

        if (trainingController.hasActiveSession) {
            if (trainingController.isRecordingSession) {
                startOrContinueTraining(librarySidebar.repeatCount,
                                        librarySidebar.intervalSeconds)
                return true
            }
            trainingController.stopTraining()
        }

        mediaBridge.pause()
        videoPlaybackPane.syncPlaybackDependentPanels()
        recordingPlaybackBridge.applyPlaybackRate(mediaBridge.playbackRate)
        return trainingController.startRecordingTraining(
                    recordingPlaybackBridge.duration,
                    librarySidebar.repeatCount,
                    librarySidebar.intervalSeconds)
    }

    function toggleOriginalTraining() {
        if (trainingController.isRecordingSession)
            return switchTrainingSource(librarySidebar.repeatCount,
                                        librarySidebar.intervalSeconds)
        startOrContinueTraining(librarySidebar.repeatCount,
                                librarySidebar.intervalSeconds)
        return true
    }

    function switchTrainingSource(repeatCount, intervalSeconds) {
        if (!recordingBridge.hasRecording
                || !recordingPlaybackBridge.hasRecording)
            return false

        var switchToRecording = !trainingController.isRecordingSession
        if (trainingController.hasActiveSession)
            trainingController.stopTraining()

        if (switchToRecording) {
            if (!recordingPlaybackBridge.hasPlayableOverlap) {
                themeBridge.reportError("录音与当前选区没有重叠")
                return false
            }
            themeBridge.reportError("")
            mediaBridge.pause()
            videoPlaybackPane.syncPlaybackDependentPanels()
            recordingPlaybackBridge.applyPlaybackRate(mediaBridge.playbackRate)
            return trainingController.startRecordingTraining(
                        recordingPlaybackBridge.duration,
                        repeatCount,
                        intervalSeconds)
        }

        recordingPlaybackBridge.pause()
        if (!segmentBridge.saveCurrentSelection(
                    waveformBridge.selectionStart,
                    waveformBridge.selectionEnd,
                    repeatCount,
                    intervalSeconds))
            return false
        return trainingController.startTraining(repeatCount, intervalSeconds)
    }

    function syncRecordingPlaybackSource() {
        if (recordingBridge.hasRecording
                && recordingBridge.recordingFilePath.length > 0) {
            if (recordingPlaybackBridge.loadRecording(
                        recordingBridge.recordingFilePath)) {
                recordingPlaybackBridge.configureTimeline(
                            recordingBridge.targetEnd
                            - recordingBridge.targetStart,
                            recordingBridge.alignmentOffset)
                recordingPlaybackBridge.applyPlaybackRate(mediaBridge.playbackRate)
            }
        } else {
            if (trainingController.hasActiveSession
                    && trainingController.isRecordingSession)
                trainingController.stopTraining()
            if (recordingPlaybackBridge.hasRecording
                    || recordingPlaybackBridge.loadedPath.length > 0)
                recordingPlaybackBridge.unload()
        }
    }

    function prepareRecordingVariantChange() {
        if (trainingController.hasActiveSession
                && trainingController.isRecordingSession)
            trainingController.stopTraining()
        recordingPlaybackBridge.pause()
    }

    function processRecordingNoiseReduction(profile) {
        prepareRecordingVariantChange()
        recordingOperationActive = true
        if (!recordingBridge.processNoiseReduction(profile)) {
            recordingOperationActive = false
            themeBridge.reportError(recordingBridge.statusMessage)
            return
        }
        successMessage = recordingBridge.statusMessage
    }

    function useOriginalRecording() {
        prepareRecordingVariantChange()
        recordingOperationActive = true
        if (!recordingBridge.useOriginalRecording()) {
            recordingOperationActive = false
            themeBridge.reportError(recordingBridge.statusMessage)
            return
        }
        successMessage = recordingBridge.statusMessage
    }

    function syncRecordingTargetFromSelection() {
        var hasRange = waveformBridge.hasSelectionStart
                && waveformBridge.hasSelectionEnd
                && waveformBridge.selectionEnd > waveformBridge.selectionStart
        recordingBridge.syncTargetRange(waveformBridge.selectionStart,
                                        waveformBridge.selectionEnd,
                                        hasRange)
    }

    function beginSelectionAdjustment() {
        if (segmentBridge.activeIndex < 0)
            return
        selectionAdjustmentActive = true
        selectionAdjustmentCueIndex = subtitleBridge.editingCueIndex
        selectionAdjustmentOriginalStart = segmentBridge.activeStart
        selectionAdjustmentOriginalEnd = segmentBridge.activeEnd
        subtitleView.beginSelectionAdjustment()
    }

    function commitSelectionAdjustment(startSecs, endSecs) {
        if (!selectionAdjustmentActive)
            return false
        selectionAdjustmentActive = false
        var cueIndex = selectionAdjustmentCueIndex
        selectionAdjustmentCueIndex = -1
        var originalStart = selectionAdjustmentOriginalStart
        var originalEnd = selectionAdjustmentOriginalEnd
        var validRange = isFinite(startSecs) && isFinite(endSecs)
                && startSecs >= 0 && endSecs > startSecs
        if (!validRange) {
            segmentBridge.previewActiveRange(originalStart, originalEnd)
            subtitleView.endSelectionAdjustment()
            return false
        }
        var segmentUpdated = segmentBridge.commitActiveRange(startSecs, endSecs)
        if (segmentUpdated && cueIndex >= 0)
            subtitleBridge.updateCueRange(cueIndex, startSecs, endSecs)
        subtitleView.endSelectionAdjustment()
        root.syncRecordingTargetFromSelection()
        return segmentUpdated
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
                    librarySidebar.repeatCount,
                    librarySidebar.intervalSeconds,
                    segmentBridge.labelPlaybackLabel)
    }

    function toggleTrainingPlayback() {
        if (!trainingController.mediaAvailable)
            return

        if (trainingController.hasActiveSession) {
            startOrContinueTraining(librarySidebar.repeatCount,
                                    librarySidebar.intervalSeconds)
            return
        }

        if (!trainingController.selectionAvailable) {
            toggleNormalPlayback()
            return
        }

        startOrContinueTraining(librarySidebar.repeatCount,
                                librarySidebar.intervalSeconds)
    }

    function toggleNormalPlayback() {
        if (!trainingController.mediaAvailable)
            return

        var wasPlaying = mediaBridge.isPlaying
        if (trainingController.hasActiveSession) {
            if (trainingController.isRecordingSession
                    && recordingPlaybackBridge.isPlaying)
                recordingPlaybackBridge.pause()
            trainingController.cancelTrainingSession()
        }

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
        var seeked = videoPlaybackPane.seekToPosition(positionSecs)
        if (seeked)
            savePlaybackProgress()
        return seeked
    }

    function savePlaybackProgress() {
        if (!mediaBridge || !libraryBridge
                || mediaBridge.loadedPath.length === 0
                || mediaBridge.preparingInitialFrame)
            return false
        return libraryBridge.savePlaybackPosition(
                    mediaBridge.loadedPath,
                    mediaBridge.currentPosition)
    }

    function showSuccessMessage(message) {
        themeBridge.reportError("")
        successMessage = message
        successMessageTimer.restart()
    }

    function recordingExportFileName() {
        var start = Math.max(0, recordingBridge ? recordingBridge.targetStart : 0)
        var end = Math.max(start, recordingBridge ? recordingBridge.targetEnd : start)
        return "recording-" + start.toFixed(3) + "-" + end.toFixed(3) + ".wav"
    }

    function openRecordingExportDialog() {
        if (!recordingBridge || !recordingBridge.hasRecording) {
            themeBridge.reportError("当前没有可导出的录音")
            return
        }
        recordingExportDialog.currentFile = recordingExportFileName()
        recordingExportDialog.open()
    }

    function localPathFromUrl(value) {
        if (!value)
            return ""

        if (typeof value.toLocalFile === "function") {
            var localPath = value.toLocalFile()
            if (localPath && localPath.length > 0)
                return localPath
        }

        var text = value.toString()
        if (text.indexOf("file://") === 0) {
            try {
                return decodeURIComponent(text.substring(7))
            } catch (error) {
                return text.substring(7)
            }
        }
        return text
    }

    function exportRecordingToPath(selectedFile) {
        var path = localPathFromUrl(selectedFile)
        if (path.length === 0) {
            themeBridge.reportError("请选择录音保存位置")
            return
        }
        if (recordingBridge.exportActiveRecording(path))
            showSuccessMessage(recordingBridge.statusMessage)
        else
            themeBridge.reportError(recordingBridge.statusMessage)
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

    function syncActiveLearningSegment(index) {
        if (index < 0 || index !== segmentBridge.activeIndex)
            return false

        var revision = ++segmentActivationRevision
        root.deferRecordingTargetSync = true
        waveformBridge.setSelectionRange(segmentBridge.activeStart,
                                         segmentBridge.activeEnd)
        subtitleView.syncSelectionEditor()
        librarySidebar.applyTrainingSettings(segmentBridge.activeRepeatCount,
                                              segmentBridge.activeIntervalSeconds)
        var seeked = videoPlaybackPane.seekToPosition(segmentBridge.activeStart)
        pendingReadySegmentIndex = !seeked && mediaBridge.preparingInitialFrame
                ? index : -1
        Qt.callLater(function() {
            if (revision !== root.segmentActivationRevision)
                return
            root.deferRecordingTargetSync = false
            if (index !== segmentBridge.activeIndex)
                return
            subtitleView.syncSelectionEditor()
            root.syncRecordingTargetFromSelection()
        })
        return true
    }

    function activateLearningSegment(index) {
        trainingController.stopTraining()
        if (!segmentBridge.activateSegment(index))
            return false
        return syncActiveLearningSegment(index)
    }

    function requestActivateLearningSegment(index) {
        if (unsavedSubtitleDialog.visible)
            return
        if (index === segmentBridge.activeIndex) {
            syncActiveLearningSegment(index)
            return
        }
        if (subtitleView.hasUnsavedSubtitleChanges) {
            pendingSegmentIndex = index
            unsavedSubtitleDialog.open()
            return
        }
        activateLearningSegment(index)
    }

    function startSegmentTraining(index) {
        if (index < 0 || index >= segmentBridge.segmentCount)
            return false
        if (subtitleView.hasUnsavedSubtitleChanges) {
            pendingSegmentIndex = index
            pendingSegmentTraining = true
            unsavedSubtitleDialog.open()
            return true
        }
        if (index !== segmentBridge.activeIndex)
            activateLearningSegment(index)
        else
            syncActiveLearningSegment(index)
        librarySidebar.applyTrainingSettings(segmentBridge.activeRepeatCount,
                                              segmentBridge.activeIntervalSeconds)
        return toggleOriginalTraining()
    }

    function tryStartPendingRecordingTraining() {
        var index = pendingRecordingTrainingIndex
        if (index < 0 || index !== segmentBridge.activeIndex
                || !recordingPlaybackBridge.hasRecording
                || !recordingPlaybackBridge.hasPlayableOverlap)
            return false
        pendingRecordingTrainingIndex = -1
        librarySidebar.applyTrainingSettings(segmentBridge.activeRepeatCount,
                                              segmentBridge.activeIntervalSeconds)
        return toggleRecordingTraining()
    }

    function startSegmentRecordingTraining(index) {
        if (index < 0 || index >= segmentBridge.segmentCount
                || !segmentBridge.segmentHasRecordingAt(index))
            return false
        if (subtitleView.hasUnsavedSubtitleChanges) {
            pendingSegmentIndex = index
            pendingSegmentTraining = true
            pendingSegmentRecordingTraining = true
            unsavedSubtitleDialog.open()
            return true
        }
        pendingRecordingTrainingIndex = index
        if (index !== segmentBridge.activeIndex)
            activateLearningSegment(index)
        else
            syncActiveLearningSegment(index)
        Qt.callLater(root.tryStartPendingRecordingTraining)
        return true
    }

    function activatePendingLearningSegment() {
        var index = pendingSegmentIndex
        var shouldTrain = pendingSegmentTraining
        var shouldUseRecording = pendingSegmentRecordingTraining
        pendingSegmentIndex = -1
        pendingSegmentTraining = false
        pendingSegmentRecordingTraining = false
        if (index >= 0 && shouldUseRecording) {
            startSegmentRecordingTraining(index)
        } else if (index >= 0 && shouldTrain) {
            startSegmentTraining(index)
        } else if (index >= 0)
            activateLearningSegment(index)
    }

    function deleteLearningSegment(index) {
        if (index < 0 || index >= segmentBridge.segmentCount)
            return false

        var deletingActive = index === segmentBridge.activeIndex
        var segmentStart = segmentBridge.segmentStartAt(index)
        var segmentEnd = segmentBridge.segmentEndAt(index)
        trainingController.stopTraining()
        if (!segmentBridge.deleteSegment(index))
            return false

        if (deletingActive)
            waveformBridge.clearSelection()
        if (!subtitleBridge.deleteCuesForRange(segmentStart, segmentEnd)) {
            themeBridge.reportError("学习片段已删除，但对应字幕删除失败")
            return false
        }
        if (!recordingBridge.deleteRecordingsForRange(segmentStart, segmentEnd)) {
            themeBridge.reportError("学习片段已删除，但对应录音删除失败")
            return false
        }
        segmentBridge.refreshRecordingRanges()
        return true
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
    visibility: Window.Maximized
    minimumWidth: 1220
    minimumHeight: 760
    title: "LLStudio"

    onClosing: root.savePlaybackProgress()

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

    AiTutorBridge {
        id: aiTutorBridge
    }

    SpeechSettingsBridge {
        id: speechSettingsBridge
    }

    SpeechRecognitionBridge {
        id: speechRecognitionBridge
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

    RecordingPlaybackBridge {
        id: recordingPlaybackBridge
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
        function onPlaybackRateChanged() {
            if (recordingPlaybackBridge.hasRecording)
                recordingPlaybackBridge.applyPlaybackRate(
                            mediaBridge.playbackRate)
        }
        function onIsPlayingChanged() {
            if (!mediaBridge.isPlaying)
                root.savePlaybackProgress()
        }
        function onPreparingInitialFrameChanged() {
            if (mediaBridge.preparingInitialFrame
                    || root.pendingReadySegmentIndex < 0)
                return
            var index = root.pendingReadySegmentIndex
            root.pendingReadySegmentIndex = -1
            if (index === segmentBridge.activeIndex)
                root.syncActiveLearningSegment(index)
        }
    }

    Connections {
        target: subtitleBridge
        function onActiveCueIndexChanged() {
            if (subtitleBridge.activeCueIndex < 0) {
                if (!mediaBridge.isPlaying
                        && subtitleBridge.editingCueIndex >= 0)
                    return
                aiTutorBridge.setSubtitleContext(
                            mediaBridge.loadedPath,
                            -1,
                            0,
                            0,
                            "",
                            "",
                            "",
                            "")
                return
            }
            if (!mediaBridge.isPlaying
                    && subtitleBridge.editingCueIndex >= 0)
                return
            aiTutorBridge.setSubtitleContext(
                        mediaBridge.loadedPath,
                        subtitleBridge.activeCueIndex,
                        subtitleBridge.activeCueStart,
                        subtitleBridge.activeCueEnd,
                        subtitleBridge.activeOriginalText,
                        subtitleBridge.activeTranslatedText,
                        "", "")
        }
    }

    Timer {
        interval: 2000
        repeat: true
        running: mediaBridge.isPlaying
        onTriggered: root.savePlaybackProgress()
    }

    Timer {
        id: successMessageTimer
        interval: 2200
        repeat: false
        onTriggered: root.successMessage = ""
    }

    Platform.FileDialog {
        id: recordingExportDialog
        title: "保存录音"
        fileMode: Platform.FileDialog.SaveFile
        nameFilters: ["WAV 音频 (*.wav)", "所有文件 (*)"]
        defaultSuffix: "wav"

        onAccepted: root.exportRecordingToPath(file)
    }

    Timer {
        id: recordingPlaybackSyncTimer
        interval: 0
        repeat: false
        onTriggered: root.syncRecordingPlaybackSource()
    }

    Connections {
        target: recordingBridge
        function onHasRecordingChanged() {
            recordingPlaybackSyncTimer.restart()
        }
        function onRecordingFilePathChanged() {
            recordingPlaybackSyncTimer.restart()
        }
        function onAlignmentOffsetChanged() {
            recordingPlaybackSyncTimer.restart()
        }
        function onRecordingRevisionChanged() {
            segmentBridge.refreshRecordingRanges()
        }
        function onStatusMessageChanged() {
            if (!root.recordingOperationActive)
                return
            root.successMessage = recordingBridge.statusMessage
            if (recordingBridge.statusMessage.indexOf("失败") >= 0) {
                themeBridge.reportError(recordingBridge.statusMessage)
                root.recordingOperationActive = false
            }
        }
        function onIsProcessingChanged() {
            if (root.recordingOperationActive && !recordingBridge.isProcessing) {
                root.successMessage = recordingBridge.statusMessage
                successMessageTimer.restart()
                root.recordingOperationActive = false
            }
        }
    }

    Connections {
        target: recordingPlaybackBridge
        function onHasRecordingChanged() {
            root.tryStartPendingRecordingTraining()
        }
        function onHasPlayableOverlapChanged() {
            root.tryStartPendingRecordingTraining()
        }
    }

    Connections {
        target: waveformBridge
        function onSelectionRevisionChanged() {
            if (root.selectionAdjustmentActive
                    && segmentBridge.activeIndex >= 0) {
                segmentBridge.previewActiveRange(
                            waveformBridge.selectionStart,
                            waveformBridge.selectionEnd)
            }
            if (!root.deferRecordingTargetSync
                    && !root.selectionAdjustmentActive)
                root.syncRecordingTargetFromSelection()
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
        recordingAvailable: recordingPlaybackBridge.hasRecording
        playbackPosition: trainingController.isRecordingSession
                          ? recordingPlaybackBridge.currentPosition
                          : mediaBridge.currentPosition
        selectionAvailable: waveformBridge.hasSelectionStart
                            && waveformBridge.hasSelectionEnd
                            && waveformBridge.selectionEnd > waveformBridge.selectionStart
        selectionStart: waveformBridge.selectionStart
        selectionEnd: waveformBridge.selectionEnd

        onLoopCompleted: {
            if (!trainingController.isRecordingSession) {
                if (trainingController.isLabelSequence)
                    segmentBridge.recordLabelPlaybackLoop()
                else if (segmentBridge.activeIndex >= 0)
                    segmentBridge.incrementCompletedLoops()
            }
        }

        onSeekAndPlayRequested: function(positionSecs) {
            if (trainingController.isRecordingSession) {
                if (recordingPlaybackBridge.seek(positionSecs))
                    recordingPlaybackBridge.play()
            } else {
                if (videoPlaybackPane.seekToPosition(positionSecs)) {
                    mediaBridge.play()
                    videoPlaybackPane.syncPlaybackDependentPanels()
                }
            }
        }

        onPauseRequested: {
            if (trainingController.isRecordingSession)
                recordingPlaybackBridge.pause()
            else {
                mediaBridge.pause()
                videoPlaybackPane.syncPlaybackDependentPanels()
            }
        }

        onPauseAtPositionRequested: function(positionSecs) {
            if (trainingController.isRecordingSession) {
                recordingPlaybackBridge.pause()
                recordingPlaybackBridge.seek(positionSecs)
            } else {
                mediaBridge.pause()
                videoPlaybackPane.seekToPosition(positionSecs)
            }
        }

        onResumePlaybackRequested: {
            if (trainingController.isRecordingSession)
                recordingPlaybackBridge.play()
            else {
                mediaBridge.play()
                videoPlaybackPane.syncPlaybackDependentPanels()
            }
        }
    }

    Timer {
        interval: 33
        repeat: true
        running: recordingPlaybackBridge.isPlaying
        onTriggered: recordingPlaybackBridge.tick()
    }

    Timer {
        interval: 100
        repeat: true
        running: speechRecognitionBridge.isRecognizing
        onTriggered: speechRecognitionBridge.poll()
    }

    Connections {
        target: speechRecognitionBridge
        function onResultRevisionChanged() {
            subtitleView.applyRecognizedText(
                        speechRecognitionBridge.resultText,
                        speechRecognitionBridge.resultStart,
                        speechRecognitionBridge.resultEnd)
        }
        function onErrorMessageChanged() {
            if (speechRecognitionBridge.errorMessage.length > 0)
                themeBridge.reportError(speechRecognitionBridge.errorMessage)
        }
    }

    color: theme.windowBg

    Platform.MenuBar {
        window: root

        Platform.Menu {
            title: "文件"
            Platform.MenuItem {
                text: "打开视频"
                shortcut: StandardKey.Open
                onTriggered: videoPlaybackPane.openVideo()
            }
            Platform.MenuItem {
                text: "打开音频"
                onTriggered: videoPlaybackPane.openAudio()
            }
            Platform.MenuItem {
                text: "导入字幕"
            }
            Platform.MenuSeparator {}
            Platform.MenuItem {
                text: "设置…"
                shortcut: StandardKey.Preferences
                role: Platform.MenuItem.PreferencesRole
                onTriggered: settingsDialog.open()
            }
        }
        Platform.Menu {
            title: "编辑"
            type: Platform.Menu.EditMenu
            Platform.MenuItem { text: "查找"; shortcut: StandardKey.Find }
        }
        Platform.Menu {
            title: "播放"
            Platform.MenuItem { text: "播放/暂停"; onTriggered: root.toggleNormalPlayback() }
            Platform.MenuItem { text: "开始训练"; onTriggered: root.toggleTrainingPlayback() }
        }
        Platform.Menu {
            title: "麦克风"
            Platform.Menu {
                title: "录音降噪"
                enabled: recordingBridge.hasRecording
                         && !recordingBridge.isRecording
                         && !recordingBridge.isProcessing
                Platform.MenuItem {
                    text: "轻度"
                    onTriggered: root.processRecordingNoiseReduction("light")
                }
                Platform.MenuItem {
                    text: "标准"
                    onTriggered: root.processRecordingNoiseReduction("standard")
                }
                Platform.MenuItem {
                    text: "强力"
                    onTriggered: root.processRecordingNoiseReduction("strong")
                }
            }
            Platform.MenuSeparator {}
            Platform.MenuItem {
                text: "使用原始录音"
                enabled: recordingBridge.hasRecording
                         && !recordingBridge.isRecording
                         && !recordingBridge.isProcessing
                         && recordingBridge.activeRecordingVariant !== "original"
                onTriggered: root.useOriginalRecording()
            }
        }
        Platform.Menu {
            title: "帮助"
            Platform.MenuItem {
                text: "快捷键…"
                onTriggered: shortcutHelpDialog.open()
            }
            Platform.MenuSeparator {}
            Platform.MenuItem {
                text: "关于 LLStudio"
                onTriggered: aboutDialog.open()
            }
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
            handle: ThemedSplitHandle {
                splitOrientation: Qt.Horizontal
                gapColor: theme.windowBg
                dividerColor: theme.border
                accentColor: theme.accent
            }

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
                canStartTraining: trainingController.mediaAvailable
                                  && (trainingController.selectionAvailable
                                      || trainingController.hasActiveSession)
                isTraining: trainingController.isTraining
                hasStartedTraining: trainingController.hasActiveSession
                isPlaybackPlaying: trainingController.isRecordingSession
                                   ? recordingPlaybackBridge.isPlaying
                                   : mediaBridge.isPlaying
                completedLoops: trainingController.completedLoops
                totalLoops: trainingController.totalLoops
                trainingStatus: trainingController.statusMessage
                onVideoOpenRequested: function(path) {
                    videoPlaybackPane.loadVideoAndRelatedAssets(path)
                }
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

            SplitView {
                id: playbackWorkspace
                SplitView.fillWidth: true
                SplitView.minimumWidth: 620
                orientation: Qt.Vertical
                handle: ThemedSplitHandle {
                    splitOrientation: Qt.Vertical
                    gapColor: theme.windowBg
                    dividerColor: theme.border
                    accentColor: theme.accent
                }

                VideoPlaybackPane {
                    id: videoPlaybackPane
                    SplitView.minimumHeight: 340
                    SplitView.preferredHeight: Math.max(
                                340, Math.round(playbackWorkspace.height * 0.6))
                    SplitView.fillHeight: root.videoFullScreen
                    fullScreenMode: root.videoFullScreen
                    darkTheme: theme.darkAppearance
                    mediaBridge: mediaBridge
                    subtitleBridge: subtitleBridge
                    waveformBridge: waveformBridge
                    onManualSeekRequested: function(positionSecs) {
                        root.seekPlaybackManually(positionSecs)
                    }
                    onNormalPlaybackToggleRequested: root.toggleNormalPlayback()
                    onVideoLoadStarted: {
                        themeBridge.reportError("")
                        root.savePlaybackProgress()
                        aiTutorBridge.setSubtitleContext("", -1, 0, 0,
                                                         "", "", "", "")
                    }
                    onVideoLoadFailed: function(message) {
                        themeBridge.reportError(message.length > 0
                                                ? message : "视频加载失败")
                    }
                    onVideoLoaded: function(path, durationSecs) {
                        trainingController.stopTraining()
                        var recorded = libraryBridge.recordOpenedVideo(
                                    path, durationSecs)
                        if (recorded)
                            librarySidebar.revealLearningVideos()
                        var restoredPosition = recorded
                                ? libraryBridge.lastPlaybackPosition(path) : 0
                        mediaBridge.prepareInitialFrame(restoredPosition)
                        segmentBridge.loadForVideoPath(path, durationSecs)
                        noteBridge.loadForVideoPath(path, durationSecs)
                        noteBridge.syncPlaybackPosition(mediaBridge.currentPosition)
                        recordingBridge.loadForVideoPath(path, durationSecs)
                        segmentBridge.refreshRecordingRanges()
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
                    SplitView.fillHeight: !root.videoFullScreen
                    subtitleBridge: subtitleBridge
                    waveformBridge: waveformBridge
                    recordingBridge: recordingBridge
                    recordingTrainingActive: trainingController.hasActiveSession
                                             && trainingController.isRecordingSession
                    recordingPlaybackPlaying: recordingPlaybackBridge.isPlaying
                    recordingPlaybackPosition: recordingPlaybackBridge.currentPosition
                    originalTrainingActive: trainingController.hasActiveSession
                                            && !trainingController.isRecordingSession
                    originalPlaybackPlaying: mediaBridge.isPlaying
                    canBeginNextSegment: segmentBridge.activeIndex >= 0
                                         && segmentBridge.activeEnd
                                            < waveformBridge.durationSecs
                    speechRecognizing: speechRecognitionBridge.isRecognizing
                    onSpeechRecognitionRequested: function(startSecs, endSecs) {
                        if (!segmentBridge.ensureSelectionSegment(
                                    startSecs,
                                    endSecs,
                                    librarySidebar.repeatCount,
                                    librarySidebar.intervalSeconds))
                            return
                        speechRecognitionBridge.recognizeSelection(
                                    mediaBridge.loadedPath,
                                    startSecs,
                                    endSecs)
                    }
                    onSelectionAdjustmentStarted: root.beginSelectionAdjustment()
                    onSelectionChangeCommitted: function(startSecs, endSecs) {
                        root.commitSelectionAdjustment(startSecs, endSecs)
                    }
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
                        recordingPlaybackBridge.pause()
                        mediaBridge.pause()
                        videoPlaybackPane.syncPlaybackDependentPanels()
                        recordingBridge.startRecording()
                    }
                    onRecordingStopRequested: recordingBridge.stopRecording()
                    onRecordingDeleteRequested: {
                        if (trainingController.hasActiveSession
                                && trainingController.isRecordingSession)
                            trainingController.stopTraining()
                        recordingPlaybackBridge.unload()
                        recordingBridge.deleteRecording()
                    }
                    onRecordingExportRequested: root.openRecordingExportDialog()
                    onRecordingTrainingToggleRequested:
                        root.toggleRecordingTraining()
                    onOriginalTrainingToggleRequested:
                        root.toggleOriginalTraining()
                    onRecordingPlaybackSeekRequested: function(positionSecs) {
                        if (trainingController.hasActiveSession
                                && trainingController.isRecordingSession) {
                            var wasPlaying = recordingPlaybackBridge.isPlaying
                            recordingPlaybackBridge.seek(positionSecs)
                            if (wasPlaying)
                                recordingPlaybackBridge.play()
                        }
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
                handle: ThemedSplitHandle {
                    splitOrientation: Qt.Vertical
                    gapColor: theme.windowBg
                    dividerColor: theme.border
                    accentColor: theme.accent
                }

                SubtitleView {
                    id: subtitleView
                    SplitView.minimumHeight: 360
                    SplitView.preferredHeight: 520
                    subtitleBridge: subtitleBridge
                    noteBridge: noteBridge
                    videoPath: mediaBridge.loadedPath
                    waveformBridge: waveformBridge
                    aiBridge: aiTutorBridge
                    onNoteCreationRequested: function(startSecs, endSecs, hasRange) {
                        root.createNote(startSecs, endSecs, hasRange)
                    }
                    onNoteNavigationRequested: function(startSecs, endSecs, hasRange) {
                        root.navigateToNote(startSecs, endSecs, hasRange)
                    }
                    onAiSubtitleContextRequested: function(cueIndex, startSecs, endSecs, text) {
                        aiTutorBridge.setSubtitleContext(
                                    mediaBridge.loadedPath,
                                    cueIndex,
                                    startSecs,
                                    endSecs,
                                    text,
                                    "",
                                    "",
                                    "")
                    }
                    onSubtitleSaveSucceeded: function(message) {
                        root.showSuccessMessage(message)
                    }
                    onNoteExportSucceeded: function(message) {
                        root.showSuccessMessage(message)
                    }
                    onNoteExportFailed: function(message) {
                        themeBridge.reportError(message.length > 0
                                                 ? message : "导出笔记失败")
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
                        root.requestActivateLearningSegment(index)
                    }
                    onSegmentTrainingRequested: function(index) {
                        root.startSegmentTraining(index)
                    }
                    onRecordingTrainingRequested: function(index) {
                        root.startSegmentRecordingTraining(index)
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
            statusMessage: themeBridge.lastError.length > 0
                           ? themeBridge.lastError : root.successMessage
            statusIsError: themeBridge.lastError.length > 0
            statusIsSuccess: themeBridge.lastError.length === 0
                             && root.successMessage.length > 0
        }
    }

    SettingsDialog {
        id: settingsDialog
        themeBridge: themeBridge
        aiSettingsBridge: aiTutorBridge
        speechSettingsBridge: speechSettingsBridge
        x: (parent.width - width) / 2
        y: (parent.height - height) / 2
        panelBg: theme.panelBg
        elevatedBg: theme.elevatedBg
        borderColor: theme.border
        textPrimary: theme.textPrimary
        textSecondary: theme.textSecondary
        accent: theme.accent
        accentBg: theme.accentBg
    }

    ShortcutHelpDialog {
        id: shortcutHelpDialog
        x: (parent.width - width) / 2
        y: (parent.height - height) / 2
        panelBg: theme.panelBg
        elevatedBg: theme.elevatedBg
        borderColor: theme.border
        textPrimary: theme.textPrimary
        textSecondary: theme.textSecondary
        accent: theme.accent
        accentBg: theme.accentBg
    }

    AboutDialog {
        id: aboutDialog
        x: (parent.width - width) / 2
        y: (parent.height - height) / 2
        panelBg: theme.panelBg
        elevatedBg: theme.elevatedBg
        borderColor: theme.border
        textPrimary: theme.textPrimary
        textSecondary: theme.textSecondary
        accent: theme.accent
        accentBg: theme.accentBg
        darkTheme: theme.darkAppearance
    }

    Dialog {
        id: unsavedSubtitleDialog
        parent: Overlay.overlay
        anchors.centerIn: parent
        title: "字幕尚未保存"
        modal: true
        closePolicy: Popup.CloseOnEscape
        width: 430

        onRejected: {
            root.pendingSegmentIndex = -1
            root.pendingSegmentTraining = false
            root.pendingSegmentRecordingTraining = false
        }

        contentItem: Label {
            padding: 18
            text: "当前字幕内容尚未保存。是否先保存字幕，再切换到其他学习片段？"
            color: theme.textPrimary
            wrapMode: Text.Wrap
        }

        footer: DialogButtonBox {
            alignment: Qt.AlignRight

            Button {
                text: subtitleView.isUpdatingSubtitle
                      ? "更新字幕并切换" : "添加字幕并切换"
                DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
                onClicked: {
                    if (!subtitleView.saveEditorText())
                        return
                    unsavedSubtitleDialog.close()
                    root.activatePendingLearningSegment()
                }
            }

            Button {
                text: "不保存并切换"
                DialogButtonBox.buttonRole: DialogButtonBox.DestructiveRole
                onClicked: {
                    subtitleView.discardUnsavedSubtitleChanges()
                    unsavedSubtitleDialog.close()
                    root.activatePendingLearningSegment()
                }
            }

            Button {
                text: "取消"
                DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
                onClicked: unsavedSubtitleDialog.reject()
            }
        }
    }

}
