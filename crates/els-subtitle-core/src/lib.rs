//! `els-subtitle-core`：字幕解析与按时间戳查询的核心业务逻辑。
//!
//! 依赖方向规则：只依赖 `els-types`。字幕跟随播放进度高亮显示的协作逻辑
//! 不写在本 crate 内（避免依赖 `els-media-core`），而是放在 `els-qt-bridge`
//! 或 `els-app` 组合根，由调用方把当前播放时间戳传进来查询。

mod parser;

pub use parser::{parse_subtitle_text, SubtitleCue};

const CUE_START_MATCH_TOLERANCE_SECS: f64 = 0.005;

#[derive(Debug, Default, Clone)]
pub struct SubtitleTrack {
    cues: Vec<SubtitleCue>,
}

/// 字幕数据提供者对外契约。
pub trait SubtitleProvider {
    fn load(&mut self, path: &str) -> els_types::AppResult<()>;
    fn cue_at(&self, timestamp_secs: f64) -> Option<&SubtitleCue>;
}

impl SubtitleTrack {
    pub fn cues(&self) -> &[SubtitleCue] {
        &self.cues
    }

    pub fn load_from_text(&mut self, text: &str) -> els_types::AppResult<()> {
        self.cues = parse_subtitle_text(text)?;
        self.sort_cues();
        Ok(())
    }

    pub fn replace_cues(&mut self, cues: Vec<SubtitleCue>) {
        self.cues = cues;
        self.sort_cues();
    }

    pub fn cue_index_for_range(&self, range: els_types::TimeRange) -> Option<usize> {
        if !valid_range(range) {
            return None;
        }

        self.cues
            .iter()
            .position(|cue| cue_start_in_range(cue, range))
    }

    /// 返回时间点对应的唯一字幕。允许极小的起点误差；字幕重叠时，
    /// 选择开始时间最晚的一条，使新字幕在其起点处优先显示。
    pub fn cue_index_at(&self, timestamp_secs: f64) -> Option<usize> {
        if !timestamp_secs.is_finite() {
            return None;
        }

        self.cues
            .iter()
            .enumerate()
            .filter(|(_, cue)| {
                timestamp_secs + CUE_START_MATCH_TOLERANCE_SECS >= cue.range.start
                    && timestamp_secs < cue.range.end
            })
            .max_by(|(_, left), (_, right)| left.range.start.total_cmp(&right.range.start))
            .map(|(index, _)| index)
    }

    pub fn add_or_update_for_range(
        &mut self,
        range: els_types::TimeRange,
        text: &str,
    ) -> els_types::AppResult<usize> {
        let text = text.trim();
        if !valid_range(range) || text.is_empty() {
            return Err(els_types::AppError::InvalidArgument(
                "subtitle requires a valid time range and non-empty text".to_string(),
            ));
        }

        if let Some(index) = self.cue_index_for_range(range) {
            self.cues[index].original_text = text.to_string();
            self.cues[index].translated_text = None;
            return Ok(index);
        }

        self.cues.push(SubtitleCue {
            range,
            original_text: text.to_string(),
            translated_text: None,
        });
        self.sort_cues();
        self.cues
            .iter()
            .position(|cue| cue.range == range && cue.original_text == text)
            .ok_or(els_types::AppError::NotFound)
    }

    pub fn update_cue_range(
        &mut self,
        index: usize,
        range: els_types::TimeRange,
    ) -> els_types::AppResult<usize> {
        if index >= self.cues.len() {
            return Err(els_types::AppError::NotFound);
        }
        if !valid_range(range) {
            return Err(els_types::AppError::InvalidArgument(
                "subtitle requires a valid time range".to_string(),
            ));
        }
        let original_text = self.cues[index].original_text.clone();
        let translated_text = self.cues[index].translated_text.clone();
        self.cues[index].range = range;
        self.sort_cues();
        self.cues
            .iter()
            .position(|cue| {
                cue.range == range
                    && cue.original_text == original_text
                    && cue.translated_text == translated_text
            })
            .ok_or(els_types::AppError::NotFound)
    }

    pub fn remove_cue(&mut self, index: usize) -> els_types::AppResult<()> {
        if index >= self.cues.len() {
            return Err(els_types::AppError::NotFound);
        }
        self.cues.remove(index);
        Ok(())
    }

