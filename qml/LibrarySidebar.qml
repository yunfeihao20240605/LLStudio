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
    property var libraryBridge
    property int selectedLibraryIndex: 0
    property string selectedVideoPath: ""
    property bool learningExpanded: true
    property bool completedExpanded: false
    property int libraryChildIndent: 16
    property bool selectionAvailable: false
    property real selectionStart: 0
    property real selectionEnd: 0
    property bool canStartTraining: false
    property bool isTraining: false
    property bool hasStartedTraining: false
    property bool isPlaybackPlaying: false
    property int completedLoops: 0
    property int totalLoops: 0
    property string trainingStatus: ""
    readonly property int repeatCount: sidebarControlPanel.repeatCount
    readonly property real intervalSeconds: sidebarControlPanel.intervalSeconds
    readonly property bool hasActiveSegment: segmentBridge && segmentBridge.activeIndex >= 0
    readonly property bool hasDraftSelection: selectionAvailable
                                               && (!hasActiveSegment
                                                   || Math.abs(selectionStart - segmentBridge.activeStart) > 0.05
                                                   || Math.abs(selectionEnd - segmentBridge.activeEnd) > 0.05)
    readonly property bool hasCurrentSegment: hasDraftSelection || hasActiveSegment
    readonly property real displayedStart: hasDraftSelection ? selectionStart
                                                             : (hasActiveSegment ? segmentBridge.activeStart : 0)
    readonly property real displayedEnd: hasDraftSelection ? selectionEnd
                                                           : (hasActiveSegment ? segmentBridge.activeEnd : 0)

    signal videoOpenRequested(string path)
    signal startTrainingRequested(int repeatCount, real intervalSeconds)

    function applyTrainingSettings(repeatCount, intervalSeconds) {
        sidebarControlPanel.applyTrainingSettings(repeatCount, intervalSeconds)
    }

    function revealLearningVideos() {
        selectedLibraryIndex = 0
        learningExpanded = true
    }

    function deleteVideo(videoPath) {
        if (!libraryBridge || !videoPath || videoPath.length === 0)
            return false
        var wasSelected = selectedVideoPath === videoPath
        if (wasSelected)
            selectedVideoPath = ""
        if (libraryBridge.removeVideo(videoPath))
            return true
        if (wasSelected)
            selectedVideoPath = videoPath
        return false
    }

    function deleteSelectedVideo() {
        return deleteVideo(selectedVideoPath)
    }

    function markVideoCompleted(videoPath) {
        if (!libraryBridge || !videoPath || videoPath.length === 0)
            return false
        if (!libraryBridge.markVideoCompleted(videoPath))
            return false
        completedExpanded = true
        return true
    }

    function restoreCompletedVideo(videoPath) {
        if (!libraryBridge || !videoPath || videoPath.length === 0)
            return false
        if (!libraryBridge.restoreCompletedVideo(videoPath))
            return false
        revealLearningVideos()
        return true
    }

    function formatSeconds(totalSeconds) {
        var safe = Math.max(0, Math.floor(totalSeconds || 0))
        var minutes = Math.floor(safe / 60)
        var seconds = safe % 60
        return (minutes < 10 ? "0" + minutes : minutes) + ":" + (seconds < 10 ? "0" + seconds : seconds)
    }

    property var libraryItems: [
        { label: "正在学习", icon: "◉" },
        { label: "已完成", icon: "✓" }
    ]

    radius: 16
    color: panelBg
    border.color: borderColor
    border.width: 1

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 14
        spacing: 12

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: 12
            color: panelBg
            border.color: borderColor

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 12

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        text: "学习库"
                        color: textPrimary
                        font.pixelSize: 16
                        font.bold: true
                    }

                    Item {
                        Layout.fillWidth: true
                    }

                    Text {
                        text: "✕"
                        color: textSecondary
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 36
                    radius: 8
                    color: elevatedBg
                    border.color: borderColor

                    Row {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        spacing: 8

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "⌕"
                            color: textSecondary
                            font.pixelSize: 14
                        }

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "搜索视频/字幕"
                            color: textSecondary
                            font.pixelSize: 13
                        }
                    }
                }

                ScrollView {
                    id: libraryScroll
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

                    ColumnLayout {
                        width: libraryScroll.availableWidth
                        spacing: 6

                        Repeater {
                            model: libraryItems

                            delegate: Item {
                                property int categoryIndex: index
                                property bool isLearningCategory: categoryIndex === 0
                                property bool isCompletedCategory: categoryIndex === 1
                                property bool showLearningVideos: isLearningCategory
                                                                          && root.learningExpanded
                                property bool showCompletedVideos: isCompletedCategory
                                                                           && root.completedExpanded

                                Layout.fillWidth: true
                                Layout.preferredHeight: categoryHeader.height
                                                        + (showLearningVideos
                                                           ? learningVideoList.implicitHeight + 4
                                                           : (showCompletedVideos
                                                              ? completedVideoList.implicitHeight + 4
                                                              : 0))

                                Rectangle {
                                    id: categoryHeader
                                    width: parent.width
                                    height: 40
                                    radius: 10
                                    color: categoryIndex === root.selectedLibraryIndex
                                           ? accentBg : "transparent"
                                    border.color: categoryIndex === root.selectedLibraryIndex
                                                  ? accent : "transparent"
                                    border.width: categoryIndex === root.selectedLibraryIndex ? 1 : 0

                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.leftMargin: 12
                                        anchors.rightMargin: 12

                                        Text {
                                            text: modelData.icon
                                            color: categoryIndex === root.selectedLibraryIndex
                                                   ? accent : textPrimary
                                            font.pixelSize: 14
                                        }

                                        Text {
                                            text: modelData.label
                                            color: categoryIndex === root.selectedLibraryIndex
                                                   ? accent : textPrimary
                                            font.pixelSize: 14
                                        }

                                        Item { Layout.fillWidth: true }

                                        Rectangle {
                                            Layout.preferredWidth: 26
                                            Layout.preferredHeight: 22
                                            radius: 11
                                            color: categoryIndex === root.selectedLibraryIndex
                                                   ? accent : elevatedBg

                                            Text {
                                                anchors.centerIn: parent
                                                text: isLearningCategory && root.libraryBridge
                                                      ? root.libraryBridge.inProgressCount
                                                      : (root.libraryBridge
                                                         ? root.libraryBridge.completedCount : 0)
                                                color: categoryIndex === root.selectedLibraryIndex
                                                       ? "#ffffff" : textSecondary
                                                font.pixelSize: 12
                                            }
                                        }

                                        Text {
                                            text: (isLearningCategory
                                                   ? root.learningExpanded
                                                   : root.completedExpanded) ? "⌃" : "⌄"
                                            color: categoryIndex === root.selectedLibraryIndex
                                                   ? accent : textSecondary
                                            font.pixelSize: 13
                                        }
                                    }

                                    MouseArea {
                                        anchors.fill: parent
                                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                                        onClicked: function(mouse) {
                                            if (isLearningCategory
                                                    && mouse.button === Qt.RightButton) {
                                                learningContextMenu.popup()
                                            } else {
                                                root.selectedLibraryIndex = categoryIndex
                                                if (isLearningCategory)
                                                    root.learningExpanded = !root.learningExpanded
                                                else if (isCompletedCategory)
                                                    root.completedExpanded = !root.completedExpanded
                                            }
                                        }
                                    }

                                    DropArea {
                                        anchors.fill: parent
                                        enabled: isLearningCategory
                                        keys: ["learning-video"]
                                        onDropped: function(drop) {
                                            if (root.libraryBridge && drop.source
                                                    && drop.source.videoPath)
                                                root.libraryBridge.moveVideoToList(
                                                            drop.source.videoPath, -1)
                                        }
                                    }
                                }

                                Column {
                                    id: learningVideoList
                                    visible: showLearningVideos
                                    anchors.top: categoryHeader.bottom
                                    anchors.topMargin: 4
                                    anchors.left: parent.left
                                    anchors.leftMargin: root.libraryChildIndent
                                    anchors.right: parent.right
                                    spacing: 4

                                    Rectangle {
                                        visible: !root.libraryBridge
                                                 || root.libraryBridge.inProgressCount === 0
                                        width: parent.width
                                        height: visible ? 42 : 0
                                        radius: 8
                                        color: elevatedBg

                                        Text {
                                            anchors.centerIn: parent
                                            text: "打开视频后会显示在这里"
                                            color: textSecondary
                                            font.pixelSize: 12
                                        }
                                    }

                                    Repeater {
                                        model: root.libraryBridge
                                               ? root.libraryBridge.ungroupedVideoCount : 0

                                        delegate: Rectangle {
                                            id: directVideoDelegate
                                            property int videoIndex: index
                                            property string videoPath: root.libraryBridge
                                                                       ? root.libraryBridge.ungroupedVideoPathAt(videoIndex)
                                                                       : ""

                                            width: learningVideoList.width
                                            height: 38
                                            radius: 8
                                            focus: root.selectedVideoPath
                                                   === directVideoDelegate.videoPath
                                            color: root.selectedVideoPath
                                                   === directVideoDelegate.videoPath
                                                   ? accentBg
                                                   : (root.segmentBridge
                                                   && root.segmentBridge.currentVideoPath
                                                      === directVideoDelegate.videoPath
                                                   ? accentBg : elevatedBg)
                                            border.color: root.selectedVideoPath
                                                          === directVideoDelegate.videoPath
                                                          ? accent : borderColor
                                            Drag.active: directDragArea.drag.active
                                            Drag.source: directVideoDelegate
                                            Drag.keys: ["learning-video"]
                                            Drag.hotSpot.x: width / 2
                                            Drag.hotSpot.y: height / 2

                                            Keys.onPressed: function(event) {
                                                if (event.key === Qt.Key_Delete) {
                                                    root.deleteSelectedVideo()
                                                    event.accepted = true
                                                }
                                            }

                                            Item {
                                                id: directDragProxy
                                                width: 1
                                                height: 1
                                            }

                                            RowLayout {
                                                z: 1
                                                anchors.fill: parent
                                                anchors.leftMargin: 10
                                                anchors.rightMargin: 8
                                                spacing: 8

                                                Text {
                                                    text: "▶"
                                                    color: accent
                                                    font.pixelSize: 11
                                                }

                                                Text {
                                                    Layout.fillWidth: true
                                                    text: {
                                                        root.libraryBridge.revision
                                                        return root.libraryBridge.ungroupedVideoTitleAt(videoIndex)
                                                    }
                                                    color: textPrimary
                                                    font.pixelSize: 13
                                                    elide: Text.ElideMiddle
                                                }

                                                Text {
                                                    text: "×"
                                                    color: textSecondary
                                                    font.pixelSize: 17

                                                    MouseArea {
                                                        anchors.fill: parent
                                                        anchors.margins: -7
                                                        cursorShape: Qt.PointingHandCursor
                                                        onClicked: root.deleteVideo(
                                                                       directVideoDelegate.videoPath)
                                                    }
                                                }
                                            }

                                            MouseArea {
                                                id: directDragArea
                                                anchors.fill: parent
                                                acceptedButtons: Qt.LeftButton | Qt.RightButton
                                                drag.target: directDragProxy
                                                onClicked: function(mouse) {
                                                    root.selectedVideoPath
                                                            = directVideoDelegate.videoPath
                                                    directVideoDelegate.forceActiveFocus(
                                                                Qt.MouseFocusReason)
                                                    if (mouse.button === Qt.RightButton)
                                                        directVideoMenu.popup()
                                                    else if (directVideoDelegate.videoPath.length > 0)
                                                        root.videoOpenRequested(directVideoDelegate.videoPath)
                                                }
                                            }

                                            Menu {
                                                id: directVideoMenu

                                                MenuItem {
                                                    text: "标记为已完成"
                                                    onTriggered: root.markVideoCompleted(
                                                                     directVideoDelegate.videoPath)
                                                }

                                                MenuSeparator {}

                                                Menu {
                                                    title: "移动到"

                                                    MenuItem {
                                                        text: "暂无自定义列表"
                                                        enabled: false
                                                        visible: !root.libraryBridge
                                                                 || root.libraryBridge.listCount === 0
                                                    }

                                                    Repeater {
                                                        model: root.libraryBridge
                                                               ? root.libraryBridge.listCount : 0

                                                        delegate: MenuItem {
                                                            property int targetListIndex: index
                                                            text: {
                                                                root.libraryBridge.revision
                                                                return root.libraryBridge.listNameAt(
                                                                            targetListIndex)
                                                            }
                                                            onTriggered: root.libraryBridge.moveVideoToList(
                                                                             directVideoDelegate.videoPath,
                                                                             targetListIndex)
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    Repeater {
                                        model: root.libraryBridge ? root.libraryBridge.listCount : 0

                                        delegate: Column {
                                            id: customListDelegate
                                            property int listIndex: index
                                            property bool expanded: true
                                            width: learningVideoList.width
                                                   + root.libraryChildIndent
                                            transform: Translate {
                                                x: -root.libraryChildIndent
                                            }
                                            spacing: 4

                                            Rectangle {
                                                id: customListHeader
                                                width: parent.width
                                                height: 36
                                                radius: 8
                                                color: "transparent"
                                                border.color: borderColor

                                                RowLayout {
                                                    anchors.fill: parent
                                                    anchors.leftMargin: 10
                                                    anchors.rightMargin: 8

                                                    Text {
                                                        text: customListDelegate.expanded ? "⌃" : "⌄"
                                                        color: textSecondary
                                                        font.pixelSize: 12
                                                    }

                                                    Text {
                                                        Layout.fillWidth: true
                                                        text: {
                                                            root.libraryBridge.revision
                                                            return root.libraryBridge.listNameAt(
                                                                        customListDelegate.listIndex)
                                                        }
                                                        color: textPrimary
                                                        font.pixelSize: 13
                                                        font.bold: true
                                                        elide: Text.ElideRight
                                                    }

                                                    Text {
                                                        text: {
                                                            root.libraryBridge.revision
                                                            return root.libraryBridge.listVideoCountAt(
                                                                        customListDelegate.listIndex)
                                                        }
                                                        color: textSecondary
                                                        font.pixelSize: 12
                                                    }
                                                }

                                                DropArea {
                                                    anchors.fill: parent
                                                    keys: ["learning-video"]
                                                    onDropped: function(drop) {
                                                        if (drop.source && drop.source.videoPath)
                                                            root.libraryBridge.moveVideoToList(
                                                                        drop.source.videoPath,
                                                                        customListDelegate.listIndex)
                                                    }
                                                }

                                                MouseArea {
                                                    anchors.fill: parent
                                                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                                                    onClicked: function(mouse) {
                                                        if (mouse.button === Qt.RightButton)
                                                            customListMenu.popup()
                                                        else
                                                            customListDelegate.expanded
                                                                    = !customListDelegate.expanded
                                                    }
                                                }

                                                Menu {
                                                    id: customListMenu
                                                    MenuItem {
                                                        text: "删除列表"
                                                        onTriggered: root.libraryBridge.deleteList(
                                                                         customListDelegate.listIndex)
                                                    }
                                                }
                                            }

                                            Repeater {
                                                model: customListDelegate.expanded && root.libraryBridge
                                                       ? root.libraryBridge.listVideoCountAt(
                                                                 customListDelegate.listIndex) : 0

                                                delegate: Rectangle {
                                                    id: groupedVideoDelegate
                                                    property int groupedVideoIndex: index
                                                    property string videoPath: root.libraryBridge.listVideoPathAt(
                                                                                   customListDelegate.listIndex,
                                                                                   groupedVideoIndex)
                                                    width: customListDelegate.width
                                                           - root.libraryChildIndent
                                                    x: root.libraryChildIndent
                                                    height: 38
                                                    radius: 8
                                                    focus: root.selectedVideoPath
                                                           === groupedVideoDelegate.videoPath
                                                    color: root.selectedVideoPath
                                                           === groupedVideoDelegate.videoPath
                                                           ? accentBg
                                                           : (root.segmentBridge
                                                           && root.segmentBridge.currentVideoPath
                                                              === groupedVideoDelegate.videoPath
                                                           ? accentBg : elevatedBg)
                                                    border.color: root.selectedVideoPath
                                                                  === groupedVideoDelegate.videoPath
                                                                  ? accent : borderColor
                                                    Drag.active: groupedDragArea.drag.active
                                                    Drag.source: groupedVideoDelegate
                                                    Drag.keys: ["learning-video"]
                                                    Drag.hotSpot.x: width / 2
                                                    Drag.hotSpot.y: height / 2

                                                    Keys.onPressed: function(event) {
                                                        if (event.key === Qt.Key_Delete) {
                                                            root.deleteSelectedVideo()
                                                            event.accepted = true
                                                        }
                                                    }

                                                    Item {
                                                        id: groupedDragProxy
                                                        width: 1
                                                        height: 1
                                                    }

                                                    RowLayout {
                                                        z: 1
                                                        anchors.fill: parent
                                                        anchors.leftMargin: 10
                                                        anchors.rightMargin: 8

                                                        Text {
                                                            text: "▶"
                                                            color: accent
                                                            font.pixelSize: 11
                                                        }

                                                        Text {
                                                            Layout.fillWidth: true
                                                            text: {
                                                                root.libraryBridge.revision
                                                                return root.libraryBridge.listVideoTitleAt(
                                                                            customListDelegate.listIndex,
                                                                            groupedVideoIndex)
                                                            }
                                                            color: textPrimary
                                                            font.pixelSize: 13
                                                            elide: Text.ElideMiddle
                                                        }

                                                        Text {
                                                            text: "×"
                                                            color: textSecondary
                                                            font.pixelSize: 17

                                                            MouseArea {
                                                                anchors.fill: parent
                                                                anchors.margins: -7
                                                                cursorShape: Qt.PointingHandCursor
                                                                onClicked: root.deleteVideo(
                                                                               groupedVideoDelegate.videoPath)
                                                            }
                                                        }
                                                    }

                                                    MouseArea {
                                                        id: groupedDragArea
                                                        anchors.fill: parent
                                                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                                                        drag.target: groupedDragProxy
                                                        onClicked: function(mouse) {
                                                            root.selectedVideoPath
                                                                    = groupedVideoDelegate.videoPath
                                                            groupedVideoDelegate.forceActiveFocus(
                                                                        Qt.MouseFocusReason)
                                                            if (mouse.button === Qt.RightButton)
                                                                groupedVideoMenu.popup()
                                                            else if (groupedVideoDelegate.videoPath.length > 0)
                                                                root.videoOpenRequested(
                                                                            groupedVideoDelegate.videoPath)
                                                        }
                                                    }

                                                    Menu {
                                                        id: groupedVideoMenu

                                                        MenuItem {
                                                            text: "标记为已完成"
                                                            onTriggered: root.markVideoCompleted(
                                                                             groupedVideoDelegate.videoPath)
                                                        }

                                                        MenuSeparator {}

                                                        Menu {
                                                            title: "移动到"

                                                            MenuItem {
                                                                text: "正在学习"
                                                                onTriggered: root.libraryBridge.moveVideoToList(
                                                                                 groupedVideoDelegate.videoPath,
                                                                                 -1)
                                                            }

                                                            MenuSeparator {}

                                                            Repeater {
                                                                model: root.libraryBridge
                                                                       ? root.libraryBridge.listCount : 0

                                                                delegate: MenuItem {
                                                                    property int targetListIndex: index
                                                                    text: {
                                                                        root.libraryBridge.revision
                                                                        return root.libraryBridge.listNameAt(
                                                                                    targetListIndex)
                                                                    }
                                                                    enabled: targetListIndex
                                                                             !== customListDelegate.listIndex
                                                                    onTriggered: root.libraryBridge.moveVideoToList(
                                                                                     groupedVideoDelegate.videoPath,
                                                                                     targetListIndex)
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                CompletedVideoList {
                                    id: completedVideoList
                                    visible: showCompletedVideos
                                    anchors.top: categoryHeader.bottom
                                    anchors.topMargin: 4
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    libraryBridge: root.libraryBridge
                                    selectedVideoPath: root.selectedVideoPath
                                    childIndent: root.libraryChildIndent
                                    elevatedBg: root.elevatedBg
                                    borderColor: root.borderColor
                                    textPrimary: root.textPrimary
                                    textSecondary: root.textSecondary
                                    accent: root.accent
                                    accentBg: root.accentBg
                                    onVideoSelected: function(path) {
                                        root.selectedVideoPath = path
                                    }
                                    onVideoOpenRequested: function(path) {
                                        root.videoOpenRequested(path)
                                    }
                                    onRestoreRequested: function(path) {
                                        root.restoreCompletedVideo(path)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Menu {
            id: learningContextMenu
            MenuItem {
                text: "新建列表"
                onTriggered: createListDialog.open()
            }
        }

        Dialog {
            id: createListDialog
            parent: Overlay.overlay
            anchors.centerIn: parent
            title: "新建学习列表"
            modal: true
            implicitWidth: 260
            standardButtons: Dialog.Ok | Dialog.Cancel

            onOpened: {
                listNameField.text = ""
                listNameField.forceActiveFocus()
            }
            onAccepted: {
                if (root.libraryBridge
                        && root.libraryBridge.createList(listNameField.text))
                    root.revealLearningVideos()
            }

            contentItem: TextField {
                id: listNameField
                placeholderText: "输入列表名称"
                selectByMouse: true
                onAccepted: createListDialog.accept()
            }
        }

        ControlPanel {
            id: sidebarControlPanel
            Layout.fillWidth: true
            Layout.preferredHeight: root.isTraining || root.totalLoops > 0
                                    ? 254 : 229
            canStartTraining: root.canStartTraining
            isTraining: root.isTraining
            hasStartedTraining: root.hasStartedTraining
            isPlaybackPlaying: root.isPlaybackPlaying
            completedLoops: root.completedLoops
            totalLoops: root.totalLoops
            selectionStart: root.selectionStart
            selectionEnd: root.selectionEnd
            trainingStatus: root.trainingStatus
            onStartTrainingRequested: function(repeatCount, intervalSeconds) {
                root.startTrainingRequested(repeatCount, intervalSeconds)
            }
            panelBg: root.panelBg
            elevatedBg: root.elevatedBg
            borderColor: root.borderColor
            textPrimary: root.textPrimary
            textSecondary: root.textSecondary
            accent: root.accent
            accentBg: root.accentBg
        }
    }
}
