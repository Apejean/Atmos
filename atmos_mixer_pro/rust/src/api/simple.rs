use crate::api::error::AtmosError;
use crate::common::commands::AudioCommand;
use crate::common::config::AppConfig;
use crate::common::utils::hash_id;
use crate::core::state::GLOBAL_STATE;
use crate::frb_generated::StreamSink;

#[flutter_rust_bridge::frb(init)]
pub fn api_init_app() {
    flutter_rust_bridge::setup_default_user_utils();
    // 무인 운영 중 시스템이 잠들지 않게 한다(앱이 켜져 있는 동안).
    crate::core::keep_awake::prevent_system_sleep();
    // (Removed start_osc_server to prevent port 8000/8001 conflict.
    // The main OSC listener handles everything now.)
}

pub fn api_update_single_band_eq(
    channel_index: usize,
    band_index: usize,
    frequency: f32,
    gain_db: f32,
    q_factor: f32,
    filter_type_idx: u8,
) {
    let _ = GLOBAL_STATE
        .command_sender
        .send(AudioCommand::UpdateSingleBandEq {
            channel: channel_index,
            band: band_index,
            freq: frequency,
            gain_db,
            q_factor,
            filter_type_idx,
        });
}

pub fn api_update_sound_source_position(sound_id: String, x: f32, y: f32, z: f32) {
    let _ = GLOBAL_STATE
        .command_sender
        .send(AudioCommand::UpdateSoundSourcePosition { sound_id, x, y, z });
}

/// Output Config(`mono_configs`/`stereo_configs`/`multi_configs`)로부터 믹서 출력 게이트
/// (`GLOBAL_STATE.enabled_channels`)에 쓸 0-based 활성화 벡터를 계산하는 순수 함수.
///
/// ## 0/1-based 규약 (OUTPUT_CHANNEL_MAPPING_UNIFICATION_SPEC.md 3절)
/// `config.json`의 세 맵은 키가 **1-based**(사용자에게 보이는 채널 번호)이고,
/// 반환하는 `Vec<bool>`과 `GLOBAL_STATE.enabled_channels`는 **0-based**(CPAL `hw_ch` 인덱스)이다.
/// 이 변환(`ch as usize - 1`)은 이 함수 안에서만 일어난다 — 호출부에서 다시 변환하지 말 것.
///
/// ## 그룹별 채널 개방 규칙
/// - Mono: `ch-1`, `ch`(1-based 페어) 두 채널을 연다 — 기존 "모노 1개가 L/R 페어를 연다" 설계 유지.
/// - Stereo: 위와 동일한 페어 개방(기존 동작 유지, 이번 수정 범위 아님).
/// - Multi: 시작 채널(`ch-1`, 0-based)부터 하드웨어 마지막 채널까지 전체 구간을 연다.
///   `ChannelSetting`에는 그룹이 몇 채널짜리인지(N) 저장되지 않고, 실제로 몇 채널을 쓸지는
///   재생되는 파일의 채널 수에 따라 트랙별 라우팅(믹서 쓰기 단계)에서 결정되므로, 이 게이트는
///   "시작 채널 이후로는 무엇이 와도 막지 않는다"는 상한만 담당한다(SPEC 3.1절).
///
/// 세 맵이 모두 비어 있으면(사용자가 Output Config를 아직 건드리지 않음) 하위호환을 위해
/// 전 채널을 개방한다.
pub fn compute_enabled_channels(config: &AppConfig, hw_len: usize) -> Vec<bool> {
    let mut enabled = vec![false; hw_len];

    // "한 번도 그룹을 열지 않은 설정"은 전체 개방으로 본다.
    //
    // 예전에는 맵이 `is_empty()`인지만 봤다. 그래서 항목은 존재하지만 전부
    // `enabled: false`인 설정(신규 생성 직후 등)에서는 이 분기를 타지 않고
    // 모든 채널이 닫힌 채로 남아 완전 무음이 됐다. 더 나쁜 건 Dart의
    // `buildChannelRoutingItems()`(lib/core/utils/channel_routing.dart)가
    // "enabled 항목이 하나라도 있는가"를 기준으로 쓴다는 점이었다. 두 규칙이
    // 어긋나서 UI는 채널을 선택지로 제시하는데 엔진은 그 채널을 음소거했다.
    // 사용자가 "선택은 되는데 소리가 안 난다"고 본 원인이다.
    //
    // 현재 데이터 모델에는 "설정된 적 있음"을 나타내는 별도 플래그가 없어
    // "전부 비활성"과 "미설정"을 구분할 수 없다. 그래서 Dart와 같은 규칙
    // (활성 항목이 하나도 없으면 전체 개방)으로 통일한다. 채널을 실제로 닫는
    // 것은 다른 그룹을 열어 상대적으로 닫는 방식이다.
    let has_enabled_group = config.mono_configs.values().any(|s| s.enabled)
        || config.stereo_configs.values().any(|s| s.enabled)
        || config.multi_configs.values().any(|s| s.enabled);
    if !has_enabled_group {
        return vec![true; hw_len];
    }

    // Mono 그룹 하나는 **채널 하나**를 연다. 예전에는 real_ch와 real_ch+1을
    // 모두 열어서, 사용자가 Mono 1만 열어도 채널 2까지 열렸다. Dart의
    // buildChannelRoutingItems도 같은 가정으로 항목을 두 개 만들다가 같은 채널이
    // 중복 노출되는 버그가 있었고, 그쪽을 1:1로 고쳤다. 게이트도 같은 규약을
    // 따라야 UI가 연 채널과 엔진이 여는 채널이 일치한다.
    // 페어를 열려면 Stereo 그룹을 쓴다(아래 루프가 real_ch와 real_ch+1을 연다).
    for (&ch, setting) in &config.mono_configs {
        if setting.enabled && ch > 0 {
            let real_ch = (ch - 1) as usize;
            if real_ch < hw_len {
                enabled[real_ch] = true;
            }
        }
    }
    for (&ch, setting) in &config.stereo_configs {
        if setting.enabled && ch > 0 {
            let real_ch = (ch - 1) as usize;
            if real_ch < hw_len {
                enabled[real_ch] = true;
            }
            if real_ch + 1 < hw_len {
                enabled[real_ch + 1] = true;
            }
        }
    }
    for (&ch, setting) in &config.multi_configs {
        if setting.enabled && ch > 0 {
            let real_ch = (ch - 1) as usize;
            if real_ch < hw_len {
                for slot in enabled.iter_mut().skip(real_ch) {
                    *slot = true;
                }
            }
        }
    }

    enabled
}

/// `compute_enabled_channels()` 결과를 `GLOBAL_STATE.enabled_channels` 원자 배열에 반영한다.
fn apply_enabled_channels(config: &AppConfig) {
    let enabled = compute_enabled_channels(config, GLOBAL_STATE.enabled_channels.len());
    for (slot, &value) in GLOBAL_STATE.enabled_channels.iter().zip(enabled.iter()) {
        slot.store(value, std::sync::atomic::Ordering::Relaxed);
    }
    request_restart_if_processing_width_changed(config);
}

/// 출력 설정에서 켠 채널 범위가 바뀌어 믹서 처리 폭(core::processing_channels)이 달라지면 엔진을
/// 다시 시작하게 한다. 믹서 폭은 엔진을 만들 때 정해지므로, 장치 유실과 같은 자기 재시작 경로를 쓴다
/// (재생 상태를 떠 두었다가 멈춘 위치부터 이어 간다).
fn request_restart_if_processing_width_changed(config: &AppConfig) {
    use std::sync::atomic::Ordering;
    if !ENGINE_ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    let hw_channels = GLOBAL_STATE.active_device_channels.load(Ordering::SeqCst) as usize;
    let current = GLOBAL_STATE.processing_channels.load(Ordering::SeqCst) as usize;
    if hw_channels == 0 || current == 0 {
        return;
    }
    let wanted = crate::core::processing_channels::mixer_width(
        crate::core::processing_channels::processing_channel_count(config, hw_channels),
    );
    if wanted != current {
        GLOBAL_STATE.log(format!(
            "출력 채널 범위가 바뀌어 믹서 처리 폭을 {current} → {wanted}채널로 바꾼다(엔진 다시 시작)"
        ));
        GLOBAL_STATE.device_needs_reset.store(true, Ordering::Release);
    }
}

pub fn api_get_config(path: String) -> AppConfig {
    let config = AppConfig::load_from_file(path).unwrap_or_default();

    apply_enabled_channels(&config);
    {
        let mut global_config = GLOBAL_STATE
            .config
            .write()
            .unwrap_or_else(|e| e.into_inner());
        GLOBAL_STATE
            .config_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *global_config = Some(config.clone());
    }
    GLOBAL_STATE.is_exhibition_mode.store(
        config.is_exhibition_mode,
        std::sync::atomic::Ordering::Relaxed,
    );

    // Apply tunings on boot immediately
    let mut tunings = Vec::new();
    for (&ch, setting) in &config.mono_configs {
        if setting.enabled {
            tunings.push((
                ch as usize - 1,
                setting.delay_ms,
                setting.eq_bands.clone(),
                setting.phase_invert,
                setting.gain_db,
            ));
        }
    }
    for (&ch, setting) in &config.stereo_configs {
        if setting.enabled {
            tunings.push((
                ch as usize - 1,
                setting.delay_ms,
                setting.eq_bands.clone(),
                setting.phase_invert,
                setting.gain_db,
            ));
        }
    }
    for (&ch, setting) in &config.multi_configs {
        if setting.enabled {
            tunings.push((
                ch as usize - 1,
                setting.delay_ms,
                setting.eq_bands.clone(),
                setting.phase_invert,
                setting.gain_db,
            ));
        }
    }
    let _ = GLOBAL_STATE
        .command_sender
        .send(AudioCommand::ApplyAllChannelTunings { tunings });

    config
}

pub fn api_save_config(path: String, config: AppConfig) -> Result<(), AtmosError> {
    config.save_to_file(path)?;
    apply_enabled_channels(&config);
    {
        let mut global_config = GLOBAL_STATE
            .config
            .write()
            .unwrap_or_else(|e| e.into_inner());
        GLOBAL_STATE
            .config_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *global_config = Some(config.clone());
    }
    GLOBAL_STATE.is_exhibition_mode.store(
        config.is_exhibition_mode,
        std::sync::atomic::Ordering::Relaxed,
    );
    Ok(())
}

pub fn api_preload_sound(file_path: String) -> Result<(), AtmosError> {
    let path = std::path::Path::new(&file_path);
    let target_sr = GLOBAL_STATE
        .engine_sample_rate
        .load(std::sync::atomic::Ordering::Relaxed);
    match crate::audio::player::SoundData::load_from_file(path, target_sr) {
        Ok(data) => {
            // 100MB limit check (samples * 4 bytes per f32)
            if data.samples.len() > 25_000_000 {
                println!(
                    "[TELEMETRY] OOM Protection triggered: File {} exceeds 100MB limit.",
                    file_path
                );
                return Err(AtmosError {
                    message: "File is too large for preload (exceeds 100MB).".to_string(),
                });
            }
            let arc_data = std::sync::Arc::new(data);
            let mut preloaded = GLOBAL_STATE
                .preloaded_sounds
                .write()
                .unwrap_or_else(|e| e.into_inner());
            preloaded.insert(file_path, arc_data);
            Ok(())
        }
        Err(e) => Err(AtmosError {
            message: format!("Preload failed: {}", e),
        }),
    }
}

pub fn api_clear_preloaded_sounds() -> Result<(), AtmosError> {
    let mut preloaded = GLOBAL_STATE
        .preloaded_sounds
        .write()
        .unwrap_or_else(|e| e.into_inner());
    preloaded.clear();
    Ok(())
}

pub fn api_get_rta_magnitudes() -> Vec<f32> {
    if let Ok(guard) = GLOBAL_STATE.rta_magnitudes_ref.read() {
        if let Some(rta) = guard.as_ref() {
            let lock = rta.read();
            return lock.clone();
        }
    }
    vec![-140.0; crate::audio::rta::RTA_BIN_COUNT]
}

pub fn api_get_spatial_gains() -> Vec<f32> {
    let out_channels = GLOBAL_STATE
        .active_device_channels
        .load(std::sync::atomic::Ordering::Relaxed) as usize;
    let limit = out_channels.min(GLOBAL_STATE.spatial_gains.len());
    let mut gains = Vec::with_capacity(limit);
    for i in 0..limit {
        gains.push(f32::from_bits(
            GLOBAL_STATE.spatial_gains[i].load(std::sync::atomic::Ordering::Relaxed),
        ));
    }
    gains
}

