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
    property bool canBeginNextSegment: false
    property real zoomFactor: 1.0
    property real minimumZoom: 1.0
    property real maximumZoom: 1000.0
    property real waveformDisplayGain: 1.8
    property bool followPlayback: true
    readonly property real waveformBackgroundLuminance: elevatedBg.r * 0.2126 + elevatedBg.g * 0.7152 + elevatedBg.b * 0.0722
    readonly property color playheadColor: waveformBackgroundLuminance > 0.55 ? "#111827" : "#ffffff"
    readonly property color playheadTextColor: waveformBackgroundLuminance > 0.55 ? "#ffffff" : "#111827"
    readonly property color selectionStartMarkerColor: waveformBackgroundLuminance > 0.55 ? "#15803d" : "#4ade80"
    readonly property color selectionEndMarkerColor: waveformBackgroundLuminance > 0.55 ? "#c2410c" : "#fb923c"
    readonly property color waveformColor: waveformBackgroundLuminance > 0.55 ? "#cfd4dc" : "#6b7280"

    signal playbackPositionRequested(real positionSecs)
    signal selectionCleared()
    signal nextSegmentRequested()

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

    function selectionLengthLabel() {
        return selectionIsValid()
                ? "选区长度：" + formatSeconds(selectionEnd() - selectionStart())
                : "选区长度：--"
    }

    function clamp(value, minimum, maximum) {
        return Math.max(minimum, Math.min(maximum, value))
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

    Menu {
        id: waveformContextMenu

        MenuItem {
            text: "从当前片段结尾开始下一片段"
            enabled: root.canBeginNextSegment
            onTriggered: root.nextSegmentRequested()
        }

        MenuSeparator {}

        MenuItem {
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

            Text {
                text: "波形视图"
                color: textPrimary
                font.pixelSize: 16
                font.bold: true
            }

            Button {
                text: "−"
                enabled: root.zoomFactor > root.minimumZoom
                onClicked: root.setZoom(root.zoomFactor - 0.5)
            }

            Slider {
                Layout.preferredWidth: 150
                from: root.minimumZoom
                to: root.maximumZoom
                stepSize: 0.5
                value: root.zoomFactor
                onMoved: root.setZoom(value)
            }

            Button {
                text: "+"
                enabled: root.zoomFactor < root.maximumZoom
                onClicked: root.setZoom(root.zoomFactor + 0.5)
            }

            Text {
                text: root.zoomFactor.toFixed(1) + "x"
                color: textSecondary
                font.pixelSize: 13
                Layout.preferredWidth: 38
            }

            Button {
                text: root.followPlayback ? "自动跟随" : "恢复跟随"
                onClicked: {
                    root.followPlayback = true
                    root.centerOnTime(waveformBridge ? waveformBridge.currentPosition : 0)
                }
            }

            CheckBox {
                id: showSelectionCheckBox
                text: "显示选区"
                checked: true
                onCheckedChanged: root.requestWaveformPaint()
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                text: "总时长：" + formatSeconds(durationSecs())
                color: textSecondary
                font.pixelSize: 13
            }

            Text {
                text: waveformBridge ? ("已加载：" + waveformBridge.loadedBinCount + "/" + waveformBridge.totalBinCount + " bins") : ""
                color: textSecondary
                font.pixelSize: 13
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
                        visible: showSelectionCheckBox.checked && root.selectionIsValid()
                        x: root.timeToContentX(selectionStart())
                        y: 26
                        width: root.timeToContentX(selectionEnd()) - x
                        height: waveformContent.height - 78
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
                            var centerY = height / 2
                            var amplitudeHeight = Math.max(1, height - 92) * 0.5
                            var loadedCount = useDetail ? totalBins : waveformBridge.loadedBinCount
                            var selectionVisible = showSelectionCheckBox.checked
                                    && root.selectionIsValid()
                            var unloadedColor = Qt.rgba(0.81, 0.83, 0.86, 0.35)

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
                                                                   minAmplitude * root.waveformDisplayGain))
                                var safeMax = Math.max(0, Math.min(1,
                                                                  maxAmplitude * root.waveformDisplayGain))
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
                                    var minAmplitude = 0
                                    var maxAmplitude = 0
                                    for (var bin = binStart; bin < binEnd; ++bin) {
                                        minAmplitude = Math.min(minAmplitude, peaks[bin * 2])
                                        maxAmplitude = Math.max(maxAmplitude, peaks[bin * 2 + 1])
                                    }
                                    var pixelX = ((dataStart + binStart * secondsPerBin - rangeStart)
                                                  / visibleDuration) * width
                                    drawBar(pixelX, Math.max(1, (binEnd - binStart) * secondsPerBin
                                                               / visibleDuration * width), minAmplitude, maxAmplitude,
                                            barColor(binStart, binEnd))
                                }
                            } else {
                                var pixelsPerBin = secondsPerBin / visibleDuration * width
                                var barWidth = Math.max(1, Math.min(6, pixelsPerBin * 0.72))
                                for (var index = firstBin; index < lastBin; ++index) {
                                    var localX = ((dataStart + index * secondsPerBin - rangeStart)
                                                  / visibleDuration) * width
                                            + (pixelsPerBin - barWidth) * 0.5
                                    drawBar(localX, barWidth, peaks[index * 2], peaks[index * 2 + 1],
                                            barColor(index, index + 1))
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
                        y: waveformContent.height / 2
                        width: waveformContent.width
                        height: 1
                        color: borderColor
                    }

                    Rectangle {
                        id: selectionStartMarker
                        z: 3
                        visible: showSelectionCheckBox.checked && waveformBridge && waveformBridge.hasSelectionStart
                        x: root.clamp(root.timeToContentX(waveformBridge ? waveformBridge.selectionStart : 0) - width / 2, 0, waveformContent.width - width)
                        y: 26
                        width: 3
                        height: waveformContent.height - 78
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
                                updateMarker(mouse)
                            }
                            onPositionChanged: function(mouse) {
                                if (pressed)
                                    updateMarker(mouse)
                            }
                        }
                    }

                    Rectangle {
                        id: selectionEndMarker
                        z: 3
                        visible: showSelectionCheckBox.checked && waveformBridge && waveformBridge.hasSelectionEnd
                        x: root.clamp(root.timeToContentX(waveformBridge ? waveformBridge.selectionEnd : 0) - width / 2, 0, waveformContent.width - width)
                        y: 26
                        width: 3
                        height: waveformContent.height - 78
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
                                updateMarker(mouse)
                            }
                            onPositionChanged: function(mouse) {
                                if (pressed)
                                    updateMarker(mouse)
                            }
                        }
                    }

                    Rectangle {
                        x: root.timeToContentX(waveformBridge ? waveformBridge.currentPosition : 0)
                        y: 22
                        width: 3
                        height: waveformContent.height - 64
                        color: root.playheadColor
                    }

                    Rectangle {
                        x: root.clamp(root.timeToContentX(waveformBridge ? waveformBridge.currentPosition : 0) - 22, 0, waveformContent.width - width)
                        y: 12
                        width: 56
                        height: 24
                        radius: 8
                        color: root.playheadColor
                        border.color: root.waveformBackgroundLuminance > 0.55 ? "#ffffff" : "#111827"
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
                                waveformContextMenu.popup()
                                return
                            }
                            root.playbackPositionRequested(root.contentXToTime(mouse.x))
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true

            Repeater {
                model: [
                    { label: root.selectionLengthLabel(), width: 170 }
                ]

                delegate: Rectangle {
                    Layout.preferredWidth: modelData.width
                    Layout.preferredHeight: 34
                    radius: 8
                    color: elevatedBg
                    border.color: borderColor

                    Text {
                        anchors.centerIn: parent
                        text: modelData.label
                        color: textPrimary
                        font.pixelSize: 13
                    }
                }
            }

            Text {
                visible: waveformBridge
                         && waveformBridge.hasSelectionStart
                         && waveformBridge.hasSelectionEnd
                         && !root.selectionIsValid()
                text: "A 必须早于 B"
                color: root.selectionEndMarkerColor
                font.pixelSize: 12
            }

            Item {
                Layout.fillWidth: true
            }

            Button {
                Layout.preferredWidth: 92
                text: waveformBridge && waveformBridge.hasSelectionStart
                      ? "A " + formatSeconds(waveformBridge.selectionStart)
                      : "设置 A"
                enabled: waveformBridge
                onClicked: {
                    if (waveformBridge)
                        waveformBridge.markSelectionStart(waveformBridge.currentPosition)
                }
            }

            Button {
                Layout.preferredWidth: 92
                text: waveformBridge && waveformBridge.hasSelectionEnd
                      ? "B " + formatSeconds(waveformBridge.selectionEnd)
                      : "设置 B"
                enabled: waveformBridge
                onClicked: {
                    if (waveformBridge)
                        waveformBridge.markSelectionEnd(waveformBridge.currentPosition)
                }
            }

            Button {
                text: "同步字幕选区"
                enabled: subtitleBridge && subtitleBridge.activeCueEnd > subtitleBridge.activeCueStart
                onClicked: {
                    if (waveformBridge && subtitleBridge)
                        waveformBridge.setSelectionRange(subtitleBridge.activeCueStart, subtitleBridge.activeCueEnd)
                }
            }

        }

        Text {
            Layout.fillWidth: true
            visible: waveformBridge && waveformBridge.statusMessage.length > 0
            text: waveformBridge ? waveformBridge.statusMessage : ""
            color: textSecondary
            font.pixelSize: 12
            elide: Text.ElideRight
        }
    }
}
