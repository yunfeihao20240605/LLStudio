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
    property int previousSegmentCount: 0
    property bool scrollToBottomPending: false
    property int markerEditingIndex: -1
    property var selectedIndices: []
    property int selectionAnchorIndex: -1

    signal segmentActivated(int index)
    signal segmentDeleteRequested(int index)
    signal labelPlaybackRequested(int index)

    function formatSeconds(totalSeconds) {
        var safe = Math.max(0, Math.floor(totalSeconds || 0))
        var minutes = Math.floor(safe / 60)
        var seconds = safe % 60
        return (minutes < 10 ? "0" + minutes : minutes) + ":" + (seconds < 10 ? "0" + seconds : seconds)
    }

    function scrollToBottom() {
        scrollToBottomPending = true
        scrollToBottomTimer.restart()
    }

    function revealActiveSegment() {
        Qt.callLater(function() {
            if (!root.segmentBridge)
                return
            var index = root.segmentBridge.activeIndex
            if (index >= 0 && index < segmentList.count)
                segmentList.positionViewAtIndex(index, ListView.Contain)
        })
    }

    function editSegmentMarker(index) {
        if (!segmentBridge || index < 0)
            return
        markerEditingIndex = index
        markerTextField.text = segmentBridge.segmentLabelAt(index)
        markerDialog.open()
    }

    function isSegmentSelected(index) {
        return selectedIndices.indexOf(index) >= 0
    }

    function clearSegmentSelection() {
        selectedIndices = []
        selectionAnchorIndex = -1
    }

    function selectOnly(index) {
        selectedIndices = [index]
        selectionAnchorIndex = index
    }

    function toggleSegmentSelection(index) {
        var next = selectedIndices.slice()
        var position = next.indexOf(index)
        if (position >= 0)
            next.splice(position, 1)
        else
            next.push(index)
        next.sort(function(left, right) { return left - right })
        selectedIndices = next
        selectionAnchorIndex = next.length > 0 ? index : -1
    }

    function selectRange(index, additive) {
        var anchor = selectionAnchorIndex >= 0 ? selectionAnchorIndex : index
        var next = additive ? selectedIndices.slice() : []
        var first = Math.min(anchor, index)
        var last = Math.max(anchor, index)
        for (var candidate = first; candidate <= last; ++candidate) {
            if (next.indexOf(candidate) < 0)
                next.push(candidate)
        }
        next.sort(function(left, right) { return left - right })
        selectedIndices = next
        if (selectionAnchorIndex < 0)
            selectionAnchorIndex = index
    }

    function handleSegmentClick(index, modifiers) {
        var shiftPressed = (modifiers & Qt.ShiftModifier) !== 0
        var controlPressed = (modifiers & Qt.ControlModifier) !== 0
        if (shiftPressed)
            selectRange(index, controlPressed)
        else if (controlPressed)
            toggleSegmentSelection(index)
        else {
            selectOnly(index)
            segmentActivated(index)
        }
    }

    function prepareContextSelection(index) {
        if (!isSegmentSelected(index))
            selectOnly(index)
    }

    function selectionTargetsFor(index) {
        return isSegmentSelected(index) ? selectedIndices.slice() : [index]
    }

    function applyLabelToSelection(index, label) {
        if (!segmentBridge)
            return false
        return segmentBridge.setSegmentLabels(selectionTargetsFor(index), label)
    }

    function allSelectionTargetsHaveLabel(index, label) {
        if (!segmentBridge)
            return false
        var targets = selectionTargetsFor(index)
        if (targets.length <= 0)
            return false
        for (var targetIndex = 0; targetIndex < targets.length; ++targetIndex) {
            if (segmentBridge.segmentLabelAt(targets[targetIndex]) !== label)
                return false
        }
        return true
    }

    onSegmentBridgeChanged: {
        clearSegmentSelection()
        previousSegmentCount = segmentBridge ? segmentBridge.segmentCount : 0
        scrollToBottom()
    }

    Component.onCompleted: scrollToBottom()

    Timer {
        id: scrollToBottomTimer
        interval: 0
        repeat: false
        onTriggered: {
            if (!segmentList.visible || segmentList.height <= 0 || segmentList.count <= 0)
                return
            segmentList.forceLayout()
            segmentList.positionViewAtIndex(segmentList.count - 1, ListView.End)
            root.scrollToBottomPending = false
        }
    }

    Connections {
        target: root.segmentBridge
        ignoreUnknownSignals: true

        function onCurrentVideoPathChanged() {
            root.clearSegmentSelection()
            root.scrollToBottom()
        }

        function onSegmentCountChanged() {
            var currentCount = root.segmentBridge ? root.segmentBridge.segmentCount : 0
            if (currentCount !== root.previousSegmentCount)
                root.clearSegmentSelection()
            if (root.previousSegmentCount === 0 && currentCount > 0)
                root.scrollToBottom()
            else if (currentCount > root.previousSegmentCount)
                root.revealActiveSegment()
            root.previousSegmentCount = currentCount
        }
    }

    Dialog {
        id: markerDialog
        parent: Overlay.overlay
        anchors.centerIn: parent
        title: root.markerEditingIndex >= 0
               && root.selectionTargetsFor(root.markerEditingIndex).length > 1
               ? "为 " + root.selectionTargetsFor(root.markerEditingIndex).length
                 + " 个片段设置标记"
               : "编辑片段标记"
        modal: true
        implicitWidth: 300
        standardButtons: Dialog.Ok | Dialog.Cancel

        onOpened: markerTextField.forceActiveFocus()
        onAccepted: {
            if (root.segmentBridge && root.markerEditingIndex >= 0)
                root.applyLabelToSelection(root.markerEditingIndex,
                                           markerTextField.text)
            root.markerEditingIndex = -1
        }
        onRejected: root.markerEditingIndex = -1

        contentItem: TextField {
            id: markerTextField
            placeholderText: "例如：场景1"
            maximumLength: 80
            selectByMouse: true
            onAccepted: markerDialog.accept()
        }
    }

    radius: 16
    color: panelBg
    border.color: borderColor
    border.width: 1

    Shortcut {
        sequence: "Escape"
        enabled: root.selectedIndices.length > 0
        onActivated: root.clearSegmentSelection()
    }

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
            id: segmentList
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: segmentBridge && segmentBridge.segmentCount > 0
            model: segmentBridge ? segmentBridge.segmentCount : 0
            spacing: 8
            clip: true

            onCountChanged: {
                if (root.scrollToBottomPending)
                    scrollToBottomTimer.restart()
            }
            onVisibleChanged: {
                if (visible && root.scrollToBottomPending)
                    scrollToBottomTimer.restart()
            }
            onHeightChanged: {
                if (height > 0 && root.scrollToBottomPending)
                    scrollToBottomTimer.restart()
            }

            delegate: Rectangle {
                id: segmentDelegate
                required property int index
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
                readonly property string marker: {
                    const _revision = bridgeRevision
                    return root.segmentBridge ? root.segmentBridge.segmentLabelAt(index) : ""
                }
                readonly property bool selected: root.isSegmentSelected(index)

                width: ListView.view.width
                height: 68
                radius: 10
                color: selected ? accentBg
                                : (root.segmentBridge
                                   && index === root.segmentBridge.activeIndex
                                   ? elevatedBg : "transparent")
                border.color: selected || (root.segmentBridge
                                            && index === root.segmentBridge.activeIndex)
                              ? accent : borderColor
                border.width: selected ? 2 : 1

                RowLayout {
                    z: 1
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 10

                    Text {
                        text: index + 1
                        color: selected ? accent : textPrimary
                        font.pixelSize: 15
                        font.bold: selected
                        Layout.preferredWidth: 18
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4

                        Text {
                            Layout.fillWidth: true
                            text: marker.length > 0
                                  ? marker + " · " + root.formatSeconds(startSecs) + " ～ "
                                    + root.formatSeconds(endSecs)
                                  : root.formatSeconds(startSecs) + " ～ " + root.formatSeconds(endSecs)
                            color: textPrimary
                            font.pixelSize: 14
                            elide: Text.ElideRight
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
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onClicked: function(mouse) {
                        if (mouse.button === Qt.RightButton) {
                            root.prepareContextSelection(index)
                            segmentContextMenu.popup()
                        } else {
                            root.handleSegmentClick(index, mouse.modifiers)
                        }
                    }
                }

                Menu {
                    id: segmentContextMenu

                    MenuItem {
                        text: segmentDelegate.marker.length > 0
                              ? "播放同标记片段：“" + segmentDelegate.marker + "”"
                              : "播放同标记片段"
                        enabled: segmentDelegate.marker.length > 0
                        onTriggered: root.labelPlaybackRequested(segmentDelegate.index)
                    }

                    Menu {
                        title: root.selectionTargetsFor(segmentDelegate.index).length > 1
                               ? "设置标记（"
                                 + root.selectionTargetsFor(segmentDelegate.index).length
                                 + " 个片段）"
                               : "设置标记"

                        Repeater {
                            model: root.segmentBridge
                                   ? root.segmentBridge.recentLabelCount : 0

                            delegate: MenuItem {
                                required property int index
                                property string candidateLabel: {
                                    const _revision = segmentDelegate.bridgeRevision
                                    return root.segmentBridge
                                            ? root.segmentBridge.recentLabelAt(index) : ""
                                }
                                text: index === 0
                                      ? candidateLabel + "（最近使用）"
                                      : candidateLabel
                                checkable: true
                                checked: {
                                    const _revision = segmentDelegate.bridgeRevision
                                    return root.allSelectionTargetsHaveLabel(
                                                segmentDelegate.index, candidateLabel)
                                }
                                onTriggered: root.applyLabelToSelection(
                                                 segmentDelegate.index, candidateLabel)
                            }
                        }

                        MenuSeparator {
                            visible: root.segmentBridge
                                     && root.segmentBridge.recentLabelCount > 0
                        }

                        MenuItem {
                            text: segmentDelegate.marker.length > 0
                                  ? "编辑当前标记…" : "新建标记…"
                            onTriggered: root.editSegmentMarker(segmentDelegate.index)
                        }
                    }

                    MenuItem {
                        text: "清除标记"
                        enabled: segmentDelegate.marker.length > 0
                                 || root.selectedIndices.length > 1
                        onTriggered: root.applyLabelToSelection(segmentDelegate.index, "")
                    }
                }
            }
        }
    }
}
