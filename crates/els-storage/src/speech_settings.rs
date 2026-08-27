use crate::schema::CREATE_SPEECH_PROVIDER_PROFILE_TABLE;
use crate::sqlite::{default_database_path, prepare_database_path};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeechProviderProfile {
    pub id: String,
    pub name: String,
    pub provider_kind: String,
    pub config_json: String,
    pub enabled: bool,
}

pub struct SpeechSettingsRepository {
    connection: Option<Connection>,
}

impl SpeechSettingsRepository {
    pub fn open_default() -> els_types::AppResult<Self> {
        Self::open_path(default_database_path())
    }

    pub fn open_path(path: impl AsRef<Path>) -> els_types::AppResult<Self> {
        let path = prepare_database_path(path)?;
        let connection = Connection::open(path).map_err(sqlite_error)?;
        connection
            .execute_batch(CREATE_SPEECH_PROVIDER_PROFILE_TABLE)
            .map_err(sqlite_error)?;
        Ok(Self {
            connection: Some(connection),
        })
    }

    pub fn disabled() -> Self {
        Self { connection: None }
    }

    pub fn load_active(&self) -> els_types::AppResult<Option<SpeechProviderProfile>> {
        self.connection()?
            .query_row(
                "SELECT id, name, provider_kind, config_json, enabled
                 FROM speech_provider_profile
                 WHERE enabled = 1
                 ORDER BY updated_at DESC
                 LIMIT 1",
                [],
                |row| {
                    Ok(SpeechProviderProfile {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        provider_kind: row.get(2)?,
                        config_json: row.get(3)?,
                        enabled: row.get::<_, i64>(4)? != 0,
                    })
                },
            )
            .optional()
            .map_err(sqlite_error)
    }

    pub fn save_active(&mut self, profile: &SpeechProviderProfile) -> els_types::AppResult<()> {
        let connection = self.connection_mut()?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        transaction
            .execute("UPDATE speech_provider_profile SET enabled = 0", [])
            .map_err(sqlite_error)?;
        transaction
            .execute(
                "INSERT INTO speech_provider_profile
                    (id, name, provider_kind, config_json, enabled, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 1, strftime('%s', 'now'))
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    provider_kind = excluded.provider_kind,
                    config_json = excluded.config_json,
                    enabled = 1,
                    updated_at = excluded.updated_at",
                params![
                    profile.id,
                    profile.name,
                    profile.provider_kind,
                    profile.config_json
                ],
            )
            .map_err(sqlite_error)?;
        transaction.commit().map_err(sqlite_error)
    }

    fn connection(&self) -> els_types::AppResult<&Connection> {
        self.connection.as_ref().ok_or_else(|| {
            els_types::AppError::Io("speech settings repository is unavailable".to_string())
        })
    }

    fn connection_mut(&mut self) -> els_types::AppResult<&mut Connection> {
        self.connection.as_mut().ok_or_else(|| {
            els_types::AppError::Io("speech settings repository is unavailable".to_string())
        })
    }
}

impl Default for SpeechSettingsRepository {
    fn default() -> Self {
        Self::open_default().unwrap_or_else(|_| Self::disabled())
    }
}

fn sqlite_error(error: rusqlite::Error) -> els_types::AppError {
    els_types::AppError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_reloads_the_active_provider_profile() {
        let path = std::env::temp_dir().join(format!(
            "els-speech-settings-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut repository = SpeechSettingsRepository::open_path(&path).unwrap();
        repository
            .save_active(&SpeechProviderProfile {
                id: "tencent-default".into(),
                name: "腾讯云".into(),
                provider_kind: "tencent-asr".into(),
                config_json: "{}".into(),
                enabled: true,
            })
            .unwrap();
        assert_eq!(
            repository.load_active().unwrap().unwrap().provider_kind,
            "tencent-asr"
        );
        let _ = std::fs::remove_file(path);
    }
}
