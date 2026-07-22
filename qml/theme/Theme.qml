import QtQuick 2.15

Item {
    id: theme

    property string mode: "auto"

    PaletteLight {
        id: lightPalette
    }

    PaletteDark {
        id: darkPalette
    }

    SystemPalette {
        id: systemPalette
        colorGroup: SystemPalette.Active
    }

    readonly property bool systemPrefersDark: luminance(systemPalette.window) < 0.5
    readonly property string effectiveMode: {
        if (mode === "dark") {
            return "dark"
        }
        if (mode === "light") {
            return "light"
        }
        return systemPrefersDark ? "dark" : "light"
    }
    readonly property var palette: effectiveMode === "dark" ? darkPalette : lightPalette

    readonly property color windowBg: palette.windowBg
    readonly property color panelBg: palette.panelBg
    readonly property color elevatedBg: palette.elevatedBg
    readonly property color border: palette.border
    readonly property color textPrimary: palette.textPrimary
    readonly property color textSecondary: palette.textSecondary
    readonly property color accent: palette.accent
    readonly property color accentBg: palette.accentBg

    function luminance(color) {
        return (color.r * 0.299) + (color.g * 0.587) + (color.b * 0.114)
    }
}
