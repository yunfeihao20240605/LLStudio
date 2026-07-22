//! SRT/VTT 字幕解析模块。

/// 单条字幕数据。
#[derive(Debug, Clone, PartialEq)]
pub struct SubtitleCue {
    pub range: els_types::TimeRange,
    pub original_text: String,
    pub translated_text: Option<String>,
}

pub fn parse_subtitle_text(text: &str) -> els_types::AppResult<Vec<SubtitleCue>> {
    let normalized = text.replace("\r\n", "\n");
    let blocks = normalized
        .split("\n\n")
        .map(str::trim)
        .filter(|block| !block.is_empty());

    let mut cues = Vec::new();
    for block in blocks {
        if block.eq_ignore_ascii_case("WEBVTT") {
            continue;
        }

        let lines = block.lines().map(str::trim).collect::<Vec<_>>();
        if lines.is_empty() {
            continue;
        }

        let timing_index = lines
            .iter()
            .position(|line| line.contains("-->"))
            .ok_or_else(|| {
                els_types::AppError::Io(format!("subtitle block missing timing line: {block}"))
            })?;

        let timing_line = lines[timing_index];
        let range = parse_time_range(timing_line)?;

        let text_lines = lines
            .iter()
            .skip(timing_index + 1)
            .filter(|line| !line.is_empty())
            .map(|line| strip_tags(line))
            .collect::<Vec<_>>();

        if text_lines.is_empty() {
            continue;
        }

        let original_text = text_lines.first().cloned().unwrap_or_else(String::new);
        let translated_text = if text_lines.len() > 1 {
            Some(text_lines[1..].join("\n"))
        } else {
            None
        };

        cues.push(SubtitleCue {
            range,
            original_text,
            translated_text,
        });
    }

    Ok(cues)
}

fn parse_time_range(line: &str) -> els_types::AppResult<els_types::TimeRange> {
    let parts = line.split("-->").map(str::trim).collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(els_types::AppError::Io(format!(
            "invalid subtitle timing line: {line}"
        )));
    }

    let start = parse_timestamp(parts[0])?;
    let end = parse_timestamp(parts[1])?;
    if end < start {
        return Err(els_types::AppError::Io(format!(
            "subtitle end time precedes start time: {line}"
        )));
    }

    Ok(els_types::TimeRange { start, end })
}

fn parse_timestamp(timestamp: &str) -> els_types::AppResult<f64> {
    let main_part = timestamp.split_whitespace().next().unwrap_or_default();
    let normalized = main_part.replace(',', ".");
    let segments = normalized.split(':').collect::<Vec<_>>();
    if segments.len() != 3 {
        return Err(els_types::AppError::Io(format!(
            "invalid subtitle timestamp: {timestamp}"
        )));
    }

    let hours = segments[0]
        .parse::<f64>()
        .map_err(|err| els_types::AppError::Io(format!("invalid subtitle hour: {err}")))?;
    let minutes = segments[1]
        .parse::<f64>()
        .map_err(|err| els_types::AppError::Io(format!("invalid subtitle minute: {err}")))?;
    let seconds = segments[2]
        .parse::<f64>()
        .map_err(|err| els_types::AppError::Io(format!("invalid subtitle second: {err}")))?;

    Ok((hours * 3600.0) + (minutes * 60.0) + seconds)
}

fn strip_tags(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut in_tag = false;

    for ch in line.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::parse_subtitle_text;

    #[test]
    fn parses_srt_blocks_with_translation() {
        let input = "1\n00:00:01,000 --> 00:00:03,500\nHello world.\n你好，世界。\n\n2\n00:00:04,000 --> 00:00:05,000\nNext line.";
        let cues = parse_subtitle_text(input).expect("parse subtitle");

        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].original_text, "Hello world.");
        assert_eq!(cues[0].translated_text.as_deref(), Some("你好，世界。"));
        assert!((cues[1].range.start - 4.0).abs() < 0.001);
    }
}
