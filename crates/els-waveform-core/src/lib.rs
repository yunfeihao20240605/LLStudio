//! `els-waveform-core`：音频波形生成与分析核心业务逻辑。
//!
//! 当前实现直接通过 Rust FFmpeg bindings 调用：
//! - libavformat
//! - libavcodec
//!
//! 以进程内方式解码音轨并聚合真实波形，不再启动外部 `ffmpeg` 子进程。

mod analyzer;
mod generator;

pub use analyzer::{WaveformBin, WaveformData};
pub use generator::AudioSource;

use ffmpeg_next as ffmpeg;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const WAVEFORM_CACHE_MAGIC: &[u8] = b"ELS_WAVEFORM_CACHE_V3";
const DETAIL_CACHE_MAGIC: &[u8] = b"ELS_WAVEFORM_DETAIL_V1";
const PREVIEW_PROGRESS_CHUNK_BINS: usize = 1_024;
const PREVIEW_SAMPLE_RATE: usize = 8_000;
const PREVIEW_MIN_BINS: usize = 1_024;
const PREVIEW_MAX_BINS: usize = 131_072;
const PREVIEW_SECONDS_PER_BIN: f64 = 0.1;
pub const DETAIL_TILE_DURATION_SECS: f64 = 10.0;
pub const DETAIL_SECONDS_PER_BIN: f64 = 0.01;
const DETAIL_SEEK_PADDING_SECS: f64 = 0.05;

#[derive(Debug, Clone, PartialEq)]
pub struct WaveformTile {
    pub tile_index: u64,
    pub start_secs: f64,
    pub end_secs: f64,
    pub seconds_per_bin: f64,
    pub bins: Vec<WaveformBin>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FfmpegWaveformEngine;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaveformQuality {
    Preview,
    Full,
}

impl WaveformQuality {
    pub fn target_bins(self) -> usize {
        match self {
            Self::Preview => PREVIEW_MIN_BINS,
            Self::Full => 1_600,
        }
    }

    pub fn target_bins_for_duration(self, duration_secs: f64) -> usize {
        match self {
            Self::Preview => adaptive_preview_bins(duration_secs),
            Self::Full => self.target_bins(),
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Self::Preview => "preview",
            Self::Full => "full",
        }
    }
}

/// 波形引擎对外契约。
pub trait WaveformEngine {
    fn generate(&self, audio: &AudioSource) -> els_types::AppResult<WaveformData>;
    fn peaks_in_range(&self, data: &WaveformData, range: els_types::TimeRange) -> Vec<WaveformBin>;
}

impl WaveformEngine for FfmpegWaveformEngine {
    fn generate(&self, audio: &AudioSource) -> els_types::AppResult<WaveformData> {
        let duration_secs = if audio.duration_secs > 0.0 {
            audio.duration_secs
        } else {
            180.0
        };

        if let Some(video_path) = audio.video_path.as_deref() {
            if !video_path.trim().is_empty() {
                return self.generate_for_quality(video_path, duration_secs, audio.quality);
            }
        }

        Ok(WaveformData {
            duration_secs,
            bins: generate_demo_bins(
                duration_secs,
                audio.quality.target_bins_for_duration(duration_secs),
            ),
        })
    }

    fn peaks_in_range(&self, data: &WaveformData, range: els_types::TimeRange) -> Vec<WaveformBin> {
        if data.bins.is_empty() || data.duration_secs <= 0.0 {
            return Vec::new();
        }

        let start_ratio = (range.start / data.duration_secs).clamp(0.0, 1.0);
        let end_ratio = (range.end / data.duration_secs).clamp(start_ratio, 1.0);
        let len = data.bins.len();
        let start_index = ((len as f64) * start_ratio).floor() as usize;
        let end_index = (((len as f64) * end_ratio).ceil() as usize).max(start_index + 1);

        data.bins[start_index.min(len)..end_index.min(len)].to_vec()
    }
}

impl FfmpegWaveformEngine {
    pub fn generate_detail_tile(
        &self,
        video_path: &str,
        video_duration_secs: f64,
        tile_index: u64,
    ) -> els_types::AppResult<WaveformTile> {
        let Some((start_secs, end_secs)) = detail_tile_range(video_duration_secs, tile_index)
        else {
            return Err(els_types::AppError::InvalidArgument(format!(
                "detail tile {tile_index} is outside video duration {video_duration_secs}"
            )));
        };

        if let Some(tile) = load_detail_tile_cache(
            video_path,
            video_duration_secs,
            tile_index,
            start_secs,
            end_secs,
        )? {
            return Ok(tile);
        }

        let tile = generate_detail_tile_cli(video_path, tile_index, start_secs, end_secs)?;
        let _ = save_detail_tile_cache(video_path, video_duration_secs, &tile);
        Ok(tile)
    }

    pub fn generate_for_quality(
        &self,
        video_path: &str,
        duration_secs: f64,
        quality: WaveformQuality,
    ) -> els_types::AppResult<WaveformData> {
        let target_bins = quality.target_bins_for_duration(duration_secs);
        if let Some(cached) = load_waveform_cache(video_path, duration_secs, quality)? {
            return Ok(cached);
        }

        let data = match quality {
            WaveformQuality::Preview => {
                generate_preview_waveform_cli(video_path, duration_secs, target_bins, |_, _| {})?
            }
            WaveformQuality::Full => {
                generate_waveform_with_bindings(video_path, duration_secs, target_bins, |_, _| {})?
            }
        };
        let _ = save_waveform_cache(video_path, &data, quality);
        Ok(data)
    }

