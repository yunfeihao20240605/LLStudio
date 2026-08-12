//! SQLite 表结构占位（video / segment / vocabulary / settings，见技术方案第 9 节）。

pub const CREATE_VIDEO_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS video (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    duration REAL NOT NULL DEFAULT 0,
    learning_status TEXT NOT NULL DEFAULT 'in_progress',
    last_opened_at INTEGER NOT NULL DEFAULT 0,
    last_position REAL NOT NULL DEFAULT 0,
    list_id INTEGER
);";

pub const CREATE_VIDEO_LIST_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS video_list (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    learning_status TEXT NOT NULL DEFAULT 'in_progress',
    created_at INTEGER NOT NULL DEFAULT 0,
    UNIQUE(learning_status, name)
);";

pub const CREATE_SEGMENT_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS segment (
    id INTEGER PRIMARY KEY,
    video_id INTEGER NOT NULL,
    start_time REAL NOT NULL,
    end_time REAL NOT NULL,
    repeat_count INTEGER NOT NULL,
    interval_seconds INTEGER NOT NULL DEFAULT 0,
    completed_loops INTEGER NOT NULL DEFAULT 0,
    label TEXT NOT NULL DEFAULT ''
);";

pub const CREATE_SEGMENT_LABEL_HISTORY_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS segment_label_history (
    video_id INTEGER NOT NULL,
    label TEXT NOT NULL,
    last_used_at INTEGER NOT NULL,
    PRIMARY KEY (video_id, label)
);";

pub const CREATE_VIDEO_NOTE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS video_note (
    id INTEGER PRIMARY KEY,
    video_id INTEGER NOT NULL,
    start_time REAL NOT NULL,
    end_time REAL,
    content TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_video_note_time
    ON video_note(video_id, start_time, id);";

pub const CREATE_RECORDING_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS recording (
    id INTEGER PRIMARY KEY,
    video_id INTEGER NOT NULL,
    range_start REAL NOT NULL,
    range_end REAL NOT NULL,
    file_path TEXT NOT NULL,
    duration REAL NOT NULL,
    sample_rate INTEGER NOT NULL,
    alignment_offset REAL NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_recording_range
    ON recording(video_id, range_start, range_end, created_at DESC);";

pub const CREATE_SETTINGS_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);";

pub const CREATE_AI_CONVERSATIONS_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS ai_conversations (
    video_path TEXT NOT NULL,
    cue_index INTEGER NOT NULL,
    messages_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (video_path, cue_index)
);";

pub const CREATE_SPEECH_PROVIDER_PROFILE_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS speech_provider_profile (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider_kind TEXT NOT NULL,
    config_json TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL DEFAULT 0
);";