pub fn api_calculate_eq_response(bands: Vec<crate::common::config::EqBand>) -> Vec<f32> {
    let freqs = crate::audio::eq_response::generate_log_frequencies(20.0, 20000.0, 100);
    crate::audio::eq_response::calculate_total_eq_curve(&bands, 48000.0, &freqs)
}

/// PlayTrack 커맨드를 만든다.
///
/// SoundInstance 생성은 스트림 버퍼(약 512KB) 등 여러 힙 할당을 동반하므로
/// 반드시 오디오 스레드 밖에서 수행해야 한다. 이 함수는 FRB 워커 스레드에서
/// 실행되는 api_play_track 계열에서만 호출된다. 오디오 스레드는 완성된
/// 인스턴스를 풀 슬롯에 옮겨 담기만 한다(Law 1).
#[allow(clippy::too_many_arguments)]
pub fn build_play_track_command(
    instance_id: u64,
    room_id: u32,
    track_id: u32,
    track_id_str: String,
    data: Option<std::sync::Arc<crate::audio::player::SoundData>>,
    streamer: Option<crate::audio::streaming::DiskStreamer>,
    stream_sample_rate: u32,
    stream_channels: u16,
    is_loop: bool,
    volume: f32,
    room_volume: f32,
    output_channel: usize,
    output_stereo: bool,
    current_position: Option<crate::common::config::Point3D>,
) -> AudioCommand {
    // 믹서가 경계 검사에 쓰는 배열과 동일한 길이로 잡아 인덱스 불일치를 없앤다.
    let spatial_channel_capacity = GLOBAL_STATE.enabled_channels.len();

    let instance = crate::audio::player::SoundInstance::new(
        instance_id,
        track_id,
        room_id,
        track_id_str,
        data,
        streamer,
        stream_sample_rate,
        stream_channels,
        is_loop,
        volume,
        output_channel,
        output_stereo,
        current_position,
        spatial_channel_capacity,
    );

    AudioCommand::PlayTrack {
        instance: Box::new(instance),
        room_volume,
    }
}

pub fn api_play_track(room_id: String, track_id: String) -> Result<(), AtmosError> {
    play_track_from(room_id, track_id, 0.0)
}

/// 재생 명령의 인스턴스를 `seconds` 지점부터 시작하게 한다(0이면 그대로).
fn with_start(mut cmd: AudioCommand, seconds: f64) -> AudioCommand {
    if seconds > 0.0 {
        if let AudioCommand::PlayTrack { instance, .. } = &mut cmd {
            instance.set_start_position(seconds);
        }
    }
    cmd
}

