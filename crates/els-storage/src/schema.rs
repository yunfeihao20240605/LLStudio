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
    name TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL DEFAULT 0
);";

pub const CREATE_SEGMENT_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS segment (
    id INTEGER PRIMARY KEY,
    video_id INTEGER NOT NULL,
    start_time REAL NOT NULL,
    end_time REAL NOT NULL,
    repeat_count INTEGER NOT NULL,
    interval_seconds INTEGER NOT NULL DEFAULT 0,
    completed_loops INTEGER NOT NULL DEFAULT 0
);";

pub const CREATE_SETTINGS_TABLE: &str = "\
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);";