    pub fn generate_for_quality_with_progress<F>(
        &self,
        video_path: &str,
        duration_secs: f64,
        quality: WaveformQuality,
        mut on_progress: F,
    ) -> els_types::AppResult<WaveformData>
    where
        F: FnMut(&[WaveformBin], usize),
    {
        let target_bins = quality.target_bins_for_duration(duration_secs);
        if let Some(cached) = load_waveform_cache(video_path, duration_secs, quality)? {
            let loaded_bin_count = cached.bins.len();
            on_progress(&cached.bins, loaded_bin_count);
            return Ok(cached);
        }

        let data = match quality {
            WaveformQuality::Preview => generate_preview_waveform_cli(
                video_path,
                duration_secs,
                target_bins,
                |bins, loaded_bin_count| on_progress(bins, loaded_bin_count),
            )?,
            WaveformQuality::Full => generate_waveform_with_bindings(
                video_path,
                duration_secs,
                target_bins,
                |bins, loaded_bin_count| on_progress(bins, loaded_bin_count),
            )?,
        };
        let _ = save_waveform_cache(video_path, &data, quality);
        Ok(data)
    }
}

pub fn detail_tile_range(duration_secs: f64, tile_index: u64) -> Option<(f64, f64)> {
    if !duration_secs.is_finite() || duration_secs <= 0.0 {
        return None;
    }
    let start_secs = tile_index as f64 * DETAIL_TILE_DURATION_SECS;
    if start_secs >= duration_secs {
        return None;
    }
    Some((
        start_secs,
        (start_secs + DETAIL_TILE_DURATION_SECS).min(duration_secs),
    ))
}

pub fn detail_tile_indices(start_secs: f64, end_secs: f64, duration_secs: f64) -> Vec<u64> {
    if !start_secs.is_finite()
        || !end_secs.is_finite()
        || !duration_secs.is_finite()
        || duration_secs <= 0.0
    {
        return Vec::new();
    }

    let start = start_secs.clamp(0.0, duration_secs);
    let end = end_secs.clamp(start, duration_secs);
    if end <= start {
        return Vec::new();
    }

    let first = (start / DETAIL_TILE_DURATION_SECS).floor() as u64;
    // The range is half-open, so an exact tile boundary belongs to the next tile only.
    let last_time = next_down(end);
    let last = (last_time / DETAIL_TILE_DURATION_SECS).floor() as u64;
    (first..=last).collect()
}

fn next_down(value: f64) -> f64 {
    if value <= 0.0 {
        return value;
    }
    f64::from_bits(value.to_bits() - 1)
}

fn adaptive_preview_bins(duration_secs: f64) -> usize {
    let duration_secs = duration_secs.max(1.0);
    let bins = (duration_secs / PREVIEW_SECONDS_PER_BIN).ceil() as usize;
    let bins = bins.clamp(PREVIEW_MIN_BINS, PREVIEW_MAX_BINS);
    round_up_to_multiple(bins, 64).min(PREVIEW_MAX_BINS)
}

fn round_up_to_multiple(value: usize, multiple: usize) -> usize {
    if multiple == 0 {
        return value;
    }
    value.div_ceil(multiple) * multiple
}

fn generate_preview_waveform_cli<F>(
    video_path: &str,
    duration_secs: f64,
    target_bins: usize,
    mut on_progress: F,
) -> els_types::AppResult<WaveformData>
where
    F: FnMut(&[WaveformBin], usize),
{
    if !Path::new(video_path).exists() {
        return Err(els_types::AppError::Io(format!(
            "video file does not exist: {video_path}"
        )));
    }
    if target_bins == 0 {
        return Ok(WaveformData {
            duration_secs,
            bins: Vec::new(),
        });
    }

    let ffmpeg_bin = ffmpeg_cli_path();
    let mut child = Command::new(&ffmpeg_bin)
        .args([
            "-v",
            "error",
            "-i",
            video_path,
            "-map",
            "0:a:0",
            "-vn",
            "-ac",
            "1",
            "-ar",
            &PREVIEW_SAMPLE_RATE.to_string(),
            "-f",
            "f32le",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            els_types::AppError::Io(format!(
                "failed to start external ffmpeg preview decode with {}: {err}",
                ffmpeg_bin.display()
            ))
        })?;

    let mut stdout = child.stdout.take().ok_or_else(|| {
        els_types::AppError::Io("failed to capture ffmpeg stdout for waveform preview".to_string())
    })?;

    let expected_samples = (duration_secs.max(1.0) * PREVIEW_SAMPLE_RATE as f64).round() as usize;
    let mut bins = vec![
        WaveformBin {
            min: 1.0,
            max: -1.0
        };
        target_bins
    ];
    let mut touched_bins = vec![false; target_bins];
    let mut sample_index = 0_usize;
    let mut read_any_sample = false;
    let mut last_emitted_bin_count = 0_usize;
    let mut carry = Vec::<u8>::new();
    let mut buffer = [0_u8; 16 * 1024];

    loop {
        let read = stdout.read(&mut buffer).map_err(|err| {
            els_types::AppError::Io(format!(
                "failed reading ffmpeg preview waveform output: {err}"
            ))
        })?;
        if read == 0 {
            break;
        }

        carry.extend_from_slice(&buffer[..read]);
        let complete_len = carry.len() - (carry.len() % 4);
        for chunk in carry[..complete_len].chunks_exact(4) {
            let sample =
                f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]).clamp(-1.0, 1.0);
            read_any_sample = true;
            let bin_index = (sample_index.saturating_mul(target_bins) / expected_samples.max(1))
                .min(target_bins - 1);
            let bin = &mut bins[bin_index];
            if !touched_bins[bin_index] {
                bin.min = sample;
                bin.max = sample;
                touched_bins[bin_index] = true;
            } else {
                if sample < bin.min {
                    bin.min = sample;
                }
                if sample > bin.max {
                    bin.max = sample;
                }
            }
            sample_index += 1;
        }
        carry.drain(..complete_len);

