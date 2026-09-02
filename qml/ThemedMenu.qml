import QtQuick 2.15
import QtQuick.Controls 2.15

Menu {
    id: control

    property color panelColor: "#ffffff"
    property color borderColor: "#d0d7de"
    property color textColor: "#1f2329"
    property color disabledTextColor: "#6b7280"
    property color hoverColor: "#eaf1fe"
    property int preferredWidth: 0
    property url menuIcon: ""

    implicitWidth: preferredWidth > 0
                   ? preferredWidth
                   : Math.max(210, contentWidth + leftPadding + rightPadding)

    // Menu instances used as submenus create their item rows from `delegate`.
    // Keep those rows on the same palette as explicitly declared ThemedMenuItems.
    delegate: ThemedMenuItem {
        textColor: control.textColor
        disabledTextColor: control.disabledTextColor
        hoverColor: control.hoverColor
        icon.source: subMenu ? subMenu.menuIcon : ""
    }

    topPadding: 5
    bottomPadding: 5
    leftPadding: 1
    rightPadding: 1

    background: Rectangle {
        implicitWidth: control.preferredWidth > 0 ? control.preferredWidth : 210
        color: control.panelColor
        border.color: control.borderColor
        border.width: 1
        radius: 7
    }
}
