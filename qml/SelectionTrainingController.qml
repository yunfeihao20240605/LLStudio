import QtQuick 2.15

QtObject {
    id: root

    property bool mediaAvailable: false
    property real playbackPosition: 0
    property bool selectionAvailable: false
    property real selectionStart: 0
    property real selectionEnd: 0

    readonly property bool isTraining: internalIsTraining
    readonly property bool isWaiting: internalIsWaiting
    readonly property bool isPaused: internalIsPaused
    readonly property bool canResume: internalIsTraining && (internalIsPaused || internalIsWaiting)
    readonly property bool hasStartedCurrentSelection: internalHasActiveSession
                                                       && selectionAvailable
                                                       && Math.abs(selectionStart - activeRangeStart) <= 0.001
                                                       && Math.abs(selectionEnd - activeRangeEnd) <= 0.001
    readonly property int completedLoops: internalCompletedLoops
    readonly property int totalLoops: internalTotalLoops
    readonly property string statusMessage: internalStatusMessage

    property bool internalIsTraining: false
    property bool internalIsWaiting: false
    property bool internalIsPaused: false
    property bool internalHasActiveSession: false
    property int internalCompletedLoops: 0
    property int internalTotalLoops: 0
    property int internalIntervalSeconds: 0
    property real activeRangeStart: 0
    property real activeRangeEnd: 0
    property string internalStatusMessage: ""

    signal seekAndPlayRequested(real positionSecs)
    signal pauseRequested()
    signal pauseAtPositionRequested(real positionSecs)
    signal resumePlaybackRequested()
    signal loopCompleted()

    function startTraining(repeatCount, intervalSeconds) {
        if (!mediaAvailable || !selectionAvailable || selectionEnd <= selectionStart)
            return false

        intervalTimer.stop()
        internalTotalLoops = Math.max(1, Math.floor(repeatCount))
        internalIntervalSeconds = Math.max(0, Math.floor(intervalSeconds))
        internalCompletedLoops = 0
        activeRangeStart = selectionStart
        activeRangeEnd = selectionEnd
        internalIsTraining = true
        internalIsWaiting = false
        internalIsPaused = false
        internalHasActiveSession = true
        internalStatusMessage = "正在播放第 1/" + internalTotalLoops + " 次"
        beginCurrentLoop()
        return true
    }

    function stopTraining() {
        if (!internalIsTraining)
            return

        intervalTimer.stop()
        internalIsTraining = false
        internalIsWaiting = false
        internalIsPaused = false
        internalHasActiveSession = false
        internalStatusMessage = "训练已停止"
        pauseRequested()
    }

    function resumeTraining() {
        if (!internalIsTraining)
            return false

        if (internalIsWaiting) {
            intervalTimer.stop()
            beginCurrentLoop()
            return true
        }

        if (internalIsPaused) {
            internalIsPaused = false
            internalStatusMessage = "正在播放第 " + (internalCompletedLoops + 1)
                    + "/" + internalTotalLoops + " 次"
            resumePlaybackRequested()
            return true
        }

        return false
    }

    function pauseTraining() {
        if (!internalIsTraining || internalIsWaiting || internalIsPaused)
            return false
        internalIsPaused = true
        internalStatusMessage = "训练已暂停"
        pauseRequested()
        return true
    }

    function notifyPlaybackPaused() {
        if (!internalIsTraining || internalIsWaiting)
            return
        internalIsPaused = true
        internalStatusMessage = "训练已暂停"
    }

    function beginCurrentLoop() {
        if (!internalIsTraining)
            return

        internalIsWaiting = false
        internalIsPaused = false
        internalStatusMessage = "正在播放第 " + (internalCompletedLoops + 1) + "/" + internalTotalLoops + " 次"
        seekAndPlayRequested(activeRangeStart)
    }

    function finishCurrentLoop() {
        if (!internalIsTraining || internalIsWaiting || internalIsPaused)
            return

        internalIsWaiting = true
        internalIsPaused = false
        pauseAtPositionRequested(activeRangeEnd)
        internalCompletedLoops += 1
        loopCompleted()

        if (internalCompletedLoops >= internalTotalLoops) {
            intervalTimer.stop()
            internalIsTraining = false
            internalIsWaiting = false
            internalIsPaused = false
            internalHasActiveSession = false
            internalStatusMessage = "训练完成"
            return
        }

        internalStatusMessage = internalIntervalSeconds > 0
                ? "等待 " + internalIntervalSeconds + " 秒"
                : "准备下一次播放"
        intervalTimer.restart()
    }

    onPlaybackPositionChanged: {
        if (internalIsTraining && !internalIsWaiting && !internalIsPaused
                && playbackPosition >= activeRangeEnd - 0.02)
            finishCurrentLoop()
    }

    onMediaAvailableChanged: {
        if (internalIsTraining && !mediaAvailable)
            stopTraining()
    }

    onSelectionAvailableChanged: {
        if (internalIsTraining && !selectionAvailable)
            stopTraining()
    }

    onSelectionStartChanged: {
        if (internalIsTraining && Math.abs(selectionStart - activeRangeStart) > 0.001)
            stopTraining()
    }

    onSelectionEndChanged: {
        if (internalIsTraining && Math.abs(selectionEnd - activeRangeEnd) > 0.001)
            stopTraining()
    }

    property Timer intervalTimer: Timer {
        interval: Math.max(1, root.internalIntervalSeconds * 1000)
        repeat: false
        onTriggered: root.beginCurrentLoop()
    }
}
