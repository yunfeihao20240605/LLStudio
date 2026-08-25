use cxx_qt_build::{CppFile, CxxQtBuilder, QmlModule};
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let mpv_prefix = detect_mpv_prefix();
    let mut builder = CxxQtBuilder::new_qml_module(QmlModule::new("com.yfhao.els.bridge"))
        .qt_module("Network")
        .qt_module("Gui")
        .qt_module("OpenGL")
        .qt_module("Quick")
        .files([
            "src/app_bootstrap.rs",
            "src/ai_tutor_bridge.rs",
            "src/theme_bridge.rs",
            "src/library_bridge.rs",
            "src/media_bridge.rs",
            "src/note_bridge.rs",
            "src/recording_bridge.rs",
            "src/recording_playback_bridge.rs",
            "src/segment_bridge.rs",
            "src/speech_recognition_bridge.rs",
            "src/speech_settings_bridge.rs",
            "src/subtitle_bridge.rs",
            "src/waveform_bridge.rs",
        ])
        .cpp_files([
            CppFile::from("src/graphics_backend.h"),
            CppFile::from("src/graphics_backend.cpp"),
            CppFile::from("src/mpv_video_item.h"),
            CppFile::from("src/mpv_video_item.cpp"),
        ]);

    if let Some(prefix) = mpv_prefix {
        let include_dir = prefix.join("include");
        if include_dir.exists() {
            unsafe {
                builder = builder.cc_builder(|cc| {
                    cc.include(&include_dir);
                });
            }
        }

        let lib_dir = prefix.join("lib");
        if lib_dir.exists() {
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
        }
    }

    println!("cargo:rerun-if-env-changed=MPV_PREFIX");
    println!("cargo:rustc-link-lib=dylib=mpv");

    builder.build().export();
}

fn detect_mpv_prefix() -> Option<PathBuf> {
    if let Ok(value) = std::env::var("MPV_PREFIX") {
        let path = PathBuf::from(value);
        if path.exists() {
            return Some(path);
        }
    }

    if let Ok(output) = Command::new("brew").args(["--prefix", "mpv"]).output() {
        if output.status.success() {
            let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !prefix.is_empty() {
                let path = PathBuf::from(prefix);
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }

    let default = PathBuf::from("/usr/local/opt/mpv");
    if default.exists() {
        Some(default)
    } else {
        None
    }
}
