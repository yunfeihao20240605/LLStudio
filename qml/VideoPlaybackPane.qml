import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import com.yfhao.els.mpv 1.0

Rectangle {
    id: root

    property color panelBg: "#ffffff"
    property color elevatedBg: "#fafbfc"
    property color borderColor: "#d0d7de"
    property color textPrimary: "#1f2329"
    property color textSecondary: "#6b7280"
    property color accent: "#2f6fed"
    property color accentBg: "#eaf1fe"
    property bool darkTheme: false
    readonly property url emptyBrandSource: darkTheme
                                            ? "qrc:/qt/qml/com/yfhao/els/app/llstudio-brand.png"
                                            : "qrc:/qt/qml/com/yfhao/els/app/llstudio-brand-light.png"
    property var mediaBridge
    property var subtitleBridge
    property var waveformBridge
    property bool fullScreenMode: false
    property bool subtitlesVisible: true
    property bool audioMode: false

    signal manualSeekRequested(real positionSecs)
    signal normalPlaybackToggleRequested()
    signal videoLoadStarted()
    signal videoLoadFailed(string message)
    signal videoLoaded(string path, real durationSecs)
    signal fullScreenToggleRequested()

    function formatSeconds(totalSeconds) {
        var safeSeconds = Math.max(0, Math.floor(totalSeconds))
        var hours = Math.floor(safeSeconds / 3600)
        var minutes = Math.floor((safeSeconds % 3600) / 60)
        var seconds = safeSeconds % 60
        if (hours > 0)
            return hours + ":" + (minutes < 10 ? "0" + minutes : minutes) + ":" + (seconds < 10 ? "0" + seconds : seconds)
        return (minutes < 10 ? "0" + minutes : minutes) + ":" + (seconds < 10 ? "0" + seconds : seconds)
    }

    function playbackRateIndex(playbackRate) {
        var rates = [0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0]
        for (var index = 0; index < rates.length; ++index) {
            if (Math.abs(rates[index] - playbackRate) < 0.001)
                return index
        }
        return 2
    }

    function activeSubtitleText() {
        if (!subtitleBridge || subtitleBridge.activeCueIndex < 0)
            return ""
        var original = subtitleBridge.activeOriginalText
        var translated = subtitleBridge.activeTranslatedText
        return translated.length > 0 ? original + "\n" + translated : original
    }

    function toggleSubtitles() {
        subtitlesVisible = !subtitlesVisible
    }

    function syncPlaybackDependentPanels() {
        if (subtitleBridge)
            subtitleBridge.syncPlaybackPosition(mediaBridge ? mediaBridge.currentPosition : 0)
        if (waveformBridge)
            waveformBridge.syncPlaybackPosition(mediaBridge ? mediaBridge.currentPosition : 0)
    }

    function seekToPosition(positionSecs) {
        if (!mediaBridge || mediaBridge.duration <= 0)
            return false

        var targetPosition = Math.max(0, Math.min(mediaBridge.duration, positionSecs))
        if (!mediaBridge.seek(targetPosition))
            return false

        syncPlaybackDependentPanels()
        return true
    }

    function loadVideoAndRelatedAssets(path) {
        loadMediaAndRelatedAssets(path, false)
    }

    function loadMediaAndRelatedAssets(path, isAudio) {
        if (!mediaBridge) {
            root.videoLoadFailed("媒体播放器不可用")
            return
        }
        if (!path || path.trim().length === 0)
            return

        root.videoLoadStarted()
        if (!mediaBridge.loadVideoPath(path)) {
            root.videoLoadFailed(mediaBridge.statusMessage)
            return
        }
        root.audioMode = isAudio

        if (subtitleBridge)
            subtitleBridge.loadForVideoPath(path)
        if (waveformBridge)
            waveformBridge.loadForVideoPath(path, mediaBridge.duration)

        root.videoLoaded(path, mediaBridge.duration)

        syncPlaybackDependentPanels()
    }

    function openVideo() {
        var pickedPath = mediaBridge ? mediaBridge.pickVideoFile() : ""
        if (pickedPath.length > 0)
            loadVideoAndRelatedAssets(pickedPath)
    }

    function openAudio() {
        var pickedPath = mediaBridge ? mediaBridge.pickAudioFile() : ""
        if (pickedPath.length > 0)
            loadMediaAndRelatedAssets(pickedPath, true)
    }

    radius: fullScreenMode ? 0 : 16
    color: panelBg
    border.color: borderColor
    border.width: fullScreenMode ? 0 : 1

    Timer {
        interval: mediaBridge && mediaBridge.preparingInitialFrame ? 33 : 120
        repeat: true
        running: mediaBridge
                 && (mediaBridge.isPlaying
                     || mediaBridge.preparingInitialFrame)
        onTriggered: {
            mediaBridge.tick()
            syncPlaybackDependentPanels()
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: root.fullScreenMode ? 0 : 12
        spacing: root.fullScreenMode ? 0 : 10

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: root.fullScreenMode ? 0 : 14
            color: root.fullScreenMode ? "#000000" : elevatedBg
            border.color: root.fullScreenMode ? "transparent" : borderColor
            border.width: root.fullScreenMode ? 0 : 1

            Item {
                anchors.fill: parent
                anchors.margins: root.fullScreenMode ? 0 : 8

                MpvVideoItem {
                    id: videoSurface
                    anchors.fill: parent
                    mpvHandleToken: mediaBridge ? mediaBridge.mpvHandleToken : "0"
                    visible: !root.audioMode
                }

                Rectangle {
                    z: 4
                    anchors.fill: parent
                    visible: root.audioMode && mediaBridge && mediaBridge.loadedPath.length > 0
                    color: root.elevatedBg

                    Column {
                        anchors.centerIn: parent
                        spacing: 12

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: "♫"
                            color: root.accent
                            font.pixelSize: 58
                        }

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: mediaBridge ? mediaBridge.mediaSummary : "音频"
                            color: root.textPrimary
                            font.pixelSize: 16
                            elide: Text.ElideMiddle
                            width: Math.min(parent.parent.width * 0.8, 420)
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: "音频播放模式"
                            color: root.textSecondary
                            font.pixelSize: 13
                        }
                    }
                }

                Image {
                    id: videoPlaceholder
                    z: 5
                    anchors.fill: parent
                    source: root.emptyBrandSource
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    visible: mediaBridge
                             && mediaBridge.loadedPath.length > 0
                             && !mediaBridge.isPlaying
                             && !mediaBridge.preparingInitialFrame
                             && mediaBridge.currentPosition < 0.05
                    opacity: visible ? 1.0 : 0.0
                    Behavior on opacity {
                        NumberAnimation { duration: 160 }
                    }
                }

                Rectangle {
                    z: 6
                    anchors.fill: parent
                    visible: !mediaBridge || mediaBridge.loadedPath.length === 0
                    radius: 12
                    color: root.elevatedBg
                    clip: true

                    Image {
                        anchors.fill: parent
                        source: root.emptyBrandSource
                        fillMode: Image.PreserveAspectCrop
                        asynchronous: true
                    }
                }

                Rectangle {
                    z: 10
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 12
                    width: Math.min(parent.width * 0.88,
                                    Math.max(120, playbackSubtitle.implicitWidth + 28))
                    height: playbackSubtitle.implicitHeight + 18
                    radius: 7
                    color: "#b3000000"
                    visible: root.subtitlesVisible && root.activeSubtitleText().length > 0

                    Text {
                        id: playbackSubtitle
                        anchors.centerIn: parent
                        width: parent.width - 24
                        text: root.activeSubtitleText()
                        color: "#ffffff"
                        font.pixelSize: 18
                        font.bold: true
                        horizontalAlignment: Text.AlignHCenter
                        wrapMode: Text.Wrap
                    }
                }

                Rectangle {
                    id: fullScreenExitButton
                    z: 20
                    anchors.top: parent.top
                    anchors.right: parent.right
                    anchors.topMargin: 16
                    anchors.rightMargin: 16
                    width: 44
                    height: 44
                    radius: 9
                    visible: root.fullScreenMode
                    color: "#99000000"
                    border.color: "#66ffffff"

                    Text {
                        anchors.centerIn: parent
                        text: "⛶"
                        color: "#ffffff"
                        font.pixelSize: 20
                    }

                    MouseArea {
                        anchors.fill: parent
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.fullScreenToggleRequested()
                    }
                }

                Rectangle {
                    z: 20
                    anchors.top: parent.top
                    anchors.right: fullScreenExitButton.left
                    anchors.topMargin: 16
                    anchors.rightMargin: 8
                    width: 44
                    height: 44
                    radius: 9
                    visible: root.fullScreenMode
                    color: root.subtitlesVisible
                           ? Qt.rgba(root.accent.r, root.accent.g, root.accent.b, 0.9)
                           : "#99000000"
                    border.color: root.subtitlesVisible ? root.accent : "#66ffffff"

                    Text {
                        anchors.centerIn: parent
                        text: "CC"
                        color: "#ffffff"
                        font.pixelSize: 13
                        font.bold: true
                    }

                    MouseArea {
                        id: fullScreenSubtitleMouseArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.toggleSubtitles()
                    }

                    ToolTip.visible: fullScreenSubtitleMouseArea.containsMouse
                    ToolTip.text: root.subtitlesVisible ? "隐藏字幕" : "显示字幕"
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: !root.fullScreenMode
            spacing: 12

            Row {
                spacing: 10

                Rectangle {
                    width: 36
                    height: 36
                    radius: 10
                    color: accent

                    Text {
                        anchors.centerIn: parent
                        text: mediaBridge && mediaBridge.isPlaying ? "||" : "▶"
                        color: "#ffffff"
                        font.pixelSize: 16
                    }

                    MouseArea {
                        anchors.fill: parent
                        enabled: mediaBridge && mediaBridge.loadedPath.length > 0
                        onClicked: root.normalPlaybackToggleRequested()
                    }
                }

                Rectangle {
                    width: 36
                    height: 36
                    radius: 10
                    color: elevatedBg
                    border.color: borderColor

                    Text {
                        anchors.centerIn: parent
                        text: "▶▶"
                        color: textPrimary
                        font.pixelSize: 13
                    }
                }

                Rectangle {
                    width: 36
                    height: 36
                    radius: 10
                    color: elevatedBg
                    border.color: borderColor

                    Text {
                        anchors.centerIn: parent
                        text: "▮▶"
                        color: textPrimary
                        font.pixelSize: 12
                    }
                }
            }

            Text {
                text: mediaBridge ? formatSeconds(mediaBridge.currentPosition) + " / " + formatSeconds(mediaBridge.duration) : "00:00 / 00:00"
                color: textSecondary
                font.pixelSize: 14
            }

            ThemedComboBox {
                Layout.preferredWidth: 84
                model: ["0.25x", "0.5x", "0.75x", "1.0x", "1.25x", "1.5x", "2.0x"]
                currentIndex: root.playbackRateIndex(mediaBridge ? mediaBridge.playbackRate : 1.0)
                panelColor: root.panelBg
                elevatedColor: root.elevatedBg
                borderColor: root.borderColor
                textColor: root.textPrimary
                disabledTextColor: root.textSecondary
                accentColor: root.accent
                accentBackgroundColor: root.accentBg
                onActivated: {
                    if (mediaBridge)
                        mediaBridge.applyPlaybackRate(parseFloat(currentText))
                }
            }

            Text {
                text: "🔊"
                color: textPrimary
                font.pixelSize: 18
            }

            ThemedSlider {
                Layout.preferredWidth: 120
                from: 0
                to: 100
                value: 70
                panelColor: root.panelBg
                trackColor: root.borderColor
                accentColor: root.accent
            }

            Item {
                Layout.fillWidth: true
            }

            Repeater {
                model: [
                    { icon: "CC", action: "subtitle" },
                    { icon: "⛶", action: "fullscreen" }
                ]

                delegate: Rectangle {
                    width: 36
                    height: 36
                    radius: 10
                    color: modelData.action === "subtitle" && root.subtitlesVisible
                           ? root.accentBg : root.elevatedBg
                    border.color: modelData.action === "subtitle" && root.subtitlesVisible
                                  ? root.accent : root.borderColor

                    Text {
                        anchors.centerIn: parent
                        text: modelData.icon
                        color: modelData.action === "subtitle" && root.subtitlesVisible
                               ? root.accent : root.textPrimary
                        font.pixelSize: modelData.action === "subtitle" ? 12 : 16
                        font.bold: modelData.action === "subtitle"
                    }

                    MouseArea {
                        id: playbackToolMouseArea
                        anchors.fill: parent
                        hoverEnabled: true
                        enabled: modelData.action === "subtitle"
                                 || (modelData.action === "fullscreen"
                                     && root.mediaBridge
                                     && root.mediaBridge.loadedPath.length > 0)
                        cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                        onClicked: {
                            if (modelData.action === "subtitle")
                                root.toggleSubtitles()
                            else if (modelData.action === "fullscreen")
                                root.fullScreenToggleRequested()
                        }
                    }

                    ToolTip.visible: playbackToolMouseArea.containsMouse
                                         && playbackToolMouseArea.enabled
                    ToolTip.text: modelData.action === "subtitle"
                                  ? (root.subtitlesVisible ? "隐藏字幕" : "显示字幕")
                                  : (modelData.action === "fullscreen" ? "全屏播放" : "")
                }
            }
        }

        ThemedSlider {
            Layout.fillWidth: true
            visible: !root.fullScreenMode
            from: 0
            to: mediaBridge && mediaBridge.duration > 0 ? mediaBridge.duration : 1
            value: mediaBridge ? mediaBridge.currentPosition : 0
            enabled: mediaBridge && mediaBridge.duration > 0
            panelColor: root.panelBg
            trackColor: root.borderColor
            accentColor: root.accent

            onMoved: {
                root.manualSeekRequested(value)
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: !root.fullScreenMode

            Text {
                Layout.fillWidth: true
                text: mediaBridge
                      ? mediaBridge.mediaSummary
                        + (mediaBridge.duration > 0
                           ? " | 总时长: " + root.formatSeconds(mediaBridge.duration)
                           : "")
                      : "Container: mp4 | Video: h264 | Audio: aac"
                color: textPrimary
                font.pixelSize: 13
                elide: Text.ElideRight
            }
        }
    }
}
