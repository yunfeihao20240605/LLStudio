import QtQuick 2.15
import QtQuick.Controls 2.15

Item {
    id: root

    property var recordingBridge
    property real visibleStart: 0
    property real visibleEnd: 1
    property real displayGain: 1.8
    property color waveformColor: "#16815d"
    property color borderColor: "#d0d7de"
    property color textColor: "#1f2329"
    property color highlightColor: "#2f6fed"
    property real displayOffset: recordingBridge ? recordingBridge.alignmentOffset : 0
    property bool alignmentDragging: false
    property real pressX: 0
    property real pressOffset: 0

    signal seekRequested(real positionSecs)
    signal alignmentCommitRequested(real offsetSecs)
    signal resetAlignmentRequested()
    signal deleteRequested()

    function requestPaint() {
        recordingCanvas.requestPaint()
    }

    function clampOffset(offset) {
        if (!recordingBridge)
            return 0
        var minimum = -recordingBridge.recordingDuration + 0.01
        var maximum = recordingBridge.targetEnd - recordingBridge.targetStart - 0.01
        return Math.max(Math.min(minimum, maximum),
                        Math.min(Math.max(minimum, maximum), offset))
    }

    function formatOffset(offset) {
        var sign = offset >= 0 ? "+" : "−"
        return sign + Math.abs(offset).toFixed(3) + "s"
    }

    onVisibleStartChanged: requestPaint()
    onVisibleEndChanged: requestPaint()
    onDisplayOffsetChanged: requestPaint()
    onDisplayGainChanged: requestPaint()
    onWaveformColorChanged: requestPaint()

    Connections {
        target: root.recordingBridge
        ignoreUnknownSignals: true

        function onRecordingRevisionChanged() { root.requestPaint() }
        function onAlignmentOffsetChanged() {
            if (!root.alignmentDragging)
                root.displayOffset = root.recordingBridge.alignmentOffset
        }
    }

    Rectangle {
        anchors.fill: parent
        color: root.alignmentDragging
               ? Qt.rgba(root.highlightColor.r, root.highlightColor.g,
                         root.highlightColor.b, 0.08)
               : "transparent"
        border.color: root.alignmentDragging ? root.highlightColor : "transparent"
        border.width: root.alignmentDragging ? 1 : 0
    }

    Canvas {
        id: recordingCanvas
        anchors.fill: parent
        antialiasing: false
        renderTarget: Canvas.Image

        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()

        onPaint: {
            var context = getContext("2d")
            context.clearRect(0, 0, width, height)
            if (!root.recordingBridge || !root.recordingBridge.hasRecording
                    || width <= 0 || height <= 0)
                return
            var peaks = root.recordingBridge.recordingPeakValues
            var binCount = Math.floor(peaks.length / 2)
            var duration = root.recordingBridge.recordingDuration
            var viewportDuration = Math.max(0.001, root.visibleEnd - root.visibleStart)
            if (binCount <= 0 || duration <= 0)
                return

            var startTime = root.recordingBridge.targetStart + root.displayOffset
            var secondsPerBin = duration / binCount
            var firstBin = Math.max(0, Math.floor((root.visibleStart - startTime)
                                                 / secondsPerBin))
            var lastBin = Math.min(binCount, Math.ceil((root.visibleEnd - startTime)
                                                       / secondsPerBin))
            var centerY = height / 2
            var amplitudeHeight = Math.max(1, height * 0.40)
            var pixelsPerBin = secondsPerBin / viewportDuration * width
            var maximumVisiblePeak = 0
            for (var peakIndex = firstBin; peakIndex < lastBin; ++peakIndex) {
                maximumVisiblePeak = Math.max(
                            maximumVisiblePeak,
                            Math.abs(peaks[peakIndex * 2]),
                            Math.abs(peaks[peakIndex * 2 + 1]))
            }
            var basePeak = maximumVisiblePeak * root.displayGain
            var autoGain = basePeak > 0.0001
                    ? Math.max(1, Math.min(16, 0.78 / basePeak)) : 1
            var renderGain = root.displayGain * autoGain

            context.fillStyle = root.waveformColor
            if (pixelsPerBin >= 1) {
                var barWidth = Math.max(1, Math.min(6, pixelsPerBin * 0.72))
                for (var index = firstBin; index < lastBin; ++index) {
                    var x = ((startTime + index * secondsPerBin - root.visibleStart)
                             / viewportDuration) * width
                            + (pixelsPerBin - barWidth) * 0.5
                    drawBar(context, x, barWidth, peaks[index * 2],
                            peaks[index * 2 + 1], centerY, amplitudeHeight,
                            renderGain)
                }
            } else {
                var binsPerPixel = 1 / Math.max(0.0001, pixelsPerBin)
                for (var pixel = 0; pixel < Math.ceil(width); ++pixel) {
                    var binStart = Math.max(firstBin,
                                            Math.floor(firstBin + pixel * binsPerPixel))
                    var binEnd = Math.min(lastBin,
                                          Math.max(binStart + 1,
                                                   Math.floor(firstBin
                                                              + (pixel + 1) * binsPerPixel)))
                    if (binStart >= lastBin)
                        break
                    var minimum = 0
                    var maximum = 0
                    for (var bin = binStart; bin < binEnd; ++bin) {
                        minimum = Math.min(minimum, peaks[bin * 2])
                        maximum = Math.max(maximum, peaks[bin * 2 + 1])
                    }
                    drawBar(context, pixel, 1, minimum, maximum,
                            centerY, amplitudeHeight, renderGain)
                }
            }
        }

        function drawBar(context, x, width, minimum, maximum,
                         centerY, amplitudeHeight, renderGain) {
            var safeMin = Math.max(-1, Math.min(0, minimum * renderGain))
            var safeMax = Math.max(0, Math.min(1, maximum * renderGain))
            var top = centerY - safeMax * amplitudeHeight
            var bottom = centerY - safeMin * amplitudeHeight
            context.fillRect(x, top, Math.max(1, width), Math.max(2, bottom - top))
        }
    }

    Text {
        anchors.left: parent.left
        anchors.leftMargin: 8
        anchors.top: parent.top
        anchors.topMargin: 5
        text: "录音"
        color: root.waveformColor
        font.pixelSize: 11
        font.bold: true
    }

    Rectangle {
        visible: root.alignmentDragging
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        anchors.topMargin: 5
        width: offsetText.implicitWidth + 16
        height: 24
        radius: 6
        color: root.highlightColor

        Text {
            id: offsetText
            anchors.centerIn: parent
            text: root.formatOffset(root.displayOffset)
            color: "#ffffff"
            font.pixelSize: 12
            font.bold: true
        }
    }

    MouseArea {
        id: alignmentMouseArea
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        preventStealing: true
        cursorShape: pressed && root.alignmentDragging
                     ? Qt.SizeHorCursor : Qt.ArrowCursor

        onPressed: function(mouse) {
            if (mouse.button === Qt.RightButton)
                return
            root.pressX = mouse.x
            root.pressOffset = root.recordingBridge
                    ? root.recordingBridge.alignmentOffset : 0
            root.displayOffset = root.pressOffset
            root.alignmentDragging = false
        }

        onPositionChanged: function(mouse) {
            if (!pressed || (pressedButtons & Qt.LeftButton) === 0)
                return
            var deltaX = mouse.x - root.pressX
            if (!root.alignmentDragging && Math.abs(deltaX) < 5)
                return
            root.alignmentDragging = true
            var seconds = deltaX / Math.max(1, width)
                    * Math.max(0.001, root.visibleEnd - root.visibleStart)
            root.displayOffset = root.clampOffset(root.pressOffset + seconds)
        }

        onReleased: function(mouse) {
            if (mouse.button === Qt.RightButton) {
                recordingMenu.popup()
                return
            }
            if (root.alignmentDragging) {
                root.alignmentCommitRequested(root.displayOffset)
                root.alignmentDragging = false
            } else {
                var position = root.visibleStart + mouse.x / Math.max(1, width)
                        * Math.max(0.001, root.visibleEnd - root.visibleStart)
                root.seekRequested(position)
            }
        }

        onCanceled: {
            root.alignmentDragging = false
            root.displayOffset = root.recordingBridge
                    ? root.recordingBridge.alignmentOffset : 0
        }
    }

    Menu {
        id: recordingMenu

        MenuItem {
            text: "重置录音对齐"
            enabled: root.recordingBridge
                     && Math.abs(root.recordingBridge.alignmentOffset) > 0.0005
            onTriggered: root.resetAlignmentRequested()
        }

        MenuItem {
            text: "删除录音"
            onTriggered: root.deleteRequested()
        }
    }
}
