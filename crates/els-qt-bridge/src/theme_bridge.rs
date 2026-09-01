//! 包装 `els_storage::SettingsStore`，对接技术方案 3.3 节的主题偏好
//! （light/dark/auto）读写，暴露给 QML 使用。
//! 只做适配转换，具体颜色 token 计算仍在 QML 层完成。

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

const AUTO_MODE: &str = "auto";
const LIGHT_MODE: &str = "light";
const DARK_MODE: &str = "dark";
const PAPER_MODE: &str = "paper";
const SKY_MODE: &str = "sky";
const MIDNIGHT_MODE: &str = "midnight";
const AURORA_MODE: &str = "aurora";
const TWILIGHT_MODE: &str = "twilight";

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, theme_mode, cxx_name = "themeMode")]
        #[qproperty(QString, last_error, cxx_name = "lastError")]
        #[qproperty(bool, library_panel_expanded, cxx_name = "libraryPanelExpanded")]
        #[qproperty(bool, details_panel_expanded, cxx_name = "detailsPanelExpanded")]
        #[qproperty(bool, waveform_on_right, cxx_name = "waveformOnRight")]
        type ThemeBridge = super::ThemeBridgeRust;

        #[qinvokable]
        #[cxx_name = "applyThemeMode"]
        fn apply_theme_mode(self: Pin<&mut ThemeBridge>, theme_mode: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "cycleThemeMode"]
        fn cycle_theme_mode(self: Pin<&mut ThemeBridge>) -> QString;

        #[qinvokable]
        #[cxx_name = "reportError"]
        fn report_error(self: Pin<&mut ThemeBridge>, message: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "saveLayoutSettings"]
        fn save_layout_settings(
            self: Pin<&mut ThemeBridge>,
            library_panel_expanded: bool,
            details_panel_expanded: bool,
            waveform_on_right: bool,
        ) -> bool;
    }
}

pub struct ThemeBridgeRust {
    theme_mode: QString,
    last_error: QString,
    library_panel_expanded: bool,
    details_panel_expanded: bool,
    waveform_on_right: bool,
    settings: els_storage::SettingsStore,
}

impl Default for ThemeBridgeRust {
    fn default() -> Self {
        let mut last_error = String::new();
        let mut settings = match els_storage::SettingsStore::open_default() {
            Ok(store) => store,
            Err(err) => {
                last_error = format!("Failed to initialize settings store: {err}");
                eprintln!("{last_error}");
                els_storage::SettingsStore::disabled()
            }
        };

        let theme_mode = match settings.get(els_storage::THEME_MODE_KEY) {
            Ok(Some(saved_mode)) if is_valid_theme_mode(&saved_mode) => saved_mode,
            Ok(Some(saved_mode)) => {
                last_error = format!(
                    "Unsupported persisted theme mode '{saved_mode}', falling back to auto"
                );
                eprintln!("{last_error}");
                AUTO_MODE.to_string()
            }
            Ok(None) => {
                if let Err(err) = settings.set(els_storage::THEME_MODE_KEY, AUTO_MODE) {
                    last_error = format!("Failed to persist default theme mode: {err}");
                    eprintln!("{last_error}");
                }
                AUTO_MODE.to_string()
            }
            Err(err) => {
                last_error = format!("Failed to read persisted theme mode: {err}");
                eprintln!("{last_error}");
                AUTO_MODE.to_string()
            }
        };

        let library_panel_expanded = read_bool_setting(
            &settings,
            els_storage::LAYOUT_LIBRARY_PANEL_EXPANDED_KEY,
            true,
        );
        let details_panel_expanded = read_bool_setting(
            &settings,
            els_storage::LAYOUT_DETAILS_PANEL_EXPANDED_KEY,
            true,
        );
        let waveform_on_right =
            read_bool_setting(&settings, els_storage::LAYOUT_WAVEFORM_ON_RIGHT_KEY, false);

        Self {
            theme_mode: QString::from(&theme_mode),
            last_error: QString::from(&last_error),
            library_panel_expanded,
            details_panel_expanded,
            waveform_on_right,
            settings,
        }
    }
}

impl qobject::ThemeBridge {
    fn apply_theme_mode(mut self: Pin<&mut Self>, theme_mode: &QString) -> bool {
        let requested_mode = theme_mode.to_string();

        if !is_valid_theme_mode(&requested_mode) {
            let error = format!("Invalid theme mode: {requested_mode}");
            eprintln!("{error}");
            self.as_mut().set_last_error(QString::from(&error));
            return false;
        }

        let persist_result = {
            let mut rust = self.as_mut().rust_mut();
            rust.settings
                .set(els_storage::THEME_MODE_KEY, &requested_mode)
        };

        match persist_result {
            Ok(()) => {
                self.as_mut().set_theme_mode(QString::from(&requested_mode));
                self.as_mut().set_last_error(QString::from(""));
                true
            }
            Err(err) => {
                let error = format!("Failed to persist theme mode: {err}");
                eprintln!("{error}");
                self.as_mut().set_last_error(QString::from(&error));
                false
            }
        }
    }

