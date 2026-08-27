//! 麦克风采集和 WAV 文件写入适配器。

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CHANNEL_SECONDS: usize = 2;

pub struct MicrophoneRecorder;

pub struct ActiveCapture {
    stream: Stream,
    writer: JoinHandle<els_types::AppResult<u64>>,
    path: PathBuf,
    sample_rate: u32,
    started_at: Instant,
}

#[derive(Clone, Debug)]
pub struct CapturedAudio {
    pub path: PathBuf,
    pub duration_secs: f64,
    pub sample_rate: u32,
}

impl MicrophoneRecorder {
    pub fn start(path: impl AsRef<Path>) -> els_types::AppResult<ActiveCapture> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(io_error)?;
        }

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| els_types::AppError::Io("没有可用的麦克风输入设备".to_string()))?;
        let supported = device
            .default_input_config()
            .map_err(|error| els_types::AppError::Io(format!("读取麦克风配置失败：{error}")))?;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.into();
        let sample_rate = config.sample_rate.0;
        let channels = usize::from(config.channels);
        if channels == 0 || sample_rate == 0 {
            return Err(els_types::AppError::Io(
                "麦克风返回了无效的音频配置".to_string(),
            ));
        }

        let capacity = (sample_rate as usize).saturating_mul(CHANNEL_SECONDS);
        let (sender, receiver) = mpsc::sync_channel::<f32>(capacity.max(1));
        let writer_path = path.clone();
        let writer = thread::spawn(move || write_wav(writer_path, sample_rate, receiver));
        let error_callback = |error| eprintln!("麦克风采集错误：{error}");

        let stream = match sample_format {
            SampleFormat::F32 => {
                let sender = sender.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| send_f32_frames(data, channels, &sender),
                    error_callback,
                    None,
                )
            }
            SampleFormat::I16 => {
                let sender = sender.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _| send_i16_frames(data, channels, &sender),
                    error_callback,
                    None,
                )
            }
            SampleFormat::U16 => device.build_input_stream(
                &config,
                move |data: &[u16], _| send_u16_frames(data, channels, &sender),
                error_callback,
                None,
            ),
            format => {
                return Err(els_types::AppError::Io(format!(
                    "暂不支持麦克风采样格式：{format:?}"
                )))
            }
        }
        .map_err(|error| els_types::AppError::Io(format!("打开麦克风失败：{error}")))?;

        stream
            .play()
            .map_err(|error| els_types::AppError::Io(format!("启动麦克风失败：{error}")))?;

        Ok(ActiveCapture {
            stream,
            writer,
            path,
            sample_rate,
            started_at: Instant::now(),
        })
    }
}

impl ActiveCapture {
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn stop(self) -> els_types::AppResult<CapturedAudio> {
        let ActiveCapture {
            stream,
            writer,
            path,
            sample_rate,
            ..
        } = self;
        drop(stream);
        let sample_count = writer
            .join()
            .map_err(|_| els_types::AppError::Io("录音写入线程异常退出".to_string()))??;
        if sample_count == 0 {
            return Err(els_types::AppError::Io("没有采集到麦克风音频".to_string()));
        }
        Ok(CapturedAudio {
            path,
            duration_secs: sample_count as f64 / sample_rate as f64,
            sample_rate,
        })
    }
}

pub fn next_recording_path(video_id: i64) -> els_types::AppResult<PathBuf> {
    let root = recordings_root(
        std::env::var_os("ELS_RECORDINGS_DIR"),
        els_types::app_data_directory(),
        std::env::temp_dir(),
    );
    let directory = root.join(video_id.to_string());
    std::fs::create_dir_all(&directory).map_err(io_error)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(directory.join(format!("recording-{timestamp}.wav")))
}

fn recordings_root(
    configured_root: Option<std::ffi::OsString>,
    app_data_directory: Option<PathBuf>,
    temp_directory: PathBuf,
) -> PathBuf {
    configured_root
        .map(PathBuf::from)
        .or_else(|| app_data_directory.map(|path| path.join("recordings")))
        .unwrap_or_else(|| temp_directory.join("LLStudio").join("recordings"))
}

