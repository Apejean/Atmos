use crate::api::simple::EngineStateUpdate;
use crate::common::commands::AudioCommand;
use crate::frb_generated::StreamSink;

use lazy_static::lazy_static;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

lazy_static! {
    pub static ref GLOBAL_STATE: Arc<GlobalEngineState> = Arc::new(GlobalEngineState::new());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomState {
    Locked,
    Active,
    Cleared,
}

use crate::audio::player::SoundData;
use crate::common::config::AppConfig;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct GlobalEngineState {
    pub command_sender: crossbeam_channel::Sender<AudioCommand>,
    pub command_receiver: crossbeam_channel::Receiver<AudioCommand>,

    pub active_room_id: RwLock<Option<String>>,
    pub is_ducking: AtomicBool,
    pub enabled_channels: Vec<AtomicBool>,
    // VU levels for up to 4096 output channels, stored as f32 bits
    pub vu_levels: Vec<AtomicU32>,
    // Spatial gains for up to 4096 output channels, stored as f32 bits
    pub spatial_gains: Vec<AtomicU32>,
    pub lufs_master: [AtomicU32; 4],
    pub sound_cache: RwLock<HashMap<String, Arc<SoundData>>>,
    pub preloaded_sounds: RwLock<HashMap<String, Arc<SoundData>>>,
    pub config: RwLock<Option<AppConfig>>,
    pub config_version: std::sync::atomic::AtomicU64,
    pub current_master_lufs: AtomicU32,
    pub current_gain_reduction_db: AtomicU32,
    pub hrtf_yaw: AtomicU32,
    pub hrtf_pitch: AtomicU32,
    pub hrtf_roll: AtomicU32,
    pub playing_track_ids: RwLock<HashMap<u64, String>>,
    pub broadcast_lock: std::sync::Mutex<()>,

    pub state_sink: RwLock<Option<StreamSink<EngineStateUpdate>>>,

    pub active_device_channels: AtomicU32,
    pub engine_sample_rate: AtomicU32,
    pub is_exhibition_mode: AtomicBool,
    pub device_needs_reset: AtomicBool,
    pub watchdog_last_callback: std::sync::atomic::AtomicU64,
    pub engine_error: RwLock<Option<String>>,
    pub rta_magnitudes_ref: RwLock<Option<Arc<parking_lot::RwLock<Vec<f32>>>>>,
    pub is_failover_mode: AtomicBool,
    /// 사용자가 켠 바이노럴 상태. 믹서는 장치 선택/재스캔 때마다 새로
    /// 만들어지는데, 예전에는 켜짐 여부가 믹서 필드에만 있어서 재시작하면
    /// 조용히 꺼졌다. UI 배지는 켜짐으로 남고 실제로는 바이노럴 없이 ch0 원본이
    /// 왼쪽 출력으로만 나갔다(실기 보고: "Ch1이 왼쪽에서만 들린다", "재스캔하고
    /// 다시 켜야 동작한다"). 새 믹서는 이 값으로 시작한다.
    pub binaural_enabled: AtomicBool,
    /// 사용자가 켠 All Mute. 바이노럴과 같은 이유로 새 믹서가 이 값으로 시작한다. 음소거 상태는 Dart
    /// 화면에만 있고 재동기화가 다시 보내지 않아, 예전에는 엔진이 스스로 재시작하면 음소거가 풀렸다.
    pub master_mute: AtomicBool,
}

impl Default for GlobalEngineState {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalEngineState {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();

        let mut vu = Vec::with_capacity(4096);
        let mut sg = Vec::with_capacity(4096);
        let mut enabled = Vec::with_capacity(4096);
        for _ in 0..4096 {
            vu.push(AtomicU32::new(0));
            sg.push(AtomicU32::new(1.0f32.to_bits()));
            enabled.push(AtomicBool::new(true));
        }
        let lufs_master = [
            AtomicU32::new(0),
            AtomicU32::new(0),
            AtomicU32::new(0),
            AtomicU32::new(0),
        ];
        Self {
            command_sender: sender,
            command_receiver: receiver,
            active_room_id: RwLock::new(None),
            is_ducking: AtomicBool::new(false),
            enabled_channels: enabled,
            vu_levels: vu,
            spatial_gains: sg,
            lufs_master,
            sound_cache: RwLock::new(HashMap::new()),
            preloaded_sounds: RwLock::new(HashMap::new()),
            config: RwLock::new(None),
            config_version: std::sync::atomic::AtomicU64::new(0),
            current_master_lufs: AtomicU32::new(0),
            current_gain_reduction_db: AtomicU32::new(0),
            hrtf_yaw: AtomicU32::new(0),
            hrtf_pitch: AtomicU32::new(0),
            hrtf_roll: AtomicU32::new(0),
            playing_track_ids: RwLock::new(HashMap::new()),
            broadcast_lock: std::sync::Mutex::new(()),
            state_sink: RwLock::new(None),
            active_device_channels: AtomicU32::new(0),
            engine_sample_rate: AtomicU32::new(48000),
            is_exhibition_mode: AtomicBool::new(false),
            device_needs_reset: AtomicBool::new(false),
            watchdog_last_callback: std::sync::atomic::AtomicU64::new(0),
            engine_error: RwLock::new(None),
            rta_magnitudes_ref: RwLock::new(None),
            is_failover_mode: AtomicBool::new(false),
            binaural_enabled: AtomicBool::new(false),
            master_mute: AtomicBool::new(false),
        }
    }

    pub fn broadcast_state(&self) {
        let room_id = self.active_room_id.read().unwrap_or_else(|e| e.into_inner()).clone();
        let ducking = self.is_ducking.load(Ordering::Relaxed);
        let playing_track_ids = {
            let guard = self.playing_track_ids.read().unwrap_or_else(|e| e.into_inner());
            let mut unique_ids: Vec<String> = guard.values().cloned().collect();
            unique_ids.sort();
            unique_ids.dedup();
            unique_ids
        };

        let update = EngineStateUpdate {
            active_room_id: room_id,
            ducking_active: ducking,
            playing_track_ids,
            engine_error: self.engine_error.read().unwrap_or_else(|e| e.into_inner()).clone(),
            output_channel_count: self.active_device_channels.load(Ordering::Relaxed),
            short_term_lufs: f32::from_bits(self.current_master_lufs.load(Ordering::Relaxed)),
            gain_reduction_db: f32::from_bits(self.current_gain_reduction_db.load(Ordering::Relaxed)),
        };

        if let Some(sink) = self.state_sink.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
            let _ = sink.add(update);
        }
    }

    pub fn set_active_room(&self, room_id: Option<String>) {
        let _lock = self.broadcast_lock.lock().unwrap_or_else(|e| e.into_inner());
        {
            let mut guard = self.active_room_id.write().unwrap_or_else(|e| e.into_inner());
            *guard = room_id;
        }
        self.broadcast_state();
    }

    pub fn set_ducking(&self, ducking: bool) {
        let _lock = self.broadcast_lock.lock().unwrap_or_else(|e| e.into_inner());
        self.is_ducking.store(ducking, Ordering::Relaxed);
        self.broadcast_state();
    }

    pub fn log(&self, msg: String) {
        println!("{}", msg);

        // 로그 파일에 남긴다. 크기를 넘으면 뒤로 밀어 개수를 제한한다(core::log_file).
        let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = crate::core::log_file::append_line(
            &crate::core::log_file::log_dir(),
            &format!("[{}] {}", time, msg),
            crate::core::log_file::MAX_LOG_BYTES,
            crate::core::log_file::KEEP_ROTATED,
        );
    }

    pub fn add_playing_track(&self, instance_id: u64, track_id: String) {
        let _lock = self.broadcast_lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut guard = self.playing_track_ids.write().unwrap_or_else(|e| e.into_inner());
        guard.insert(instance_id, track_id);
        drop(guard);
        self.broadcast_state();
    }

    pub fn remove_playing_track(&self, instance_id: u64) {
        let _lock = self.broadcast_lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut guard = self.playing_track_ids.write().unwrap_or_else(|e| e.into_inner());
        if guard.remove(&instance_id).is_some() {
            drop(guard);
            self.broadcast_state();
        }
    }

    pub fn remove_playing_tracks_by_track_id(&self, track_id: &str) {
        let _lock = self.broadcast_lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut guard = self.playing_track_ids.write().unwrap_or_else(|e| e.into_inner());
        let initial_len = guard.len();
        guard.retain(|_, v| v != track_id);
        if guard.len() != initial_len {
            drop(guard);
            self.broadcast_state();
        }
    }

    /// 방 비우기: 그 방 트랙만 재생 목록에서 뺀다. 엔진의 ClearRoom도 그 방 인스턴스만 멈춘다.
    /// 예전에는 목록 전체를 지워서 다른 방이 계속 재생 중인데 목록에서 사라졌다(재시작 복원과 루프
    /// 중복 방지가 이 목록을 본다).
    pub fn remove_playing_tracks_of_room(&self, room_id: &str) {
        let track_ids: Vec<String> = self
            .config
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(|c| c.rooms.iter().find(|r| r.id == room_id))
            .map(|r| r.tracks.iter().map(|t| t.id.clone()).collect())
            .unwrap_or_default();
        let _lock = self.broadcast_lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut guard = self.playing_track_ids.write().unwrap_or_else(|e| e.into_inner());
        let initial_len = guard.len();
        guard.retain(|_, v| !track_ids.contains(v));
        if guard.len() != initial_len {
            drop(guard);
            self.broadcast_state();
        }
    }

    pub fn clear_playing_tracks(&self) {
        let _lock = self.broadcast_lock.lock().unwrap_or_else(|e| e.into_inner());
        {
            let mut guard = self.playing_track_ids.write().unwrap_or_else(|e| e.into_inner());
            guard.clear();
        }
        self.broadcast_state();
    }
}


