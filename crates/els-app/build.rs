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
            .qml_file("../../qml/WaveformView.qml")
            .qml_file("../../qml/RecordingWaveformTrack.qml")
            .qml_file("../../qml/ControlPanel.qml")
            .qml_file("../../qml/SelectionTrainingController.qml")
            .qml_file("../../qml/SubtitleView.qml")
            .qml_file("../../qml/NoteView.qml")
            .qml_file("../../qml/SegmentListView.qml")
            .qml_file("../../qml/StatusBar.qml")
            .qml_file("../../qml/theme/Theme.qml")
            .qml_file("../../qml/theme/PaletteLight.qml")
            .qml_file("../../qml/theme/PaletteDark.qml"),
    )
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
