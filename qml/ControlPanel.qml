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
    property bool canStartTraining: false
    property bool isTraining: false
    property bool hasStartedTraining: false
    property bool isPlaybackPlaying: false
    property int completedLoops: 0
    property int totalLoops: 0
    property real selectionStart: 0
    property real selectionEnd: 0
    property string trainingStatus: ""
    readonly property int repeatCount: repeatCountSpinBox.value
    readonly property int intervalSeconds: intervalSecondsSpinBox.value

    signal startTrainingRequested(int repeatCount, int intervalSeconds)

    function formatSeconds(totalSeconds) {
        var safe = Math.max(0, Math.floor(totalSeconds || 0))
        var minutes = Math.floor(safe / 60)
        var seconds = safe % 60
        return (minutes < 10 ? "0" + minutes : minutes) + ":" + (seconds < 10 ? "0" + seconds : seconds)
    }

    function applyTrainingSettings(repeatCount, intervalSeconds) {
        if (isTraining)
            return
        repeatCountSpinBox.value = Math.max(repeatCountSpinBox.from,
                                            Math.min(repeatCountSpinBox.to, repeatCount))
        intervalSecondsSpinBox.value = Math.max(intervalSecondsSpinBox.from,
                                                Math.min(intervalSecondsSpinBox.to, intervalSeconds))
    }

    radius: 16
    color: panelBg
    border.color: borderColor
    border.width: 1

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 12

        RowLayout {
            Layout.fillWidth: true

            Text {
                text: "学习控制"
                color: textPrimary
                font.pixelSize: 16
                font.bold: true
            }

            Text {
                visible: trainingStatus.length > 0
                text: trainingStatus
                color: isTraining ? accent : textSecondary
                font.pixelSize: 12
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                visible: isTraining || totalLoops > 0
                text: "已完成 " + completedLoops + "/" + totalLoops
                color: textSecondary
                font.pixelSize: 12
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            Rectangle {
                Layout.preferredWidth: 220
                Layout.preferredHeight: 56
                radius: 10
                color: elevatedBg
                border.color: canStartTraining ? accent : borderColor

                Column {
                    anchors.centerIn: parent
                    spacing: 4

                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: canStartTraining ? "当前训练选区" : "请先设置 A～B 选区"
                        color: canStartTraining ? textSecondary : textPrimary
                        font.pixelSize: 12
                    }

                    Text {
                        visible: canStartTraining
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: formatSeconds(selectionStart) + " ～ " + formatSeconds(selectionEnd)
                        color: textPrimary
                        font.pixelSize: 14
                        font.bold: true
                    }
                }
            }

            Item {
                Layout.fillWidth: true
            }

            ColumnLayout {
                spacing: 6

                Text {
                    text: "循环次数"
                    color: textSecondary
                    font.pixelSize: 12
                }

                SpinBox {
                    id: repeatCountSpinBox
                    from: 1
                    to: 50
                    value: 10
                    enabled: !isTraining
                    Layout.preferredWidth: 96
                }
            }

            ColumnLayout {
                spacing: 6

                Text {
                    text: "间隔时间(秒)"
                    color: textSecondary
                    font.pixelSize: 12
                }

                SpinBox {
                    id: intervalSecondsSpinBox
                    from: 0
                    to: 30
                    value: 3
                    enabled: !isTraining
                    Layout.preferredWidth: 110
                }
            }

            Rectangle {
                Layout.preferredWidth: 136
                Layout.preferredHeight: 48
                radius: 10
                color: canStartTraining ? accent : borderColor

                Text {
                    anchors.centerIn: parent
                    text: !root.hasStartedTraining ? "▶  开始训练"
                          : (root.isPlaybackPlaying ? "Ⅱ  暂停播放" : "▶  继续播放")
                    color: "#ffffff"
                    font.pixelSize: 15
                    font.bold: true
                }

                MouseArea {
                    anchors.fill: parent
                    enabled: root.canStartTraining
                    onClicked: {
                        root.startTrainingRequested(repeatCountSpinBox.value,
                                                    intervalSecondsSpinBox.value)
                    }
                }
            }
        }
    }
}
