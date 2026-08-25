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
    property var recordingBridge
    property bool recordingTrainingActive: false
    property bool recordingPlaybackPlaying: false
    property real recordingPlaybackPosition: 0
    property bool originalTrainingActive: false
    property bool originalPlaybackPlaying: false
    property bool contextMenuOnOriginalTrack: false
    property bool canBeginNextSegment: false
    property bool speechRecognizing: false
    readonly property bool compactMode: width < 720
    property real zoomFactor: 1.0
    property real minimumZoom: 1.0
    property real maximumZoom: 2000.0
    readonly property var zoomLevels: [1, 2, 5, 10, 20, 50, 100, 200,
                                       500, 1000, 2000]
    property real waveformDisplayGain: 1.8
    property bool followPlayback: true
    readonly property bool comparisonVisible: recordingBridge
                                              && recordingBridge.hasRecording
                                              && recordingBridge.recordingPeakValues.length > 0
    readonly property real waveformBackgroundLuminance: elevatedBg.r * 0.2126 + elevatedBg.g * 0.7152 + elevatedBg.b * 0.0722
    readonly property color playheadColor: accent
    readonly property color playheadTextColor: "#ffffff"
    readonly property color selectionStartMarkerColor: waveformBackgroundLuminance > 0.55 ? "#15803d" : "#4ade80"
    readonly property color selectionEndMarkerColor: waveformBackgroundLuminance > 0.55 ? "#c2410c" : "#fb923c"
    readonly property color waveformColor: waveformBackgroundLuminance > 0.55 ? "#cfd4dc" : "#6b7280"

    signal playbackPositionRequested(real positionSecs)
    signal noteCreationRequested(real startSecs, real endSecs, bool hasRange)
    signal selectionCleared()
    signal nextSegmentRequested()
    signal recordingStartRequested()
    signal recordingStopRequested()
    signal recordingDeleteRequested()
    signal recordingTrainingToggleRequested()
    signal recordingPlaybackSeekRequested(real positionSecs)
    signal originalTrainingToggleRequested()
    signal speechRecognitionRequested(real startSecs, real endSecs)
    signal selectionAdjustmentStarted()
    signal selectionChangeCommitted(real startSecs, real endSecs)

    function formatSeconds(totalSeconds) {
        var safe = Math.max(0, Math.floor(totalSeconds || 0))
        var hours = Math.floor(safe / 3600)
        var minutes = Math.floor((safe % 3600) / 60)
        var seconds = safe % 60
        if (hours > 0)
            return hours + ":" + (minutes < 10 ? "0" + minutes : minutes) + ":" + (seconds < 10 ? "0" + seconds : seconds)
        return "00:" + (minutes < 10 ? "0" + minutes : minutes) + ":" + (seconds < 10 ? "0" + seconds : seconds)
    }

    function durationSecs() {
        return waveformBridge && waveformBridge.durationSecs > 0 ? waveformBridge.durationSecs : 180
    }

    function selectionStart() {
        return waveformBridge ? waveformBridge.selectionStart : 0
    }

    function selectionEnd() {
        return waveformBridge ? waveformBridge.selectionEnd : 0
    }

    function selectionIsValid() {
        return waveformBridge
                && waveformBridge.hasSelectionStart
                && waveformBridge.hasSelectionEnd
                && waveformBridge.selectionEnd > waveformBridge.selectionStart
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

    function selectionLengthLabel() {
        return selectionIsValid()
                ? "选区长度：" + formatSeconds(selectionEnd() - selectionStart())
                : "选区长度：--"
    }

    function clamp(value, minimum, maximum) {
        return Math.max(minimum, Math.min(maximum, value))
    }

    function zoomToSliderPosition(zoom) {
        var ratio = maximumZoom / minimumZoom
        if (ratio <= 1)
            return 0
        return Math.log(clamp(zoom, minimumZoom, maximumZoom) / minimumZoom)
                / Math.log(ratio)
    }

    function sliderPositionToZoom(position) {
        var ratio = maximumZoom / minimumZoom
        return minimumZoom * Math.pow(ratio, clamp(position, 0, 1))
    }

    function stepZoom(direction) {
        if (direction > 0) {
            for (var nextIndex = 0; nextIndex < zoomLevels.length; ++nextIndex) {
                if (zoomLevels[nextIndex] > zoomFactor + 0.001) {
                    setZoom(zoomLevels[nextIndex])
                    return
                }
            }
            setZoom(maximumZoom)
            return
        }
        for (var previousIndex = zoomLevels.length - 1;
             previousIndex >= 0; --previousIndex) {
            if (zoomLevels[previousIndex] < zoomFactor - 0.001) {
                setZoom(zoomLevels[previousIndex])
                return
            }
        }
        setZoom(minimumZoom)
    }

    function timeToContentX(seconds) {
        return clamp(seconds / Math.max(0.001, durationSecs()), 0, 1) * waveformContent.width
    }

    function contentXToTime(x) {
        return clamp(x / Math.max(1, waveformContent.width), 0, 1) * durationSecs()
    }

    function visibleStart() {
        return contentXToTime(waveformFlickable.contentX)
    }

    function visibleEnd() {
        return contentXToTime(waveformFlickable.contentX + waveformFlickable.width)
    }

    function tickInterval() {
        var visibleDuration = Math.max(0.001, visibleEnd() - visibleStart())
        var roughInterval = visibleDuration / 6
        var magnitude = Math.pow(10, Math.floor(Math.log(roughInterval) / Math.LN10))
        var normalized = roughInterval / magnitude
        var niceMultiplier = normalized <= 1 ? 1 : (normalized <= 2 ? 2 : (normalized <= 5 ? 5 : 10))
        return Math.max(0.1, niceMultiplier * magnitude)
    }

    function setZoom(nextZoom) {
        var clampedZoom = clamp(nextZoom, minimumZoom, maximumZoom)
        if (Math.abs(clampedZoom - zoomFactor) < 0.001)
            return

        var anchorX = waveformFlickable.width / 2
        var anchorTime = contentXToTime(waveformFlickable.contentX + anchorX)
        zoomFactor = clampedZoom
        Qt.callLater(function() {
            var maximumX = Math.max(0, waveformFlickable.contentWidth - waveformFlickable.width)
            waveformFlickable.contentX = clamp(timeToContentX(anchorTime) - anchorX, 0, maximumX)
        })
    }

    function centerOnTime(seconds) {
        Qt.callLater(function() {
            var maximumX = Math.max(0, waveformFlickable.contentWidth - waveformFlickable.width)
            waveformFlickable.contentX = clamp(timeToContentX(seconds) - waveformFlickable.width / 2, 0, maximumX)
        })
    }

    function revealRecordingRange() {
        if (!recordingBridge || !recordingBridge.hasRecording)
            return
        var rangeDuration = Math.max(0.01,
                                     recordingBridge.targetEnd
                                     - recordingBridge.targetStart)
        var targetZoom = clamp(durationSecs() * 0.72 / rangeDuration,
                               minimumZoom, maximumZoom)
        zoomFactor = Math.max(zoomFactor, targetZoom)
        followPlayback = false
        Qt.callLater(function() {
            var center = (recordingBridge.targetStart
                          + recordingBridge.targetEnd) / 2
            var maximumX = Math.max(0, waveformFlickable.contentWidth
                                    - waveformFlickable.width)
            waveformFlickable.contentX = clamp(
                        timeToContentX(center) - waveformFlickable.width / 2,
                        0, maximumX)
            root.requestWaveformPaint()
            recordingTrack.requestPaint()
        })
    }

    function resetViewport() {
        followPlayback = true
        zoomFactor = minimumZoom
        Qt.callLater(function() {
            waveformFlickable.contentX = 0
        })
    }

    function followPlaybackPosition() {
        if (!followPlayback || zoomFactor <= minimumZoom || !waveformBridge)
            return

        var playheadX = timeToContentX(waveformBridge.currentPosition)
        var leftLimit = waveformFlickable.contentX + waveformFlickable.width * 0.15
        var rightLimit = waveformFlickable.contentX + waveformFlickable.width * 0.82
        if (playheadX < leftLimit || playheadX > rightLimit) {
            var maximumX = Math.max(0, waveformFlickable.contentWidth - waveformFlickable.width)
            waveformFlickable.contentX = clamp(playheadX - waveformFlickable.width * 0.3, 0, maximumX)
        }
    }

    function requestWaveformPaint() {
        if (waveformCanvas)
            waveformCanvas.requestPaint()
    }

    function formatRecordingTime(totalSeconds) {
        var safe = Math.max(0, totalSeconds || 0)
        var minutes = Math.floor(safe / 60)
        var seconds = safe - minutes * 60
        return (minutes < 10 ? "0" + minutes : minutes) + ":"
                + (seconds < 10 ? "0" : "") + seconds.toFixed(1)
    }

    function scheduleDetailRequest() {
        if (zoomFactor >= 200 && waveformBridge)
            detailRequestTimer.restart()
    }

    onAccentChanged: requestWaveformPaint()
    onElevatedBgChanged: requestWaveformPaint()
    onBorderColorChanged: requestWaveformPaint()
    onWaveformDisplayGainChanged: requestWaveformPaint()
    onZoomFactorChanged: {
        requestWaveformPaint()
        scheduleDetailRequest()
    }
    onComparisonVisibleChanged: {
        requestWaveformPaint()
        if (comparisonVisible)
            revealRecordingRange()
    }

    radius: 16
    color: panelBg
    border.color: borderColor
    border.width: 1

    Timer {
        interval: 80
        repeat: true
        running: waveformBridge && (waveformBridge.isLoading || waveformBridge.isDetailLoading)
        onTriggered: {
            if (waveformBridge)
                waveformBridge.pollBackgroundTask()
        }
    }

    Timer {
        interval: 80
        repeat: true
        running: recordingBridge
                 && (recordingBridge.isRecording || recordingBridge.isProcessing)
        onTriggered: recordingBridge.pollBackgroundTask()
    }

    Timer {
        id: detailRequestTimer
        interval: 100
        repeat: false
        onTriggered: {
            if (waveformBridge && root.zoomFactor >= 200)
                waveformBridge.requestDetailRange(root.visibleStart(), root.visibleEnd(), root.zoomFactor)
        }
    }

    Connections {
        target: waveformBridge
        ignoreUnknownSignals: true

        function onCurrentPositionChanged() {
            root.followPlaybackPosition()
        }

        function onPeakRevisionChanged() {
            root.requestWaveformPaint()
        }

        function onDetailRevisionChanged() {
            root.requestWaveformPaint()
        }

        function onSelectionStartChanged() {
            root.requestWaveformPaint()
        }

        function onSelectionEndChanged() {
            root.requestWaveformPaint()
        }

        function onDurationSecsChanged() {
            root.resetViewport()
            root.scheduleDetailRequest()
        }
    }

    ThemedMenu {
        id: waveformContextMenu
        panelColor: root.panelBg
        borderColor: root.borderColor

        ThemedMenuItem {
            textColor: root.textPrimary
            disabledTextColor: root.textSecondary
            hoverColor: root.accentBg
            visible: root.contextMenuOnOriginalTrack
            text: !root.originalTrainingActive ? "播放原音"
                  : (root.originalPlaybackPlaying
                     ? "暂停播放原音" : "继续播放原音")
            enabled: root.waveformBridge && root.waveformBridge.durationSecs > 0
            onTriggered: root.originalTrainingToggleRequested()
        }

        ThemedMenuSeparator {
            separatorColor: root.borderColor
            visible: root.contextMenuOnOriginalTrack
        }

        ThemedMenuItem {
            textColor: root.textPrimary
            disabledTextColor: root.textSecondary
            hoverColor: root.accentBg
            text: waveformBridge
                  && waveformBridge.hasSelectionStart
                  && waveformBridge.hasSelectionEnd
                  && waveformBridge.selectionEnd > waveformBridge.selectionStart
                  ? "为当前选区添加笔记" : "在当前时间添加笔记"
            enabled: waveformBridge && waveformBridge.durationSecs > 0
            onTriggered: {
                var hasRange = waveformBridge.hasSelectionStart
                        && waveformBridge.hasSelectionEnd
                        && waveformBridge.selectionEnd > waveformBridge.selectionStart
                root.noteCreationRequested(
                            hasRange ? waveformBridge.selectionStart
                                     : waveformBridge.currentPosition,
                            hasRange ? waveformBridge.selectionEnd
                                     : waveformBridge.currentPosition,
                            hasRange)
            }
        }

        ThemedMenuSeparator { separatorColor: root.borderColor }

        ThemedMenuItem {
            textColor: root.textPrimary
            disabledTextColor: root.textSecondary
            hoverColor: root.accentBg
            text: root.speechRecognizing ? "正在识别当前片段…" : "识别当前片段字幕"
            enabled: !root.speechRecognizing
                     && waveformBridge
                     && waveformBridge.hasSelectionStart
                     && waveformBridge.hasSelectionEnd
                     && waveformBridge.selectionEnd > waveformBridge.selectionStart
            onTriggered: root.speechRecognitionRequested(
                             waveformBridge.selectionStart,
                             waveformBridge.selectionEnd)
        }

        ThemedMenuSeparator { separatorColor: root.borderColor }

        ThemedMenuItem {
            textColor: root.textPrimary
            disabledTextColor: root.textSecondary
            hoverColor: root.accentBg
            text: "从当前片段结尾开始下一片段"
            enabled: root.canBeginNextSegment
            onTriggered: root.nextSegmentRequested()
        }

        ThemedMenuSeparator { separatorColor: root.borderColor }

        ThemedMenuItem {
            textColor: root.textPrimary
            disabledTextColor: root.textSecondary
            hoverColor: root.accentBg
            text: "清除选区"
            enabled: waveformBridge
                     && (waveformBridge.hasSelectionStart || waveformBridge.hasSelectionEnd)
            onTriggered: {
                if (waveformBridge && waveformBridge.clearSelection())
                    root.selectionCleared()
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            spacing: root.compactMode ? 6 : 8

            Text {
                text: "波形视图"
                color: textPrimary
                font.pixelSize: 16
                font.bold: true
            }

            ThemedToolButton {
                Layout.preferredWidth: 44
                text: "−"
                enabled: root.zoomFactor > root.minimumZoom
                onClicked: root.stepZoom(-1)
                panelColor: root.panelBg
                borderColor: root.borderColor
                textColor: root.textPrimary
                disabledTextColor: root.textSecondary
                accentColor: root.accent
                accentBackgroundColor: root.accentBg
            }

            ThemedSlider {
                Layout.fillWidth: true
                Layout.minimumWidth: 72
                Layout.maximumWidth: 150
                from: 0
                to: 1
                stepSize: 0
                value: root.zoomToSliderPosition(root.zoomFactor)
                onMoved: root.setZoom(root.sliderPositionToZoom(value))
                panelColor: root.panelBg
                trackColor: root.borderColor
                accentColor: root.accent
            }

            ThemedToolButton {
                Layout.preferredWidth: 44
                text: "+"
                enabled: root.zoomFactor < root.maximumZoom
                onClicked: root.stepZoom(1)
                panelColor: root.panelBg
                borderColor: root.borderColor
                textColor: root.textPrimary
                disabledTextColor: root.textSecondary
                accentColor: root.accent
                accentBackgroundColor: root.accentBg
            }

            Text {
                text: root.zoomFactor.toFixed(1) + "x"
                color: textSecondary
                font.pixelSize: 13
                Layout.preferredWidth: 58
            }

            ThemedToolButton {
                text: root.followPlayback ? "自动跟随" : "恢复跟随"
                onClicked: {
                    root.followPlayback = true
                    root.centerOnTime(waveformBridge ? waveformBridge.currentPosition : 0)
                }
                panelColor: root.panelBg
                borderColor: root.borderColor
                textColor: root.textPrimary
                disabledTextColor: root.textSecondary
                accentColor: root.accent
                accentBackgroundColor: root.accentBg
            }

            Text {
                Layout.fillWidth: true
                Layout.minimumWidth: 0
                text: waveformBridge
                      ? "已加载：" + waveformBridge.loadedBinCount
                        + "/" + waveformBridge.totalBinCount + " bins"
                      : ""
                color: textSecondary
                font.pixelSize: 12
                horizontalAlignment: Text.AlignRight
                elide: Text.ElideRight
                maximumLineCount: 1
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: 12
            color: elevatedBg
            border.color: borderColor

            Flickable {
                id: waveformFlickable
                anchors.fill: parent
                anchors.margins: 12
                contentWidth: Math.max(width, width * root.zoomFactor)
                contentHeight: height
                clip: true
                interactive: root.zoomFactor > root.minimumZoom
                boundsBehavior: Flickable.StopAtBounds

                onMovementStarted: root.followPlayback = false
                onContentXChanged: {
                    root.requestWaveformPaint()
                    root.scheduleDetailRequest()
                }
                onWidthChanged: root.scheduleDetailRequest()

                ScrollBar.horizontal: ScrollBar {
                    id: waveformScrollBar
                    policy: root.zoomFactor > root.minimumZoom ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff
                    height: 10
                    onPressedChanged: {
                        if (pressed)
                            root.followPlayback = false
                    }
                }

                Item {
                    id: waveformContent
                    width: waveformFlickable.contentWidth
                    height: waveformFlickable.height

                    readonly property real currentTickInterval: root.tickInterval()
                    readonly property real firstVisibleTick: Math.ceil(root.visibleStart() / currentTickInterval) * currentTickInterval
                    readonly property int visibleTickCount: Math.max(0, Math.ceil((root.visibleEnd() - firstVisibleTick) / currentTickInterval) + 1)
                    readonly property real trackTop: 30
                    readonly property real trackBottom: height - 44
                    readonly property real trackHeight: comparisonVisible
                                                        ? (trackBottom - trackTop) / 2
                                                        : trackBottom - trackTop
                    readonly property real originalCenterY: trackTop + trackHeight / 2

                    Repeater {
                        model: waveformContent.visibleTickCount

                        delegate: Item {
                            readonly property real tickTime: waveformContent.firstVisibleTick + index * waveformContent.currentTickInterval
                            x: root.timeToContentX(tickTime)
                            y: 0
                            width: 1
                            height: waveformContent.height

                            Text {
                                x: 4
                                y: 0
                                text: formatSeconds(parent.tickTime)
                                color: textSecondary
                                font.pixelSize: 12
                            }

                            Rectangle {
                                y: 20
                                width: 1
                                height: waveformContent.height - 74
                                color: Qt.rgba(borderColor.r, borderColor.g, borderColor.b, 0.45)
                            }
                        }
                    }

                    Rectangle {
                        visible: root.selectionIsValid()
                        x: root.timeToContentX(selectionStart())
                        y: waveformContent.trackTop
                        width: root.timeToContentX(selectionEnd()) - x
                        height: waveformContent.trackBottom - waveformContent.trackTop
                        color: Qt.rgba(accent.r, accent.g, accent.b, 0.14)
                        border.color: Qt.rgba(accent.r, accent.g, accent.b, 0.5)
                    }

                    Canvas {
                        id: waveformCanvas
                        x: waveformFlickable.contentX
                        y: 0
                        width: waveformFlickable.width
                        height: waveformContent.height
                        antialiasing: false
                        renderTarget: Canvas.Image

                        onXChanged: requestPaint()
                        onWidthChanged: requestPaint()
                        onHeightChanged: requestPaint()

                        onPaint: {
                            var context = getContext("2d")
                            context.clearRect(0, 0, width, height)

                            if (!waveformBridge || waveformBridge.totalBinCount <= 0 || width <= 0)
                                return

                            var duration = root.durationSecs()
                            var rangeStart = root.visibleStart()
                            var rangeEnd = root.visibleEnd()
                            var visibleDuration = Math.max(0.001, rangeEnd - rangeStart)
                            var detailPeaks = waveformBridge.detailPeakValues
                            var useDetail = root.zoomFactor >= 200
                                    && detailPeaks && detailPeaks.length > 0
                                    && waveformBridge.detailStart <= rangeStart + 0.0001
                                    && waveformBridge.detailEnd >= rangeEnd - 0.0001
                            var revision = useDetail ? waveformBridge.detailRevision
                                                     : waveformBridge.peakRevision
                            var peaks = useDetail ? detailPeaks : waveformBridge.peakValues
                            var totalBins = useDetail ? Math.floor(detailPeaks.length / 2)
                                                      : waveformBridge.totalBinCount
                            var dataStart = useDetail ? waveformBridge.detailStart : 0
                            var secondsPerBin = useDetail ? waveformBridge.detailBinDuration
                                                          : duration / Math.max(1, totalBins)
                            if (!peaks || peaks.length < totalBins * 2 || secondsPerBin <= 0)
                                return

                            var firstBin = Math.max(0, Math.floor((rangeStart - dataStart) / secondsPerBin))
                            var lastBin = Math.min(totalBins, Math.ceil((rangeEnd - dataStart) / secondsPerBin))
                            var visibleBins = Math.max(1, lastBin - firstBin)
                            var binsPerPixel = visibleBins / Math.max(1, width)
                            var centerY = waveformContent.originalCenterY
                            var amplitudeHeight = Math.max(1,
                                    waveformContent.trackHeight * 0.42)
                            var loadedCount = useDetail ? totalBins : waveformBridge.loadedBinCount
                            var selectionVisible = root.selectionIsValid()
                            var unloadedColor = Qt.rgba(0.81, 0.83, 0.86, 0.35)
                            var renderedPeaks = []
                            if (binsPerPixel >= 1) {
                                for (var referencePixel = 0;
                                     referencePixel < Math.ceil(width);
                                     ++referencePixel) {
                                    var referenceStart = Math.max(firstBin,
                                        Math.floor(firstBin + referencePixel * binsPerPixel))
                                    var referenceEnd = Math.min(lastBin,
                                        Math.max(referenceStart + 1,
                                            Math.floor(firstBin + (referencePixel + 1)
                                                       * binsPerPixel)))
                                    if (referenceStart >= lastBin)
                                        break
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
                            } else {
                                for (var referenceIndex = firstBin;
                                     referenceIndex < lastBin; ++referenceIndex) {
                                    renderedPeaks.push(Math.max(
                                        Math.abs(peaks[referenceIndex * 2]),
                                        Math.abs(peaks[referenceIndex * 2 + 1])))
                                }
                            }
                            var renderGain = root.normalizedRenderGain(
                                        renderedPeaks, root.zoomFactor,
                                        root.waveformDisplayGain)

                            function barColor(binStart, binEnd) {
                                if (binEnd > loadedCount)
                                    return unloadedColor
                                var sampleTime = dataStart + ((binStart + binEnd) * 0.5) * secondsPerBin
                                return selectionVisible && sampleTime >= root.selectionStart()
                                        && sampleTime <= root.selectionEnd()
                                        ? root.accent : root.waveformColor
                            }

                            function drawBar(x, barWidth, minAmplitude, maxAmplitude, color) {
                                var safeMin = Math.max(-1, Math.min(0,
                                                                   minAmplitude * renderGain))
                                var safeMax = Math.max(0, Math.min(1,
                                                                  maxAmplitude * renderGain))
                                var top = centerY - safeMax * amplitudeHeight
                                var bottom = centerY - safeMin * amplitudeHeight
                                context.fillStyle = color
                                context.fillRect(x, top, Math.max(1, barWidth), Math.max(2, bottom - top))
                            }

                            if (binsPerPixel >= 1) {
                                var pixelCount = Math.ceil(width)
                                for (var pixel = 0; pixel < pixelCount; ++pixel) {
                                    var binStart = Math.max(firstBin, Math.floor(firstBin + pixel * binsPerPixel))
                                    var binEnd = Math.min(lastBin, Math.max(binStart + 1,
                                                                           Math.floor(firstBin + (pixel + 1) * binsPerPixel)))
                                    if (binStart >= lastBin)
                                        break
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
                                    var pixelX = ((dataStart + binStart * secondsPerBin - rangeStart)
                                                  / visibleDuration) * width
                                    var aggregateWidth = Math.max(1,
                                            (binEnd - binStart) * secondsPerBin
                                            / visibleDuration * width)
                                    var aggregateColor = barColor(binStart, binEnd)
                                    drawBar(pixelX, aggregateWidth, rmsMinimum,
                                            rmsMaximum, aggregateColor)
                                }
                            } else {
                                var pixelsPerBin = secondsPerBin / visibleDuration * width
                                if (root.zoomFactor >= 200) {
                                    for (var index = firstBin; index < lastBin; ++index) {
                                        var nextIndex = Math.min(lastBin - 1, index + 1)
                                        var leftX = ((dataStart + index * secondsPerBin - rangeStart)
                                                     / visibleDuration) * width
                                        var rightX = ((dataStart + (index + 1) * secondsPerBin
                                                       - rangeStart) / visibleDuration) * width
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
                                        context.fillStyle = barColor(index, index + 1)
                                        context.fill()
                                    }
                                } else {
                                    var barWidth = Math.max(1,
                                            Math.min(6, pixelsPerBin * 0.72))
                                    for (var index = firstBin; index < lastBin; ++index) {
                                        var localX = ((dataStart + index * secondsPerBin - rangeStart)
                                                      / visibleDuration) * width
                                                + (pixelsPerBin - barWidth) * 0.5
                                        drawBar(localX, barWidth, peaks[index * 2],
                                                peaks[index * 2 + 1],
                                                barColor(index, index + 1))
                                    }
                                }
                            }

                            // Keep a dependency on the revision even though data is read as a sequence.
                            if (revision < 0)
                                context.clearRect(0, 0, 0, 0)
                        }
                    }

                    Rectangle {
                        id: waveformMidline
                        x: 0
                        y: waveformContent.originalCenterY
                        width: waveformContent.width
                        height: 1
                        color: borderColor
                    }

                    Rectangle {
                        visible: root.comparisonVisible
                        x: 0
                        y: waveformContent.trackTop + waveformContent.trackHeight
                        width: waveformContent.width
                        height: 1
                        color: root.borderColor
                    }

                    Text {
                        visible: root.comparisonVisible
                        x: waveformFlickable.contentX + 8
                        y: waveformContent.trackTop + 5
                        text: "原音"
                        color: root.textSecondary
                        font.pixelSize: 11
                        font.bold: true
                    }

                    RecordingWaveformTrack {
                        id: recordingTrack
                        z: 4
                        visible: root.comparisonVisible
                        x: waveformFlickable.contentX
                        y: waveformContent.trackTop + waveformContent.trackHeight
                        width: waveformFlickable.width
                        height: waveformContent.trackHeight
                        recordingBridge: root.recordingBridge
                        visibleStart: root.visibleStart()
                        visibleEnd: root.visibleEnd()
                        displayGain: root.waveformDisplayGain
                        trainingPlaybackActive: root.recordingTrainingActive
                        trainingPlaybackPlaying: root.recordingPlaybackPlaying
                        trainingPlaybackPosition: root.recordingPlaybackPosition
                        borderColor: root.borderColor
                        menuBackgroundColor: root.panelBg
                        menuHoverColor: root.accentBg
                        textColor: root.textPrimary
                        highlightColor: root.accent
                        onSeekRequested: function(positionSecs) {
                            if (root.recordingTrainingActive && root.recordingBridge) {
                                var localPosition = positionSecs
                                        - root.recordingBridge.targetStart
                                root.recordingPlaybackSeekRequested(root.clamp(
                                    localPosition, 0,
                                    root.recordingBridge.recordingDuration))
                            } else {
                                root.playbackPositionRequested(positionSecs)
                            }
                        }
                        onTrainingPlaybackToggleRequested:
                            root.recordingTrainingToggleRequested()
                        onResetAlignmentRequested: {
                            if (root.recordingBridge)
                                root.recordingBridge.resetAlignment()
                        }
                        onDeleteRequested: root.recordingDeleteRequested()
                    }

                    Rectangle {
                        id: selectionStartMarker
                        z: 6
                        visible: waveformBridge && waveformBridge.hasSelectionStart
                        x: root.clamp(root.timeToContentX(waveformBridge ? waveformBridge.selectionStart : 0) - width / 2, 0, waveformContent.width - width)
                        y: waveformContent.trackTop
                        width: 3
                        height: waveformContent.trackBottom - waveformContent.trackTop
                        color: root.selectionStartMarkerColor

                        Rectangle {
                            x: -7
                            y: 0
                            width: 18
                            height: 18
                            radius: 5
                            color: root.selectionStartMarkerColor

                            Text {
                                anchors.centerIn: parent
                                text: "A"
                                color: "#ffffff"
                                font.pixelSize: 11
                                font.bold: true
                            }
                        }

                        MouseArea {
                            id: selectionStartDragArea
                            x: -9
                            y: 0
                            width: 21
                            height: parent.height
                            acceptedButtons: Qt.LeftButton
                            preventStealing: true
                            cursorShape: Qt.SizeHorCursor

                            function updateMarker(mouse) {
                                if (!root.waveformBridge)
                                    return
                                var point = selectionStartDragArea.mapToItem(waveformContent,
                                                                             mouse.x,
                                                                             mouse.y)
                                root.waveformBridge.markSelectionStart(root.contentXToTime(point.x))
                            }

                            onPressed: function(mouse) {
                                root.followPlayback = false
                                root.selectionAdjustmentStarted()
                                updateMarker(mouse)
                            }
                            onPositionChanged: function(mouse) {
                                if (pressed)
                                    updateMarker(mouse)
                            }
                            onReleased: root.selectionChangeCommitted(
                                            root.selectionStart(), root.selectionEnd())
                        }
                    }

                    Rectangle {
                        id: selectionEndMarker
                        z: 6
                        visible: waveformBridge && waveformBridge.hasSelectionEnd
                        x: root.clamp(root.timeToContentX(waveformBridge ? waveformBridge.selectionEnd : 0) - width / 2, 0, waveformContent.width - width)
                        y: waveformContent.trackTop
                        width: 3
                        height: waveformContent.trackBottom - waveformContent.trackTop
                        color: root.selectionEndMarkerColor

                        Rectangle {
                            x: -7
                            y: 0
                            width: 18
                            height: 18
                            radius: 5
                            color: root.selectionEndMarkerColor

                            Text {
                                anchors.centerIn: parent
                                text: "B"
                                color: "#ffffff"
                                font.pixelSize: 11
                                font.bold: true
                            }
                        }

                        MouseArea {
                            id: selectionEndDragArea
                            x: -9
                            y: 0
                            width: 21
                            height: parent.height
                            acceptedButtons: Qt.LeftButton
                            preventStealing: true
                            cursorShape: Qt.SizeHorCursor

                            function updateMarker(mouse) {
                                if (!root.waveformBridge)
                                    return
                                var point = selectionEndDragArea.mapToItem(waveformContent,
                                                                           mouse.x,
                                                                           mouse.y)
                                root.waveformBridge.markSelectionEnd(root.contentXToTime(point.x))
                            }

                            onPressed: function(mouse) {
                                root.followPlayback = false
                                root.selectionAdjustmentStarted()
                                updateMarker(mouse)
                            }
                            onPositionChanged: function(mouse) {
                                if (pressed)
                                    updateMarker(mouse)
                            }
                            onReleased: root.selectionChangeCommitted(
                                            root.selectionStart(), root.selectionEnd())
                        }
                    }

                    Rectangle {
                        z: 5
                        visible: !root.recordingTrainingActive
                        x: root.timeToContentX(waveformBridge ? waveformBridge.currentPosition : 0)
                        y: 22
                        width: 3
                        height: waveformContent.height - 64
                        color: root.playheadColor
                    }

                    Rectangle {
                        z: 7
                        visible: !root.recordingTrainingActive
                        x: root.clamp(root.timeToContentX(waveformBridge ? waveformBridge.currentPosition : 0) - 22, 0, waveformContent.width - width)
                        y: 12
                        width: 56
                        height: 24
                        radius: 8
                        color: root.playheadColor
                        border.color: root.accentBg
                        border.width: 1

                        Text {
                            anchors.centerIn: parent
                            text: formatSeconds(waveformBridge ? waveformBridge.currentPosition : 0)
                            color: root.playheadTextColor
                            font.pixelSize: 12
                        }
                    }

                    MouseArea {
                        z: 1
                        anchors.fill: parent
                        preventStealing: false
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        onClicked: function(mouse) {
                            if (mouse.button === Qt.RightButton) {
                                var trackTop = waveformContent.trackTop
                                var trackBottom = trackTop + waveformContent.trackHeight
                                root.contextMenuOnOriginalTrack = mouse.y >= trackTop
                                        && mouse.y < trackBottom
                                waveformContextMenu.popup()
                                return
                            }
                            root.playbackPositionRequested(root.contentXToTime(mouse.x))
                        }
                    }
                }
            }
        }

        GridLayout {
            Layout.fillWidth: true
            columns: root.compactMode ? 1 : 2
            columnSpacing: 8
            rowSpacing: 6

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Rectangle {
                    Layout.fillWidth: true
                    Layout.minimumWidth: 100
                    Layout.maximumWidth: 170
                    Layout.preferredHeight: 34
                    radius: 8
                    color: elevatedBg
                    border.color: borderColor

                    Text {
                        anchors.fill: parent
                        anchors.margins: 8
                        text: root.selectionLengthLabel()
                        color: textPrimary
                        font.pixelSize: 13
                        verticalAlignment: Text.AlignVCenter
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                    }
                }

                Text {
                    Layout.fillWidth: true
                    visible: waveformBridge
                             && waveformBridge.hasSelectionStart
                             && waveformBridge.hasSelectionEnd
                             && !root.selectionIsValid()
                    text: "A 必须早于 B"
                    color: root.selectionEndMarkerColor
                    font.pixelSize: 12
                    elide: Text.ElideRight
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignRight
                spacing: 8

                Item {
                    Layout.fillWidth: true
                }

                Button {
                    visible: root.recordingBridge && root.recordingBridge.hasVideo
                    Layout.preferredWidth: 124
                    enabled: root.recordingBridge
                             && root.recordingBridge.hasTarget
                             && (!root.recordingBridge.isProcessing
                                 || root.recordingBridge.isLoadingWaveform)
                    text: root.recordingBridge && root.recordingBridge.isRecording
                          ? "■  停止 " + root.formatRecordingTime(
                                root.recordingBridge.recordingElapsed)
                          : (root.recordingBridge
                             && root.recordingBridge.isProcessing
                             && !root.recordingBridge.isLoadingWaveform
                             ? "处理中" : "●  录音")
                    onClicked: {
                        if (root.recordingBridge.isRecording)
                            root.recordingStopRequested()
                        else
                            root.recordingStartRequested()
                    }
                    ToolTip.visible: hovered
                    ToolTip.text: root.recordingBridge
                                  ? root.recordingBridge.statusMessage : ""
                }

                ThemedToolButton {
                    Layout.preferredWidth: 92
                    text: waveformBridge && waveformBridge.hasSelectionStart
                          ? "A " + formatSeconds(waveformBridge.selectionStart)
                          : "设置 A"
                    enabled: waveformBridge
                    panelColor: root.panelBg
                    borderColor: root.borderColor
                    textColor: root.textPrimary
                    disabledTextColor: root.textSecondary
                    accentColor: root.accent
                    accentBackgroundColor: root.accentBg
                    onClicked: {
                        if (waveformBridge) {
                            root.selectionAdjustmentStarted()
                            waveformBridge.markSelectionStart(waveformBridge.currentPosition)
                            root.selectionChangeCommitted(root.selectionStart(),
                                                          root.selectionEnd())
                        }
                    }
                }

                ThemedToolButton {
                    Layout.preferredWidth: 92
                    text: waveformBridge && waveformBridge.hasSelectionEnd
                          ? "B " + formatSeconds(waveformBridge.selectionEnd)
                          : "设置 B"
                    enabled: waveformBridge
                    panelColor: root.panelBg
                    borderColor: root.borderColor
                    textColor: root.textPrimary
                    disabledTextColor: root.textSecondary
                    accentColor: root.accent
                    accentBackgroundColor: root.accentBg
                    onClicked: {
                        if (waveformBridge) {
                            root.selectionAdjustmentStarted()
                            waveformBridge.markSelectionEnd(waveformBridge.currentPosition)
                            root.selectionChangeCommitted(root.selectionStart(),
                                                          root.selectionEnd())
                        }
                    }
                }
            }
        }
    }
}
