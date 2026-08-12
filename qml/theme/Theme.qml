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

    PalettePaper {
        id: paperPalette
    }

    PaletteSky {
        id: skyPalette
    }

    PaletteMidnight {
        id: midnightPalette
    }

    PaletteAurora {
        id: auroraPalette
    }

    PaletteTwilight {
        id: twilightPalette
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
        if (mode === "paper") {
            return "paper"
        }
        if (mode === "sky") {
            return "sky"
        }
        if (mode === "midnight") {
            return "midnight"
        }
        if (mode === "aurora") {
            return "aurora"
        }
        if (mode === "twilight") {
            return "twilight"
        }
        return systemPrefersDark ? "dark" : "light"
    }
    readonly property bool darkAppearance: effectiveMode === "dark"
                                                || effectiveMode === "midnight"
                                                || effectiveMode === "aurora"
                                                || effectiveMode === "twilight"
    readonly property var activePalette: {
        if (effectiveMode === "dark")
            return darkPalette
        if (effectiveMode === "midnight")
            return midnightPalette
        if (effectiveMode === "aurora")
            return auroraPalette
        if (effectiveMode === "twilight")
            return twilightPalette
        if (effectiveMode === "paper")
            return paperPalette
        if (effectiveMode === "sky")
            return skyPalette
        return lightPalette
    }

    readonly property color windowBg: activePalette.windowBg
    readonly property color panelBg: activePalette.panelBg
    readonly property color elevatedBg: activePalette.elevatedBg
    readonly property color border: activePalette.border
    readonly property color textPrimary: activePalette.textPrimary
    readonly property color textSecondary: activePalette.textSecondary
    readonly property color accent: activePalette.accent
    readonly property color accentBg: activePalette.accentBg

    function luminance(color) {
        return (color.r * 0.299) + (color.g * 0.587) + (color.b * 0.114)
    }
}