fn write_wav(
    path: PathBuf,
    sample_rate: u32,
    receiver: mpsc::Receiver<f32>,
) -> els_types::AppResult<u64> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|error| els_types::AppError::Io(format!("创建录音文件失败：{error}")))?;
    let mut sample_count = 0_u64;
    while let Ok(sample) = receiver.recv() {
        let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
        writer
            .write_sample(value)
            .map_err(|error| els_types::AppError::Io(format!("写入录音失败：{error}")))?;
        sample_count = sample_count.saturating_add(1);
    }
    writer
        .finalize()
        .map_err(|error| els_types::AppError::Io(format!("完成录音文件失败：{error}")))?;
    Ok(sample_count)
}

fn send_f32_frames(data: &[f32], channels: usize, sender: &SyncSender<f32>) {
    send_mono_frames(data, channels, sender, |sample| *sample);
}

fn send_i16_frames(data: &[i16], channels: usize, sender: &SyncSender<f32>) {
    send_mono_frames(data, channels, sender, |sample| {
        *sample as f32 / i16::MAX as f32
    });
}

fn send_u16_frames(data: &[u16], channels: usize, sender: &SyncSender<f32>) {
    send_mono_frames(data, channels, sender, |sample| {
        (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0
    });
}

fn send_mono_frames<T>(
    data: &[T],
    channels: usize,
    sender: &SyncSender<f32>,
    convert: impl Fn(&T) -> f32,
) {
    for frame in data.chunks_exact(channels) {
        let mono = frame.iter().map(&convert).sum::<f32>() / channels as f32;
        let _ = sender.try_send(mono);
    }
}

fn io_error(error: std::io::Error) -> els_types::AppError {
    els_types::AppError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{next_recording_path, recordings_root, write_wav};
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::sync::mpsc;

    #[test]
    fn writes_mono_pcm_wav_with_the_expected_duration() {
        let path = std::env::temp_dir().join(format!(
            "els-capture-{}-{}.wav",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let (sender, receiver) = mpsc::channel();
        for sample in [0.0, 0.5, -0.5, 1.0] {
            sender.send(sample).expect("sample");
        }
        drop(sender);
        assert_eq!(write_wav(path.clone(), 4, receiver).expect("write"), 4);

        let reader = hound::WavReader::open(&path).expect("open wav");
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().sample_rate, 4);
        assert_eq!(reader.duration(), 4);
        drop(reader);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn recording_root_prefers_override_then_app_data_then_temp() {
        let temp_directory = PathBuf::from("/tmp/els-capture-test");
        let app_data = PathBuf::from("/app-data/LLStudio");

        assert_eq!(
            recordings_root(
                Some(OsString::from("/custom-recordings")),
                Some(app_data.clone()),
                temp_directory.clone(),
            ),
            PathBuf::from("/custom-recordings")
        );
        assert_eq!(
            recordings_root(None, Some(app_data.clone()), temp_directory.clone()),
            app_data.join("recordings")
        );
        assert_eq!(
            recordings_root(None, None, temp_directory),
            PathBuf::from("/tmp/els-capture-test/LLStudio/recordings")
        );
    }

    #[test]
    fn next_recording_path_creates_video_directory() {
        let root = std::env::temp_dir().join(format!(
            "els-recordings-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let previous = std::env::var_os("ELS_RECORDINGS_DIR");
        std::env::set_var("ELS_RECORDINGS_DIR", &root);

        let path = next_recording_path(42).expect("recording path");
        let expected_directory = root.join("42");
        assert_eq!(path.parent(), Some(expected_directory.as_path()));
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("recording-"));

        match previous {
            Some(value) => std::env::set_var("ELS_RECORDINGS_DIR", value),
            None => std::env::remove_var("ELS_RECORDINGS_DIR"),
        }
        let _ = std::fs::remove_dir_all(root);
    }
}