/// `start_seconds`(파일 기준)부터 재생한다. 재시작 복원(core::restart_resume)이 멈춘 위치를 넘긴다.
pub(crate) fn play_track_from(
    room_id: String,
    track_id: String,
    start_seconds: f64,
) -> Result<(), AtmosError> {
    if crate::core::state::debug_flags::trace_cmd() {
        eprintln!("[CMD] 재생 요청 room={room_id} track={track_id}");
    }
    let config_guard = GLOBAL_STATE
        .config
        .read()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(config) = config_guard.as_ref() {
        if let Some(room) = config.rooms.iter().find(|r| r.id == room_id) {
            if let Some(track) = room.tracks.iter().find(|t| t.id == track_id) {
                let instance_id = crate::core::state::next_instance_id();

                let _ = GLOBAL_STATE
                    .command_sender
                    .send(AudioCommand::SetMasterVolume {
                        room_id: hash_id(&room_id),
                        volume: room.volume,
                    });
                if track.is_loop || track.is_streaming {
                    // Prevent duplicate playback of the same looping track if it is a BGM
                    if track.is_loop {
                        let is_playing = {
                            let guard = GLOBAL_STATE
                                .playing_track_ids
                                .read()
                                .unwrap_or_else(|e| e.into_inner());
                            guard.values().any(|id| id == &track_id)
                        };
                        if is_playing {
                            if crate::core::state::debug_flags::trace_cmd() {
                                eprintln!("[CMD] 재생 건너뜀(이미 재생 중인 루프) track={track_id}");
                            }
                            return Ok(());
                        }
                    }

                    let target_sr = GLOBAL_STATE
                        .engine_sample_rate
                        .load(std::sync::atomic::Ordering::Relaxed);
                    // Start DiskStreamer for BGM or streaming tracks
                    match crate::audio::streaming::DiskStreamer::new_at(
                        track.file_path.clone(),
                        track.is_loop,
                        target_sr,
                        start_seconds,
                    ) {
                        Ok(streamer) => {
                            let sample_rate = streamer.sample_rate;
                            let channels = streamer.channels;
                            if crate::core::state::debug_flags::trace_cmd() {
                                eprintln!("[CMD] 재생 시작(스트리밍) track={track_id} instance={instance_id}");
                            }
                            GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                            GLOBAL_STATE
                                .command_sender
                                .send(with_start(build_play_track_command(
                                    instance_id,
                                    hash_id(&room_id),
                                    hash_id(&track_id),
                                    track_id.clone(),
                                    None,
                                    Some(streamer),
                                    sample_rate,
                                    channels,
                                    track.is_loop,
                                    track.volume,
                                    room.volume,
                                    track.output_channel as usize,
                                    track.output_stereo,
                                    None,
                                ), start_seconds))
                                .map_err(|e| AtmosError {
                                    message: e.to_string(),
                                })?;
                            return Ok(());
                        }
                        Err(e) => {
                            return Err(AtmosError {
                                message: format!("Streamer init failed: {}", e),
                            });
                        }
                    }
                } else {
                    let preloaded_data = {
                        let preloaded_guard = GLOBAL_STATE
                            .preloaded_sounds
                            .read()
                            .unwrap_or_else(|e| e.into_inner());
                        preloaded_guard.get(&track.file_path).cloned()
                    };

                    if let Some(data) = preloaded_data {
                        // Protect against multiple rapid clicks for looping tracks
                        if track.is_loop {
                            let is_playing = {
                                let guard = GLOBAL_STATE
                                    .playing_track_ids
                                    .read()
                                    .unwrap_or_else(|e| e.into_inner());
                                guard.values().any(|id| id == &track_id)
                            };
                            if is_playing {
                                return Ok(());
                            }
                        }

                        GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                        GLOBAL_STATE
                            .command_sender
                            .send(with_start(build_play_track_command(
                                instance_id,
                                hash_id(&room_id),
                                hash_id(&track_id),
                                track_id.clone(),
                                Some(data.clone()),
                                None,
                                data.sample_rate,
                                data.channels,
                                false,
                                track.volume,
                                room.volume,
                                track.output_channel as usize,
                                track.output_stereo,
                                None,
                            ), start_seconds))
                            .map_err(|e| AtmosError {
                                message: e.to_string(),
                            })?;
                        return Ok(());
                    }

                    let cache_guard = GLOBAL_STATE
                        .sound_cache
                        .read()
                        .unwrap_or_else(|e| e.into_inner());
                    if let Some(data) = cache_guard.get(&track.file_path) {
                        GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                        GLOBAL_STATE
                            .command_sender
                            .send(with_start(build_play_track_command(
                                instance_id,
                                hash_id(&room_id),
                                hash_id(&track_id),
                                track_id.clone(),
                                Some(data.clone()),
                                None,
                                data.sample_rate,
                                data.channels,
                                false,
                                track.volume,
                                room.volume,
                                track.output_channel as usize,
                                track.output_stereo,
                                None,
                            ), start_seconds))
                            .map_err(|e| AtmosError {
                                message: e.to_string(),
                            })?;
                        return Ok(());
                    } else {
                        // Cache miss -> Load into RAM dynamically (obeys 100% RAM rule for SFX)
                        let path = std::path::Path::new(&track.file_path);
                        if let Ok(metadata) = std::fs::metadata(path) {
                            if metadata.len() > 500 * 1024 * 1024 {
                                return Err(AtmosError {
                                    message: format!("파일 용량이 너무 큽니다 (500MB 초과). BGM(Loop)으로 설정하거나 용량을 줄이세요: {}", track.file_path),
                                });
                            }
                        }
                        let target_sr = GLOBAL_STATE
                            .engine_sample_rate
                            .load(std::sync::atomic::Ordering::Relaxed);
                        match crate::audio::player::SoundData::load_from_file(path, target_sr) {
                            Ok(data) => {
                                let arc_data = std::sync::Arc::new(data);
                                {
                                    let mut cache = GLOBAL_STATE
                                        .sound_cache
                                        .write()
                                        .unwrap_or_else(|e| e.into_inner());
                                    cache.insert(track.file_path.clone(), arc_data.clone());
                                }
                                GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                                GLOBAL_STATE
                                    .command_sender
                                    .send(with_start(build_play_track_command(
                                        instance_id,
                                        hash_id(&room_id),
                                        hash_id(&track_id),
                                        track_id.clone(),
                                        Some(arc_data.clone()),
                                        None,
                                        arc_data.sample_rate,
                                        arc_data.channels,
                                        false,
                                        track.volume,
                                        room.volume,
                                        track.output_channel as usize,
                                        track.output_stereo,
                                        None,
                                    ), start_seconds))
                                    .map_err(|e| AtmosError {
                                        message: e.to_string(),
                                    })?;
                                return Ok(());
                            }
                            Err(e) => {
                                return Err(AtmosError {
                                    message: format!(
                                        "Cache miss and dynamic RAM loading failed for {}: {}",
                                        track.file_path, e
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    Err(AtmosError {
        message: "Room or track not found".to_string(),
    })
}

pub fn api_stop_track(room_id: String, track_id: String) -> Result<(), AtmosError> {
    crate::core::restart_resume::cancel_track(&track_id);
    if crate::core::state::debug_flags::trace_cmd() {
        eprintln!("[CMD] 정지 요청 room={room_id} track={track_id}");
    }
    GLOBAL_STATE.remove_playing_tracks_by_track_id(&track_id);
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::StopTrack {
            room_id: hash_id(&room_id),
            track_id: hash_id(&track_id),
        })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_stop_all() -> Result<(), AtmosError> {
    crate::core::restart_resume::cancel_all();
    if crate::core::state::debug_flags::trace_cmd() {
        eprintln!("[CMD] 전체 정지 요청");
    }
    let _lock = GLOBAL_STATE
        .broadcast_lock
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    {
        let mut guard = GLOBAL_STATE
            .playing_track_ids
            .write()
            .unwrap_or_else(|e| e.into_inner());
        guard.clear();
    }
    {
        let mut guard = GLOBAL_STATE
            .active_room_id
            .write()
            .unwrap_or_else(|e| e.into_inner());
        *guard = None;
    }
    GLOBAL_STATE.broadcast_state();
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::StopAll)
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_set_active_room(room_id: Option<String>) -> Result<(), AtmosError> {
    GLOBAL_STATE.set_active_room(room_id);
    Ok(())
}

pub fn api_clear_room(room_id: String) -> Result<(), AtmosError> {
    {
        let mut guard = GLOBAL_STATE
            .active_room_id
            .write()
            .unwrap_or_else(|e| e.into_inner());
        if guard.as_ref() != Some(&room_id) {
            return Err(AtmosError {
                message: "Room is not active or already cleared".to_string(),
            });
        }
        *guard = None;
    }
    crate::core::restart_resume::cancel_room(&room_id);
    GLOBAL_STATE.remove_playing_tracks_of_room(&room_id);
    GLOBAL_STATE.broadcast_state();

    let _lock = GLOBAL_STATE
        .broadcast_lock
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::ClearRoom {
            room_id: hash_id(&room_id),
        })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_set_master_mute(muted: bool) -> Result<(), AtmosError> {
    // 엔진이 재시작돼도 유지되도록 전역에 먼저 기록한다(state.rs 주석 참고).
    GLOBAL_STATE
        .master_mute
        .store(muted, std::sync::atomic::Ordering::Relaxed);
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::SetMasterMute { muted })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_set_master_volume(room_id: String, volume: f32) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::SetMasterVolume {
            room_id: hash_id(&room_id),
            volume,
        })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

#[flutter_rust_bridge::frb(sync)]
pub fn api_set_track_volume(
    room_id: String,
    track_id: String,
    volume: f32,
) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::SetTrackVolume {
            room_id: hash_id(&room_id),
            track_id: hash_id(&track_id),
            volume,
        })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_set_track_output(
    room_id: String,
    track_id: String,
    output_channel: usize,
    output_stereo: bool,
) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::SetTrackOutput {
            room_id: hash_id(&room_id),
            track_id: hash_id(&track_id),
            output_channel,
            output_stereo,
        })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_set_channel_delay(channel: usize, delay_ms: f32) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::SetChannelDelay { channel, delay_ms })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

pub fn api_set_channel_eq(
    channel: usize,
    bands: Vec<crate::common::config::EqBand>,
) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::SetChannelEq { channel, bands })
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(())
}

use std::sync::atomic::AtomicU64;
lazy_static::lazy_static! {
    static ref VU_THREAD_RUNNING: AtomicU64 = AtomicU64::new(0);
}

pub fn api_create_vu_stream(sink: StreamSink<Vec<f32>>) {
    let session_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::from_secs(0))
        .as_millis() as u64;
    VU_THREAD_RUNNING.store(session_id, std::sync::atomic::Ordering::Relaxed);

    std::thread::spawn(move || loop {
        if VU_THREAD_RUNNING.load(std::sync::atomic::Ordering::Relaxed) != session_id {
            break;
        }
        let max_channels = GLOBAL_STATE
            .active_device_channels
            .load(std::sync::atomic::Ordering::Relaxed) as usize;
        let limit = if max_channels > 0 { max_channels } else { 64 };
        let mut levels: Vec<f32> = GLOBAL_STATE
            .vu_levels
            .iter()
            .take(limit)
            .map(|v| f32::from_bits(v.load(std::sync::atomic::Ordering::Relaxed)))
            .collect();

        for lufs in &GLOBAL_STATE.lufs_master {
            levels.push(f32::from_bits(
                lufs.load(std::sync::atomic::Ordering::Relaxed),
            ));
        }
        if sink.add(levels).is_err() {
            break; // Stop thread if port is closed
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    });
}

// 엔진 세대는 오디오 콜백도 읽어야 해서 엔진 모듈에 둔다(옛 스트림이 명령을 가로채지 않게).
use crate::audio::engine::ENGINE_GENERATION;

lazy_static::lazy_static! {
    pub static ref ENGINE_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    static ref ENGINE_THREAD: std::sync::Mutex<Option<std::thread::JoinHandle<()>>> = std::sync::Mutex::new(None);
    static ref STREAM_STATUS_SINK: std::sync::RwLock<Option<StreamSink<String>>> = std::sync::RwLock::new(None);
}

pub fn broadcast_stream_status(status: String) {
    if let Some(sink) = STREAM_STATUS_SINK
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
    {
        let _ = sink.add(status);
    }
}

pub fn api_create_stream_status_stream(sink: StreamSink<String>) {
    let mut guard = STREAM_STATUS_SINK
        .write()
        .unwrap_or_else(|e| e.into_inner());
    *guard = Some(sink.clone());
    drop(guard);

    // Initial status
    if ENGINE_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
        if crate::core::state::GLOBAL_STATE
            .is_failover_mode
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            let _ = sink.add("Failover".to_string());
        } else {
            let _ = sink.add("Running".to_string());
        }
    } else {
        let _ = sink.add("Stopped".to_string());
    }
}

pub fn api_init_audio_system(device_name: Option<String>) -> Result<(), AtmosError> {
    // 사용자가 엔진을 직접 다시 띄우면 재시작 복원 대기는 버린다(core::restart_resume).
    // 세대를 먼저 올려야 감시 루프의 스냅샷이 버린 뒤에 끼어들지 못한다.
    stop_audio_engine();
    crate::core::restart_resume::cancel_all();
    // 비상 전환 뒤 이 장치가 돌아오면 이름을 지정해 다시 연다(core::device_return).
    crate::core::device_return::remember_requested_device(device_name.clone());
    start_audio_system(device_name)
}

/// 자동 재연결(재기동 스레드)용 기동. 재시작 복원 대기를 지킨다.
fn init_audio_system(device_name: Option<String>) -> Result<(), AtmosError> {
    stop_audio_engine();
    start_audio_system(device_name)
}

/// 엔진 기동 본체. 부르기 전에 `stop_audio_engine`으로 세대를 올려 둔다.
fn start_audio_system(device_name: Option<String>) -> Result<(), AtmosError> {
    let rx = crate::core::state::GLOBAL_STATE.command_receiver.clone();

    let gen = ENGINE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    let (tx, rx_init) = std::sync::mpsc::channel();

    let prev_handle = {
        if let Ok(mut guard) = ENGINE_THREAD.lock() {
            guard.take()
        } else {
            None
        }
    };

    let handle = std::thread::spawn(move || {
        // 백그라운드 스레드 내에서 이전 스레드가 완전히 종료될 때까지 대기 (UI 프리징 방지)
        if let Some(h) = prev_handle {
            let _ = h.join();
        }

        std::thread::sleep(std::time::Duration::from_millis(100)); // safe margin

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
        }

        let mut engine = crate::audio::engine::AudioEngine::new();
        crate::audio::engine::ENGINE_INIT_SIGNAL.store(false, std::sync::atomic::Ordering::SeqCst);
        let device_name_clone = device_name.clone();
        // 아래 Ok 분기에서 페일오버 해제 여부를 판단할 때 쓴다. device_name_clone은
        // engine.start()로 이동되므로 미리 값만 뽑아 둔다.
        let requested_explicit_device = device_name_clone.is_some();

        let is_asio = if let Some(ref name) = device_name_clone {
            name.starts_with("[ASIO]")
        } else {
            if let Some(config) = crate::core::state::GLOBAL_STATE
                .config
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
            {
                config
                    .device_name
                    .as_ref()
                    .map(|n| n.starts_with("[ASIO]"))
                    .unwrap_or(false)
            } else {
                false
            }
        };

        match engine.start(device_name_clone, rx, gen) {
            Ok(_) => {
                ENGINE_ACTIVE.store(true, std::sync::atomic::Ordering::SeqCst);
                if crate::core::state::debug_flags::trace_cmd() {
                    // 엔진 관리 스레드(오디오 콜백 아님). 시작/종료가 짝을 이루지 않으면
                    // 두 믹서가 명령 큐를 나눠 먹는 중이다(재생/정지가 엇갈린다).
                    eprintln!("[CMD] 엔진 시작 세대={gen}");
                }

                // Wait for callback to actually fire (Atomic spin-wait)
                let start = std::time::Instant::now();
                while !crate::audio::engine::ENGINE_INIT_SIGNAL
                    .load(std::sync::atomic::Ordering::Acquire)
                {
                    if start.elapsed().as_millis() > 2000 {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                let _ = tx.send(Ok(()));

                // 사용자가 지정한 장치로 정상 기동했으면 페일오버를 해제한다.
                //
                // is_failover_mode는 긴급 폴백 경로에서 true로만 저장되고 **어디에서도
                // false로 되돌리지 않았다**. 그런데 믹서 출력단이 이 플래그를 보고
                // -40dB(×0.01)를 걸기 때문에, 한 번 페일오버가 걸리면 이후 장치를 다시
                // 잡아도 앱을 재시작할 때까지 사실상 무음이 됐다(실기 증상: "인터페이스를
                // 켜고 리스캔해서 다시 잡고 재생해도 소리가 안 남"). 상태 배너도 같은
                // 이유로 계속 떴다.
                //
                // 폴백 경로는 장치 이름 없이(None) 열리므로 여기서 해제되지 않는다. 즉 "기본 장치로
                // 임시 전환된 상태"는 그대로 유지된다. 설정의 장치가 돌아오면 자기 재시작이 그 이름을
                // 지정해 열어 여기서 풀린다(core::device_return).
                if requested_explicit_device
                    && crate::core::state::GLOBAL_STATE
                        .is_failover_mode
                        .swap(false, std::sync::atomic::Ordering::Relaxed)
                {
                    crate::core::state::GLOBAL_STATE
                        .log("비상 전환 해제: 지정한 장치로 다시 열었다(출력 감쇠 해제)".to_string());
                }

                if crate::core::state::GLOBAL_STATE
                    .is_failover_mode
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    broadcast_stream_status("Failover".to_string());
                } else {
                    broadcast_stream_status("Running".to_string());
                }

                // Keep thread alive until next generation, and watch for DeviceNotAvailable
                let mut last_device_count: Option<usize> = None;
                let mut last_device_names: Vec<String> = Vec::new();
                let mut monitor_timer = 0;
                let mut load_timer = 0;
                let mut last_load_log: Option<std::time::Instant> = None;

                loop {
                    if ENGINE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != gen {
                        break;
                    }

                    if crate::core::state::GLOBAL_STATE
                        .device_needs_reset
                        .load(std::sync::atomic::Ordering::Acquire)
                    {
                        println!("⚠️ [디버깅] 장치 유실·재설정 요청 수신 (DeviceNotAvailable / kAsioResetRequest)");
                        crate::core::state::GLOBAL_STATE
                            .device_needs_reset
                            .store(false, std::sync::atomic::Ordering::Release);
                        break;
                    }

                    if let Some(err) = crate::core::state::GLOBAL_STATE
                        .engine_error
                        .read()
                        .unwrap_or_else(|e| e.into_inner())
                        .as_ref()
                    {
                        if err == "DeviceNotAvailable" {
                            println!(
                                "⚠️ [디버깅] 오디오 장치 유실 감지! (rtrb SPSC 환경, 큐 대기 상태)"
                            );
                            break;
                        }
                    }

                    // Watchdog: Check if callback hasn't fired in >1000ms
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or(std::time::Duration::from_secs(0))
                        .as_millis() as u64;
                    let last_cb = crate::core::state::GLOBAL_STATE
                        .watchdog_last_callback
                        .load(std::sync::atomic::Ordering::Relaxed);
                    if last_cb > 0 && (now_ms > last_cb + 1000) {
                        // 다음 발생 때 트리거를 좁히려면 실제 공백 길이를 남겨야 한다.
                        // 장치 열거로 인한 일시 정지(수백 ms~수 초)와 스레드 사망은
                        // 공백 길이가 다르게 찍힌다. println!은 앱 로그에 남지 않아
                        // 사후 분석이 불가능했으므로 파일 로그로 남긴다.
                        crate::core::state::GLOBAL_STATE.log(format!(
                            "🚨 [Watchdog] 오디오 콜백 공백 {}ms (임계 1000ms)! 엔진 강제 재기동...",
                            now_ms - last_cb
                        ));
                        break;
                    }

                    // 오디오 처리 부하(10초마다): 처리 시간이 자기 버퍼 길이의 70%를 넘은 콜백이나, 앞
                    // 콜백과의 간격이 자기 버퍼 길이의 1.5배를 넘은(장치가 이전 버퍼를 다시 냈을 수 있는)
                    // 콜백이 있으면 앱 로그에 남긴다. 넘치면 소리가 끊기고 느려진다(2026-10-08 RME 94채널).
                    // 같은 경고는 1분에 한 번만, 정상일 때는 10분에 한 번 요약을 남긴다.
                    load_timer += 1;
                    if load_timer >= 100 {
                        load_timer = 0;
                        let state = &crate::core::state::GLOBAL_STATE;
                        let relaxed = std::sync::atomic::Ordering::Relaxed;
                        let max_us = state.callback_max_us.swap(0, relaxed);
                        let overruns = state.callback_overruns.swap(0, relaxed);
                        let total_us = state.callback_total_us.swap(0, relaxed);
                        let count = state.callback_count.swap(0, relaxed);
                        let gap_max_us = state.callback_gap_max_us.swap(0, relaxed);
                        let heavy = state.callback_heavy.swap(0, relaxed);
                        let late = state.callback_late.swap(0, relaxed);
                        let budget_us = state.callback_budget_us.load(relaxed);
                        let avg_us = total_us.checked_div(count).unwrap_or(0);
                        let warn = heavy > 0 || late > 0 || overruns > 0;
                        let since_log = last_load_log.map(|t: std::time::Instant| t.elapsed());
                        let due = match since_log {
                            None => true,
                            Some(e) if warn => e >= std::time::Duration::from_secs(60),
                            Some(e) => e >= std::time::Duration::from_secs(600),
                        };
                        if budget_us > 0 && due {
                            state.log(format!(
                                "오디오 처리 부하{}: 최근 10초 콜백 {}회 평균 {:.1}ms · 최대 {:.1}ms / 예산 {:.1}ms, 예산 70% 넘음 {}회, 예산 초과 {}회, 늦은 콜백 {}회(간격 최대 {:.1}ms), 처리 채널 {}개{}",
                                if warn { " 높음" } else { "" },
                                count,
                                f64::from(avg_us) / 1000.0,
                                f64::from(max_us) / 1000.0,
                                f64::from(budget_us) / 1000.0,
                                heavy,
                                overruns,
                                late,
                                f64::from(gap_max_us) / 1000.0,
                                state.processing_channels.load(relaxed),
                                if warn { " — 쓰지 않는 출력 채널을 끄거나 리버브를 줄이면 줄어든다" } else { "" },
                            ));
                            last_load_log = Some(std::time::Instant::now());
                        }
                    }

                    // Device Topology Monitoring (Every 3000ms = 30 * 100ms)
                    monitor_timer += 1;
                    if monitor_timer >= 30 {
                        monitor_timer = 0;
                        if !is_asio {
                            if let Some(current_names) = monitored_output_device_names() {
                                if let Some(last_count) = last_device_count {
                                    if last_count != current_names.len()
                                        || last_device_names != current_names
                                    {
                                        println!("🔄 [Device Monitor] Detected devicelist change! (Topology changed)");
                                        let mut err_guard = crate::core::state::GLOBAL_STATE
                                            .engine_error
                                            .write()
                                            .unwrap_or_else(|e| e.into_inner());
                                        *err_guard = Some("DeviceListChanged".to_string());
                                        // Trigger auto-recovery by breaking the loop (similar to Ableton's behavior)
                                        break;
                                    }
                                }
                                last_device_count = Some(current_names.len());
                                last_device_names = current_names;
                            }
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_millis(100));
                }

                if crate::core::state::debug_flags::trace_cmd() {
                    eprintln!("[CMD] 엔진 종료 세대={gen}");
                }
                // 세대가 그대로면 자기 재시작(워치독·장치 오류·장치 목록 변화)이다. 옛 엔진을
                // 버리기 전에 재생 상태를 떠 둔다(core::restart_resume, 세대는 그 안에서 확인한다).
                crate::core::restart_resume::take_snapshot(gen);
                drop(engine);
                ENGINE_ACTIVE.store(false, std::sync::atomic::Ordering::SeqCst);
                broadcast_stream_status("Stopped".to_string());
                // 스트림이 사라진 뒤에도 마지막 콜백 시각이 남아 있으면, 다음 세대가
                // 아직 첫 콜백을 내지 못한 사이에 워치독이 낡은 값으로 오발한다.
                crate::core::state::GLOBAL_STATE
                    .watchdog_last_callback
                    .store(0, std::sync::atomic::Ordering::Relaxed);

                // If it wasn't a manual stop, auto restart
                if ENGINE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) == gen {
                    // 재기동은 반드시 **별도 스레드**에서 해야 한다. 이 코드를 실행하는
                    // 주체가 엔진 스레드 자신이기 때문이다. init_audio_system은
                    // ENGINE_THREAD에 들어 있는 핸들(= 바로 이 스레드)을 새 스레드에서
                    // join한 뒤, 자신은 rx_init.recv()로 engine.start() 결과를 기다린다.
                    // 그래서 여기서 직접 부르면 새 스레드는 이 스레드의 종료를 기다리고
                    // 이 스레드는 새 스레드의 결과를 기다려 서로 영구 교착된다. 그 결과
                    // 스트림이 다시 열리지 않고 앱이 조용히 무음이 된다(실기 증상:
                    // 워치독 발동 후 75분간 무음, Stream: 로그가 다시 찍히지 않음,
                    // 리스캔의 apiInitAudioSystem await도 끝나지 않음).
                    // 여기서 스레드를 띄우고 즉시 반환하면 이 스레드가 종료되므로
                    // 새 스레드의 join이 풀린다.
                    let current_device = device_name.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(100)); // Fast auto hot-reload
                        if ENGINE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != gen {
                            return; // 그 사이 사용자가 직접 정지/재기동했다
                        }
                        // 비상 엔진(기본 장치)이면 설정의 장치가 돌아왔는지 보고, 돌아왔으면 이름을 지정해 연다.
                        // 그래야 비상 감쇠가 풀린다(core::device_return).
                        let failover = crate::core::state::GLOBAL_STATE
                            .is_failover_mode
                            .load(std::sync::atomic::Ordering::Relaxed);
                        let requested = crate::core::device_return::requested_device();
                        let available: Vec<String> = if failover && requested.is_some() {
                            api_get_output_devices()
                                .map(|devices| devices.into_iter().map(|d| d.name).collect())
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        };
                        let restart_device = crate::core::device_return::device_for_self_restart(
                            current_device.as_deref(),
                            failover,
                            requested.as_deref(),
                            &available,
                        );
                        if restart_device != current_device {
                            if let Some(ref name) = restart_device {
                                crate::core::state::GLOBAL_STATE.log(format!(
                                    "비상 전환 해제 시도: 설정의 장치 '{name}'가 돌아와 이름을 지정해 다시 연다"
                                ));
                            }
                        }
                        println!("🔄 [디버깅] 자동 재연결(Hot-Reload) 수행!");
                        broadcast_stream_status("HotReloading".to_string());
                        let started = match init_audio_system(restart_device) {
                            Ok(()) => true,
                            Err(_e) => {
                                // Emergency Failover
                                println!("🚨 [Failover] 장치 재연결 실패. WASAPI 기본 장치로 강제 비상 전환!");
                                crate::core::state::GLOBAL_STATE.log(
                                    "비상 전환: 장치를 다시 열지 못해 기본 장치로 임시 전환한다(출력 −40dB)".to_string(),
                                );
                                crate::core::state::GLOBAL_STATE
                                    .is_failover_mode
                                    .store(true, std::sync::atomic::Ordering::Relaxed);
                                init_audio_system(None).is_ok() // None forces default OS device
                            }
                        };
                        if started {
                            crate::core::restart_resume::resume_after_resync();
                        }
                    });
                }
            }
            Err(e) => {
                let _ = tx.send(Err(e));

                // Auto reconnect on initial boot failure if it was DeviceNotAvailable
                // Wait, boot failures are returned immediately. So we don't block here.
            }
        }
    });

    if let Ok(mut guard) = ENGINE_THREAD.lock() {
        *guard = Some(handle);
    }

    rx_init
        .recv()
        .unwrap_or_else(|_| Err("Failed to communicate with audio thread".to_string()))
        .map_err(|e| AtmosError {
            message: format!("Failed to start audio engine: {}", e),
        })
}

