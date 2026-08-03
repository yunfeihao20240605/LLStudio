//! 真实音频波形与选区交互的 QML 桥接层。

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QVector};
use els_waveform_core::WaveformEngine;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        include!("cxx-qt-lib/core/qvector/qvector_f32.h");
        type QString = cxx_qt_lib::QString;
        type QVector_f32 = cxx_qt_lib::QVector<f32>;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QVector_f32, peak_values, cxx_name = "peakValues")]
        #[qproperty(i32, peak_revision, cxx_name = "peakRevision")]
        #[qproperty(i32, loaded_bin_count, cxx_name = "loadedBinCount")]
        #[qproperty(i32, total_bin_count, cxx_name = "totalBinCount")]
        #[qproperty(f64, duration_secs, cxx_name = "durationSecs")]
        #[qproperty(f64, current_position, cxx_name = "currentPosition")]
        #[qproperty(f64, selection_start, cxx_name = "selectionStart")]
        #[qproperty(f64, selection_end, cxx_name = "selectionEnd")]
        #[qproperty(bool, has_selection_start, cxx_name = "hasSelectionStart")]
        #[qproperty(bool, has_selection_end, cxx_name = "hasSelectionEnd")]
        #[qproperty(i32, selection_revision, cxx_name = "selectionRevision")]
        #[qproperty(bool, is_loading, cxx_name = "isLoading")]
        #[qproperty(QString, status_message, cxx_name = "statusMessage")]
        #[qproperty(bool, has_error, cxx_name = "hasError")]
        #[qproperty(QVector_f32, detail_peak_values, cxx_name = "detailPeakValues")]
        #[qproperty(f64, detail_start, cxx_name = "detailStart")]
        #[qproperty(f64, detail_end, cxx_name = "detailEnd")]
        #[qproperty(f64, detail_bin_duration, cxx_name = "detailBinDuration")]
        #[qproperty(i32, detail_revision, cxx_name = "detailRevision")]
        #[qproperty(bool, is_detail_loading, cxx_name = "isDetailLoading")]
        type WaveformBridge = super::WaveformBridgeRust;

        #[qinvokable]
        #[cxx_name = "loadForVideoPath"]
        fn load_for_video_path(
            self: Pin<&mut WaveformBridge>,
            video_path: &QString,
            duration_secs: f64,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "syncPlaybackPosition"]
        fn sync_playback_position(self: Pin<&mut WaveformBridge>, position_secs: f64) -> bool;

        #[qinvokable]
        #[cxx_name = "setSelectionRange"]
        fn set_selection_range(
            self: Pin<&mut WaveformBridge>,
            start_secs: f64,
            end_secs: f64,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "markSelectionStart"]
        fn mark_selection_start(self: Pin<&mut WaveformBridge>, position_secs: f64) -> bool;

        #[qinvokable]
        #[cxx_name = "markSelectionEnd"]
        fn mark_selection_end(self: Pin<&mut WaveformBridge>, position_secs: f64) -> bool;

        #[qinvokable]
        #[cxx_name = "startNewSelectionAt"]
        fn start_new_selection_at(self: Pin<&mut WaveformBridge>, position_secs: f64) -> bool;

        #[qinvokable]
        #[cxx_name = "clearSelection"]
        fn clear_selection(self: Pin<&mut WaveformBridge>) -> bool;

        #[qinvokable]
        #[cxx_name = "pollBackgroundTask"]
        fn poll_background_task(self: Pin<&mut WaveformBridge>) -> bool;

        #[qinvokable]
        #[cxx_name = "requestDetailRange"]
        fn request_detail_range(
            self: Pin<&mut WaveformBridge>,
            visible_start: f64,
            visible_end: f64,
            zoom_factor: f64,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "peakMinAt"]
        fn peak_min_at(self: Pin<&mut WaveformBridge>, index: i32) -> f32;

        #[qinvokable]
        #[cxx_name = "peakMaxAt"]
        fn peak_max_at(self: Pin<&mut WaveformBridge>, index: i32) -> f32;

        #[qinvokable]
        #[cxx_name = "isBinLoadedAt"]
        fn is_bin_loaded_at(self: Pin<&mut WaveformBridge>, index: i32) -> bool;
    }
}