        emit_progress(
            &bins,
            visible_bin_count(sample_index, expected_samples, target_bins),
            false,
            &mut last_emitted_bin_count,
            &mut on_progress,
        );
    }

    let output = child.wait_with_output().map_err(|err| {
        els_types::AppError::Io(format!("failed waiting for ffmpeg preview decode: {err}"))
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(els_types::AppError::Io(format!(
            "external ffmpeg preview decode failed: {}",
            stderr.trim()
        )));
    }
    if !carry.is_empty() {
        return Err(els_types::AppError::Io(
            "ffmpeg preview output ended with incomplete sample bytes".to_string(),
        ));
    }
    if !read_any_sample {
        return Err(els_types::AppError::Io(
            "external ffmpeg preview decode returned no audio samples".to_string(),
        ));
    }

    let normalized_bins = bins
        .into_iter()
        .zip(touched_bins)
        .map(|(bin, touched)| {
            if touched {
                bin
            } else {
                WaveformBin { min: 0.0, max: 0.0 }
            }
        })
        .collect::<Vec<_>>();

    emit_progress(
        &normalized_bins,
        normalized_bins.len(),
        true,
        &mut last_emitted_bin_count,
        &mut on_progress,
    );

    Ok(WaveformData {
        duration_secs,
        bins: normalized_bins,
    })
}

fn generate_detail_tile_cli(
    video_path: &str,
    tile_index: u64,
    start_secs: f64,
    end_secs: f64,
) -> els_types::AppResult<WaveformTile> {
    if !Path::new(video_path).exists() {
        return Err(els_types::AppError::Io(format!(
            "video file does not exist: {video_path}"
        )));
    }

    let seek_start = (start_secs - DETAIL_SEEK_PADDING_SECS).max(0.0);
    let decode_duration = end_secs - seek_start + DETAIL_SEEK_PADDING_SECS;
    let bin_count = ((end_secs - start_secs) / DETAIL_SECONDS_PER_BIN).ceil() as usize;
    let ffmpeg_bin = ffmpeg_cli_path();
    let mut child = Command::new(&ffmpeg_bin)
        .args([
            "-v",
            "error",
            "-ss",
            &format!("{seek_start:.6}"),
            "-i",
            video_path,
            "-t",
            &format!("{decode_duration:.6}"),
            "-map",
            "0:a:0",
            "-vn",
            "-ac",
            "1",
            "-ar",
            &PREVIEW_SAMPLE_RATE.to_string(),
            "-f",
            "f32le",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            els_types::AppError::Io(format!(
                "failed to start external ffmpeg detail decode with {}: {err}",
                ffmpeg_bin.display()
            ))
        })?;

    let mut stdout = child.stdout.take().ok_or_else(|| {
        els_types::AppError::Io("failed to capture ffmpeg stdout for detail waveform".to_string())
    })?;
    let mut bytes = Vec::new();
    stdout.read_to_end(&mut bytes).map_err(|err| {
        els_types::AppError::Io(format!(
            "failed reading ffmpeg detail waveform output: {err}"
        ))
    })?;
    let output = child.wait_with_output().map_err(|err| {
        els_types::AppError::Io(format!("failed waiting for ffmpeg detail decode: {err}"))
    })?;
    if !output.status.success() {
        return Err(els_types::AppError::Io(format!(
            "external ffmpeg detail decode failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    if bytes.len() % 4 != 0 {
        return Err(els_types::AppError::Io(
            "ffmpeg detail output ended with incomplete sample bytes".to_string(),
        ));
    }

    let mut bins = vec![
        WaveformBin {
            min: 1.0,
            max: -1.0
        };
        bin_count
    ];
    let mut touched = vec![false; bin_count];
    for (sample_index, chunk) in bytes.chunks_exact(4).enumerate() {
        let sample_time = seek_start + sample_index as f64 / PREVIEW_SAMPLE_RATE as f64;
        if sample_time < start_secs || sample_time >= end_secs {
            continue;
        }
        let bin_index = ((sample_time - start_secs) / DETAIL_SECONDS_PER_BIN).floor() as usize;
        let Some(bin) = bins.get_mut(bin_index) else {
            continue;
        };
        let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]).clamp(-1.0, 1.0);
        if touched[bin_index] {
            bin.min = bin.min.min(sample);
            bin.max = bin.max.max(sample);
        } else {
            bin.min = sample;
            bin.max = sample;
            touched[bin_index] = true;
        }
    }

    if !touched.iter().any(|value| *value) {
        return Err(els_types::AppError::Io(
            "external ffmpeg detail decode returned no audio samples".to_string(),
        ));
    }
    for (bin, was_touched) in bins.iter_mut().zip(touched) {
        if !was_touched {
            *bin = WaveformBin { min: 0.0, max: 0.0 };
        }
    }

    Ok(WaveformTile {
        tile_index,
        start_secs,
        end_secs,
        seconds_per_bin: DETAIL_SECONDS_PER_BIN,
        bins,
    })
}

fn ffmpeg_cli_path() -> PathBuf {
    if let Ok(value) = std::env::var("ELS_FFMPEG_BIN") {
        return PathBuf::from(value);
    }

    for candidate in [
        "/opt/local/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "/opt/homebrew/bin/ffmpeg",
    ] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return path;
        }
    }

    PathBuf::from("ffmpeg")
}