    pub fn remove_cues_for_range(
        &mut self,
        range: els_types::TimeRange,
    ) -> els_types::AppResult<usize> {
        if !valid_range(range) {
            return Err(els_types::AppError::InvalidArgument(
                "subtitle requires a valid time range".to_string(),
            ));
        }

        let original_count = self.cues.len();
        self.cues.retain(|cue| !cue_start_in_range(cue, range));
        Ok(original_count - self.cues.len())
    }

    pub fn to_srt(&self) -> String {
        let blocks = self
            .cues
            .iter()
            .enumerate()
            .map(|(index, cue)| {
                let mut text = cue.original_text.clone();
                if let Some(translated) = cue
                    .translated_text
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    text.push('\n');
                    text.push_str(translated);
                }
                format!(
                    "{}\n{} --> {}\n{}",
                    index + 1,
                    format_srt_timestamp(cue.range.start),
                    format_srt_timestamp(cue.range.end),
                    text
                )
            })
            .collect::<Vec<_>>();
        if blocks.is_empty() {
            String::new()
        } else {
            format!("{}\n", blocks.join("\n\n"))
        }
    }

    pub fn save_srt(&self, path: &str) -> els_types::AppResult<()> {
        std::fs::write(path, self.to_srt())
            .map_err(|err| els_types::AppError::Io(format!("failed to save subtitle file: {err}")))
    }

    fn sort_cues(&mut self) {
        self.cues.sort_by(|left, right| {
            left.range
                .start
                .total_cmp(&right.range.start)
                .then_with(|| left.range.end.total_cmp(&right.range.end))
        });
    }
}

impl SubtitleProvider for SubtitleTrack {
    fn load(&mut self, path: &str) -> els_types::AppResult<()> {
        let text = std::fs::read_to_string(path).map_err(|err| {
            els_types::AppError::Io(format!("failed to read subtitle file: {err}"))
        })?;
        self.load_from_text(&text)
    }

    fn cue_at(&self, timestamp_secs: f64) -> Option<&SubtitleCue> {
        self.cue_index_at(timestamp_secs)
            .and_then(|index| self.cues.get(index))
    }
}

fn valid_range(range: els_types::TimeRange) -> bool {
    range.start.is_finite()
        && range.end.is_finite()
        && range.start >= 0.0
        && range.end > range.start
}

fn cue_start_in_range(cue: &SubtitleCue, range: els_types::TimeRange) -> bool {
    cue.range.start >= range.start - CUE_START_MATCH_TOLERANCE_SECS && cue.range.start < range.end
}

