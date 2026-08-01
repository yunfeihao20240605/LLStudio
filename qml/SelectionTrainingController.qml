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
    readonly property bool hasActiveSession: internalHasActiveSession
    readonly property bool isLabelSequence: internalSessionLabel.length > 0
    readonly property bool hasStartedCurrentSelection: internalHasActiveSession
                                                       && !isLabelSequence
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
    property var internalRanges: []
    property int internalCurrentRangeIndex: 0
    property string internalSessionLabel: ""

    signal seekAndPlayRequested(real positionSecs)
    signal pauseRequested()
    signal pauseAtPositionRequested(real positionSecs)
    signal resumePlaybackRequested()
    signal loopCompleted()

    function startTraining(repeatCount, intervalSeconds) {
        if (!mediaAvailable || !selectionAvailable || selectionEnd <= selectionStart)
            return false

        return startRangeSequence([
            { start: selectionStart, end: selectionEnd }
        ], repeatCount, intervalSeconds, "")
    }

    function startRangeSequence(ranges, repeatCount, intervalSeconds, label) {
        if (!mediaAvailable || !ranges || ranges.length <= 0)
            return false

        var normalizedRanges = []
        for (var index = 0; index < ranges.length; ++index) {
            var start = Number(ranges[index].start)
            var end = Number(ranges[index].end)
            if (!isFinite(start) || !isFinite(end) || start < 0 || end <= start)
                return false
            normalizedRanges.push({ start: start, end: end })
        }

        intervalTimer.stop()
        internalTotalLoops = Math.max(1, Math.floor(repeatCount))
        internalIntervalSeconds = Math.max(0, Math.floor(intervalSeconds))
        internalCompletedLoops = 0
        internalRanges = normalizedRanges
        internalCurrentRangeIndex = 0
        internalSessionLabel = label ? String(label) : ""
        activeRangeStart = normalizedRanges[0].start
        activeRangeEnd = normalizedRanges[0].end
        internalIsTraining = true
        internalIsWaiting = false
        internalIsPaused = false
        internalHasActiveSession = true
        beginCurrentLoop()
        return true
    }

    function stopTraining() {
        if (!internalIsTraining)
            return

        clearTrainingSession("训练已停止")
        pauseRequested()
    }

    function cancelTrainingSession() {
        if (!internalHasActiveSession && !internalIsTraining)
            return false

        clearTrainingSession("")
        return true
    }

    function clearTrainingSession(message) {
        intervalTimer.stop()
        internalIsTraining = false
        internalIsWaiting = false
        internalIsPaused = false
        internalHasActiveSession = false
        internalRanges = []
        internalCurrentRangeIndex = 0
        internalSessionLabel = ""
        internalStatusMessage = message
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
            internalStatusMessage = playingStatus()
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
        internalCurrentRangeIndex = 0
        beginCurrentRange()
    }

    function beginCurrentRange() {
        if (!internalIsTraining || internalCurrentRangeIndex >= internalRanges.length)
            return

        var range = internalRanges[internalCurrentRangeIndex]
        activeRangeStart = range.start
        activeRangeEnd = range.end
        internalStatusMessage = playingStatus()
        seekAndPlayRequested(activeRangeStart)
    }

    function playingStatus() {
        var loopText = "第 " + (internalCompletedLoops + 1) + "/" + internalTotalLoops + " 次"
        if (!isLabelSequence)
            return "正在播放 " + loopText
        return "正在播放“" + internalSessionLabel + "” 范围 "
                + (internalCurrentRangeIndex + 1) + "/" + internalRanges.length
                + " " + loopText
    }

    function finishCurrentRange() {
        if (!internalIsTraining || internalIsWaiting || internalIsPaused)
            return

        pauseAtPositionRequested(activeRangeEnd)
        if (internalCurrentRangeIndex + 1 < internalRanges.length) {
            internalCurrentRangeIndex += 1
            beginCurrentRange()
            return
        }
        finishCurrentLoop()
    }

    function finishCurrentLoop() {
        if (!internalIsTraining || internalIsWaiting || internalIsPaused)
            return

        internalIsWaiting = true
        internalIsPaused = false
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
            finishCurrentRange()
    }

    onMediaAvailableChanged: {
        if (internalIsTraining && !mediaAvailable)
            stopTraining()
    }

    onSelectionAvailableChanged: {
        if (internalIsTraining && !isLabelSequence && !selectionAvailable)
            stopTraining()
    }

    onSelectionStartChanged: {
        if (internalIsTraining && !isLabelSequence
                && Math.abs(selectionStart - activeRangeStart) > 0.001)
            stopTraining()
    }

    onSelectionEndChanged: {
        if (internalIsTraining && !isLabelSequence
                && Math.abs(selectionEnd - activeRangeEnd) > 0.001)
            stopTraining()
    }

    property Timer intervalTimer: Timer {
        interval: Math.max(1, root.internalIntervalSeconds * 1000)
        repeat: false
        onTriggered: root.beginCurrentLoop()
    }
}
