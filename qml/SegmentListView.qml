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
    property var segmentBridge

    signal segmentActivated(int index)
    signal segmentDeleteRequested(int index)

    function formatSeconds(totalSeconds) {
        var safe = Math.max(0, Math.floor(totalSeconds || 0))
        var minutes = Math.floor(safe / 60)
        var seconds = safe % 60
        return (minutes < 10 ? "0" + minutes : minutes) + ":" + (seconds < 10 ? "0" + seconds : seconds)
    }

    radius: 16
    color: panelBg
    border.color: borderColor
    border.width: 1

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 10

        RowLayout {
            Layout.fillWidth: true

            Text {
                text: "学习片段列表"
                color: textPrimary
                font.pixelSize: 16
                font.bold: true
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                text: segmentBridge ? segmentBridge.segmentCount + " 个" : "0 个"
                color: textSecondary
                font.pixelSize: 12
            }
        }

        Text {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: !segmentBridge || segmentBridge.segmentCount === 0
            text: "开始训练后，当前 A～B 选区会自动保存到这里"
            color: textSecondary
            font.pixelSize: 13
            wrapMode: Text.Wrap
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }

        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: segmentBridge && segmentBridge.segmentCount > 0
            model: segmentBridge ? segmentBridge.segmentCount : 0
            spacing: 8
            clip: true

            delegate: Rectangle {
                readonly property int bridgeRevision: root.segmentBridge ? root.segmentBridge.revision : 0
                readonly property real startSecs: {
                    const _revision = bridgeRevision
                    return root.segmentBridge ? root.segmentBridge.segmentStartAt(index) : 0
                }
                readonly property real endSecs: {
                    const _revision = bridgeRevision
                    return root.segmentBridge ? root.segmentBridge.segmentEndAt(index) : 0
                }
                readonly property int repeatCount: {
                    const _revision = bridgeRevision
                    return root.segmentBridge ? root.segmentBridge.segmentRepeatCountAt(index) : 0
                }
                readonly property int intervalSeconds: {
                    const _revision = bridgeRevision
                    return root.segmentBridge ? root.segmentBridge.segmentIntervalSecondsAt(index) : 0
                }
                readonly property int completedLoops: {
                    const _revision = bridgeRevision
                    return root.segmentBridge ? root.segmentBridge.segmentCompletedLoopsAt(index) : 0
                }

                width: ListView.view.width
                height: 68
                radius: 10
                color: root.segmentBridge && index === root.segmentBridge.activeIndex ? accentBg : "transparent"
                border.color: root.segmentBridge && index === root.segmentBridge.activeIndex ? accent : borderColor
                border.width: 1

                RowLayout {
                    z: 1
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 10

                    Text {
                        text: index + 1
                        color: textPrimary
                        font.pixelSize: 15
                        Layout.preferredWidth: 18
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4

                        Text {
                            text: root.formatSeconds(startSecs) + " ～ " + root.formatSeconds(endSecs)
                            color: textPrimary
                            font.pixelSize: 14
                        }

                        Text {
                            text: Math.max(0, Math.round(endSecs - startSecs)) + "秒  ×" + repeatCount
                                  + "  间隔" + intervalSeconds + "秒  累计" + completedLoops + "次"
                            color: textSecondary
                            font.pixelSize: 12
                        }
                    }

                    Text {
                        text: "▶"
                        color: accent
                        font.pixelSize: 16
                    }

                    Text {
                        text: "🗑"
                        color: textSecondary
                        font.pixelSize: 15

                        MouseArea {
                            anchors.fill: parent
                            anchors.margins: -8
                            onClicked: root.segmentDeleteRequested(index)
                        }
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    z: 0
                    onClicked: root.segmentActivated(index)
                }
            }
        }
    }
}
