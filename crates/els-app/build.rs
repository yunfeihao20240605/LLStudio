use cxx_qt_build::{CxxQtBuilder, QmlModule};
use std::path::PathBuf;

fn main() {
    embed_macos_info_plist();
    CxxQtBuilder::new_qml_module(
        QmlModule::new("com.yfhao.els.app")
            .qml_file("../../qml/Main.qml")
            .qml_file("../../qml/LibrarySidebar.qml")
            .qml_file("../../qml/CompletedVideoList.qml")
            .qml_file("../../qml/VideoPlaybackPane.qml")
            .qml_file("../../qml/AiPanel.qml")
            .qml_file("../../qml/AiSettingsPane.qml")
            .qml_file("../../qml/WaveformView.qml")
            .qml_file("../../qml/RecordingWaveformTrack.qml")
            .qml_file("../../qml/ControlPanel.qml")
            .qml_file("../../qml/SelectionTrainingController.qml")
            .qml_file("../../qml/ThemedSplitHandle.qml")
            .qml_file("../../qml/ThemedSlider.qml")
            .qml_file("../../qml/ThemedComboBox.qml")
            .qml_file("../../qml/ThemedToolButton.qml")
            .qml_file("../../qml/ThemedSpinBox.qml")
            .qml_file("../../qml/ThemedTextField.qml")
            .qml_file("../../qml/ThemedTextArea.qml")
            .qml_file("../../qml/ThemedMenu.qml")
            .qml_file("../../qml/ThemedMenuItem.qml")
            .qml_file("../../qml/ThemedMenuSeparator.qml")
            .qml_file("../../qml/SubtitleView.qml")
            .qml_file("../../qml/NoteView.qml")
            .qml_file("../../qml/SegmentListView.qml")
            .qml_file("../../qml/SettingsDialog.qml")
            .qml_file("../../qml/ShortcutHelpDialog.qml")
            .qml_file("../../qml/SpeechSettingsPane.qml")
            .qml_file("../../qml/StatusBar.qml")
            .qml_file("../../qml/theme/Theme.qml")
            .qml_file("../../qml/theme/PaletteLight.qml")
            .qml_file("../../qml/theme/PaletteDark.qml")
            .qml_file("../../qml/theme/PalettePaper.qml")
            .qml_file("../../qml/theme/PaletteSky.qml")
            .qml_file("../../qml/theme/PaletteMidnight.qml")
            .qml_file("../../qml/theme/PaletteAurora.qml")
            .qml_file("../../qml/theme/PaletteTwilight.qml"),
    )
    .qrc("../../qml/video_resources.qrc")
    .qt_module("Network")
    .qt_module("QuickControls2")
    .build();
}

fn embed_macos_info_plist() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let plist =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default()).join("Info.plist");
    println!("cargo:rerun-if-changed={}", plist.display());
    println!(
        "cargo:rustc-link-arg=-Wl,-sectcreate,__TEXT,__info_plist,{}",
        plist.display()
    );
}