    fn cycle_theme_mode(mut self: Pin<&mut Self>) -> QString {
        let next_mode = next_theme_mode(&self.rust().theme_mode.to_string());
        let next_mode_qstring = QString::from(next_mode);
        let _ = self.as_mut().apply_theme_mode(&next_mode_qstring);
        next_mode_qstring
    }

    fn report_error(mut self: Pin<&mut Self>, message: &QString) -> bool {
        self.as_mut()
            .set_last_error(QString::from(message.to_string()));
        true
    }

    fn save_layout_settings(
        mut self: Pin<&mut Self>,
        library_panel_expanded: bool,
        details_panel_expanded: bool,
        waveform_on_right: bool,
    ) -> bool {
        let persist_result = {
            let mut rust = self.as_mut().rust_mut();
            rust.settings
                .set(
                    els_storage::LAYOUT_LIBRARY_PANEL_EXPANDED_KEY,
                    if library_panel_expanded {
                        "true"
                    } else {
                        "false"
                    },
                )
                .and_then(|_| {
                    rust.settings.set(
                        els_storage::LAYOUT_DETAILS_PANEL_EXPANDED_KEY,
                        if details_panel_expanded {
                            "true"
                        } else {
                            "false"
                        },
                    )
                })
                .and_then(|_| {
                    rust.settings.set(
                        els_storage::LAYOUT_WAVEFORM_ON_RIGHT_KEY,
                        if waveform_on_right { "true" } else { "false" },
                    )
                })
        };

        match persist_result {
            Ok(()) => true,
            Err(err) => {
                let error = format!("Failed to persist layout settings: {err}");
                eprintln!("{error}");
                self.as_mut().set_last_error(QString::from(&error));
                false
            }
        }
    }
}

fn read_bool_setting(settings: &els_storage::SettingsStore, key: &str, default: bool) -> bool {
    match settings.get(key).ok().flatten().as_deref() {
        Some("true") | Some("1") => true,
        Some("false") | Some("0") => false,
        _ => default,
    }
}

fn is_valid_theme_mode(theme_mode: &str) -> bool {
    matches!(
        theme_mode,
        AUTO_MODE
            | LIGHT_MODE
            | DARK_MODE
            | PAPER_MODE
            | SKY_MODE
            | MIDNIGHT_MODE
            | AURORA_MODE
            | TWILIGHT_MODE
    )
}

fn next_theme_mode(theme_mode: &str) -> &'static str {
    match theme_mode {
        AUTO_MODE => LIGHT_MODE,
        LIGHT_MODE => DARK_MODE,
        DARK_MODE => MIDNIGHT_MODE,
        MIDNIGHT_MODE => AURORA_MODE,
        AURORA_MODE => TWILIGHT_MODE,
        TWILIGHT_MODE => PAPER_MODE,
        PAPER_MODE => SKY_MODE,
        _ => AUTO_MODE,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_valid_theme_mode, next_theme_mode, ThemeBridgeRust};

    #[test]
    fn dark_theme_variants_are_valid_and_in_cycle_order() {
        assert!(is_valid_theme_mode("midnight"));
        assert!(is_valid_theme_mode("aurora"));
        assert!(is_valid_theme_mode("twilight"));
        assert_eq!(next_theme_mode("dark"), "midnight");
        assert_eq!(next_theme_mode("midnight"), "aurora");
        assert_eq!(next_theme_mode("aurora"), "twilight");
        assert_eq!(next_theme_mode("twilight"), "paper");
    }

    #[test]
    fn default_theme_mode_is_auto_and_persisted() {
        let db_path = std::env::temp_dir().join(format!(
            "els-theme-bridge-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock before unix epoch")
                .as_nanos()
        ));
        std::env::set_var("ELS_DB_PATH", &db_path);

        let bridge = ThemeBridgeRust::default();
        assert_eq!(bridge.theme_mode.to_string(), "auto");

        let store = els_storage::SettingsStore::open_default().expect("reopen settings store");
        let persisted = store
            .get(els_storage::THEME_MODE_KEY)
            .expect("read persisted theme mode");
        assert_eq!(persisted.as_deref(), Some("auto"));

        let _ = std::fs::remove_file(&db_path);
        std::env::remove_var("ELS_DB_PATH");
    }
}