fn generate_waveform_with_bindings<F>(
    video_path: &str,
    duration_secs: f64,
    target_bins: usize,
    mut on_progress: F,
) -> els_types::AppResult<WaveformData>
where
    F: FnMut(&[WaveformBin], usize),
{
    if !Path::new(video_path).exists() {
        return Err(els_types::AppError::Io(format!(
            "video file does not exist: {video_path}"
        )));
    }

    ffmpeg::init().map_err(ffmpeg_error)?;

    let mut input = ffmpeg::format::input(video_path).map_err(ffmpeg_error)?;
    let stream = input
        .streams()
        .best(ffmpeg::media::Type::Audio)
        .ok_or(els_types::AppError::NotFound)?;
    let stream_index = stream.index();

    let context = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
        .map_err(ffmpeg_error)?;
    let mut decoder = context.decoder().audio().map_err(ffmpeg_error)?;
    let source_sample_rate = decoder.rate().max(1);
    let expected_samples = (duration_secs.max(1.0) * source_sample_rate as f64).round() as usize;
    let mut bins = vec![
        WaveformBin {
            min: 1.0,
            max: -1.0
        };
        target_bins
    ];
    let mut touched_bins = vec![false; target_bins];
    let mut mono_sample_index = 0_usize;
    let mut read_any_sample = false;
    let mut last_emitted_bin_count = 0_usize;

    let mut decoded = ffmpeg::frame::Audio::empty();
    for (current_stream, packet) in input.packets() {
        if current_stream.index() != stream_index {
            continue;
        }
        decoder.send_packet(&packet).map_err(ffmpeg_error)?;
        while decoder.receive_frame(&mut decoded).is_ok() {
            collect_frame_bins(
                &decoded,
                expected_samples,
                &mut bins,
                &mut touched_bins,
                &mut mono_sample_index,
                &mut read_any_sample,
            )?;

            emit_progress(
                &bins,
                visible_bin_count(mono_sample_index, expected_samples, target_bins),
                false,
                &mut last_emitted_bin_count,
                &mut on_progress,
            );
        }
    }

    decoder.send_eof().map_err(ffmpeg_error)?;
    while decoder.receive_frame(&mut decoded).is_ok() {
        collect_frame_bins(
            &decoded,
            expected_samples,
            &mut bins,
            &mut touched_bins,
            &mut mono_sample_index,
            &mut read_any_sample,
        )?;

        emit_progress(
            &bins,
            visible_bin_count(mono_sample_index, expected_samples, target_bins),
            false,
            &mut last_emitted_bin_count,
            &mut on_progress,
        );
    }

    if !read_any_sample {
        return Err(els_types::AppError::Io(
            "FFmpeg bindings did not return any decoded audio samples".to_string(),
        ));
    }

    let normalized_bins = bins
        .into_iter()
        .zip(touched_bins)
        .map(|(bin, touched)| {
            if touched {
                bin
            } else {
                WaveformBin { min: 0.0, max: 0.0 }
            }
        })
        .collect::<Vec<_>>();

    emit_progress(
        &normalized_bins,
        normalized_bins.len(),
        true,
        &mut last_emitted_bin_count,
        &mut on_progress,
    );

    Ok(WaveformData {
        duration_secs,
        bins: normalized_bins,
    })
}

fn emit_progress<F>(
    bins: &[WaveformBin],
    loaded_bin_count: usize,
    force: bool,
    last_emitted_bin_count: &mut usize,
    on_progress: &mut F,
) where
    F: FnMut(&[WaveformBin], usize),
{
    let loaded_bin_count = loaded_bin_count.min(bins.len());
    if loaded_bin_count == 0 {
        return;
    }
    if force && *last_emitted_bin_count == loaded_bin_count {
        return;
    }

    let should_emit = force
        || *last_emitted_bin_count == 0
        || loaded_bin_count
            >= (*last_emitted_bin_count + PREVIEW_PROGRESS_CHUNK_BINS).min(bins.len());
    if !should_emit {
        return;
    }

    on_progress(bins, loaded_bin_count);
    *last_emitted_bin_count = loaded_bin_count;
}

fn visible_bin_count(sample_index: usize, expected_samples: usize, total_bins: usize) -> usize {
    if total_bins == 0 {
        return 0;
    }
    let numerator = sample_index.saturating_mul(total_bins);
    let count = numerator.div_ceil(expected_samples.max(1));
    count.clamp(1, total_bins)
}

fn collect_frame_bins(
    frame: &ffmpeg::frame::Audio,
    expected_samples: usize,
    bins: &mut [WaveformBin],
    touched_bins: &mut [bool],
    mono_sample_index: &mut usize,
    read_any_sample: &mut bool,
) -> els_types::AppResult<()> {
    if frame.samples() == 0 || bins.is_empty() {
        return Ok(());
    }

    let channels = frame.channels() as usize;
    if channels == 0 {
        return Ok(());
    }

    for sample in iterate_mono_samples(frame)? {
        *read_any_sample = true;
        let bin_index = ((*mono_sample_index).saturating_mul(bins.len()) / expected_samples.max(1))
            .min(bins.len() - 1);
        let bin = &mut bins[bin_index];
        if !touched_bins[bin_index] {
            bin.min = sample;
            bin.max = sample;
            touched_bins[bin_index] = true;
        } else {
            if sample < bin.min {
                bin.min = sample;
            }
            if sample > bin.max {
                bin.max = sample;
            }
        }
        *mono_sample_index += 1;
    }

    Ok(())
}

