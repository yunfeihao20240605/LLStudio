use crate::Segment;

pub const PLAYBACK_RANGE_MERGE_EPSILON_SECS: f64 = 0.02;

#[derive(Debug, Clone, PartialEq)]
pub struct LabelPlaybackPlan {
    pub label: String,
    pub ranges: Vec<els_types::TimeRange>,
    pub member_segment_ids: Vec<i64>,
}

pub fn build_label_playback_plan(segments: &[Segment], label: &str) -> Option<LabelPlaybackPlan> {
    let label = label.trim();
    if label.is_empty() {
        return None;
    }

    let mut members = segments
        .iter()
        .filter(|segment| segment.label.trim() == label)
        .cloned()
        .collect::<Vec<_>>();
    members.sort_by(|left, right| {
        left.range
            .start
            .total_cmp(&right.range.start)
            .then_with(|| left.range.end.total_cmp(&right.range.end))
            .then_with(|| left.id.cmp(&right.id))
    });
    if members.is_empty() {
        return None;
    }

    let mut ranges = Vec::<els_types::TimeRange>::new();
    for segment in &members {
        if let Some(current) = ranges.last_mut() {
            if segment.range.start <= current.end + PLAYBACK_RANGE_MERGE_EPSILON_SECS {
                current.end = current.end.max(segment.range.end);
                continue;
            }
        }
        ranges.push(segment.range);
    }

    Some(LabelPlaybackPlan {
        label: label.to_string(),
        ranges,
        member_segment_ids: members.iter().filter_map(|segment| segment.id).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::build_label_playback_plan;

    fn segment(id: i64, label: &str, start: f64, end: f64) -> crate::Segment {
        crate::Segment {
            id: Some(id),
            video_id: 1,
            range: els_types::TimeRange { start, end },
            repeat_count: 1,
            interval_seconds: 0.0,
            completed_loops: 0,
            label: label.to_string(),
        }
    }

    #[test]
    fn filters_sorts_and_merges_overlapping_ranges() {
        let segments = vec![
            segment(4, "场景2", 40.0, 50.0),
            segment(3, "场景1", 30.0, 35.0),
            segment(2, "场景1", 18.0, 25.0),
            segment(1, "场景1", 10.0, 20.0),
        ];
        let plan = build_label_playback_plan(&segments, "场景1").expect("playback plan");

        assert_eq!(plan.member_segment_ids, vec![1, 2, 3]);
        assert_eq!(
            plan.ranges,
            vec![
                els_types::TimeRange {
                    start: 10.0,
                    end: 25.0,
                },
                els_types::TimeRange {
                    start: 30.0,
                    end: 35.0,
                },
            ]
        );
    }

    #[test]
    fn merges_contained_adjacent_and_chained_ranges() {
        let segments = vec![
            segment(1, "A", 10.0, 20.0),
            segment(2, "A", 12.0, 15.0),
            segment(3, "A", 20.01, 25.0),
            segment(4, "A", 24.0, 30.0),
        ];
        let plan = build_label_playback_plan(&segments, "A").expect("playback plan");
        assert_eq!(
            plan.ranges,
            vec![els_types::TimeRange {
                start: 10.0,
                end: 30.0,
            }]
        );
    }

    #[test]
    fn rejects_empty_or_unknown_labels() {
        let segments = vec![segment(1, "A", 1.0, 2.0)];
        assert!(build_label_playback_plan(&segments, "").is_none());
        assert!(build_label_playback_plan(&segments, "B").is_none());
    }
}