pub fn api_start_audio_engine(device_name: Option<String>) {
    let _ = api_init_audio_system(device_name);
}

/// 재생 중인 트랙의 파일 기준 재생 위치. 루프는 한 바퀴 안의 위치다.
#[derive(Debug, Clone)]
pub struct PlaybackPosition {
    pub track_id: String,
    pub seconds: f64,
}

/// 재생 중인 트랙별 재생 위치(초). 오디오 스레드가 원자 칸에 적어 둔 값을 읽기만 한다.
pub fn api_get_playback_positions() -> Vec<PlaybackPosition> {
    let playing = GLOBAL_STATE
        .playing_track_ids
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let mut out: Vec<PlaybackPosition> = crate::audio::playback_cursor::CURSOR_TABLE
        .snapshot()
        .into_iter()
        .filter_map(|(instance_id, seconds)| {
            playing
                .get(&instance_id)
                .map(|track_id| PlaybackPosition { track_id: track_id.clone(), seconds })
        })
        .collect();
    out.sort_by(|a, b| a.track_id.cmp(&b.track_id).then(a.seconds.total_cmp(&b.seconds)));
    out
}

/// Dart가 `seq`번 엔진 재시작 뒤 재동기화를 마쳤다고 알린다(core::restart_resume).
pub fn api_ack_engine_restart(seq: u32) {
    crate::core::restart_resume::ack(seq);
}

pub fn api_is_engine_ready() -> bool {
    if !ENGINE_ACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
        return false;
    }
    crate::audio::engine::ENGINE_INIT_SIGNAL.load(std::sync::atomic::Ordering::Acquire)
}

pub fn api_stop_audio_engine() {
    stop_audio_engine();
    crate::core::restart_resume::cancel_all();
}

fn stop_audio_engine() {
    let _ = ENGINE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    println!("✅ [디버깅] 백엔드 오디오 엔진 명시적 종료 지시 완료. (비동기 종료 진행)");
    broadcast_stream_status("Stopped".to_string());
}

pub fn api_open_asio_panel() {
    crate::core::state::GLOBAL_STATE.log("ASIO Control Panel opening is no longer supported directly. Please open it via your driver's application (e.g. TotalMix).".to_string());
}

