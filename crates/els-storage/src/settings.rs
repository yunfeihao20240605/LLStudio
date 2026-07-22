//! 用户偏好设置存取实现（例如 3.3 节的主题模式 light/dark/auto）。

use crate::schema::CREATE_SETTINGS_TABLE;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

pub const THEME_MODE_KEY: &str = "theme_mode";

const DEFAULT_DB_FILE_NAME: &str = "english-learning-studio.sqlite3";

/// `settings` 表的存取封装。
///
/// 当前阶段默认把 SQLite 文件放在工作目录下，可通过 `ELS_DB_PATH` 覆盖，
/// 这样本地开发和自动化测试都能使用同一套接口。
pub struct SettingsStore {
    connection: Option<Connection>,
    db_path: PathBuf,
}

impl SettingsStore {
    pub fn open_default() -> els_types::AppResult<Self> {
        let db_path = default_database_path();
        let connection = Connection::open(&db_path).map_err(sqlite_error)?;
        ensure_schema(&connection)?;

        Ok(Self {
            connection: Some(connection),
            db_path,
        })
    }

    pub fn disabled() -> Self {
        Self {
            connection: None,
            db_path: default_database_path(),
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn get(&self, key: &str) -> els_types::AppResult<Option<String>> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sqlite_error)
    }

    pub fn set(&mut self, key: &str, value: &str) -> els_types::AppResult<()> {
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn connection(&self) -> els_types::AppResult<&Connection> {
        self.connection
            .as_ref()
            .ok_or_else(|| els_types::AppError::Io("settings store is unavailable".to_string()))
    }
}

impl Default for SettingsStore {
    fn default() -> Self {
        match Self::open_default() {
            Ok(store) => store,
            Err(err) => {
                eprintln!("Failed to initialize settings store: {err}");
                Self::disabled()
            }
        }
    }
}

fn ensure_schema(connection: &Connection) -> els_types::AppResult<()> {
    connection
        .execute_batch(CREATE_SETTINGS_TABLE)
        .map_err(sqlite_error)
}

fn default_database_path() -> PathBuf {
    if let Ok(path) = std::env::var("ELS_DB_PATH") {
        return PathBuf::from(path);
    }

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(DEFAULT_DB_FILE_NAME)
}

fn sqlite_error(error: rusqlite::Error) -> els_types::AppError {
    els_types::AppError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{SettingsStore, THEME_MODE_KEY};

    #[test]
    fn persisting_settings_across_store_instances() {
        let db_path = std::env::temp_dir().join(format!(
            "els-settings-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock before unix epoch")
                .as_nanos()
        ));
        std::env::set_var("ELS_DB_PATH", &db_path);

        let mut first_store = SettingsStore::open_default().expect("open first store");
        first_store
            .set(THEME_MODE_KEY, "dark")
            .expect("persist dark mode");

        let second_store = SettingsStore::open_default().expect("open second store");
        let persisted = second_store
            .get(THEME_MODE_KEY)
            .expect("read persisted theme mode");

        assert_eq!(persisted.as_deref(), Some("dark"));

        let _ = std::fs::remove_file(&db_path);
        std::env::remove_var("ELS_DB_PATH");
    }
}
