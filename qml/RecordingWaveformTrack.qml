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
    property color menuBackgroundColor: "#ffffff"
    property color menuHoverColor: "#eaf1fe"
    property color textColor: "#1f2329"
    property color highlightColor: "#2f6fed"
    property real dragPreviewOffset: 0
    property bool alignmentDragging: false
    readonly property real effectiveOffset: alignmentDragging
                                                ? dragPreviewOffset
                                                : (recordingBridge
                                                   ? recordingBridge.alignmentOffset : 0)
    property bool trainingPlaybackActive: false
    property bool trainingPlaybackPlaying: false
    property real trainingPlaybackPosition: 0
    readonly property real trainingPlaybackVideoPosition: recordingBridge
                                                        ? recordingBridge.targetStart
                                                          + trainingPlaybackPosition
                                                        : 0
    property real pressX: 0
    property real pressOffset: 0

    signal seekRequested(real positionSecs)
    signal trainingPlaybackToggleRequested()
    signal resetAlignmentRequested()
    signal deleteRequested()

    function requestPaint() {
        recordingCanvas.requestPaint()
    }

    function clampOffset(offset) {
        if (!recordingBridge)
            return 0
        var minimum = -recordingBridge.targetStart
        var maximum = Math.max(0, recordingBridge.videoDuration
                               - recordingBridge.recordingDuration)
                - recordingBridge.targetStart
        return Math.max(Math.min(minimum, maximum),
                        Math.min(Math.max(minimum, maximum), offset))
    }

    function formatOffset(offset) {
        var sign = offset >= 0 ? "+" : "−"
        return sign + Math.abs(offset).toFixed(3) + "s"
    }

    function normalizedRenderGain(renderedPeaks, zoom, displayGain) {
        if (!renderedPeaks || renderedPeaks.length <= 0)
            return 1
        var sampledPeaks = renderedPeaks.slice()
        sampledPeaks.sort(function(left, right) { return left - right })
        var percentileIndex = Math.floor((sampledPeaks.length - 1) * 0.95)
        var referencePeak = sampledPeaks[Math.max(0, percentileIndex)]
        if (!isFinite(referencePeak) || referencePeak <= 0.0001)
            return 1
        var zoomProgress = Math.max(0, Math.min(1,
                                 Math.log(Math.max(1, zoom)) / Math.log(200)))
        var targetHeight = (0.63 + 0.15 * zoomProgress) * displayGain / 1.8
        targetHeight = Math.max(0.35, Math.min(0.9, targetHeight))
        return Math.max(0.25, Math.min(16, targetHeight / referencePeak))
    }

    onVisibleStartChanged: requestPaint()
    onVisibleEndChanged: requestPaint()
    onEffectiveOffsetChanged: requestPaint()
    onDisplayGainChanged: requestPaint()
    onWaveformColorChanged: requestPaint()

    Connections {
        target: root.recordingBridge
        ignoreUnknownSignals: true

        function onRecordingRevisionChanged() {
            root.requestPaint()
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

            var startTime = root.recordingBridge.targetStart + root.effectiveOffset
            var secondsPerBin = duration / binCount
            var firstBin = Math.max(0, Math.floor((root.visibleStart - startTime)
                                                 / secondsPerBin))
            var lastBin = Math.min(binCount, Math.ceil((root.visibleEnd - startTime)
                                                       / secondsPerBin))
            var centerY = height / 2
            var amplitudeHeight = Math.max(1, height * 0.40)
            var pixelsPerBin = secondsPerBin / viewportDuration * width
            var renderedPeaks = []
            if (pixelsPerBin >= 1) {
                for (var referenceIndex = firstBin;
                     referenceIndex < lastBin; ++referenceIndex) {
                    renderedPeaks.push(Math.max(
                        Math.abs(peaks[referenceIndex * 2]),
                        Math.abs(peaks[referenceIndex * 2 + 1])))
                }
            } else {
                var referenceFirstPixel = Math.max(0, Math.floor(
                    (Math.max(root.visibleStart, startTime) - root.visibleStart)
                    / viewportDuration * width))
                var referenceLastPixel = Math.min(Math.ceil(width), Math.ceil(
                    (Math.min(root.visibleEnd, startTime + duration) - root.visibleStart)
                    / viewportDuration * width))
                for (var referencePixel = referenceFirstPixel;
                     referencePixel < referenceLastPixel; ++referencePixel) {
                    var referencePixelStart = root.visibleStart
                            + referencePixel / Math.max(1, width) * viewportDuration
                    var referencePixelEnd = root.visibleStart
                            + (referencePixel + 1) / Math.max(1, width) * viewportDuration
                    var referenceStart = Math.max(firstBin, Math.floor(
                        (referencePixelStart - startTime) / secondsPerBin))
                    var referenceEnd = Math.min(lastBin, Math.max(referenceStart + 1,
                        Math.ceil((referencePixelEnd - startTime) / secondsPerBin)))
                    var negativeEnergy = 0
                    var positiveEnergy = 0
                    var referenceCount = 0
                    for (var referenceBin = referenceStart;
                         referenceBin < referenceEnd; ++referenceBin) {
                        negativeEnergy += peaks[referenceBin * 2]
                                * peaks[referenceBin * 2]
                        positiveEnergy += peaks[referenceBin * 2 + 1]
                                * peaks[referenceBin * 2 + 1]
                        ++referenceCount
                    }
                    renderedPeaks.push(Math.max(
                        Math.sqrt(negativeEnergy / Math.max(1, referenceCount)),
                        Math.sqrt(positiveEnergy / Math.max(1, referenceCount))))
                }
            }
            var effectiveZoom = Math.max(1,
                    root.recordingBridge.videoDuration / viewportDuration)
            var renderGain = root.normalizedRenderGain(
                        renderedPeaks, effectiveZoom, root.displayGain)

            context.fillStyle = root.waveformColor
            if (pixelsPerBin >= 1) {
                if (effectiveZoom >= 200) {
                    for (var index = firstBin; index < lastBin; ++index) {
                        var nextIndex = Math.min(lastBin - 1, index + 1)
                        var leftX = ((startTime + index * secondsPerBin
                                      - root.visibleStart) / viewportDuration) * width
                        var rightX = ((startTime + (index + 1) * secondsPerBin
                                       - root.visibleStart) / viewportDuration) * width
                        var currentMin = Math.max(-1, Math.min(0,
                            peaks[index * 2] * renderGain))
                        var currentMax = Math.max(0, Math.min(1,
                            peaks[index * 2 + 1] * renderGain))
                        var nextMin = Math.max(-1, Math.min(0,
                            peaks[nextIndex * 2] * renderGain))
                        var nextMax = Math.max(0, Math.min(1,
                            peaks[nextIndex * 2 + 1] * renderGain))
                        context.beginPath()
                        context.moveTo(leftX,
                            centerY - currentMax * amplitudeHeight)
                        context.lineTo(rightX,
                            centerY - nextMax * amplitudeHeight)
                        context.lineTo(rightX,
                            centerY - nextMin * amplitudeHeight)
                        context.lineTo(leftX,
                            centerY - currentMin * amplitudeHeight)
                        context.closePath()
                        context.fill()
                    }
                } else {
                    var barWidth = Math.max(1, Math.min(6, pixelsPerBin * 0.72))
                    for (var index = firstBin; index < lastBin; ++index) {
                        var x = ((startTime + index * secondsPerBin - root.visibleStart)
                                 / viewportDuration) * width
                                + (pixelsPerBin - barWidth) * 0.5
                        drawBar(context, x, barWidth, peaks[index * 2],
                                peaks[index * 2 + 1], centerY, amplitudeHeight,
                                renderGain)
                    }
                }
            } else {
                var recordingEndTime = startTime + duration
                var firstPixel = Math.max(0, Math.floor(
                    (Math.max(root.visibleStart, startTime) - root.visibleStart)
                    / viewportDuration * width))
                var lastPixel = Math.min(Math.ceil(width), Math.ceil(
                    (Math.min(root.visibleEnd, recordingEndTime) - root.visibleStart)
                    / viewportDuration * width))
                for (var pixel = firstPixel; pixel < lastPixel; ++pixel) {
                    var pixelStartTime = root.visibleStart
                            + pixel / Math.max(1, width) * viewportDuration
                    var pixelEndTime = root.visibleStart
                            + (pixel + 1) / Math.max(1, width) * viewportDuration
                    var binStart = Math.max(firstBin, Math.floor(
                        (pixelStartTime - startTime) / secondsPerBin))
                    var binEnd = Math.min(lastBin, Math.max(binStart + 1,
                        Math.ceil((pixelEndTime - startTime) / secondsPerBin)))
                    var negativeEnergy = 0
                    var positiveEnergy = 0
                    var aggregateCount = 0
                    for (var bin = binStart; bin < binEnd; ++bin) {
                        negativeEnergy += peaks[bin * 2] * peaks[bin * 2]
                        positiveEnergy += peaks[bin * 2 + 1] * peaks[bin * 2 + 1]
                        ++aggregateCount
                    }
                    var rmsMinimum = -Math.sqrt(
                                negativeEnergy / Math.max(1, aggregateCount))
                    var rmsMaximum = Math.sqrt(
                                positiveEnergy / Math.max(1, aggregateCount))
                    drawBar(context, pixel, 1, rmsMinimum, rmsMaximum,
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

    Rectangle {
        z: 2
        visible: root.trainingPlaybackActive && root.recordingBridge
                 && root.trainingPlaybackVideoPosition >= root.visibleStart
                 && root.trainingPlaybackVideoPosition <= root.visibleEnd
        x: {
            if (!root.recordingBridge)
                return 0
            var ratio = (root.trainingPlaybackVideoPosition - root.visibleStart)
                    / Math.max(0.001, root.visibleEnd - root.visibleStart)
            return Math.max(0, Math.min(parent.width - width,
                                        ratio * parent.width - width / 2))
        }
        y: 0
        width: 3
        height: parent.height
        color: root.waveformColor
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
            text: root.formatOffset(root.dragPreviewOffset)
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
            root.dragPreviewOffset = root.pressOffset
            root.alignmentDragging = false
        }

        onPositionChanged: function(mouse) {
            if (!pressed || (mouse.buttons & Qt.LeftButton) === 0)
                return
            var deltaX = mouse.x - root.pressX
            if (!root.alignmentDragging && Math.abs(deltaX) < 5)
                return
            root.alignmentDragging = true
            var seconds = deltaX / Math.max(1, width)
                    * Math.max(0.001, root.visibleEnd - root.visibleStart)
            root.dragPreviewOffset = root.clampOffset(root.pressOffset + seconds)
        }

        onReleased: function(mouse) {
            if (mouse.button === Qt.RightButton) {
                recordingMenu.popup()
                return
            }
            if (root.alignmentDragging) {
                if (root.recordingBridge)
                    root.recordingBridge.saveAlignmentOffset(
                                root.dragPreviewOffset)
                root.alignmentDragging = false
            } else {
                var position = root.visibleStart + mouse.x / Math.max(1, width)
                        * Math.max(0.001, root.visibleEnd - root.visibleStart)
                root.seekRequested(position)
            }
        }

        onCanceled: {
            root.alignmentDragging = false
        }
    }

    ThemedMenu {
        id: recordingMenu
        panelColor: root.menuBackgroundColor
        borderColor: root.borderColor

        ThemedMenuItem {
            textColor: root.textColor
            disabledTextColor: root.textColor
            hoverColor: root.menuHoverColor
            text: !root.trainingPlaybackActive ? "播放录音"
                  : (root.trainingPlaybackPlaying
                     ? "暂停播放录音" : "继续播放录音")
            enabled: root.recordingBridge && root.recordingBridge.hasRecording
            onTriggered: root.trainingPlaybackToggleRequested()
        }

        ThemedMenuSeparator { separatorColor: root.borderColor }

        ThemedMenuItem {
            textColor: root.textColor
            disabledTextColor: root.textColor
            hoverColor: root.menuHoverColor
            text: "重置录音对齐"
            enabled: root.recordingBridge
                     && Math.abs(root.recordingBridge.alignmentOffset) > 0.0005
            onTriggered: root.resetAlignmentRequested()
        }

        ThemedMenuItem {
            textColor: root.textColor
            disabledTextColor: root.textColor
            hoverColor: root.menuHoverColor
            text: "删除录音"
            onTriggered: root.deleteRequested()
        }
    }
}
