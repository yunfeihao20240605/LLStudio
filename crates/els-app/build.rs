use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("com.yfhao.els.app")
            .qml_file("../../qml/Main.qml")
            .qml_file("../../qml/LibrarySidebar.qml")
            .qml_file("../../qml/CompletedVideoList.qml")
            .qml_file("../../qml/VideoPlaybackPane.qml")
            .qml_file("../../qml/WaveformView.qml")
            .qml_file("../../qml/ControlPanel.qml")
            .qml_file("../../qml/SelectionTrainingController.qml")
            .qml_file("../../qml/SubtitleView.qml")
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
