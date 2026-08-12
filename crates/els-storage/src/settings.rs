//! 用户偏好设置存取实现（例如 3.3 节的主题模式 light/dark/auto）。

use crate::schema::{CREATE_AI_CONVERSATIONS_TABLE, CREATE_SETTINGS_TABLE};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

pub const THEME_MODE_KEY: &str = "theme_mode";
pub const AI_BASE_URL_KEY: &str = "ai.base_url";
pub const AI_API_KEY_KEY: &str = "ai.api_key";
pub const AI_MODEL_KEY: &str = "ai.model";
pub const AI_SYSTEM_PROMPT_KEY: &str = "ai.system_prompt";

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

pub struct AiSettingsRepository {
    store: SettingsStore,
}

impl AiSettingsRepository {
    pub fn open_default() -> els_types::AppResult<Self> {
        Ok(Self {
            store: SettingsStore::open_default()?,
        })
    }

    pub fn disabled() -> Self {
        Self {
            store: SettingsStore::disabled(),
        }
    }

    pub fn load(&self) -> els_types::AppResult<els_ai_core::AiConfig> {
        let get = |key: &str| Ok(self.store.get(key)?.unwrap_or_default());
        Ok(els_ai_core::AiConfig {
            protocol: els_ai_core::AiProtocol::OpenAiCompatible,
            base_url: get(AI_BASE_URL_KEY)?,
            api_key: get(AI_API_KEY_KEY)?,
            model: get(AI_MODEL_KEY)?,
            system_prompt: get(AI_SYSTEM_PROMPT_KEY)?,
        })
    }

    pub fn save(&mut self, config: &els_ai_core::AiConfig) -> els_types::AppResult<()> {
        self.store.set(AI_BASE_URL_KEY, &config.base_url)?;
        self.store.set(AI_API_KEY_KEY, &config.api_key)?;
        self.store.set(AI_MODEL_KEY, &config.model)?;
        self.store.set(AI_SYSTEM_PROMPT_KEY, &config.system_prompt)
    }

    pub fn load_conversation(
        &self,
        video_path: &str,
        cue_index: i32,
    ) -> els_types::AppResult<Vec<els_ai_core::ChatMessage>> {
        let connection = self.store.connection()?;
        let json = connection
            .query_row(
                "SELECT messages_json FROM ai_conversations
                 WHERE video_path = ?1 AND cue_index = ?2",
                params![video_path, cue_index],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sqlite_error)?;
        match json {
            Some(value) => serde_json::from_str(&value)
                .map_err(|error| els_types::AppError::Io(error.to_string())),
            None => Ok(Vec::new()),
        }
    }

    pub fn save_conversation(
        &mut self,
        video_path: &str,
        cue_index: i32,
        messages: &[els_ai_core::ChatMessage],
    ) -> els_types::AppResult<()> {
        let json = serde_json::to_string(messages)
            .map_err(|error| els_types::AppError::Io(error.to_string()))?;
        let connection = self.store.connection()?;
        connection
            .execute(
                "INSERT INTO ai_conversations (video_path, cue_index, messages_json, updated_at)
                 VALUES (?1, ?2, ?3, strftime('%s', 'now'))
                 ON CONFLICT(video_path, cue_index) DO UPDATE SET
                    messages_json = excluded.messages_json,
                    updated_at = excluded.updated_at",
                params![video_path, cue_index, json],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub fn delete_conversation(
        &mut self,
        video_path: &str,
        cue_index: i32,
    ) -> els_types::AppResult<()> {
        let connection = self.store.connection()?;
        connection
            .execute(
                "DELETE FROM ai_conversations WHERE video_path = ?1 AND cue_index = ?2",
                params![video_path, cue_index],
            )
            .map_err(sqlite_error)?;
        Ok(())
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
        .and_then(|_| connection.execute_batch(CREATE_AI_CONVERSATIONS_TABLE))
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