fn iterate_mono_samples(frame: &ffmpeg::frame::Audio) -> els_types::AppResult<Vec<f32>> {
    use ffmpeg::format::sample::Type;
    use ffmpeg::format::Sample;

    let channels = frame.channels() as usize;
    let samples = frame.samples();
    let format = frame.format();

    let mut mono = Vec::with_capacity(samples);
    for sample_index in 0..samples {
        let mut mixed = 0.0_f32;
        for channel_index in 0..channels {
            let value = match format {
                Sample::U8(Type::Planar) => decode_u8(frame.data(channel_index), sample_index),
                Sample::I16(Type::Planar) => decode_i16(frame.data(channel_index), sample_index),
                Sample::I32(Type::Planar) => decode_i32(frame.data(channel_index), sample_index),
                Sample::I64(Type::Planar) => decode_i64(frame.data(channel_index), sample_index),
                Sample::F32(Type::Planar) => decode_f32(frame.data(channel_index), sample_index),
                Sample::F64(Type::Planar) => decode_f64(frame.data(channel_index), sample_index),
                Sample::U8(Type::Packed) => {
                    decode_u8(frame.data(0), sample_index * channels + channel_index)
                }
                Sample::I16(Type::Packed) => {
                    decode_i16(frame.data(0), sample_index * channels + channel_index)
                }
                Sample::I32(Type::Packed) => {
                    decode_i32(frame.data(0), sample_index * channels + channel_index)
                }
                Sample::I64(Type::Packed) => {
                    decode_i64(frame.data(0), sample_index * channels + channel_index)
                }
                Sample::F32(Type::Packed) => {
                    decode_f32(frame.data(0), sample_index * channels + channel_index)
                }
                Sample::F64(Type::Packed) => {
                    decode_f64(frame.data(0), sample_index * channels + channel_index)
                }
                Sample::None => {
                    return Err(els_types::AppError::InvalidArgument(
                        "unsupported audio sample format: none".to_string(),
                    ));
                }
            };
            mixed += value;
        }
        mono.push((mixed / channels as f32).clamp(-1.0, 1.0));
    }

    Ok(mono)
}

fn decode_u8(bytes: &[u8], index: usize) -> f32 {
    bytes
        .get(index)
        .map(|value| ((*value as f32) - 128.0) / 128.0)
        .unwrap_or(0.0)
}

fn decode_i16(bytes: &[u8], index: usize) -> f32 {
    decode_fixed_bytes::<2>(bytes, index, |value| {
        i16::from_le_bytes(value) as f32 / 32768.0
    })
}

fn decode_i32(bytes: &[u8], index: usize) -> f32 {
    decode_fixed_bytes::<4>(bytes, index, |value| {
        i32::from_le_bytes(value) as f32 / 2147483648.0
    })
}

fn decode_i64(bytes: &[u8], index: usize) -> f32 {
    decode_fixed_bytes::<8>(bytes, index, |value| {
        (i64::from_le_bytes(value) as f64 / 9223372036854775808.0) as f32
    })
}

fn decode_f32(bytes: &[u8], index: usize) -> f32 {
    decode_fixed_bytes::<4>(bytes, index, |value| f32::from_le_bytes(value)).clamp(-1.0, 1.0)
}

fn decode_f64(bytes: &[u8], index: usize) -> f32 {
    decode_fixed_bytes::<8>(bytes, index, |value| f64::from_le_bytes(value) as f32).clamp(-1.0, 1.0)
}

fn decode_fixed_bytes<const N: usize>(
    bytes: &[u8],
    index: usize,
    decode: impl FnOnce([u8; N]) -> f32,
) -> f32 {
    let start = index.saturating_mul(N);
    let end = start + N;
    let Some(chunk) = bytes.get(start..end) else {
        return 0.0;
    };

    let mut fixed = [0_u8; N];
    fixed.copy_from_slice(chunk);
    decode(fixed)
}

fn load_waveform_cache(
    video_path: &str,
    duration_secs: f64,
    quality: WaveformQuality,
) -> els_types::AppResult<Option<WaveformData>> {
    let cache_file = cache_file_path(video_path, quality)?;
    if !cache_file.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&cache_file)
        .map_err(|err| els_types::AppError::Io(format!("failed to read waveform cache: {err}")))?;
    if bytes.len() < WAVEFORM_CACHE_MAGIC.len() + 8 + 4 {
        return Ok(None);
    }
    if !bytes.starts_with(WAVEFORM_CACHE_MAGIC) {
        return Ok(None);
    }

    let mut cursor = WAVEFORM_CACHE_MAGIC.len();
    let cached_duration = read_f64(&bytes, &mut cursor)?;
    if (cached_duration - duration_secs).abs() > 1.0 {
        return Ok(None);
    }

    let bin_count = read_u32(&bytes, &mut cursor)? as usize;
    if bin_count != quality.target_bins_for_duration(duration_secs) {
        return Ok(None);
    }

    let expected_len = cursor + bin_count.saturating_mul(8);
    if bytes.len() != expected_len {
        return Ok(None);
    }

    let mut bins = Vec::with_capacity(bin_count);
    for _ in 0..bin_count {
        let min = read_f32(&bytes, &mut cursor)?;
        let max = read_f32(&bytes, &mut cursor)?;
        bins.push(WaveformBin { min, max });
    }

    if bins.is_empty() {
        return Ok(None);
    }

    Ok(Some(WaveformData {
        duration_secs: cached_duration,
        bins,
    }))
}

fn save_waveform_cache(
    video_path: &str,
    data: &WaveformData,
    quality: WaveformQuality,
) -> els_types::AppResult<()> {
    let cache_file = cache_file_path(video_path, quality)?;
    if let Some(parent) = cache_file.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            els_types::AppError::Io(format!("failed to create waveform cache dir: {err}"))
        })?;
    }

    let mut bytes = Vec::with_capacity(WAVEFORM_CACHE_MAGIC.len() + 8 + 4 + data.bins.len() * 8);
    bytes.extend_from_slice(WAVEFORM_CACHE_MAGIC);
    bytes.extend_from_slice(&data.duration_secs.to_le_bytes());
    bytes.extend_from_slice(&(data.bins.len() as u32).to_le_bytes());
    for bin in &data.bins {
        bytes.extend_from_slice(&bin.min.to_le_bytes());
        bytes.extend_from_slice(&bin.max.to_le_bytes());
    }

    let mut file = fs::File::create(&cache_file).map_err(|err| {
        els_types::AppError::Io(format!("failed to create waveform cache file: {err}"))
    })?;
    file.write_all(&bytes).map_err(|err| {
        els_types::AppError::Io(format!("failed to write waveform cache file: {err}"))
    })?;
    Ok(())
}

