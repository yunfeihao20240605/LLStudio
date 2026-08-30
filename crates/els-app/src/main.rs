//! `els-app`：组合根（composition root）。
//!
//! 这是 workspace 中**唯一**知道所有具体实现类型的地方：负责实例化各
//! core crate 的具体实现、注入到 `els-storage`（持久化）与 `els-qt-bridge`
//! （Qt 适配层），最终启动 QML 引擎。除本文件外，其他 crate 都只依赖
//! trait，不知道具体实现是什么。

use cxx_qt::casting::Upcast;
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QQmlEngine, QUrl, QString};
use std::pin::Pin;

fn main() {
    configure_process_environment();
    els_qt_bridge::graphics_backend::force_opengl_backend();

    cxx_qt::init_crate!(els_qt_bridge);
    els_qt_bridge::mpv_video_item::register_qml_type();

    let mut app = QGuiApplication::new();
    if let Some(mut app) = app.as_mut() {
        app.as_mut().set_application_name(&QString::from("LLStudio"));
        app.as_mut()
            .set_application_version(&QString::from(env!("CARGO_PKG_VERSION")));
    }
    let mut engine = QQmlApplicationEngine::new();
    let main_qml = QUrl::from("qrc:/qt/qml/com/yfhao/els/app/qml/Main.qml");

    if let Some(engine) = engine.as_mut() {
        engine.load(&main_qml);
    }

    if let Some(engine) = engine.as_mut() {
        let engine: Pin<&mut QQmlEngine> = engine.upcast_pin();
        engine
            .on_quit(|_| {
                println!("QML engine requested quit");
            })
            .release();
    }

    println!("Loaded Main.qml via cxx-qt: qrc:/qt/qml/com/yfhao/els/app/qml/Main.qml");

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}

fn configure_process_environment() {
    std::env::set_var("LC_ALL", "C.UTF-8");
    std::env::set_var("LC_NUMERIC", "C");
    std::env::set_var("LANG", "C.UTF-8");
    std::env::set_var("QT_QUICK_CONTROLS_STYLE", "Basic");

    #[cfg(not(target_os = "windows"))]
    unsafe {
        libc::setenv(c"LC_ALL".as_ptr(), c"C.UTF-8".as_ptr(), 1);
        libc::setenv(c"LC_NUMERIC".as_ptr(), c"C".as_ptr(), 1);
        libc::setenv(c"LANG".as_ptr(), c"C.UTF-8".as_ptr(), 1);
        libc::setenv(c"QT_QUICK_CONTROLS_STYLE".as_ptr(), c"Basic".as_ptr(), 1);
        libc::setlocale(libc::LC_ALL, c"C.UTF-8".as_ptr());
        libc::setlocale(libc::LC_NUMERIC, c"C".as_ptr());
    }

    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            let ffmpeg_name = if cfg!(target_os = "windows") {
                "ffmpeg.exe"
            } else {
                "ffmpeg"
            };
            let ffprobe_name = if cfg!(target_os = "windows") {
                "ffprobe.exe"
            } else {
                "ffprobe"
            };
            if directory.join(ffmpeg_name).exists() {
                std::env::set_var("ELS_FFMPEG_BIN", directory.join(ffmpeg_name));
            }
            if directory.join(ffprobe_name).exists() {
                std::env::set_var("ELS_FFPROBE_BIN", directory.join(ffprobe_name));
            }
            let current_path = std::env::var_os("PATH").unwrap_or_default();
            let mut paths = std::env::split_paths(&current_path).collect::<Vec<_>>();
            if !paths.iter().any(|path| path == directory) {
                paths.insert(0, directory.to_path_buf());
                if let Ok(path) = std::env::join_paths(paths) {
                    std::env::set_var("PATH", path);
                }
            }
        }
    }
}