pub struct WaveformBridgeRust {
    peak_values: QVector<f32>,
    peak_revision: i32,
    loaded_bin_count: i32,
    total_bin_count: i32,
    duration_secs: f64,
    current_position: f64,
    selection_start: f64,
    selection_end: f64,
    has_selection_start: bool,
    has_selection_end: bool,
    selection_revision: i32,
    is_loading: bool,
    status_message: QString,
    has_error: bool,
    detail_peak_values: QVector<f32>,
    detail_start: f64,
    detail_end: f64,
    detail_bin_duration: f64,
    detail_revision: i32,
    is_detail_loading: bool,
    engine: els_waveform_core::FfmpegWaveformEngine,
    waveform: els_waveform_core::WaveformData,
    loaded_bins: Vec<bool>,
    active_task_id: u64,
    task_receiver: Option<Receiver<WaveformTaskEvent>>,
    detail_receiver: Option<Receiver<DetailTaskEvent>>,
    video_path: Option<String>,
    detail_tiles: HashMap<u64, els_waveform_core::WaveformTile>,
    detail_pending: HashSet<u64>,
    detail_request_id: u64,
    detail_requested_start: f64,
    detail_requested_end: f64,
}

impl Default for WaveformBridgeRust {
    fn default() -> Self {
        let engine = els_waveform_core::FfmpegWaveformEngine;
        let waveform = engine
            .generate(&els_waveform_core::AudioSource::default())
            .unwrap_or_default();
        let total_bin_count = waveform.bins.len() as i32;

        Self {
            peak_values: flatten_bins(&waveform.bins),
            peak_revision: 1,
            loaded_bin_count: total_bin_count,
            total_bin_count,
            duration_secs: waveform.duration_secs,
            current_position: 0.0,
            selection_start: 0.0,
            selection_end: 0.0,
            has_selection_start: false,
            has_selection_end: false,
            selection_revision: 1,
            is_loading: false,
            status_message: QString::from("Generated preview waveform"),
            has_error: false,
            detail_peak_values: QVector::from(Vec::new()),
            detail_start: 0.0,
            detail_end: 0.0,
            detail_bin_duration: els_waveform_core::DETAIL_SECONDS_PER_BIN,
            detail_revision: 1,
            is_detail_loading: false,
            engine,
            waveform,
            loaded_bins: vec![true; total_bin_count as usize],
            active_task_id: 0,
            task_receiver: None,
            detail_receiver: None,
            video_path: None,
            detail_tiles: HashMap::new(),
            detail_pending: HashSet::new(),
            detail_request_id: 0,
            detail_requested_start: 0.0,
            detail_requested_end: 0.0,
        }
    }
}