fn load_detail_tile_cache(
    video_path: &str,
    duration_secs: f64,
    tile_index: u64,
    start_secs: f64,
    end_secs: f64,
) -> els_types::AppResult<Option<WaveformTile>> {
    let cache_file = detail_cache_file_path(video_path, tile_index)?;
    if !cache_file.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&cache_file).map_err(|err| {
        els_types::AppError::Io(format!("failed to read detail waveform cache: {err}"))
    })?;
    if !bytes.starts_with(DETAIL_CACHE_MAGIC) {
        return Ok(None);
    }

    let mut cursor = DETAIL_CACHE_MAGIC.len();
    let cached_duration = read_f64(&bytes, &mut cursor)?;
    let cached_tile_index = read_u64(&bytes, &mut cursor)?;
    let cached_start = read_f64(&bytes, &mut cursor)?;
    let cached_end = read_f64(&bytes, &mut cursor)?;
    let seconds_per_bin = read_f64(&bytes, &mut cursor)?;
    let bin_count = read_u32(&bytes, &mut cursor)? as usize;
    let expected_bins = ((end_secs - start_secs) / DETAIL_SECONDS_PER_BIN).ceil() as usize;
    if (cached_duration - duration_secs).abs() > 1.0
        || cached_tile_index != tile_index
        || (cached_start - start_secs).abs() > f64::EPSILON
        || (cached_end - end_secs).abs() > f64::EPSILON
        || (seconds_per_bin - DETAIL_SECONDS_PER_BIN).abs() > f64::EPSILON
        || bin_count != expected_bins
        || bytes.len() != cursor + bin_count.saturating_mul(8)
    {
        return Ok(None);
    }

    let mut bins = Vec::with_capacity(bin_count);
    for _ in 0..bin_count {
        bins.push(WaveformBin {
            min: read_f32(&bytes, &mut cursor)?,
            max: read_f32(&bytes, &mut cursor)?,
        });
    }
    Ok(Some(WaveformTile {
        tile_index,
        start_secs: cached_start,
        end_secs: cached_end,
        seconds_per_bin,
        bins,
    }))
}

fn save_detail_tile_cache(
    video_path: &str,
    duration_secs: f64,
    tile: &WaveformTile,
) -> els_types::AppResult<()> {
    let cache_file = detail_cache_file_path(video_path, tile.tile_index)?;
    if let Some(parent) = cache_file.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            els_types::AppError::Io(format!("failed to create detail waveform cache dir: {err}"))
        })?;
    }
    let mut bytes = Vec::with_capacity(DETAIL_CACHE_MAGIC.len() + 44 + tile.bins.len() * 8);
    bytes.extend_from_slice(DETAIL_CACHE_MAGIC);
    bytes.extend_from_slice(&duration_secs.to_le_bytes());
    bytes.extend_from_slice(&tile.tile_index.to_le_bytes());
    bytes.extend_from_slice(&tile.start_secs.to_le_bytes());
    bytes.extend_from_slice(&tile.end_secs.to_le_bytes());
    bytes.extend_from_slice(&tile.seconds_per_bin.to_le_bytes());
    bytes.extend_from_slice(&(tile.bins.len() as u32).to_le_bytes());
    for bin in &tile.bins {
        bytes.extend_from_slice(&bin.min.to_le_bytes());
        bytes.extend_from_slice(&bin.max.to_le_bytes());
    }
    let temp_file = cache_file.with_extension(format!(
        "{}.tmp",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::write(&temp_file, bytes).map_err(|err| {
        els_types::AppError::Io(format!("failed to write detail waveform cache: {err}"))
    })?;
    fs::rename(&temp_file, &cache_file).map_err(|err| {
        let _ = fs::remove_file(&temp_file);
        els_types::AppError::Io(format!("failed to publish detail waveform cache: {err}"))
    })
}

fn read_f64(bytes: &[u8], cursor: &mut usize) -> els_types::AppResult<f64> {
    let value = bytes.get(*cursor..(*cursor + 8)).ok_or_else(|| {
        els_types::AppError::Io("waveform cache truncated while reading f64".into())
    })?;
    let mut raw = [0_u8; 8];
    raw.copy_from_slice(value);
    *cursor += 8;
    Ok(f64::from_le_bytes(raw))
}

fn read_f32(bytes: &[u8], cursor: &mut usize) -> els_types::AppResult<f32> {
    let value = bytes.get(*cursor..(*cursor + 4)).ok_or_else(|| {
        els_types::AppError::Io("waveform cache truncated while reading f32".into())
    })?;
    let mut raw = [0_u8; 4];
    raw.copy_from_slice(value);
    *cursor += 4;
    Ok(f32::from_le_bytes(raw))
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> els_types::AppResult<u32> {
    let value = bytes.get(*cursor..(*cursor + 4)).ok_or_else(|| {
        els_types::AppError::Io("waveform cache truncated while reading u32".into())
    })?;
    let mut raw = [0_u8; 4];
    raw.copy_from_slice(value);
    *cursor += 4;
    Ok(u32::from_le_bytes(raw))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> els_types::AppResult<u64> {
    let value = bytes.get(*cursor..(*cursor + 8)).ok_or_else(|| {
        els_types::AppError::Io("waveform cache truncated while reading u64".into())
    })?;
    let mut raw = [0_u8; 8];
    raw.copy_from_slice(value);
    *cursor += 8;
    Ok(u64::from_le_bytes(raw))
}

fn cache_file_path(video_path: &str, quality: WaveformQuality) -> els_types::AppResult<PathBuf> {
    let mut hasher = DefaultHasher::new();
    video_path.hash(&mut hasher);

    let metadata = fs::metadata(video_path).map_err(|err| {
        els_types::AppError::Io(format!(
            "failed to stat video file for waveform cache: {err}"
        ))
    })?;
    metadata.len().hash(&mut hasher);
    if let Ok(modified) = metadata.modified() {
        if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
            duration.as_secs().hash(&mut hasher);
            duration.subsec_nanos().hash(&mut hasher);
        }
    }
    quality.hash(&mut hasher);

    let cache_dir = waveform_cache_dir()?;
    Ok(cache_dir.join(format!(
        "{:016x}-{}.waveform",
        hasher.finish(),
        quality.tag()
    )))
}

fn detail_cache_file_path(video_path: &str, tile_index: u64) -> els_types::AppResult<PathBuf> {
    let mut hasher = DefaultHasher::new();
    video_path.hash(&mut hasher);
    let metadata = fs::metadata(video_path).map_err(|err| {
        els_types::AppError::Io(format!("failed to stat video file for detail cache: {err}"))
    })?;
    metadata.len().hash(&mut hasher);
    if let Ok(modified) = metadata.modified() {
        if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
            duration.as_secs().hash(&mut hasher);
            duration.subsec_nanos().hash(&mut hasher);
        }
    }
    "detail-10ms-v1".hash(&mut hasher);
    Ok(waveform_cache_dir()?.join(format!(
        "{:016x}-detail-10ms-tile-{tile_index:06}.waveform",
        hasher.finish()
    )))
}

fn waveform_cache_dir() -> els_types::AppResult<PathBuf> {
    if let Ok(value) = std::env::var("ELS_WAVEFORM_CACHE_DIR") {
        return Ok(PathBuf::from(value));
    }

    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home)
            .join("Library")
            .join("Caches")
            .join("EnglishLearningStudio")
            .join("waveforms"));
    }

    Ok(std::env::temp_dir()
        .join("EnglishLearningStudio")
        .join("waveforms"))
}

