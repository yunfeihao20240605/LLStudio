import QtQuick 2.15
import QtQuick.Controls 2.15

Rectangle {
    id: root

    property int splitOrientation: Qt.Horizontal
    property color gapColor: "#f5f6f8"
    property color dividerColor: "#e3e5e8"
    property color accentColor: "#2f6fed"
    readonly property bool handleHovered: SplitHandle.hovered
    readonly property bool handlePressed: SplitHandle.pressed

    implicitWidth: splitOrientation === Qt.Horizontal ? 7 : 1
    implicitHeight: splitOrientation === Qt.Vertical ? 7 : 1
    color: gapColor

    Rectangle {
        anchors.centerIn: parent
        width: root.splitOrientation === Qt.Horizontal ? 1 : parent.width
        height: root.splitOrientation === Qt.Horizontal ? parent.height : 1
        color: root.handlePressed || root.handleHovered
               ? root.accentColor : root.dividerColor
        opacity: root.handlePressed ? 1 : (root.handleHovered ? 0.72 : 1)
    }
}