impl qobject::WaveformBridge {
    fn load_for_video_path(
        mut self: Pin<&mut Self>,
        video_path: &QString,
        duration_secs: f64,
    ) -> bool {
        let requested_path = video_path.to_string();
        let task_id = self.rust().active_task_id + 1;
        let (sender, receiver) = mpsc::channel();
        let engine = self.rust().engine;
        let quality = els_waveform_core::WaveformQuality::Preview;
        let target_bin_count = quality.target_bins_for_duration(duration_secs);
        let total_bin_count = target_bin_count as i32;
        let zero_bins =
            vec![els_waveform_core::WaveformBin { min: 0.0, max: 0.0 }; total_bin_count as usize];

        let worker_path = requested_path.clone();
        thread::spawn(move || {
            let preview_result = if worker_path.trim().is_empty() {
                engine.generate(&els_waveform_core::AudioSource {
                    video_path: None,
                    duration_secs,
                    quality,
                })
            } else {
                engine.generate_for_quality_with_progress(
                    &worker_path,
                    duration_secs,
                    quality,
                    |bins, loaded_bin_count| {
                        let _ = sender.send(WaveformTaskEvent::Partial {
                            task_id,
                            bins: bins.to_vec(),
                            loaded_bin_count: loaded_bin_count as i32,
                            total_bin_count,
                            status: format!(
                                "Generating preview waveform... {loaded_bin_count}/{} bins",
                                target_bin_count
                            ),
                        });
                    },
                )
            };

            match preview_result {
                Ok(waveform) => {
                    let status = if worker_path.trim().is_empty() {
                        "Generated fallback preview waveform".to_string()
                    } else {
                        format!("Loaded preview waveform for {}", worker_path)
                    };
                    let _ = sender.send(WaveformTaskEvent::Finished {
                        task_id,
                        waveform,
                        status,
                    });
                }
                Err(err) => {
                    let _ = sender.send(WaveformTaskEvent::Failed {
                        task_id,
                        error: format!("Failed to generate preview waveform: {err}"),
                    });
                }
            }
        });

        self.as_mut().rust_mut().active_task_id = task_id;
        self.as_mut().rust_mut().video_path = if requested_path.trim().is_empty() {
            None
        } else {
            Some(requested_path)
        };
        self.as_mut().rust_mut().task_receiver = Some(receiver);
        self.as_mut().rust_mut().detail_receiver = None;
        self.as_mut().rust_mut().detail_tiles.clear();
        self.as_mut().rust_mut().detail_pending.clear();
        self.as_mut()
            .set_detail_peak_values(QVector::from(Vec::new()));
        self.as_mut().set_detail_start(0.0);
        self.as_mut().set_detail_end(0.0);
        self.as_mut().bump_detail_revision();
        self.as_mut().set_is_detail_loading(false);
        self.as_mut().rust_mut().waveform = els_waveform_core::WaveformData {
            duration_secs: duration_secs.max(0.0),
            bins: zero_bins.clone(),
        };
        self.as_mut().rust_mut().loaded_bins = vec![false; total_bin_count as usize];
        self.as_mut().set_peak_values(flatten_bins(&zero_bins));
        self.as_mut().bump_peak_revision();
        self.as_mut().set_loaded_bin_count(0);
        self.as_mut().set_total_bin_count(total_bin_count);
        self.as_mut().set_duration_secs(duration_secs.max(0.0));
        self.as_mut().set_current_position(0.0);
        self.as_mut().set_selection_start(0.0);
        self.as_mut().set_selection_end(0.0);
        self.as_mut().set_has_selection_start(false);
        self.as_mut().set_has_selection_end(false);
        self.as_mut().bump_selection_revision();
        self.as_mut().set_is_loading(true);
        self.as_mut().set_has_error(false);
        self.as_mut()
            .set_status_message(QString::from("Generating preview waveform..."));
        true
    }

