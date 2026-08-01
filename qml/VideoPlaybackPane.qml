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
    property var mediaBridge
    property var subtitleBridge
    property var waveformBridge
    property bool fullScreenMode: false
    property bool subtitlesVisible: true

    signal manualSeekRequested()
    signal normalPlaybackToggleRequested()
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
        if (!mediaBridge || !path || path.trim().length === 0)
            return

        if (!mediaBridge.loadVideoPath(path))
            return

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

    radius: fullScreenMode ? 0 : 16
    color: panelBg
    border.color: borderColor
    border.width: fullScreenMode ? 0 : 1

    Timer {
        interval: 120
        repeat: true
        running: mediaBridge && mediaBridge.isPlaying
        onTriggered: {
            mediaBridge.tick()
            syncPlaybackDependentPanels()
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: root.fullScreenMode ? 0 : 12
        spacing: root.fullScreenMode ? 0 : 10

        RowLayout {
            Layout.fillWidth: true
            visible: !root.fullScreenMode
            spacing: 8

            Rectangle {
                Layout.preferredWidth: 170
                Layout.preferredHeight: 34
                radius: 9
                color: elevatedBg
                border.color: borderColor

                Row {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    spacing: 10

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: mediaBridge && mediaBridge.loadedPath.length > 0 ? mediaBridge.loadedPath.split("/").slice(-1)[0] : "TED_AI_未来.mp4"
                        color: textPrimary
                        font.pixelSize: 14
                        elide: Text.ElideRight
                        width: 120
                    }

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: "×"
                        color: textSecondary
                        font.pixelSize: 16
                    }
                }
            }

            Rectangle {
                Layout.preferredWidth: 34
                Layout.preferredHeight: 34
                radius: 9
                color: elevatedBg
                border.color: borderColor

                Text {
                    anchors.centerIn: parent
                    text: "+"
                    color: textSecondary
                    font.pixelSize: 20
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: root.openVideo()
                }
            }

            Item {
                Layout.fillWidth: true
            }
        }

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
                }

                Rectangle {
                    anchors.fill: parent
                    visible: !mediaBridge || mediaBridge.loadedPath.length === 0
                    radius: 12
                    color: "transparent"

                    Column {
                        anchors.centerIn: parent
                        spacing: 10

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: "◌"
                            color: borderColor
                            font.pixelSize: 68
                        }

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: "视频播放区域"
                            color: textSecondary
                            font.pixelSize: 24
                        }

                        Text {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: "点击“文件 > 打开视频”加载本地视频"
                            color: textSecondary
                            font.pixelSize: 14
                        }
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

            ComboBox {
                Layout.preferredWidth: 84
                model: ["0.25x", "0.5x", "0.75x", "1.0x", "1.25x", "1.5x", "2.0x"]
                currentIndex: root.playbackRateIndex(mediaBridge ? mediaBridge.playbackRate : 1.0)
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

            Slider {
                Layout.preferredWidth: 120
                from: 0
                to: 100
                value: 70
            }

            Item {
                Layout.fillWidth: true
            }

            Repeater {
                model: [
                    { icon: "CC", action: "subtitle" },
                    { icon: "⚙", action: "settings" },
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

        Slider {
            Layout.fillWidth: true
            visible: !root.fullScreenMode
            from: 0
            to: mediaBridge && mediaBridge.duration > 0 ? mediaBridge.duration : 1
            value: mediaBridge ? mediaBridge.currentPosition : 0
            enabled: mediaBridge && mediaBridge.duration > 0

            onMoved: {
                root.manualSeekRequested()
                seekToPosition(value)
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: !root.fullScreenMode

            Text {
                Layout.fillWidth: true
                text: mediaBridge ? mediaBridge.mediaSummary : "Container: mp4 | Video: h264 | Audio: aac"
                color: textPrimary
                font.pixelSize: 13
                elide: Text.ElideRight
            }

            Text {
                text: mediaBridge ? mediaBridge.statusMessage : "Ready"
                color: textSecondary
                font.pixelSize: 13
            }
        }
    }
}