pub fn api_get_asio_panel() {
    // dummy function
}
pub fn api_create_device_event_stream(sink: StreamSink<String>) {
    std::thread::spawn(move || {
        let mut last_err: Option<String> = None;
        // 엔진 준비 완료를 Dart에 알린다.
        //
        // 예전에는 이 스트림이 장치 유실 문자열만 보냈고 "EngineReady"는 Rust
        // 어디에서도 발신하지 않았다. 그런데 Dart 세 곳이 그 문자열을 기다린다:
        //   - global_state.dart: 설정 변경으로 엔진을 재기동한 뒤 준비 대기
        //   - audio_init_splash_screen.dart: 앱 부팅 시 초기화 대기
        //   - main.dart: 엔진이 스스로 되살아났을 때(워치독 복구) 재동기화
        // 앞의 둘은 항상 5초 타임아웃으로 빠졌고(사용자에게 "오디오 엔진 연결
        // 시간 초과" 오류까지 표시), 마지막 하나는 실행된 적 없는 죽은 코드였다.
        let mut last_ready = false;
        // 자기 재시작 순번. 바뀔 때마다 알린다 — 준비 상태 폴링과 달리 짧은 재기동도 놓치지 않고,
        // 순번이 매번 달라 Dart 쪽에서 같은 값으로 걸러지지 않는다.
        let mut last_restart_seq =
            crate::core::restart_resume::RESTART_SEQ.load(std::sync::atomic::Ordering::Acquire);
        loop {
            // 준비 상태가 false -> true로 바뀌면 알린다. last_ready가 false로
            // 시작하므로, 스트림을 만든 시점에 이미 준비돼 있으면 첫 폴링에서
            // 즉시 한 번 발신한다(준비 직후 스트림을 만드는 경쟁 상황 대비).
            // 재기동으로 true -> false -> true가 되면 다시 발신하므로 워치독
            // 복구 훅도 그때 동작한다.
            let ready = api_is_engine_ready();
            if ready && !last_ready && sink.add("EngineReady".to_string()).is_err() {
                break; // Stop thread if port is closed
            }
            last_ready = ready;
            let restart_seq =
                crate::core::restart_resume::RESTART_SEQ.load(std::sync::atomic::Ordering::Acquire);
            if restart_seq != last_restart_seq {
                last_restart_seq = restart_seq;
                if sink.add(format!("EngineRestarted:{restart_seq}")).is_err() {
                    break; // Stop thread if port is closed
                }
            }

            let current_err = GLOBAL_STATE
                .engine_error
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            if current_err != last_err {
                if let Some(ref err) = current_err {
                    if (err == "DeviceNotAvailable" || err.contains("Disconnected"))
                        && sink.add(err.clone()).is_err()
                    {
                        break; // Stop thread if port is closed
                    }
                }
                last_err = current_err;
            }
            // 재시작 알림 지연은 재개 전 무음 길이에 그대로 더해진다.
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });
}

pub fn api_force_restart_engine(device_name: Option<String>) {
    println!("🔄 [디버깅] 백엔드 오디오 엔진 강제 재시작 요청됨.");
    api_stop_audio_engine();

    let _ = api_init_audio_system(device_name);
}

pub fn api_start_osc_listener(port: u16) {
    let listener = crate::osc::listener::OscListener::new();
    listener.start(port);
}

#[derive(Debug, Clone)]
pub struct EngineStateUpdate {
    pub active_room_id: Option<String>,
    pub ducking_active: bool,
    pub playing_track_ids: Vec<String>,
    pub engine_error: Option<String>,
    pub output_channel_count: u32,
    pub short_term_lufs: f32,
    pub gain_reduction_db: f32,
}

pub fn api_create_engine_state_stream(sink: StreamSink<EngineStateUpdate>) {
    let playing_track_ids = {
        let guard = GLOBAL_STATE
            .playing_track_ids
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let mut unique_ids: Vec<String> = guard.values().cloned().collect();
        unique_ids.sort();
        unique_ids.dedup();
        unique_ids
    };

    let initial_state = EngineStateUpdate {
        active_room_id: GLOBAL_STATE
            .active_room_id
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone(),
        ducking_active: GLOBAL_STATE
            .is_ducking
            .load(std::sync::atomic::Ordering::Relaxed),
        playing_track_ids,
        engine_error: GLOBAL_STATE
            .engine_error
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone(),
        output_channel_count: GLOBAL_STATE
            .active_device_channels
            .load(std::sync::atomic::Ordering::Relaxed),
        short_term_lufs: f32::from_bits(
            GLOBAL_STATE
                .current_master_lufs
                .load(std::sync::atomic::Ordering::Relaxed),
        ),
        gain_reduction_db: f32::from_bits(
            GLOBAL_STATE
                .current_gain_reduction_db
                .load(std::sync::atomic::Ordering::Relaxed),
        ),
    };
    let _ = sink.add(initial_state);
    *GLOBAL_STATE
        .state_sink
        .write()
        .unwrap_or_else(|e| e.into_inner()) = Some(sink);
}

pub fn api_preload_all_sounds(config: AppConfig) -> Result<(), AtmosError> {
    let mut needed_files = std::collections::HashSet::new();
    for room in &config.rooms {
        for track in &room.tracks {
            if !track.is_loop && !track.is_streaming {
                needed_files.insert(track.file_path.clone());
            }
        }
    }

    let mut missing_files = Vec::new();
    {
        let cache = GLOBAL_STATE
            .sound_cache
            .read()
            .unwrap_or_else(|e| e.into_inner());
        for file in &needed_files {
            if !cache.contains_key(file) {
                missing_files.push(file.clone());
            }
        }
    }

    let mut newly_loaded = Vec::new();
    let mut errors = Vec::new();
    for file in missing_files {
        let path = std::path::Path::new(&file);

        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > 500 * 1024 * 1024 {
                let err_msg = format!("파일 용량이 너무 큽니다 (500MB 초과). BGM(Loop)으로 설정하거나 용량을 줄이세요: {}", file);
                GLOBAL_STATE.log(err_msg.clone());
                errors.push(err_msg);
                continue;
            }
        }

        // Load into RAM regardless of size (SFX only, loop=false)
        let target_sr = GLOBAL_STATE
            .engine_sample_rate
            .load(std::sync::atomic::Ordering::Relaxed);
        match crate::audio::player::SoundData::load_from_file(path, target_sr) {
            Ok(data) => {
                GLOBAL_STATE.log(format!("Loaded sound file: {}", file));
                newly_loaded.push((file, std::sync::Arc::new(data)));
            }
            Err(e) => {
                let err_msg = format!("Failed to load sound file {}: {}", file, e);
                GLOBAL_STATE.log(err_msg.clone());
                errors.push(err_msg);

                // Negative cache to prevent disk spam on every UI tick
                let empty_data = crate::audio::player::SoundData {
                    sample_rate: 48000,
                    channels: 2,
                    samples: vec![],
                };
                newly_loaded.push((file, std::sync::Arc::new(empty_data)));
            }
        }
    }

    {
        let mut cache = GLOBAL_STATE
            .sound_cache
            .write()
            .unwrap_or_else(|e| e.into_inner());
        cache.retain(|path, _| needed_files.contains(path));
        for (path, data) in newly_loaded {
            cache.insert(path, data);
        }
    }

    apply_enabled_channels(&config);

    let mut global_config = GLOBAL_STATE
        .config
        .write()
        .unwrap_or_else(|e| e.into_inner());
    GLOBAL_STATE
        .config_version
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    *global_config = Some(config.clone());
    drop(global_config);

    // Check if active_room_id exists in the new config
    let active_room_id = {
        let guard = GLOBAL_STATE
            .active_room_id
            .read()
            .unwrap_or_else(|e| e.into_inner());
        guard.clone()
    };

    if let Some(active_id) = active_room_id {
        let room_exists = config.rooms.iter().any(|r| r.id == active_id);
        if !room_exists {
            // Room was deleted! Safely clear the room
            let _ = api_clear_room(active_id);
        }
    }

    if !errors.is_empty() {
        return Err(AtmosError {
            message: errors.join(" | "),
        });
    }

    Ok(())
}

#[derive(Clone, Debug)]
pub struct OutputDeviceInfo {
    pub name: String,
    pub max_channels: u32,
    pub channel_names: Vec<String>,
}