    fn request_detail_range(
        mut self: Pin<&mut Self>,
        visible_start: f64,
        visible_end: f64,
        zoom_factor: f64,
    ) -> bool {
        if zoom_factor < 200.0 || self.rust().video_path.is_none() {
            return false;
        }
        let duration = self.rust().duration_secs;
        let start = visible_start.min(visible_end).clamp(0.0, duration);
        let end = visible_end.max(visible_start).clamp(start, duration);
        if end <= start {
            return false;
        }
        let same_request = (self.rust().detail_requested_start - start).abs() < 0.001
            && (self.rust().detail_requested_end - end).abs() < 0.001;
        if same_request && self.rust().detail_receiver.is_some() {
            return true;
        }
        if self.rust().detail_receiver.is_some() {
            self.as_mut().rust_mut().detail_receiver = None;
            self.as_mut().rust_mut().detail_pending.clear();
        }
        self.as_mut().rust_mut().detail_requested_start = start;
        self.as_mut().rust_mut().detail_requested_end = end;
        self.as_mut().rebuild_detail_view(start, end);

        let prefetch = ((end - start) * 0.5).max(els_waveform_core::DETAIL_TILE_DURATION_SECS);
        let request_start = (start - prefetch).max(0.0);
        let request_end = (end + prefetch).min(duration);
        let tile_indices =
            els_waveform_core::detail_tile_indices(request_start, request_end, duration);
        let mut needed = Vec::new();
        for tile_index in tile_indices {
            let missing = !self.rust().detail_tiles.contains_key(&tile_index);
            let newly_pending =
                missing && self.as_mut().rust_mut().detail_pending.insert(tile_index);
            if newly_pending {
                needed.push(tile_index);
            }
        }

        if needed.is_empty() {
            self.as_mut().set_is_detail_loading(false);
            return true;
        }

        let request_id = self.rust().detail_request_id.saturating_add(1);
        self.as_mut().rust_mut().detail_request_id = request_id;
        let path = self.rust().video_path.clone().unwrap_or_default();
        let engine = self.rust().engine;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            for tile_index in needed {
                match engine.generate_detail_tile(&path, duration, tile_index) {
                    Ok(tile) => {
                        let _ = sender.send(DetailTaskEvent::Finished { request_id, tile });
                    }
                    Err(error) => {
                        let _ = sender.send(DetailTaskEvent::Failed {
                            request_id,
                            tile_index,
                            error: format!(
                                "Failed to generate detail waveform tile {tile_index}: {error}"
                            ),
                        });
                    }
                }
            }
            let _ = sender.send(DetailTaskEvent::Complete { request_id });
        });
        self.as_mut().rust_mut().detail_receiver = Some(receiver);
        self.as_mut().set_is_detail_loading(true);
        true
    }

    fn sync_playback_position(mut self: Pin<&mut Self>, position_secs: f64) -> bool {
        self.as_mut().set_current_position(position_secs.max(0.0));
        true
    }

    fn set_selection_range(mut self: Pin<&mut Self>, start_secs: f64, end_secs: f64) -> bool {
        if !start_secs.is_finite() || !end_secs.is_finite() {
            return false;
        }

        let duration = self.rust().duration_secs.max(0.0);
        let clamp_to_duration = |value: f64| {
            if duration > 0.0 {
                value.clamp(0.0, duration)
            } else {
                value.max(0.0)
            }
        };
        let start = clamp_to_duration(start_secs.min(end_secs));
        let end = clamp_to_duration(end_secs.max(start_secs));

        if end <= start {
            self.as_mut().set_selection_start(0.0);
            self.as_mut().set_selection_end(0.0);
            self.as_mut().set_has_selection_start(false);
            self.as_mut().set_has_selection_end(false);
            self.as_mut().bump_selection_revision();
            return true;
        }

        self.as_mut().set_selection_start(start);
        self.as_mut().set_selection_end(end);
        self.as_mut().set_has_selection_start(true);
        self.as_mut().set_has_selection_end(true);
        self.as_mut().bump_selection_revision();
        true
    }

    fn mark_selection_start(mut self: Pin<&mut Self>, position_secs: f64) -> bool {
        if !position_secs.is_finite() {
            return false;
        }

        let duration = self.rust().duration_secs.max(0.0);
        let position = if duration > 0.0 {
            position_secs.clamp(0.0, duration)
        } else {
            position_secs.max(0.0)
        };

        self.as_mut().set_selection_start(position);
        self.as_mut().set_has_selection_start(true);
        self.as_mut().bump_selection_revision();
        true
    }

    fn mark_selection_end(mut self: Pin<&mut Self>, position_secs: f64) -> bool {
        if !position_secs.is_finite() {
            return false;
        }

        let duration = self.rust().duration_secs.max(0.0);
        let position = if duration > 0.0 {
            position_secs.clamp(0.0, duration)
        } else {
            position_secs.max(0.0)
        };
        self.as_mut().set_selection_end(position);
        self.as_mut().set_has_selection_end(true);
        self.as_mut().bump_selection_revision();
        true
    }

    fn start_new_selection_at(mut self: Pin<&mut Self>, position_secs: f64) -> bool {
        if !position_secs.is_finite() {
            return false;
        }

        let duration = self.rust().duration_secs.max(0.0);
        let position = if duration > 0.0 {
            position_secs.clamp(0.0, duration)
        } else {
            position_secs.max(0.0)
        };

        self.as_mut().set_selection_start(position);
        self.as_mut().set_selection_end(0.0);
        self.as_mut().set_has_selection_start(true);
        self.as_mut().set_has_selection_end(false);
        self.as_mut().bump_selection_revision();
        true
    }

    fn clear_selection(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().set_selection_start(0.0);
        self.as_mut().set_selection_end(0.0);
        self.as_mut().set_has_selection_start(false);
        self.as_mut().set_has_selection_end(false);
        self.as_mut().bump_selection_revision();
        true
    }

    fn poll_background_task(mut self: Pin<&mut Self>) -> bool {
        let mut changed = false;
        let mut preview_receiver = self.as_mut().rust_mut().task_receiver.take();
        while let Some(receiver) = preview_receiver.as_ref() {
            match receiver.try_recv() {
                Ok(WaveformTaskEvent::Partial {
                    task_id,
                    bins,
                    loaded_bin_count,
                    total_bin_count,
                    status,
                }) => {
                    if task_id != self.rust().active_task_id {
                        continue;
                    }
                    let duration_secs = self.rust().duration_secs;
                    self.as_mut().set_peak_values(flatten_bins(&bins));
                    self.as_mut().bump_peak_revision();
                    self.as_mut().rust_mut().loaded_bins =
                        build_loaded_prefix(bins.len(), loaded_bin_count as usize);
                    self.as_mut().rust_mut().waveform = els_waveform_core::WaveformData {
                        duration_secs,
                        bins,
                    };
                    self.as_mut().set_loaded_bin_count(loaded_bin_count);
                    self.as_mut().set_total_bin_count(total_bin_count);
                    self.as_mut().set_status_message(QString::from(&status));
                    changed = true;
                }
                Ok(WaveformTaskEvent::Finished {
                    task_id,
                    waveform,
                    status,
                }) => {
                    if task_id != self.rust().active_task_id {
                        continue;
                    }
                    let loaded_bin_count = waveform.bins.len() as i32;
                    let total_bin_count = loaded_bin_count;
                    let duration_secs = waveform.duration_secs;
                    self.as_mut().set_peak_values(flatten_bins(&waveform.bins));
                    self.as_mut().bump_peak_revision();
                    self.as_mut().rust_mut().loaded_bins = vec![true; waveform.bins.len()];
                    self.as_mut().rust_mut().waveform = waveform;
                    self.as_mut().set_loaded_bin_count(loaded_bin_count);
                    self.as_mut().set_total_bin_count(total_bin_count);
                    self.as_mut().set_duration_secs(duration_secs);
                    self.as_mut().set_status_message(QString::from(&status));
                    self.as_mut().set_has_error(false);
                    self.as_mut().set_is_loading(false);
                    preview_receiver = None;
                    changed = true;
                    break;
                }
                Ok(WaveformTaskEvent::Failed { task_id, error }) => {
                    if task_id == self.rust().active_task_id {
                        eprintln!("{error}");
                        self.as_mut().set_status_message(QString::from(&error));
                        self.as_mut().set_has_error(true);
                        self.as_mut().set_is_loading(false);
                        changed = true;
                    }
                    preview_receiver = None;
                    break;
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    self.as_mut()
                        .set_status_message(QString::from("Waveform task stopped unexpectedly"));
                    self.as_mut().set_has_error(true);
                    self.as_mut().set_is_loading(false);
                    preview_receiver = None;
                    changed = true;
                    break;
                }
            }
        }
        self.as_mut().rust_mut().task_receiver = preview_receiver;

        let mut detail_receiver = self.as_mut().rust_mut().detail_receiver.take();
        while let Some(receiver) = detail_receiver.as_ref() {
            match receiver.try_recv() {
                Ok(DetailTaskEvent::Finished { request_id, tile }) => {
                    if request_id != self.rust().detail_request_id {
                        continue;
                    }
                    self.as_mut()
                        .rust_mut()
                        .detail_pending
                        .remove(&tile.tile_index);
                    self.as_mut()
                        .rust_mut()
                        .detail_tiles
                        .insert(tile.tile_index, tile);
                    if self.rust().detail_tiles.len() > 64 {
                        let protected = els_waveform_core::detail_tile_indices(
                            self.rust().detail_requested_start,
                            self.rust().detail_requested_end,
                            self.rust().duration_secs,
                        );
                        if let Some(evicted) =
                            self.rust().detail_tiles.keys().copied().find(|index| {
                                !self.rust().detail_pending.contains(index)
                                    && !protected.contains(index)
                            })
                        {
                            self.as_mut().rust_mut().detail_tiles.remove(&evicted);
                        }
                    }
                    let start = self.rust().detail_requested_start;
                    let end = self.rust().detail_requested_end;
                    self.as_mut().rebuild_detail_view(start, end);
                    changed = true;
                }
                Ok(DetailTaskEvent::Failed {
                    request_id,
                    tile_index,
                    error,
                }) => {
                    if request_id == self.rust().detail_request_id {
                        self.as_mut().rust_mut().detail_pending.remove(&tile_index);
                        eprintln!("{error}");
                        self.as_mut().set_status_message(QString::from(&error));
                        changed = true;
                    }
                }
                Ok(DetailTaskEvent::Complete { request_id }) => {
                    if request_id == self.rust().detail_request_id {
                        self.as_mut().set_is_detail_loading(false);
                    }
                    detail_receiver = None;
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.as_mut().set_is_detail_loading(false);
                    detail_receiver = None;
                    break;
                }
            }
        }
        self.as_mut().rust_mut().detail_receiver = detail_receiver;
        changed
    }

    fn rebuild_detail_view(mut self: Pin<&mut Self>, visible_start: f64, visible_end: f64) {
        let duration = self.rust().duration_secs;
        let indices = els_waveform_core::detail_tile_indices(visible_start, visible_end, duration);
        if indices.is_empty()
            || indices
                .iter()
                .any(|index| !self.rust().detail_tiles.contains_key(index))
        {
            return;
        }

        let first = self.rust().detail_tiles.get(&indices[0]).cloned();
        let last = self
            .rust()
            .detail_tiles
            .get(indices.last().unwrap_or(&indices[0]))
            .cloned();
        let (Some(first), Some(last)) = (first, last) else {
            return;
        };
        let mut bins = Vec::new();
        for index in indices {
            if let Some(tile) = self.rust().detail_tiles.get(&index) {
                bins.extend_from_slice(&tile.bins);
            }
        }
        self.as_mut().set_detail_peak_values(flatten_bins(&bins));
        self.as_mut().set_detail_start(first.start_secs);
        self.as_mut().set_detail_end(last.end_secs);
        self.as_mut().set_detail_bin_duration(first.seconds_per_bin);
        self.as_mut().bump_detail_revision();
    }

    fn peak_min_at(self: Pin<&mut Self>, index: i32) -> f32 {
        self.rust()
            .waveform
            .bins
            .get(index.max(0) as usize)
            .map(|bin| bin.min)
            .unwrap_or(0.0)
    }

    fn peak_max_at(self: Pin<&mut Self>, index: i32) -> f32 {
        self.rust()
            .waveform
            .bins
            .get(index.max(0) as usize)
            .map(|bin| bin.max)
            .unwrap_or(0.0)
    }

    fn is_bin_loaded_at(self: Pin<&mut Self>, index: i32) -> bool {
        self.rust()
            .loaded_bins
            .get(index.max(0) as usize)
            .copied()
            .unwrap_or(false)
    }

    fn bump_peak_revision(mut self: Pin<&mut Self>) {
        let next = self.rust().peak_revision.saturating_add(1);
        self.as_mut().set_peak_revision(next);
    }

    fn bump_selection_revision(mut self: Pin<&mut Self>) {
        let next = self.rust().selection_revision.wrapping_add(1).max(1);
        self.as_mut().set_selection_revision(next);
    }

    fn bump_detail_revision(mut self: Pin<&mut Self>) {
        let next = self.rust().detail_revision.wrapping_add(1).max(1);
        self.as_mut().set_detail_revision(next);
    }
}

enum WaveformTaskEvent {
    Partial {
        task_id: u64,
        bins: Vec<els_waveform_core::WaveformBin>,
        loaded_bin_count: i32,
        total_bin_count: i32,
        status: String,
    },
    Finished {
        task_id: u64,
        waveform: els_waveform_core::WaveformData,
        status: String,
    },
    Failed {
        task_id: u64,
        error: String,
    },
}

enum DetailTaskEvent {
    Finished {
        request_id: u64,
        tile: els_waveform_core::WaveformTile,
    },
    Failed {
        request_id: u64,
        tile_index: u64,
        error: String,
    },
    Complete {
        request_id: u64,
    },
}

fn flatten_bins(bins: &[els_waveform_core::WaveformBin]) -> QVector<f32> {
    let mut values = Vec::with_capacity(bins.len() * 2);
    for bin in bins {
        values.push(bin.min);
        values.push(bin.max);
    }
    QVector::from(values)
}

fn build_loaded_prefix(bin_count: usize, loaded_bin_count: usize) -> Vec<bool> {
    let mut loaded = vec![false; bin_count];
    for index in 0..loaded_bin_count.min(bin_count) {
        loaded[index] = true;
    }
    loaded
}
