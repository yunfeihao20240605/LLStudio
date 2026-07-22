//! 包装 `els_storage::SettingsStore`，对接技术方案 3.3 节的主题偏好
//! （light/dark/auto）读写，暴露给 QML 使用。
//! 只做适配转换，具体颜色 token 计算仍在 QML 层完成。

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

const AUTO_MODE: &str = "auto";
const LIGHT_MODE: &str = "light";
const DARK_MODE: &str = "dark";

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
        type ThemeBridge = super::ThemeBridgeRust;

        #[qinvokable]
        #[cxx_name = "applyThemeMode"]
        fn apply_theme_mode(self: Pin<&mut ThemeBridge>, theme_mode: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "cycleThemeMode"]
        fn cycle_theme_mode(self: Pin<&mut ThemeBridge>) -> QString;
    }
}

pub struct ThemeBridgeRust {
    theme_mode: QString,
    last_error: QString,
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

        Self {
            theme_mode: QString::from(&theme_mode),
            last_error: QString::from(&last_error),
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
}

fn is_valid_theme_mode(theme_mode: &str) -> bool {
    matches!(theme_mode, AUTO_MODE | LIGHT_MODE | DARK_MODE)
}

fn next_theme_mode(theme_mode: &str) -> &'static str {
    match theme_mode {
        AUTO_MODE => LIGHT_MODE,
        LIGHT_MODE => DARK_MODE,
        _ => AUTO_MODE,
    }
}

#[cfg(test)]
mod tests {
    use super::ThemeBridgeRust;

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
