import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

Item {
    id: root

    property var libraryBridge
    property string selectedVideoPath: ""
    property int childIndent: 16
    property color elevatedBg: "#fafbfc"
    property color borderColor: "#d0d7de"
    property color textPrimary: "#1f2329"
    property color textSecondary: "#6b7280"
    property color accent: "#2f6fed"
    property color accentBg: "#eaf1fe"

    signal videoSelected(string path)
    signal videoOpenRequested(string path)
    signal restoreRequested(string path)

    implicitHeight: completedContent.implicitHeight

    Column {
        id: completedContent
        width: parent.width
        spacing: 4

        Rectangle {
            visible: !root.libraryBridge || root.libraryBridge.completedCount === 0
            x: root.childIndent
            width: parent.width - root.childIndent
            height: visible ? 42 : 0
            radius: 8
            color: root.elevatedBg

            Text {
                anchors.centerIn: parent
                text: "暂无已完成视频"
                color: root.textSecondary
                font.pixelSize: 12
            }
        }

        Repeater {
            model: root.libraryBridge
                   ? root.libraryBridge.completedUngroupedVideoCount : 0

            delegate: Rectangle {
                id: ungroupedVideoDelegate
                property int videoIndex: index
                property string videoPath: root.libraryBridge
                                                   ? root.libraryBridge.completedUngroupedVideoPathAt(
                                                         videoIndex) : ""

                x: root.childIndent
                width: completedContent.width - root.childIndent
                height: 38
                radius: 8
                color: root.selectedVideoPath === videoPath
                       ? root.accentBg : root.elevatedBg
                border.color: root.selectedVideoPath === videoPath
                              ? root.accent : root.borderColor

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    anchors.rightMargin: 8
                    spacing: 8

                    Text {
                        text: "✓"
                        color: root.accent
                        font.pixelSize: 12
                    }

                    Text {
                        Layout.fillWidth: true
                        text: {
                            root.libraryBridge.revision
                            return root.libraryBridge.completedUngroupedVideoTitleAt(
                                        ungroupedVideoDelegate.videoIndex)
                        }
                        color: root.textPrimary
                        font.pixelSize: 13
                        elide: Text.ElideMiddle
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onClicked: function(mouse) {
                        root.videoSelected(ungroupedVideoDelegate.videoPath)
                        if (mouse.button === Qt.RightButton)
                            ungroupedVideoMenu.popup()
                        else
                            root.videoOpenRequested(ungroupedVideoDelegate.videoPath)
                    }
                }

                ThemedMenu {
                    id: ungroupedVideoMenu
                    panelColor: root.elevatedBg
                    borderColor: root.borderColor
                    textColor: root.textPrimary
                    disabledTextColor: root.textSecondary
                    hoverColor: root.accentBg

                    ThemedMenuItem {
                        textColor: root.textPrimary
                        disabledTextColor: root.textSecondary
                        hoverColor: root.accentBg
                        text: "移回正在学习"
                        onTriggered: root.restoreRequested(
                                         ungroupedVideoDelegate.videoPath)
                    }
                }
            }
        }

        Repeater {
            model: root.libraryBridge ? root.libraryBridge.completedListCount : 0

            delegate: Column {
                id: completedListDelegate
                property int listIndex: index
                property bool expanded: true
                width: completedContent.width
                spacing: 4

                Rectangle {
                    width: parent.width
                    height: 36
                    radius: 8
                    color: "transparent"
                    border.color: root.borderColor

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 8

                        Text {
                            text: completedListDelegate.expanded ? "⌃" : "⌄"
                            color: root.textSecondary
                            font.pixelSize: 12
                        }

                        Text {
                            Layout.fillWidth: true
                            text: {
                                root.libraryBridge.revision
                                return root.libraryBridge.completedListNameAt(
                                            completedListDelegate.listIndex)
                            }
                            color: root.textPrimary
                            font.pixelSize: 13
                            font.bold: true
                            elide: Text.ElideRight
                        }

                        Text {
                            text: {
                                root.libraryBridge.revision
                                return root.libraryBridge.completedListVideoCountAt(
                                            completedListDelegate.listIndex)
                            }
                            color: root.textSecondary
                            font.pixelSize: 12
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: completedListDelegate.expanded
                                   = !completedListDelegate.expanded
                    }
                }

                Repeater {
                    model: completedListDelegate.expanded && root.libraryBridge
                           ? root.libraryBridge.completedListVideoCountAt(
                                 completedListDelegate.listIndex) : 0

                    delegate: Rectangle {
                        id: groupedVideoDelegate
                        property int videoIndex: index
                        property string videoPath: root.libraryBridge.completedListVideoPathAt(
                                                       completedListDelegate.listIndex,
                                                       videoIndex)

                        width: completedListDelegate.width - root.childIndent
                        x: root.childIndent
                        height: 38
                        radius: 8
                        color: root.selectedVideoPath === videoPath
                               ? root.accentBg : root.elevatedBg
                        border.color: root.selectedVideoPath === videoPath
                                      ? root.accent : root.borderColor

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            anchors.rightMargin: 8
                            spacing: 8

                            Text {
                                text: "✓"
                                color: root.accent
                                font.pixelSize: 12
                            }

                            Text {
                                Layout.fillWidth: true
                                text: {
                                    root.libraryBridge.revision
                                    return root.libraryBridge.completedListVideoTitleAt(
                                                completedListDelegate.listIndex,
                                                groupedVideoDelegate.videoIndex)
                                }
                                color: root.textPrimary
                                font.pixelSize: 13
                                elide: Text.ElideMiddle
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            onClicked: function(mouse) {
                                root.videoSelected(groupedVideoDelegate.videoPath)
                                if (mouse.button === Qt.RightButton)
                                    groupedVideoMenu.popup()
                                else
                                    root.videoOpenRequested(groupedVideoDelegate.videoPath)
                            }
                        }

                        ThemedMenu {
                            id: groupedVideoMenu
                            panelColor: root.elevatedBg
                            borderColor: root.borderColor
                            textColor: root.textPrimary
                            disabledTextColor: root.textSecondary
                            hoverColor: root.accentBg

                            ThemedMenuItem {
                                textColor: root.textPrimary
                                disabledTextColor: root.textSecondary
                                hoverColor: root.accentBg
                                text: "移回正在学习"
                                onTriggered: root.restoreRequested(
                                                 groupedVideoDelegate.videoPath)
                            }
                        }
                    }
                }
            }
        }
    }
}