fn format_srt_timestamp(seconds: f64) -> String {
    let total_millis = (seconds.max(0.0) * 1_000.0).round() as u64;
    let hours = total_millis / 3_600_000;
    let minutes = (total_millis % 3_600_000) / 60_000;
    let secs = (total_millis % 60_000) / 1_000;
    let millis = total_millis % 1_000;
    format!("{hours:02}:{minutes:02}:{secs:02},{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::{SubtitleProvider, SubtitleTrack};
    use els_types::TimeRange;

    #[test]
    fn cue_query_returns_matching_subtitle() {
        let mut track = SubtitleTrack::default();
        track
            .load_from_text(
                "1\n00:00:01,000 --> 00:00:03,000\nHello.\n\n2\n00:00:04,000 --> 00:00:05,000\nBye.",
            )
            .expect("load subtitle text");

        let cue = track.cue_at(2.0).expect("cue at 2.0");
        assert_eq!(cue.original_text, "Hello.");
        assert!(track.cue_at(3.5).is_none());
    }

    #[test]
    fn selection_adds_or_updates_and_serializes_srt() {
        let mut track = SubtitleTrack::default();
        track
            .add_or_update_for_range(
                TimeRange {
                    start: 80.5,
                    end: 86.2,
                },
                "原来的文字",
            )
            .expect("add first cue");
        track
            .add_or_update_for_range(
                TimeRange {
                    start: 87.0,
                    end: 91.5,
                },
                "听写的文字",
            )
            .expect("add second cue");

        assert_eq!(
            track.to_srt(),
            "1\n00:01:20,500 --> 00:01:26,200\n原来的文字\n\n2\n00:01:27,000 --> 00:01:31,500\n听写的文字\n"
        );

        let updated_index = track
            .add_or_update_for_range(
                TimeRange {
                    start: 87.003,
                    end: 91.504,
                },
                "修改后的文字",
            )
            .expect("update overlapping cue");
        assert_eq!(updated_index, 1);
        assert_eq!(track.cues().len(), 2);
        assert_eq!(track.cues()[1].original_text, "修改后的文字");
        assert_eq!(track.cues()[1].range.start, 87.0);

        assert_eq!(
            track.cue_index_for_range(TimeRange {
                start: 91.657,
                end: 95.0,
            }),
            None
        );
        track
            .add_or_update_for_range(
                TimeRange {
                    start: 91.657,
                    end: 95.0,
                },
                "相邻的新字幕",
            )
            .expect("add adjacent cue");
        assert_eq!(track.cues().len(), 3);
        assert_eq!(track.cues()[2].original_text, "相邻的新字幕");
    }

    #[test]
    fn updating_cue_range_preserves_text_and_reorders_track() {
        let mut track = SubtitleTrack::default();
        track
            .load_from_text(
                "1\n00:00:01,000 --> 00:00:02,000\nFirst\n\n2\n00:00:03,000 --> 00:00:04,000\nSecond",
            )
            .expect("load cues");
        let updated_index = track
            .update_cue_range(
                1,
                TimeRange {
                    start: 0.25,
                    end: 0.75,
                },
            )
            .expect("update range");
        assert_eq!(updated_index, 0);
        assert_eq!(track.cues()[0].original_text, "Second");
        assert_eq!(track.cues()[0].range.start, 0.25);
        assert_eq!(track.cues()[0].range.end, 0.75);
    }

    #[test]
    fn assigns_a_cue_to_the_segment_where_the_cue_starts() {
        let mut track = SubtitleTrack::default();
        track
            .add_or_update_for_range(
                TimeRange {
                    start: 230.345,
                    end: 233.164,
                },
                "okay, so I was Miranda's second assistant.",
            )
            .expect("add source cue");

        assert_eq!(
            track.cue_index_for_range(TimeRange {
                start: 230.345,
                end: 232.850,
            }),
            Some(0)
        );
        assert_eq!(
            track.cue_index_for_range(TimeRange {
                start: 233.007,
                end: 235.0,
            }),
            None
        );
    }

    #[test]
    fn removes_all_cues_that_start_in_a_segment() {
        let mut track = SubtitleTrack::default();
        track
            .load_from_text(
                "1\n00:00:01,000 --> 00:00:02,000\nBefore\n\n\
                 2\n00:00:03,000 --> 00:00:04,000\nFirst\n\n\
                 3\n00:00:05,500 --> 00:00:06,000\nSecond\n\n\
                 4\n00:00:08,000 --> 00:00:09,000\nAfter",
            )
            .expect("load cues");

        let removed = track
            .remove_cues_for_range(TimeRange {
                start: 3.0,
                end: 8.0,
            })
            .expect("remove segment cues");

        assert_eq!(removed, 2);
        assert_eq!(track.cues().len(), 2);
        assert_eq!(track.cues()[0].original_text, "Before");
        assert_eq!(track.cues()[1].original_text, "After");
    }

    #[test]
    fn prefers_the_latest_starting_cue_in_an_overlap() {
        let mut track = SubtitleTrack::default();
        track
            .add_or_update_for_range(
                TimeRange {
                    start: 230.345,
                    end: 233.164,
                },
                "previous subtitle",
            )
            .expect("add previous cue");
        track
            .add_or_update_for_range(
                TimeRange {
                    start: 233.007,
                    end: 235.010,
                },
                "current subtitle",
            )
            .expect("add overlapping cue");

        assert_eq!(track.cue_index_at(233.000), Some(0));
        assert_eq!(track.cue_index_at(233.006646), Some(1));
        assert_eq!(track.cue_index_at(233.100), Some(1));
        assert_eq!(track.cue_index_at(235.010), None);
        assert_eq!(track.cue_index_at(f64::NAN), None);
    }
}