fn generate_demo_bins(duration_secs: f64, target_bins: usize) -> Vec<WaveformBin> {
    let mut bins = Vec::with_capacity(target_bins);
    for index in 0..target_bins {
        let x = index as f32 / target_bins as f32;
        let envelope = 0.16 + (x * std::f32::consts::PI * 6.0).sin().abs() * 0.58;
        let detail = ((x * 33.0).sin() * 0.18) + ((x * 71.0).cos() * 0.08);
        let amplitude = (envelope + detail).clamp(0.05, 0.94);
        let asymmetry = ((x * 19.0).cos() * 0.08).clamp(-0.08, 0.08);
        let min = (-amplitude + asymmetry).clamp(-1.0, 0.0);
        let max = (amplitude + asymmetry).clamp(0.0, 1.0);
        let _ = duration_secs;
        bins.push(WaveformBin { min, max });
    }
    bins
}

fn ffmpeg_error(error: ffmpeg::Error) -> els_types::AppError {
    els_types::AppError::Io(format!("ffmpeg error: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{
        cache_file_path, detail_tile_indices, detail_tile_range, generate_detail_tile_cli,
        load_detail_tile_cache, load_waveform_cache, save_detail_tile_cache, save_waveform_cache,
        AudioSource, FfmpegWaveformEngine, WaveformBin, WaveformEngine, WaveformQuality,
        WaveformTile, DETAIL_SECONDS_PER_BIN, WAVEFORM_CACHE_MAGIC,
    };
    use std::f32::consts::PI;
    use std::fs;

    #[test]
    fn generates_non_empty_demo_waveform() {
        let engine = FfmpegWaveformEngine;
        let data = engine
            .generate(&AudioSource::default())
            .expect("generate waveform");

        assert_eq!(data.duration_secs, 180.0);
        assert_eq!(
            data.bins.len(),
            WaveformQuality::Preview.target_bins_for_duration(180.0)
        );
    }

    #[test]
    fn extracts_bins_for_selected_range() {
        let engine = FfmpegWaveformEngine;
        let data = engine
            .generate(&AudioSource::default())
            .expect("generate waveform");

        let peaks = engine.peaks_in_range(
            &data,
            els_types::TimeRange {
                start: 10.0,
                end: 20.0,
            },
        );

        assert!(!peaks.is_empty());
        assert!(peaks.len() < data.bins.len());
    }

    #[test]
    fn waveform_cache_roundtrip() {
        let cache_dir =
            std::env::temp_dir().join(format!("els-waveform-cache-test-{}", std::process::id()));
        std::env::set_var("ELS_WAVEFORM_CACHE_DIR", &cache_dir);

        let video_path = cache_dir.join("demo.mp4");
        fs::create_dir_all(&cache_dir).expect("create cache dir");
        fs::write(&video_path, b"demo").expect("write demo file");

        let data = crate::WaveformData {
            duration_secs: 12.0,
            bins: (0..WaveformQuality::Preview.target_bins_for_duration(12.0))
                .map(|index| {
                    if index % 2 == 0 {
                        WaveformBin {
                            min: -0.5,
                            max: 0.4,
                        }
                    } else {
                        WaveformBin {
                            min: -0.2,
                            max: 0.7,
                        }
                    }
                })
                .collect(),
        };
        save_waveform_cache(
            &video_path.to_string_lossy(),
            &data,
            WaveformQuality::Preview,
        )
        .expect("save cache");

        let restored = load_waveform_cache(
            &video_path.to_string_lossy(),
            12.0,
            WaveformQuality::Preview,
        )
        .expect("load cache")
        .expect("cache hit");

        let cache_file = cache_file_path(&video_path.to_string_lossy(), WaveformQuality::Preview)
            .expect("cache file path");
        let cache_bytes = fs::read(cache_file).expect("read cache bytes");

        assert!(cache_bytes.starts_with(WAVEFORM_CACHE_MAGIC));
        assert_eq!(
            restored.bins.len(),
            WaveformQuality::Preview.target_bins_for_duration(12.0)
        );
        assert_eq!(restored.bins[0], data.bins[0]);

        let detail_tile = WaveformTile {
            tile_index: 1,
            start_secs: 10.0,
            end_secs: 12.0,
            seconds_per_bin: DETAIL_SECONDS_PER_BIN,
            bins: vec![
                WaveformBin {
                    min: -0.3,
                    max: 0.6
                };
                200
            ],
        };
        save_detail_tile_cache(&video_path.to_string_lossy(), 12.0, &detail_tile)
            .expect("save detail cache");
        let restored_detail =
            load_detail_tile_cache(&video_path.to_string_lossy(), 12.0, 1, 10.0, 12.0)
                .expect("load detail cache")
                .expect("detail cache hit");
        assert_eq!(restored_detail, detail_tile);

        let _ = fs::remove_dir_all(cache_dir);
        std::env::remove_var("ELS_WAVEFORM_CACHE_DIR");
    }

    #[test]
    fn generates_waveform_from_real_audio_file() {
        let temp_dir = std::env::temp_dir().join(format!(
            "els-waveform-real-audio-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&temp_dir).expect("create temp dir");
        let audio_path = temp_dir.join("tone.wav");
        write_test_wav_file(&audio_path).expect("write wav file");

        let engine = FfmpegWaveformEngine;
        let data = engine
            .generate(&AudioSource {
                video_path: Some(audio_path.to_string_lossy().to_string()),
                duration_secs: 1.0,
                quality: WaveformQuality::Preview,
            })
            .expect("generate waveform from audio file");

        assert_eq!(
            data.bins.len(),
            WaveformQuality::Preview.target_bins_for_duration(1.0)
        );
        assert!(data.bins.iter().any(|bin| bin.max > 0.1));
        assert!(data.bins.iter().any(|bin| bin.min < -0.1));

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn preview_progress_reports_multiple_chunks() {
        let temp_dir =
            std::env::temp_dir().join(format!("els-waveform-progress-test-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).expect("create temp dir");
        let audio_path = temp_dir.join("tone.wav");
        write_test_wav_file(&audio_path).expect("write wav file");

        let engine = FfmpegWaveformEngine;
        let mut progress = Vec::new();
        let data = engine
            .generate_for_quality_with_progress(
                &audio_path.to_string_lossy(),
                1.0,
                WaveformQuality::Preview,
                |bins, loaded| progress.push((bins.len(), loaded)),
            )
            .expect("generate waveform with progress");

        assert_eq!(
            data.bins.len(),
            WaveformQuality::Preview.target_bins_for_duration(1.0)
        );
        assert!(progress.len() >= 2);
        assert_eq!(
            progress.last().map(|(_, loaded)| *loaded),
            Some(data.bins.len())
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    fn write_test_wav_file(path: &std::path::Path) -> std::io::Result<()> {
        let sample_rate = 8_000_u32;
        let duration_secs = 1_u32;
        let sample_count = sample_rate * duration_secs;
        let bytes_per_sample = 2_u16;
        let channels = 1_u16;
        let byte_rate = sample_rate * channels as u32 * bytes_per_sample as u32;
        let block_align = channels * bytes_per_sample;
        let data_size = sample_count * bytes_per_sample as u32;

        let mut bytes = Vec::with_capacity((44 + data_size) as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());

        for index in 0..sample_count {
            let phase = (index as f32 / sample_rate as f32) * 2.0 * PI * 440.0;
            let sample = (phase.sin() * i16::MAX as f32 * 0.6) as i16;
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        fs::write(path, bytes)
    }

    #[test]
    fn preview_bins_scale_with_duration() {
        assert_eq!(
            WaveformQuality::Preview.target_bins_for_duration(60.0),
            1_024
        );
        assert_eq!(
            WaveformQuality::Preview.target_bins_for_duration(3_600.0),
            36_032
        );
        assert_eq!(
            WaveformQuality::Preview.target_bins_for_duration(6_600.0),
            66_048
        );
        assert_eq!(
            WaveformQuality::Preview.target_bins_for_duration(20_000.0),
            131_072
        );
    }

    #[test]
    fn detail_tiles_use_half_open_ranges() {
        assert_eq!(detail_tile_range(240.0, 22), Some((220.0, 230.0)));
        assert_eq!(detail_tile_range(225.25, 22), Some((220.0, 225.25)));
        assert_eq!(detail_tile_range(225.25, 23), None);
        assert_eq!(detail_tile_indices(227.0, 234.0, 240.0), vec![22, 23]);
        assert_eq!(detail_tile_indices(230.0, 231.0, 240.0), vec![23]);
        assert_eq!(detail_tile_indices(220.0, 230.0, 240.0), vec![22]);
    }

    #[test]
    fn generates_ten_millisecond_detail_bins() {
        let temp_dir =
            std::env::temp_dir().join(format!("els-waveform-detail-test-{}", std::process::id()));
        fs::create_dir_all(&temp_dir).expect("create temp dir");
        let audio_path = temp_dir.join("tone.wav");
        write_test_wav_file(&audio_path).expect("write wav file");

        let tile = generate_detail_tile_cli(&audio_path.to_string_lossy(), 0, 0.0, 1.0)
            .expect("generate detail tile");
        assert_eq!(tile.bins.len(), 100);
        assert_eq!(tile.seconds_per_bin, DETAIL_SECONDS_PER_BIN);
        assert!(tile.bins.iter().any(|bin| bin.max > 0.1));
        assert!(tile.bins.iter().any(|bin| bin.min < -0.1));

        let _ = fs::remove_dir_all(temp_dir);
    }
}