pub fn api_get_output_devices() -> Result<Vec<OutputDeviceInfo>, AtmosError> {
    std::thread::spawn(|| {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
        }

        use cpal::traits::{DeviceTrait, HostTrait};

        let is_engine_active = ENGINE_ACTIVE.load(std::sync::atomic::Ordering::SeqCst);

        let mut skip_asio_scan = false;
        let mut active_asio_device = None;
        if is_engine_active {
            if let Some(config) = crate::core::state::GLOBAL_STATE
                .config
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
            {
                if let Some(ref saved_name) = config.device_name {
                    if saved_name.starts_with("[ASIO]") {
                        skip_asio_scan = true;
                        active_asio_device = Some(saved_name.clone());
                    }
                }
            }
        }

        let target_prefix = if skip_asio_scan {
            Some("[WASAPI]")
        } else {
            None
        };
        let hosts = crate::audio::engine::get_hosts(target_prefix)
            .map_err(|e| AtmosError { message: e })?;

        let mut device_info_list = Vec::new();

        if skip_asio_scan {
            if let Some(saved_name) = active_asio_device {
                let active_ch = crate::core::state::GLOBAL_STATE
                    .active_device_channels
                    .load(std::sync::atomic::Ordering::SeqCst);
                let max_channels = if active_ch > 0 { active_ch } else { 2 };
                let actual_name = saved_name.replace("[ASIO] ", "").trim().to_string();
                #[cfg(target_os = "macos")]
                let channel_names =
                    crate::audio::channel_names::get_channel_names_mac(&actual_name, max_channels);
                #[cfg(target_os = "windows")]
                let channel_names =
                    crate::audio::channel_names::get_channel_names_win(&actual_name, max_channels);
                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                let channel_names =
                    crate::audio::channel_names::get_channel_names_fallback(max_channels);

                device_info_list.push(OutputDeviceInfo {
                    name: saved_name,
                    max_channels,
                    channel_names,
                });
            }
        }

        for host in hosts {
            let prefix = format!("[{}] ", host.id().name());
            let is_asio_host = host.id().name() == "ASIO";

            if is_asio_host && skip_asio_scan {
                continue; // Do not call host.output_devices() because querying ASIO devices breaks the COM lock!
            }

            let devices_result = if is_asio_host {
                #[cfg(target_os = "windows")]
                {
                    let (tx, rx) = std::sync::mpsc::channel();
                    std::thread::spawn(move || {
                        let res =
                            cpal::host_from_id(cpal::HostId::Asio).map(|h| h.output_devices());
                        let _ = tx.send(res);
                    });
                    match rx.recv_timeout(std::time::Duration::from_secs(2)) {
                        Ok(Ok(Ok(d))) => Ok(d),
                        Ok(Ok(Err(e))) => Err(e),
                        Ok(Err(_e)) => Err(cpal::DevicesError::BackendSpecific {
                            err: cpal::BackendSpecificError {
                                description: "ASIO host init failed".to_string(),
                            },
                        }),
                        Err(_) => Err(cpal::DevicesError::BackendSpecific {
                            err: cpal::BackendSpecificError {
                                description: "ASIO scan timed out (buggy driver?)".to_string(),
                            },
                        }),
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    host.output_devices()
                }
            } else {
                host.output_devices()
            };

            #[allow(unused_mut)]
            let mut devices_result = devices_result;
            if devices_result.is_err() && is_asio_host {
                for _ in 0..2 {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    #[cfg(target_os = "windows")]
                    {
                        let (tx, rx) = std::sync::mpsc::channel();
                        std::thread::spawn(move || {
                            let res =
                                cpal::host_from_id(cpal::HostId::Asio).map(|h| h.output_devices());
                            let _ = tx.send(res);
                        });
                        match rx.recv_timeout(std::time::Duration::from_secs(2)) {
                            Ok(Ok(Ok(d))) => {
                                devices_result = Ok(d);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            let devices = match devices_result {
                Ok(d) => d,
                Err(e) => {
                    eprintln!(
                        "Failed to get output devices for host {}: {:?}",
                        host.id().name(),
                        e
                    );
                    // If ASIO fails completely but we have a saved ASIO device, inject it manually to prevent UI reset
                    if is_asio_host {
                        if let Some(config) = crate::core::state::GLOBAL_STATE
                            .config
                            .read()
                            .unwrap_or_else(|e| e.into_inner())
                            .as_ref()
                        {
                            if let Some(ref saved_name) = config.device_name {
                                if saved_name.starts_with("[ASIO]") {
                                    let actual_name =
                                        saved_name.replace("[ASIO] ", "").trim().to_string();
                                    let max_channels = if crate::core::state::GLOBAL_STATE
                                        .active_device_channels
                                        .load(std::sync::atomic::Ordering::SeqCst)
                                        > 0
                                    {
                                        crate::core::state::GLOBAL_STATE
                                            .active_device_channels
                                            .load(std::sync::atomic::Ordering::SeqCst)
                                    } else {
                                        2
                                    };

                                    #[cfg(target_os = "windows")]
                                    let channel_names =
                                        crate::audio::channel_names::get_channel_names_win(
                                            &actual_name,
                                            max_channels,
                                        );
                                    #[cfg(not(target_os = "windows"))]
                                    let channel_names = {
                                        let _ = actual_name;
                                        vec![]
                                    };

                                    device_info_list.push(OutputDeviceInfo {
                                        name: saved_name.clone(),
                                        max_channels,
                                        channel_names,
                                    });
                                }
                            }
                        }
                    }
                    continue;
                }
            };

            for device in devices {
                if let Ok(name_str) = device.name() {
                    let actual_name = name_str.replace('\0', "").trim().to_string();
                    let name = format!("{}{}", prefix, actual_name);
                    let mut max_channels = 2; // Default fallback

                    // Avoid querying configs if it's ASIO (though skip_asio_scan handles active ASIO lock already)
                    if is_asio_host && is_engine_active {
                        let mut is_the_active_device = false;
                        if let Some(config) = crate::core::state::GLOBAL_STATE
                            .config
                            .read()
                            .unwrap_or_else(|e| e.into_inner())
                            .as_ref()
                        {
                            if let Some(ref saved_name) = config.device_name {
                                if saved_name.trim() == name.trim() {
                                    is_the_active_device = true;
                                }
                            }
                        }

                        if is_the_active_device {
                            let active_ch = crate::core::state::GLOBAL_STATE
                                .active_device_channels
                                .load(std::sync::atomic::Ordering::SeqCst);
                            max_channels = if active_ch > 0 { active_ch } else { 2 };
                        } else {
                            max_channels = 2;
                        }
                    } else {
                        // Skip capability probing for known buggy generic ASIO drivers to prevent app hangs
                        let actual_name_lower = actual_name.to_lowercase();
                        let is_buggy_generic_asio = is_asio_host
                            && (actual_name_lower.contains("generic low latency")
                                || actual_name_lower.contains("fl studio")
                                || actual_name_lower.contains("asio4all")
                                || actual_name_lower.contains("realtek asio"));

                        let mut configs_result = if is_buggy_generic_asio {
                            Err(cpal::SupportedStreamConfigsError::BackendSpecific {
                                err: cpal::BackendSpecificError {
                                    description: "Skipped buggy driver".to_string(),
                                },
                            })
                        } else {
                            device.supported_output_configs()
                        };

                        if configs_result.is_err() && is_asio_host && !is_buggy_generic_asio {
                            for _ in 0..3 {
                                std::thread::sleep(std::time::Duration::from_millis(500));
                                configs_result = device.supported_output_configs();
                                if configs_result.is_ok() {
                                    break;
                                }
                            }
                        }

                        if let Ok(supported_configs) = configs_result {
                            for config in supported_configs {
                                let channels = config.channels() as u32;
                                if channels > max_channels {
                                    max_channels = channels;
                                }
                            }
                        }
                        if let Ok(default_config) = device.default_output_config() {
                            let channels = default_config.channels() as u32;
                            if channels > max_channels {
                                max_channels = channels;
                            }
                        }

                        // Fallback: If ASIO max_channels is still 2 after query fails or returns only 2, and it matches the saved active ASIO device, reuse the active channel count
                        if is_asio_host && max_channels <= 2 {
                            if let Some(config) = crate::core::state::GLOBAL_STATE
                                .config
                                .read()
                                .unwrap_or_else(|e| e.into_inner())
                                .as_ref()
                            {
                                if let Some(ref saved_name) = config.device_name {
                                    if saved_name.trim() == name.trim() {
                                        let active_ch = crate::core::state::GLOBAL_STATE
                                            .active_device_channels
                                            .load(std::sync::atomic::Ordering::SeqCst);
                                        if active_ch > max_channels {
                                            max_channels = active_ch;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    #[cfg(target_os = "macos")]
                    let channel_names = crate::audio::channel_names::get_channel_names_mac(
                        &actual_name,
                        max_channels,
                    );
                    #[cfg(target_os = "windows")]
                    let channel_names = crate::audio::channel_names::get_channel_names_win(
                        &actual_name,
                        max_channels,
                    );
                    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                    let channel_names =
                        crate::audio::channel_names::get_channel_names_fallback(max_channels);

                    device_info_list.push(OutputDeviceInfo {
                        name,
                        max_channels,
                        channel_names,
                    });
                }
            }

            // cpal 0.15.3(macOS)의 output_devices()는 출력 전용 장치(맥북 내장 스피커 등)를
            // 빠뜨린다(audio::coreaudio_fallback 문서 참고). 전체 장치 중 CoreAudio가 출력
            // 채널을 보고하는데 아직 목록에 없는 장치를 덧붙인다.
            #[cfg(target_os = "macos")]
            if let Ok(all_devices) = host.devices() {
                for device in all_devices {
                    let Ok(name_str) = device.name() else { continue };
                    let actual_name = name_str.replace('\0', "").trim().to_string();
                    let name = format!("{}{}", prefix, actual_name);
                    if device_info_list.iter().any(|d| d.name == name) {
                        continue;
                    }
                    let Some(config) = crate::audio::coreaudio_fallback::output_config(&actual_name)
                    else {
                        continue;
                    };
                    let max_channels = config.channels() as u32;
                    let channel_names = crate::audio::channel_names::get_channel_names_mac(
                        &actual_name,
                        max_channels,
                    );
                    device_info_list.push(OutputDeviceInfo {
                        name,
                        max_channels,
                        channel_names,
                    });
                }
            }
        }

        Ok(device_info_list)
    })
    .join()
    .unwrap_or_else(|_| {
        Err(AtmosError {
            message: "Thread panicked during device lookup".to_string(),
        })
    })
}

/// 장치 목록 감시(엔진 점검 루프, 3초마다)가 비교할 출력 장치 이름. 읽지 못하면 None.
///
/// Windows에서는 WASAPI 장치만 센다. `api_get_output_devices`는 ASIO 목록도 읽는데, cpal은 ASIO
/// 목록을 만들 때 설치된 ASIO 드라이버를 하나씩 전부 불러온다(Generic Low Latency, FL Studio, 장치
/// 드라이버 등). WASAPI로 재생하는 동안 3초마다 그렇게 해서 Generic Low Latency ASIO 드라이버가
/// 계속 떴다(2026-10-08 Windows 실기). 감시는 WASAPI로 재생할 때만 돌고 WASAPI 장치 변화만 보면 된다.
fn monitored_output_device_names() -> Option<Vec<String>> {
    #[cfg(target_os = "windows")]
    {
        std::thread::spawn(|| {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
            use cpal::traits::{DeviceTrait, HostTrait};
            let devices = cpal::default_host().output_devices().ok()?;
            Some(
                devices
                    .filter_map(|d| d.name().ok())
                    .map(|n| format!("[WASAPI] {}", n.replace('\0', "").trim()))
                    .collect::<Vec<String>>(),
            )
        })
        .join()
        .ok()
        .flatten()
    }
    #[cfg(not(target_os = "windows"))]
    {
        api_get_output_devices()
            .ok()
            .map(|devices| devices.into_iter().map(|d| d.name).collect())
    }
}

/// 출력 장치 부재를 뜻하는 오류 메시지 접두사.
///
/// Dart 쪽 `OutputChannelsNotifier._refresh()`
/// (lib/core/state/global_state.dart)가 이 문자열을 부분 일치로 검사해서
/// `OutputChannelsStatus.noDevice`와 일반 오류를 구분한다. AtmosError에는
/// 종류를 구분하는 필드가 없어 메시지가 유일한 단서이므로, 여기 값을 바꾸면
/// Dart 매처도 함께 바꿔야 한다. 아래 테스트가 그 계약을 잠근다.
pub const ERR_NO_DEFAULT_OUTPUT_DEVICE: &str = "No default output device";

/// 저장된 장치가 시스템에서 사라졌을 때의 오류 메시지 접두사.
/// 자세한 내용은 [`ERR_NO_DEFAULT_OUTPUT_DEVICE`] 참고.
pub const ERR_DEVICE_NOT_FOUND: &str = "Device not found";

pub fn api_get_device_channel_count(device_name: Option<String>) -> Result<u32, AtmosError> {
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
        }
        use cpal::traits::{DeviceTrait, HostTrait};

        let is_engine_active = ENGINE_ACTIVE.load(std::sync::atomic::Ordering::SeqCst);
        let mut active_asio_device = None;
        if is_engine_active {
            if let Some(config) = crate::core::state::GLOBAL_STATE
                .config
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
            {
                if let Some(ref saved_name) = config.device_name {
                    if saved_name.starts_with("[ASIO]") {
                        active_asio_device = Some(saved_name.clone());
                    }
                }
            }
        }

        let device = if let Some(ref name) = device_name {
            if is_engine_active && name.starts_with("[ASIO]") {
                if let Some(ref active_name) = active_asio_device {
                    if name.trim() == active_name.trim() {
                        let active_ch = crate::core::state::GLOBAL_STATE
                            .active_device_channels
                            .load(std::sync::atomic::Ordering::SeqCst);
                        return Ok(if active_ch > 0 { active_ch } else { 2 });
                    }
                }
                return Ok(2); // Any inactive ASIO device defaults to 2 (stereo) while engine is running
            }

            let target_prefix = if name.starts_with("[ASIO]") {
                Some("[ASIO]")
            } else if name.starts_with("[WASAPI]") {
                Some("[WASAPI]")
            } else if name.starts_with("[CoreAudio]") {
                Some("[CoreAudio]")
            } else {
                None
            };

            let hosts = crate::audio::engine::get_hosts(target_prefix)
                .map_err(|e| AtmosError { message: e })?;

            let mut found_device = None;
            let target_name = name
                .replace("[ASIO] ", "")
                .replace("[WASAPI] ", "")
                .replace("[CoreAudio] ", "");
            let target_name = target_name.replace('\0', "").trim().to_string();

            for host in &hosts {
                if let Ok(devices) = host.output_devices() {
                    for d in devices {
                        if let Ok(d_name) = d.name() {
                            if d_name.replace('\0', "").trim() == target_name {
                                found_device = Some(d);
                                break;
                            }
                        }
                    }
                }
                #[cfg(target_os = "macos")]
                if found_device.is_none() {
                    found_device = crate::audio::coreaudio_fallback::find_output_device(host, &target_name);
                }
                if found_device.is_some() {
                    break;
                }
            }
            found_device.ok_or_else(|| AtmosError {
                message: format!("{}: {}", ERR_DEVICE_NOT_FOUND, name),
            })?
        } else {
            cpal::default_host()
                .default_output_device()
                .ok_or_else(|| AtmosError {
                    message: ERR_NO_DEFAULT_OUTPUT_DEVICE.to_string(),
                })?
        };

        let mut max_channels = 0;
        if let Ok(supported_configs) = device.supported_output_configs() {
            for config in supported_configs {
                let channels = config.channels() as u32;
                if channels > max_channels {
                    max_channels = channels;
                }
            }
        }

        if let Ok(default_config) = device.default_output_config() {
            let channels = default_config.channels() as u32;
            if channels > max_channels {
                max_channels = channels;
            }
        }

        // cpal이 설정을 못 읽는 출력 전용 장치(audio::coreaudio_fallback 문서 참고).
        #[cfg(target_os = "macos")]
        if max_channels == 0 {
            if let Some(config) = device.name().ok().and_then(|n| {
                crate::audio::coreaudio_fallback::output_config(n.replace('\0', "").trim())
            }) {
                max_channels = config.channels() as u32;
            }
        }

        Ok(max_channels)
    })
    .join()
    .unwrap_or_else(|_| {
        Err(AtmosError {
            message: "Thread panicked during channel count".to_string(),
        })
    })
}

pub fn api_get_device_channel_names(
    device_name: Option<String>,
) -> Result<Vec<String>, AtmosError> {
    let max_channels = api_get_device_channel_count(device_name.clone())?;

    // 기본 장치("선택 안 함" = None)일 때 문자열 "Default"를 그대로 CoreAudio
    // 이름 조회에 넘기면, 그런 이름의 장치는 실존하지 않으므로 조회가 항상
    // 실패해 get_channel_names_mac/_win이 "Channel 1".."Channel N"으로
    // 폴백한다. 그 이름들에는 필터가 가상 채널을 판별할 토큰(daw/loopback/
    // virtual)이 없어 전부 물리로 오분류된다(실기 증상: 기본값 선택 시
    // "물리 출력 12채널", 장치를 명시적으로 고르면 올바르게 "6채널").
    // 실제 기본 출력 장치의 진짜 이름을 조회해 넘겨야 한다.
    let actual_name = if let Some(ref name) = device_name {
        if let Some(idx) = name.find("] ") {
            name[idx + 2..].to_string()
        } else {
            name.clone()
        }
    } else {
        use cpal::traits::{DeviceTrait, HostTrait};
        cpal::default_host()
            .default_output_device()
            .and_then(|d| d.name().ok())
            .unwrap_or_else(|| "Default".to_string())
    };

    #[cfg(target_os = "macos")]
    let channel_names =
        crate::audio::channel_names::get_channel_names_mac(&actual_name, max_channels);
    #[cfg(target_os = "windows")]
    let channel_names =
        crate::audio::channel_names::get_channel_names_win(&actual_name, max_channels);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let channel_names = crate::audio::channel_names::get_channel_names_fallback(max_channels);

    Ok(channel_names)
}

pub fn api_export_logs(destination_dir: String) -> Result<(), AtmosError> {
    // 지금 로그와 뒤로 밀린 로그(core::log_file)를 모두 복사한다.
    let copied = crate::core::log_file::export_to(
        &crate::core::log_file::log_dir(),
        std::path::Path::new(&destination_dir),
    )
    .map_err(|e| AtmosError {
        message: format!("Failed to copy log file: {}", e),
    })?;
    if copied == 0 {
        return Err(AtmosError {
            message: "Log file does not exist".to_string(),
        });
    }
    Ok(())
}

pub fn api_play_all_loop_tracks() -> Result<(), AtmosError> {
    let config = {
        let guard = GLOBAL_STATE
            .config
            .read()
            .unwrap_or_else(|e| e.into_inner());
        guard.as_ref().cloned()
    };

    if let Some(config) = config {
        for room in config.rooms {
            for track in room.tracks {
                if track.is_loop {
                    let _ = api_play_track(room.id.clone(), track.id.clone());
                }
            }
        }
    }
    Ok(())
}

pub fn api_load_preset(config: AppConfig) -> Result<(), AtmosError> {
    api_stop_all()?;

    GLOBAL_STATE.is_exhibition_mode.store(
        config.is_exhibition_mode,
        std::sync::atomic::Ordering::Relaxed,
    );

    // This will sync GLOBAL_STATE config, enabled_channels, and manage the cache
    api_preload_all_sounds(config)?;
    Ok(())
}

pub fn api_trigger_test_error(message: String) -> Result<(), AtmosError> {
    *GLOBAL_STATE
        .engine_error
        .write()
        .unwrap_or_else(|e| e.into_inner()) = Some(message);
    GLOBAL_STATE.broadcast_state();
    Ok(())
}

pub fn api_get_audio_file_channels(file_path: String) -> u32 {
    if let Ok(cache) = crate::core::state::GLOBAL_STATE.sound_cache.read() {
        if let Some(data) = cache.get(&file_path) {
            return data.channels as u32;
        }
    }
    let path = std::path::Path::new(&file_path);
    crate::audio::player::SoundData::probe_channels(path)
}

use crate::common::config::EqBand;

pub struct ChannelTuningParams {
    pub channel: u32,
    pub delay_ms: f32,
    pub eq_bands: Vec<EqBand>,
    pub phase_invert: bool,
    pub gain_db: f32,
}

pub fn api_apply_all_channel_tunings(tunings: Vec<ChannelTuningParams>) -> Result<(), AtmosError> {
    let cmd_tunings = tunings
        .iter()
        .map(|t| {
            (
                t.channel as usize,
                t.delay_ms,
                t.eq_bands.clone(),
                t.phase_invert,
                t.gain_db,
            )
        })
        .collect();
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::ApplyAllChannelTunings {
            tunings: cmd_tunings,
        })
        .map_err(|e| AtmosError {
            message: format!("Failed to apply all channel tunings: {}", e),
        })?;

    if let Ok(mut config_guard) = GLOBAL_STATE.config.write() {
        GLOBAL_STATE
            .config_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Some(config) = config_guard.as_mut() {
            for t in tunings {
                let ch_key = t.channel + 1;
                if let Some(setting) = config.mono_configs.get_mut(&ch_key) {
                    setting.delay_ms = t.delay_ms;
                    setting.eq_bands = t.eq_bands.clone();
                    setting.phase_invert = t.phase_invert;
                    setting.gain_db = t.gain_db;
                } else if let Some(setting) = config.stereo_configs.get_mut(&ch_key) {
                    setting.delay_ms = t.delay_ms;
                    setting.eq_bands = t.eq_bands.clone();
                    setting.phase_invert = t.phase_invert;
                    setting.gain_db = t.gain_db;
                } else if let Some(setting) = config.multi_configs.get_mut(&ch_key) {
                    setting.delay_ms = t.delay_ms;
                    setting.eq_bands = t.eq_bands.clone();
                    setting.phase_invert = t.phase_invert;
                    setting.gain_db = t.gain_db;
                }
            }
        }
    }

    Ok(())
}

pub fn api_apply_channel_tuning(
    channel: u32,
    delay_ms: f32,
    eq_bands: Vec<EqBand>,
    phase_invert: bool,
    gain_db: f32,
) -> Result<(), AtmosError> {
    println!(
        "🔥 [디버깅] api_apply_channel_tuning 호출됨. 채널: {}, 딜레이: {}ms, 위상반전: {}, 게인: {}dB",
        channel, delay_ms, phase_invert, gain_db
    );
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::ApplyChannelTuning {
            channel: channel as usize,
            delay_ms,
            eq_bands: eq_bands.clone(),
            phase_invert,
            gain_db,
        })
        .map_err(|e| AtmosError {
            message: format!("Failed to apply channel tuning: {}", e),
        })?;

    // 인메모리 Config 상태 업데이트 (믹서 재시작 시 복구용)
    if let Ok(mut config_guard) = GLOBAL_STATE.config.write() {
        GLOBAL_STATE
            .config_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Some(config) = config_guard.as_mut() {
            let ch_key = channel + 1;
            if let Some(setting) = config.mono_configs.get_mut(&ch_key) {
                setting.delay_ms = delay_ms;
                setting.eq_bands = eq_bands.clone();
                setting.phase_invert = phase_invert;
                setting.gain_db = gain_db;
            } else if let Some(setting) = config.stereo_configs.get_mut(&ch_key) {
                setting.delay_ms = delay_ms;
                setting.eq_bands = eq_bands.clone();
                setting.phase_invert = phase_invert;
                setting.gain_db = gain_db;
            } else if let Some(setting) = config.multi_configs.get_mut(&ch_key) {
                setting.delay_ms = delay_ms;
                setting.eq_bands = eq_bands.clone();
                setting.phase_invert = phase_invert;
                setting.gain_db = gain_db;
            }
        }
    }

    Ok(())
}

pub fn api_apply_global_tuning(
    master_headroom_db: f32,
    peak_limiter_enabled: bool,
) -> Result<(), AtmosError> {
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::ApplyGlobalTuning {
            master_headroom_db,
            peak_limiter_enabled,
        })
        .map_err(|e| AtmosError {
            message: format!("Failed to apply global tuning: {}", e),
        })?;

    if let Ok(mut config_guard) = GLOBAL_STATE.config.write() {
        GLOBAL_STATE
            .config_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Some(config) = config_guard.as_mut() {
            config.master_headroom_db = master_headroom_db;
            config.peak_limiter_enabled = peak_limiter_enabled;
        }
    }
    Ok(())
}

#[derive(serde::Deserialize, Default)]
#[serde(default)]
pub struct SpatialConfigPayload {
    pub channel_positions: Vec<Option<crate::common::config::Point3D>>,
    pub room_zones: Vec<crate::common::config::RoomZone>,
    pub trajectory: Option<crate::common::config::Trajectory>,
    pub track_positions: std::collections::HashMap<String, crate::common::config::Point3D>,
}

pub fn api_update_spatial_config_json(json_payload: String) -> Result<(), AtmosError> {
    let payload: SpatialConfigPayload =
        serde_json::from_str(&json_payload).map_err(|e| AtmosError {
            message: format!("Failed to parse spatial config JSON: {}", e),
        })?;

    // 리스너(마네킹) 기준점은 `SpatialConfigPayload`에 필드로 넣지 않고 JSON에서
    // 직접 뽑는다. 그 구조체는 flutter_rust_bridge가 생성한 코드
    // (`frb_generated.rs`)가 참조하고 있어서 필드를 추가하면 재생성이 필요한데,
    // 이 저장소의 코드젠은 freezed 의존성 누락으로 현재 실패한다
    // (`channel_names.rs`의 TODO 주석 참고). 반면 이 함수의 입력은 JSON
    // 문자열이라 FFI 시그니처를 건드리지 않고 필드를 늘릴 수 있다.
    //
    // 없으면 None으로 두고 엔진이 폴백한다(구버전 payload 호환).
    let listener_position = serde_json::from_str::<serde_json::Value>(&json_payload)
        .ok()
        .and_then(|v| v.get("listener_position").cloned())
        .and_then(|lp| {
            let x = lp.get("x")?.as_f64()? as f32;
            let y = lp.get("y")?.as_f64()? as f32;
            let z = lp.get("z").and_then(|v| v.as_f64()).unwrap_or(1.2) as f32;
            Some(crate::common::config::Point3D {
                x,
                y,
                z,
                ..Default::default()
            })
        });

    // 채널별 스피커가 속한 방 ID도 listener_position과 같은 이유로 JSON에서 직접 뽑는다
    // (Point3D에 필드를 추가하면 FRB 재생성이 필요하다). 방마다 로컬 좌표라 RoomZone이
    // 원점에서 겹치므로 좌표만으로는 방을 알 수 없다(acoustic::bind_channel_zone 참고).
    let raw_positions = serde_json::from_str::<serde_json::Value>(&json_payload)
        .ok()
        .and_then(|v| v.get("channel_positions").cloned());
    // 지금 보고 있는 방(헤드폰 미리듣기 기준). 같은 이유로 JSON에서 직접 뽑는다.
    let active_room_id = serde_json::from_str::<serde_json::Value>(&json_payload)
        .ok()
        .and_then(|v| v.get("active_room_id").cloned())
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok());

    let channel_room_ids: Vec<Option<u32>> = (0..payload.channel_positions.len())
        .map(|i| {
            raw_positions
                .as_ref()
                .and_then(|arr| arr.get(i))
                .and_then(|ch| ch.get("room_id"))
                .and_then(|id| id.as_u64())
                .and_then(|id| u32::try_from(id).ok())
        })
        .collect();

    // 채널별 서브우퍼 지정(스피커 인스펙터의 Set as LFE Subwoofer)도 같은 이유로 JSON에서
    // 직접 뽑는다. 방마다 자기 방 서브로만 저역을 보내는 라우팅 표를 여기서(비-오디오
    // 스레드) 미리 계산해 두고, 오디오 스레드는 표를 바꿔 끼우기만 한다(Law 1).
    // 스피커가 없는 채널은 서브로 치지 않는다(라우팅 표와 믹서가 같은 걸 보게).
    let has_speaker: Vec<bool> = payload.channel_positions.iter().map(|p| p.is_some()).collect();
    let channel_is_sub: Vec<bool> = (0..payload.channel_positions.len())
        .map(|i| {
            has_speaker[i]
                && raw_positions
                    .as_ref()
                    .and_then(|arr| arr.get(i))
                    .and_then(|ch| ch.get("is_subwoofer"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
        })
        .collect();
    let bass_route =
        crate::audio::bass_route::compute_bass_route(&channel_is_sub, &channel_room_ids, &has_speaker);
    if crate::core::state::debug_flags::trace_cmd() {
        // 비-오디오 스레드(FRB 워커). 화면 번호(CH1 = 내부 0)로 찍고, 드래그 중 매 프레임
        // 찍히지 않게 내용이 바뀔 때만 남긴다.
        let subs: Vec<String> = (0..channel_is_sub.len())
            .filter(|&i| channel_is_sub[i])
            .map(|i| format!("CH{}(방 {:?})", i + 1, channel_room_ids.get(i).copied().flatten()))
            .collect();
        let routes: Vec<String> = bass_route
            .iter()
            .enumerate()
            .filter_map(|(i, r)| r.map(|s| format!("CH{}→CH{}", i + 1, s + 1)))
            .collect();
        let line = format!("[CMD] 베이스 매니지먼트: 서브 {subs:?}, 저역 보냄 {routes:?}");
        static LAST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
        let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
        if *last != line {
            eprintln!("{line}");
            *last = line;
        }
    }

    // 채널별 현장 물리 밴드(헤드폰 미리듣기 전용)도 같은 이유로 JSON에서 직접 뽑는다.
    // 형식: "sim_bands": [{"on", "type"(EqType 순서 번호), "freq", "gain", "q"}, ...] — 슬롯 고정.
    let channel_sim_bands: Vec<
        [crate::common::config::EqBand; crate::audio::binaural::MAX_SIM_BANDS],
    > = (0..payload.channel_positions.len())
        .map(|i| {
            let mut bands: [crate::common::config::EqBand; crate::audio::binaural::MAX_SIM_BANDS] =
                std::array::from_fn(|_| crate::common::config::EqBand::default());
            if let Some(list) = raw_positions
                .as_ref()
                .and_then(|arr| arr.get(i))
                .and_then(|ch| ch.get("sim_bands"))
                .and_then(|v| v.as_array())
            {
                for (slot, b) in bands.iter_mut().zip(list.iter()) {
                    *slot = sim_band_from_json(b);
                }
            }
            bands
        })
        .collect();

    // 초기반사음(1차 반사) 탭을 비-오디오 스레드(FRB 워커 풀)에서 미리 계산한다.
    // api_calculate_eq_response_curve와 달리 #[frb(sync)]가 없어 오디오 렌더 콜백과 무관한
    // 워커 스레드에서 실행되므로, 여기서의 힙 할당/삼각함수 반복 계산은 Law 1/2 위반이 아니다.
    // 스피커가 어떤 RoomZone에도 바인딩되지 않으면 해당 채널은 6슬롯 모두 gain=0(무음)으로 채운다.
    // pan_deg로 트림되지 않은 원본(raw) 물리적 스피커 위치(payload.channel_positions)를 그대로 사용한다.
    let mut early_reflection_taps: Vec<
        [crate::audio::acoustic::EarlyReflectionTap;
            crate::audio::acoustic::MAX_EARLY_REFLECTION_TAPS],
    > = Vec::with_capacity(payload.channel_positions.len());
    for (pos_opt, room_id) in payload.channel_positions.iter().zip(&channel_room_ids) {
        let taps = match pos_opt {
            Some(pos) => {
                let bound_zone =
                    crate::audio::acoustic::bind_channel_zone(&payload.room_zones, *room_id, pos);
                match bound_zone {
                    Some(zone) => crate::audio::acoustic::compute_early_reflection_taps(pos, zone),
                    None => Default::default(),
                }
            }
            None => Default::default(),
        };
        early_reflection_taps.push(taps);
    }

    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::UpdateSpatialConfig {
            listener_position,
            channel_positions: payload.channel_positions,
            channel_room_ids,
            active_room_id,
            room_zones: payload.room_zones,
            trajectory: payload.trajectory,
            track_positions: payload.track_positions,
            early_reflection_taps,
            channel_is_sub,
            bass_route,
            channel_sim_bands,
        })
        .map_err(|e| AtmosError {
            message: format!("Failed to send UpdateSpatialConfig: {}", e),
        })?;

    Ok(())
}

/// 공간 설정 JSON의 물리 밴드 하나 → EqBand(형식은 api_update_spatial_config_json 주석 참고).
fn sim_band_from_json(v: &serde_json::Value) -> crate::common::config::EqBand {
    use crate::common::config::EqType;
    let num = |k: &str, default: f64| v.get(k).and_then(|x| x.as_f64()).unwrap_or(default) as f32;
    let filter_type = match v.get("type").and_then(|x| x.as_u64()).unwrap_or(2) {
        0 => EqType::LowCut,
        1 => EqType::LowShelf,
        2 => EqType::Bell,
        3 => EqType::Notch,
        4 => EqType::HighShelf,
        _ => EqType::HighCut,
    };
    crate::common::config::EqBand {
        enabled: v.get("on").and_then(|x| x.as_bool()).unwrap_or(false),
        freq: num("freq", 1000.0),
        gain: num("gain", 0.0),
        q_factor: num("q", 0.707),
        filter_type,
        slope_db_per_oct: 12,
    }
}

#[flutter_rust_bridge::frb(sync)]
pub fn api_set_channel_pan_deg(channel: usize, pan_deg: f32) {
    let _ = crate::core::state::GLOBAL_STATE
        .command_sender
        .send(crate::common::commands::AudioCommand::SetChannelPanDeg { channel, pan_deg });
}

#[flutter_rust_bridge::frb(sync)]
pub fn api_set_channel_early_ref_mix(channel: usize, mix: f32) {
    let _ = crate::core::state::GLOBAL_STATE
        .command_sender
        .send(crate::common::commands::AudioCommand::SetChannelEarlyRefMix { channel, mix });
}

use crate::common::config::Point3D;

pub fn api_calculate_bezier_point(
    t: f32,
    p0: Point3D,
    p1: Point3D,
    p2: Point3D,
    p3: Point3D,
) -> Point3D {
    let u = 1.0 - t;
    let tt = t * t;
    let uu = u * u;
    let uuu = uu * u;
    let ttt = tt * t;

    let mut p = Point3D {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        ..Default::default()
    };

    p.x = uuu * p0.x;
    p.x += 3.0 * uu * t * p1.x;
    p.x += 3.0 * u * tt * p2.x;
    p.x += ttt * p3.x;

    p.y = uuu * p0.y;
    p.y += 3.0 * uu * t * p1.y;
    p.y += 3.0 * u * tt * p2.y;
    p.y += ttt * p3.y;

    p.z = uuu * p0.z;
    p.z += 3.0 * uu * t * p1.z;
    p.z += 3.0 * u * tt * p2.z;
    p.z += ttt * p3.z;

    p
}

pub fn api_calculate_dbap_heatmap(
    listener_pos: Point3D,
    channel_positions: Vec<Point3D>,
) -> Vec<f32> {
    let blur_radius = 2.0f32;
    let mut weights = Vec::with_capacity(channel_positions.len());
    let mut sum_sq = 0.0;

    for pos in &channel_positions {
        let dx = pos.x - listener_pos.x;
        let dy = pos.y - listener_pos.y;
        let dz = pos.z - listener_pos.z;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        let weight = 1.0 / (dist.powi(2) + blur_radius.powi(2));
        weights.push(weight);
        sum_sq += weight * weight;
    }

    let norm_factor = if sum_sq > 0.0 {
        1.0 / sum_sq.sqrt()
    } else {
        0.0
    };

    for w in &mut weights {
        *w *= norm_factor;
    }

    weights
}

#[flutter_rust_bridge::frb(sync)]
pub fn api_calculate_eq_response_curve(
    bands: Vec<crate::common::config::EqBand>,
    num_points: usize,
    sample_rate: f32,
) -> Vec<f32> {
    let n_pts = if num_points == 0 { 300 } else { num_points };
    let fs = if sample_rate <= 0.0 {
        48000.0
    } else {
        sample_rate
    };
    let mut response_db = vec![0.0f32; n_pts];

    for i in 0..n_pts {
        // Logarithmic frequency sampling from 20 Hz to 20,000 Hz
        let f = 20.0 * (20000.0 / 20.0f32).powf(i as f32 / (n_pts - 1) as f32);
        let omega = 2.0 * std::f32::consts::PI * f / fs;
        let mut total_db = 0.0f32;

        for band in &bands {
            if !band.enabled {
                continue;
            }
            let fc = band.freq.clamp(20.0, 20000.0);
            let gain_db = band.gain;
            let q = band.q_factor.max(0.1);
            let w0 = 2.0 * std::f32::consts::PI * fc / fs;
            let alpha = w0.sin() / (2.0 * q);
            let a = 10.0f32.powf(gain_db / 40.0);

            // Biquad coefficients based on EqType
            let (b0, b1, b2, a0, a1, a2) = match band.filter_type {
                crate::common::config::EqType::Bell => (
                    1.0 + alpha * a,
                    -2.0 * w0.cos(),
                    1.0 - alpha * a,
                    1.0 + alpha / a,
                    -2.0 * w0.cos(),
                    1.0 - alpha / a,
                ),
                crate::common::config::EqType::LowCut => (
                    (1.0 + w0.cos()) / 2.0,
                    -(1.0 + w0.cos()),
                    (1.0 + w0.cos()) / 2.0,
                    1.0 + alpha,
                    -2.0 * w0.cos(),
                    1.0 - alpha,
                ),
                crate::common::config::EqType::HighCut => (
                    (1.0 - w0.cos()) / 2.0,
                    1.0 - w0.cos(),
                    (1.0 - w0.cos()) / 2.0,
                    1.0 + alpha,
                    -2.0 * w0.cos(),
                    1.0 - alpha,
                ),
                crate::common::config::EqType::LowShelf => (
                    a * ((a + 1.0) - (a - 1.0) * w0.cos() + 2.0 * a.sqrt() * alpha),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * w0.cos()),
                    a * ((a + 1.0) - (a - 1.0) * w0.cos() - 2.0 * a.sqrt() * alpha),
                    (a + 1.0) + (a - 1.0) * w0.cos() + 2.0 * a.sqrt() * alpha,
                    -2.0 * ((a - 1.0) + (a + 1.0) * w0.cos()),
                    (a + 1.0) + (a - 1.0) * w0.cos() - 2.0 * a.sqrt() * alpha,
                ),
                crate::common::config::EqType::HighShelf => (
                    a * ((a + 1.0) + (a - 1.0) * w0.cos() + 2.0 * a.sqrt() * alpha),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * w0.cos()),
                    a * ((a + 1.0) + (a - 1.0) * w0.cos() - 2.0 * a.sqrt() * alpha),
                    (a + 1.0) - (a - 1.0) * w0.cos() + 2.0 * a.sqrt() * alpha,
                    2.0 * ((a - 1.0) - (a + 1.0) * w0.cos()),
                    (a + 1.0) - (a - 1.0) * w0.cos() - 2.0 * a.sqrt() * alpha,
                ),
                crate::common::config::EqType::Notch => (
                    1.0,
                    -2.0 * w0.cos(),
                    1.0,
                    1.0 + alpha,
                    -2.0 * w0.cos(),
                    1.0 - alpha,
                ),
            };

            // Normalize coefficients by a0
            let nb0 = b0 / a0;
            let nb1 = b1 / a0;
            let nb2 = b2 / a0;
            let na1 = a1 / a0;
            let na2 = a2 / a0;

            // Magnitude response calculation |H(e^{j w})|^2
            let cos_w = omega.cos();
            let cos_2w = (2.0 * omega).cos();
            let sin_w = omega.sin();
            let sin_2w = (2.0 * omega).sin();

            let num_real = nb0 + nb1 * cos_w + nb2 * cos_2w;
            let num_imag = nb1 * sin_w + nb2 * sin_2w;
            let den_real = 1.0 + na1 * cos_w + na2 * cos_2w;
            let den_imag = na1 * sin_w + na2 * sin_2w;

            let num_sq = num_real * num_real + num_imag * num_imag;
            let den_sq = den_real * den_real + den_imag * den_imag;

            if den_sq > 0.0 {
                let mag_sq = num_sq / den_sq;
                if mag_sq > 0.0 {
                    total_db += 10.0 * mag_sq.log10();
                }
            }
        }
        response_db[i] = total_db.clamp(-30.0, 30.0);
    }

    response_db
}

pub fn api_get_active_output_channels(
    device_name: Option<String>,
) -> Result<Vec<String>, AtmosError> {
    api_get_device_channel_names(device_name)
}

pub fn api_get_osc_metrics() -> crate::osc::metrics::OscMetricsDto {
    crate::osc::metrics::GLOBAL_OSC_METRICS.get_metrics()
}

pub fn api_reset_osc_metrics() {
    crate::osc::metrics::GLOBAL_OSC_METRICS.reset();
}

#[flutter_rust_bridge::frb(sync)]
pub fn api_set_binaural_enabled(enabled: bool) {
    // 엔진이 재시작돼도 유지되도록 전역에 먼저 기록한다(state.rs 주석 참고).
    crate::core::state::GLOBAL_STATE
        .binaural_enabled
        .store(enabled, std::sync::atomic::Ordering::Relaxed);
    let _ = crate::core::state::GLOBAL_STATE
        .command_sender
        .send(crate::common::commands::AudioCommand::SetBinauralEnabled { enabled });
}

#[flutter_rust_bridge::frb(sync)]
pub fn api_set_reverb_params(mix: f32, decay: f32) {
    let _ = crate::core::state::GLOBAL_STATE
        .command_sender
        .send(crate::common::commands::AudioCommand::SetReverbParams { mix, decay });
}

#[flutter_rust_bridge::frb(sync)]
pub fn api_set_channel_reverb_send(channel: usize, send: f32) {
    let _ = crate::core::state::GLOBAL_STATE
        .command_sender
        .send(crate::common::commands::AudioCommand::SetChannelReverbSend { channel, send });
}

#[flutter_rust_bridge::frb(sync)]
pub fn api_set_spatial_reverb(
    is_enabled: bool,
    room_size: f32,
    decay_time: f32,
    pre_delay_ms: f32,
    damp: f32,
    density: f32,
    dry_wet: f32,
) {
    let _ = crate::core::state::GLOBAL_STATE.command_sender.send(
        crate::common::commands::AudioCommand::SetSpatialReverb {
            is_enabled,
            room_size,
            decay_time,
            pre_delay_ms,
            damp,
            density,
            dry_wet,
        },
    );
}

#[flutter_rust_bridge::frb(sync)]
pub fn api_set_channel_spatial_reverb(
    channel: usize,
    is_enabled: bool,
    room_size: f32,
    decay_time: f32,
    pre_delay_ms: f32,
    damp: f32,
    density: f32,
    dry_wet: f32,
) {
    let _ = crate::core::state::GLOBAL_STATE.command_sender.send(
        crate::common::commands::AudioCommand::SetChannelSpatialReverb {
            channel,
            is_enabled,
            room_size,
            decay_time,
            pre_delay_ms,
            damp,
            density,
            dry_wet,
        },
    );
}

/// LFE +10dB 토글(베이스 매니지먼트 패널). 서브 채널 자기 신호(.1 LFE 트랙)를 120Hz
/// 로우패스 **이후**에 +10dB 올린다. 메인에서 넘어온 저역에는 걸지 않는다.
/// 서브 레벨은 원래 최종 출력단이나 하드웨어에서 맞추고, 이건 바이노럴 미리듣기나
/// 소프트웨어로 맞춰야 할 때 쓴다.
#[flutter_rust_bridge::frb(sync)]
pub fn api_set_lfe_boost_enabled(enabled: bool) {
    let _ = crate::core::state::GLOBAL_STATE
        .command_sender
        .send(crate::common::commands::AudioCommand::SetLfeBoostEnabled { enabled });
}

/// 베이스 매니지먼트 크로스오버 주파수(모든 방 공통). 서브우퍼 지정은 방별이라
/// 스피커 속성으로 공간 설정 payload에 실려 온다(api_update_spatial_config_json).
#[flutter_rust_bridge::frb(sync)]
pub fn api_set_crossover_frequency(freq: f32) {
    let _ = crate::core::state::GLOBAL_STATE
        .command_sender
        .send(crate::common::commands::AudioCommand::SetCrossoverFrequency { freq });
}
