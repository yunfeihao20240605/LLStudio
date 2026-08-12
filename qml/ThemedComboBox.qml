pragma ComponentBehavior: Bound

import QtQuick 2.15
import QtQuick.Controls 2.15

ComboBox {
    id: control

    property color panelColor: "#ffffff"
    property color elevatedColor: "#fafbfc"
    property color borderColor: "#d0d7de"
    property color textColor: "#1f2329"
    property color disabledTextColor: "#6b7280"
    property color accentColor: "#2f6fed"
    property color accentBackgroundColor: "#eaf1fe"

    implicitHeight: 36
    leftPadding: 12
    rightPadding: 34

    contentItem: Text {
        leftPadding: 0
        rightPadding: 0
        text: control.displayText
        color: control.enabled ? control.textColor : control.disabledTextColor
        opacity: control.enabled ? 1 : 0.7
        font: control.font
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    indicator: Text {
        x: control.width - width - 12
        y: control.topPadding + (control.availableHeight - height) / 2
        text: control.popup.visible ? "⌃" : "⌄"
        color: control.enabled ? control.accentColor : control.disabledTextColor
        opacity: control.enabled ? 1 : 0.65
        font.pixelSize: 15
        font.bold: true
    }

    background: Rectangle {
        radius: 7
        color: control.panelColor
        border.color: control.activeFocus || control.popup.visible
                      ? control.accentColor : control.borderColor
        border.width: control.activeFocus || control.popup.visible ? 2 : 1
    }

    delegate: ItemDelegate {
        id: option
        required property int index
        required property var modelData

        width: control.width
        height: 38
        highlighted: control.highlightedIndex === index

        contentItem: Text {
            text: option.modelData
            color: option.index === control.currentIndex
                   ? "#ffffff" : control.textColor
            font: control.font
            verticalAlignment: Text.AlignVCenter
            elide: Text.ElideRight
        }

        background: Rectangle {
            color: option.index === control.currentIndex
                   ? control.accentColor
                   : (option.highlighted
                      ? control.accentBackgroundColor : "transparent")
        }
    }

    popup: Popup {
        y: control.height + 2
        width: control.width
        implicitHeight: Math.min(contentItem.implicitHeight + topPadding + bottomPadding,
                                 280)
        padding: 1

        contentItem: ListView {
            clip: true
            implicitHeight: contentHeight
            model: control.popup.visible ? control.delegateModel : null
            currentIndex: control.highlightedIndex
            ScrollIndicator.vertical: ScrollIndicator {}
        }

        background: Rectangle {
            color: control.panelColor
            border.color: control.borderColor
            border.width: 1
            radius: 7
        }
    }
}