/// 재생 인스턴스 번호. 재생 목록과 커서 표(audio::playback_cursor)의 열쇠라 겹치면 안 되고, 0은 커서 표의
/// 빈 칸 표시라 쓰지 않는다. 예전에는 시계(나노초)를 썼는데 macOS 해상도가 1µs라 동시 재생에서 겹쳤다.
pub fn next_instance_id() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// 오디오 스레드에서 쓰는 디버그 플래그.
///
/// `std::env::var`는 전역 락을 잡고 결과 String을 힙에 할당한다. DSP Law 1/2가
/// 금지하는 동작인데, 예전에는 오디오 콜백과 커맨드 핸들러 안에서 직접 불렀다.
/// 스피커를 드래그하면 방위각이 매 콜백 갱신되면서 채널마다 이 호출이 돌아
/// 오디오 스레드가 긁히는 소리("드드륵")가 났다.
///
/// 프로세스 시작 후 한 번만 읽고 그 뒤로는 원자적 bool 읽기만 한다.
pub mod debug_flags {
    use std::sync::OnceLock;

    static BINAURAL: OnceLock<bool> = OnceLock::new();
    static TRACE_CMD: OnceLock<bool> = OnceLock::new();

    /// `ATMOS_DEBUG_BINAURAL`이 설정되어 있는가.
    #[inline(always)]
    pub fn binaural() -> bool {
        *BINAURAL.get_or_init(|| std::env::var("ATMOS_DEBUG_BINAURAL").is_ok())
    }

    /// `ATMOS_TRACE_CMD`가 설정되어 있는가.
    #[inline(always)]
    pub fn trace_cmd() -> bool {
        *TRACE_CMD.get_or_init(|| std::env::var("ATMOS_TRACE_CMD").is_ok())
    }
}
