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
/// ## 채널 개방 규칙(`core::output_groups::open_channels`)
/// 켠 Mono k → CHk, 켠 Stereo k → CHk·CHk+1, 켠 Multi k → CHk부터 그 자리를 쓰는 다채널 트랙의 파일 채널 수만큼
/// (트랙이 없으면 CHk 하나). 줄은 종류마다 따로 켜고 끄며, 트랙은 자기 종류의 줄이 켜져 있어야 소리를 낸다(2026-11-22
/// 사용자 결정 "이전처럼 에이블톤처럼 따로", 설계 docs/superpowers/specs/2026-11-22-output-config-separate-rows-design.md).
/// 1.3.15·1.3.16에서는 세 줄을 열린 채널 하나로 묶고 Multi k는 CHk 하나만 열었다. 1.3.14까지는 Multi k가 장치 끝까지 열었다.
///
/// 켠 것이 하나도 없으면(세 맵이 비었거나 전부 꺼짐) 어느 채널도 열지 않는다 — 소리 없음
/// (사용자 결정 2026-11-21, 설계 docs/superpowers/specs/2026-11-21-empty-output-config-silent-design.md).
pub fn compute_enabled_channels(config: &AppConfig, hw_len: usize) -> Vec<bool> {
    crate::core::output_groups::open_channels(config, hw_len)
}

/// `compute_enabled_channels()` 결과를 `GLOBAL_STATE.enabled_channels` 원자 배열에, 종류별 줄을 줄 표
/// (`core::output_groups::apply_row_tables`)에 반영한다. 화면(`buildChannelRoutingItems`, lib/core/utils/channel_routing.dart)도
/// 같은 줄로 트랙이 고를 자리를 만든다.
fn apply_enabled_channels(config: &AppConfig) {
    crate::core::output_groups::apply_row_tables(config);
    let enabled = compute_enabled_channels(config, GLOBAL_STATE.enabled_channels.len());
    // 열린 채널이 없어지면 한 번 남긴다(켠 그룹이 없음 — 소리 없음, 2026-11-21).
    static NO_OPEN_CHANNEL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let none_open = !enabled.iter().any(|&open| open);
    if NO_OPEN_CHANNEL.swap(none_open, std::sync::atomic::Ordering::Relaxed) != none_open && none_open {
        GLOBAL_STATE.log(
            "출력 설정에서 켠 채널 그룹이 없어 어느 채널로도 내보내지 않는다 — 출력 설정에서 그룹을 켜야 소리가 난다"
                .to_string(),
        );
    }
    for (slot, &value) in GLOBAL_STATE.enabled_channels.iter().zip(enabled.iter()) {
        slot.store(value, std::sync::atomic::Ordering::Relaxed);
    }
    request_restart_if_processing_width_changed(config);
}

/// 지금 설정으로 채널 게이트를 다시 계산한다. 음원 채널 수 확인이 늦게 끝났을 때 부른다
/// (`core::output_groups::cached_file_channels`). 설정이 아직 없으면 아무것도 하지 않는다.
pub(crate) fn reapply_enabled_channels() {
    let config = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner()).clone();
    if let Some(config) = config {
        apply_enabled_channels(&config);
    }
}

/// 출력 설정에서 켠 채널 범위가 바뀌어 믹서 처리 폭(core::processing_channels)이 달라지면 엔진을
/// 다시 시작하게 한다. 믹서 폭은 엔진을 만들 때 정해지므로 자기 재시작 경로를 쓴다(재생 상태를 떠 두었다가
/// 멈춘 위치부터 이어 간다). 요청은 장치 유실 신호와 따로 남기고, 엔진 관리 스레드가 꺼낼 때 폭을 다시 비교한다
/// (`take_width_restart_request` — 저장 한 번에 두 번 재시작하지 않게, 2026-11-21). 로그는 요청이 새로 생길 때만.
fn request_restart_if_processing_width_changed(config: &AppConfig) {
    use std::sync::atomic::Ordering;
    if !ENGINE_ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    if let Some((current, wanted)) = crate::core::processing_channels::width_change(config) {
        if !GLOBAL_STATE.processing_width_restart.swap(true, Ordering::AcqRel) {
            GLOBAL_STATE.log(format!(
                "출력 채널 범위가 바뀌어 믹서 처리 폭을 {current} → {wanted}채널로 바꾼다(엔진 다시 시작)"
            ));
        }
    }
}

pub fn api_get_config(path: String) -> AppConfig {
    // 없거나 손상됐으면 임시 파일·직전 정상본으로 되살린다(설정 파일 손상 복구). 예전에는 손상이면 기본값을
    // 돌려주고 다음 저장이 원본을 덮었으며, 화면에는 아무 알림이 없었다.
    let (config, recovery) = AppConfig::load_with_recovery(&path);
    if let Some(r) = &recovery {
        crate::core::config_notice::record(std::path::Path::new(&path), r);
    }

    apply_enabled_channels(&config);
    {
        let mut global_config = GLOBAL_STATE
            .config
            .write()
            .unwrap_or_else(|e| e.into_inner());
        GLOBAL_STATE
            .config_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let old = global_config.take();
        *global_config = Some(config.clone());
        // 방식이나 표가 바뀐 트랙의 경로를 맞춘다(매트릭스 라우팅 6.6).
        resync_routing(old.as_ref(), &config);
    }
    GLOBAL_STATE.is_exhibition_mode.store(
        config.is_exhibition_mode,
        std::sync::atomic::Ordering::Relaxed,
    );
    crate::core::ducking::apply_settings(&config);

    // Apply tunings on boot immediately
    // 믹서는 받은 순서대로 적용하므로(채널마다 마지막 값이 남는다) 멀티 → 스테레오 → 모노 순서로 보내 그 채널의 모노 줄
    // 보정값이 이기게 한다. 예전에는 모노가 먼저라, 출력 설정에서 새로 켠 스테레오·멀티 줄의 기본값이 덮었다(2026-11-22).
    let mut tunings = Vec::new();
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
    let _ = GLOBAL_STATE
        .command_sender
        .send(AudioCommand::ApplyAllChannelTunings { tunings });

    config
}

pub fn api_save_config(path: String, config: AppConfig) -> Result<(), AtmosError> {
    // 앱 설정은 직전 정상본(config.json.bak)을 남기며 저장한다.
    config.save_with_backup(path)?;
    apply_enabled_channels(&config);
    {
        let mut global_config = GLOBAL_STATE
            .config
            .write()
            .unwrap_or_else(|e| e.into_inner());
        GLOBAL_STATE
            .config_version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let old = global_config.take();
        *global_config = Some(config.clone());
        // 방식이나 표가 바뀐 트랙의 경로를 맞춘다(매트릭스 라우팅 6.6).
        resync_routing(old.as_ref(), &config);
    }
    GLOBAL_STATE.is_exhibition_mode.store(
        config.is_exhibition_mode,
        std::sync::atomic::Ordering::Relaxed,
    );
    crate::core::ducking::apply_settings(&config);
    Ok(())
}

pub fn api_preload_sound(file_path: String) -> Result<(), AtmosError> {
    let path = std::path::Path::new(&file_path);
    let target_sr = GLOBAL_STATE
        .engine_sample_rate
        .load(std::sync::atomic::Ordering::Relaxed);
    // 머리말로 먼저 본다: 통째로 읽은 뒤 거절하면(예전) 다채널 큰 파일은 그 사이 수 GB를 잡는다. 이런 파일은
    // 재생할 때 디스크에서 읽는다(D3, player::plays_from_disk).
    if crate::audio::player::too_big_for_memory(path, target_sr) {
        return Err(AtmosError {
            message: "File is too large for preload (exceeds 100MB).".to_string(),
        });
    }
    match crate::audio::player::SoundData::load_from_file_with_source(path, target_sr) {
        Ok((data, source_rate)) => {
            GLOBAL_STATE.note_file_source_rate(&file_path, source_rate);
            // 100MB limit check (samples * 4 bytes per f32)
            if data.samples.len() as u64 > crate::audio::player::PRELOAD_MAX_SAMPLES {
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
        instance,
        room_volume,
    }
}

pub fn api_play_track(room_id: String, track_id: String) -> Result<(), AtmosError> {
    crate::core::event_log::record_request("play", &format!("{room_id}/{track_id}"));
    play_track_from(room_id, track_id, 0.0)
}

/// 매트릭스 방식이고 트랙에 연결 표가 있으면 재생 명령의 인스턴스를 표 경로로 시작하게 한다(오디오 스레드 밖,
/// 매트릭스 라우팅 설계 6.6). 파일 채널 수는 인스턴스가 연 실제 파일의 것을 쓴다. 그 밖에는 그대로(매핑 경로).
fn with_routes(
    mut cmd: AudioCommand,
    mode: crate::common::config::OutputRoutingMode,
    room: &crate::common::config::RoomConfig,
    track: &crate::common::config::TrackConfig,
) -> AudioCommand {
    if mode != crate::common::config::OutputRoutingMode::Matrix || track.routes.is_empty() {
        return cmd;
    }
    if let AudioCommand::PlayTrack { instance, .. } = &mut cmd {
        let resolved = crate::audio::route::resolve_routes(&track.routes, Some(instance.stream_channels as usize));
        log_dropped_routes(room, track, &resolved);
        instance.set_initial_routes(&resolved.entries);
    }
    cmd
}

/// 버린 칸을 앱 로그에 남긴다. 같은 트랙은 설정 버전마다 한 번만(효과음을 다시 틀 때마다 쌓이지 않게).
fn log_dropped_routes(
    room: &crate::common::config::RoomConfig,
    track: &crate::common::config::TrackConfig,
    r: &crate::audio::route::ResolvedRoutes,
) {
    static LOGGED: std::sync::Mutex<Option<(u64, std::collections::HashSet<String>)>> = std::sync::Mutex::new(None);
    if r.dropped() == 0 {
        return;
    }
    let version = GLOBAL_STATE.config_version.load(std::sync::atomic::Ordering::Relaxed);
    let mut guard = LOGGED.lock().unwrap_or_else(|e| e.into_inner());
    let fresh = match guard.as_mut() {
        Some((v, set)) if *v == version => set.insert(track.id.clone()),
        _ => {
            *guard = Some((version, std::collections::HashSet::from([track.id.clone()])));
            true
        }
    };
    if fresh {
        GLOBAL_STATE.log(format!(
            "라우팅 표: 방 '{}' 트랙 '{}' 칸 {}개 중 {}개를 쓰지 않는다(음량 {}, 파일 채널 {}, 출력 {}, 겹침 {}, 상한 {})",
            room.name,
            track.name,
            r.entries.len() + r.dropped(),
            r.dropped(),
            r.dropped_bad_gain,
            r.dropped_file_channel,
            r.dropped_output,
            r.dropped_duplicate,
            r.dropped_over_limit
        ));
    }
}

/// 설정을 통째로 바꾼 뒤 부른다(매트릭스 라우팅 6.6). 옛 설정과 견줘 방식이 바뀌었으면 모든 트랙에, 매트릭스 그대로이고
/// 표가 달라진 트랙에는 그 트랙에 새 설정의 경로 명령을 차례로 보낸다(매트릭스이고 표가 있으면 표, 아니면 매핑).
/// 재생 중인 인스턴스는 10ms 크로스페이드하고, 같은 경로면 아무 일도 없다. 매핑 방식에서 매핑 값만 바뀐 경우는 예전처럼
/// 보내지 않는다.
pub(crate) fn resync_routing(old: Option<&AppConfig>, new: &AppConfig) {
    use crate::common::config::OutputRoutingMode;
    let old_mode = old.map(|c| c.output_routing_mode).unwrap_or_default();
    let mode_changed = old_mode != new.output_routing_mode;
    if !mode_changed && new.output_routing_mode != OutputRoutingMode::Matrix {
        return;
    }
    let old_routes: std::collections::HashMap<&str, &Vec<crate::common::config::TrackRoute>> = old
        .map(|c| c.rooms.iter().flat_map(|r| r.tracks.iter()).map(|t| (t.id.as_str(), &t.routes)).collect())
        .unwrap_or_default();
    for room in &new.rooms {
        for track in &room.tracks {
            let changed = mode_changed
                || old_routes.get(track.id.as_str()).map_or(!track.routes.is_empty(), |r| **r != track.routes);
            if !changed {
                continue;
            }
            let (room_id, track_id) = (hash_id(&room.id), hash_id(&track.id));
            let cmd = if new.output_routing_mode == OutputRoutingMode::Matrix && !track.routes.is_empty() {
                AudioCommand::SetTrackRoutes {
                    room_id,
                    track_id,
                    entries: crate::audio::route::resolve_routes(&track.routes, None).entries,
                }
            } else {
                AudioCommand::SetTrackOutput {
                    room_id,
                    track_id,
                    output_channel: track.output_channel as usize,
                    output_stereo: track.output_stereo,
                }
            };
            let _ = GLOBAL_STATE.command_sender.send(cmd);
        }
    }
}

/// 재생 명령의 인스턴스를 `seconds` 지점부터 시작하게 하고(0이면 그대로), 그룹이면 대기로 표시한다
/// (그룹 시작 설계 3.1: StartGroup이 올 때까지 소리·위치가 움직이지 않는다).
fn with_start(mut cmd: AudioCommand, seconds: f64, group_id: Option<u64>) -> AudioCommand {
    if let AudioCommand::PlayTrack { instance, .. } = &mut cmd {
        if seconds > 0.0 {
            instance.set_start_position(seconds);
        }
        if let Some(id) = group_id {
            instance.arm_for_group(id);
        }
    }
    cmd
}

/// `start_seconds`(파일 기준)부터 재생한다. 하나씩 트는 재생(대시보드 트랙 버튼, OSC 트랙 play)이 쓴다.
pub(crate) fn play_track_from(
    room_id: String,
    track_id: String,
    start_seconds: f64,
) -> Result<(), AtmosError> {
    play_track_from_in(room_id, track_id, start_seconds, None)
}

/// 장치를 기다리거나 실패한 상태면 재생을 버리는 오류(유지 스레드 설계 D11, OSC 규격 §4). 재생 진입점에서 한 번,
/// 큐에 보내기 바로 앞에서 한 번 더 본다 — 파일을 통째로 읽거나 스트리머를 만드는 동안 대기로 바뀌면 유지자가 큐를
/// 이미 재개 항목으로 옮긴 뒤라 그 명령이 큐에 남아 장치 복귀 때 날 수 있었다(2026-11-19 검토 세션). 남는 틈은 마이크로초.
fn reject_if_engine_down() -> Result<(), AtmosError> {
    if crate::core::engine_keeper::accepts_play() {
        return Ok(());
    }
    Err(AtmosError {
        message: format!(
            "{} ({})",
            crate::osc::reply::ENGINE_WAITING_PREFIX,
            crate::core::engine_keeper::state().name()
        ),
    })
}

/// [`play_track_from`]과 같고, `group`이 있으면 인스턴스를 그 그룹의 대기로 보내고 디스크 트랙의 첫 묶음 표시를
/// 모은다(core::group_start::play_group이 모아 함께 시작한다).
pub(crate) fn play_track_from_in(
    room_id: String,
    track_id: String,
    start_seconds: f64,
    mut group: Option<&mut crate::core::group_start::GroupStart>,
) -> Result<(), AtmosError> {
    let group_id = group.as_ref().map(|g| g.id);
    if crate::core::state::debug_flags::trace_cmd() {
        eprintln!("[CMD] 재생 요청 room={room_id} track={track_id}");
    }
    // 장치를 기다리거나 실패한 동안의 재생 명령은 쌓지 않고 버린다(유지 스레드 설계 D11, OSC 규격 §4). 큐에 남았다가
    // 장치가 돌아올 때 한꺼번에 나면 안 된다. 짧은 자기 재시작 중(상태 Running 유지)에는 지금처럼 받는다.
    reject_if_engine_down()?;
    // 재개를 기다리는 트랙인가(루프 중복 막기에 쓴다, restart_resume::is_pending — 맨 안쪽 잠금만 쥔다).
    let resume_waiting = crate::core::restart_resume::is_pending(&track_id);
    let config_guard = GLOBAL_STATE
        .config
        .read()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(config) = config_guard.as_ref() {
        if let Some(room) = config.rooms.iter().find(|r| r.id == room_id) {
            if let Some(track) = room.tracks.iter().find(|t| t.id == track_id) {
                // 파일을 읽어야 하는데(스트리밍 시작, 캐시에 없음) 파일이 없으면 이유를 분명히 돌려주고 앱 로그에
                // 남긴다. 예전에는 로딩 단계의 영어 오류만 돌아가 화면에는 "치명적 시스템 오류"로만 보였고 로그에는
                // 아무것도 없었다(2026-10-10 Windows P6-E: 맥 프로젝트에서 못 찾은 파일을 재생). 메모리에 미리
                // 읽어 둔 음원은 원본 파일이 없어도 재생한다.
                let missing_file = || -> Option<AtmosError> {
                    if std::path::Path::new(&track.file_path).exists() {
                        return None;
                    }
                    GLOBAL_STATE.log(format!(
                        "재생 실패: 파일을 찾을 수 없다 — 방 '{}' 트랙 '{}': {}",
                        room.name, track.name, track.file_path
                    ));
                    Some(AtmosError {
                        message: format!("파일을 찾을 수 없습니다: {}", track.file_path),
                    })
                };
                let instance_id = crate::core::state::next_instance_id();

                let _ = GLOBAL_STATE
                    .command_sender
                    .send(AudioCommand::SetMasterVolume {
                        room_id: hash_id(&room_id),
                        volume: room.volume,
                    });
                // 루프·스트리밍 켬, 그리고 메모리에 올리기에 너무 큰 단발(약 100MB 넘음)은 디스크에서 읽는다(사용자 결정
                // 2026-11-24 D3 — 예전에는 미리 읽기가 실패해 그 단발이 나지 않았다). 단발만 머리말을 읽는다.
                let engine_sr = GLOBAL_STATE
                    .engine_sample_rate
                    .load(std::sync::atomic::Ordering::Relaxed);
                if crate::audio::player::plays_from_disk(
                    track.is_loop,
                    track.is_streaming,
                    std::path::Path::new(&track.file_path),
                    engine_sr,
                ) {
                    // Prevent duplicate playback of the same looping track if it is a BGM
                    if track.is_loop {
                        let is_playing = {
                            let guard = GLOBAL_STATE
                                .playing_track_ids
                                .read()
                                .unwrap_or_else(|e| e.into_inner());
                            guard.values().any(|id| id == &track_id)
                        } || resume_waiting; // 자기 재시작 중 재개 대기(스냅샷이 재생 목록을 비운다)
                        if is_playing {
                            if crate::core::state::debug_flags::trace_cmd() {
                                eprintln!("[CMD] 재생 건너뜀(이미 재생 중인 루프) track={track_id}");
                            }
                            return Ok(());
                        }
                    }

                    if let Some(err) = missing_file() {
                        return Err(err);
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
                            // 스트리머를 만드는 동안 대기로 바뀌었으면 보내지 않는다. 그룹 준비 표시를 넣기 전에 봐야, 거절된
                            // 트랙 때문에 같은 그룹이 첫 묶음 대기 시간(1초)을 다 채우지 않는다.
                            reject_if_engine_down()?;
                            if let Some(g) = group.as_mut() {
                                // 그룹 시작은 이 표시가 켜질 때까지(첫 묶음 준비) 기다린 뒤 함께 시작한다.
                                g.ready.push(streamer.first_chunk_ready.clone());
                            }
                            if crate::core::state::debug_flags::trace_cmd() {
                                eprintln!("[CMD] 재생 시작(스트리밍) track={track_id} instance={instance_id}");
                            }
                            GLOBAL_STATE.note_track_source_rate(
                                &track_id,
                                &track.file_path,
                                Some(streamer.source_sample_rate),
                            );
                            GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                            GLOBAL_STATE
                                .command_sender
                                .send(with_start(with_routes(build_play_track_command(
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
                                ), config.output_routing_mode, room, track), start_seconds, group_id))
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
                            } || resume_waiting; // 자기 재시작 중 재개 대기(스냅샷이 재생 목록을 비운다)
                            if is_playing {
                                return Ok(());
                            }
                        }

                        GLOBAL_STATE.note_track_source_rate(&track_id, &track.file_path, None);
                        reject_if_engine_down()?; // 파일을 읽는 동안 대기로 바뀌었으면 보내지 않는다
                        GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                        GLOBAL_STATE
                            .command_sender
                            .send(with_start(with_routes(build_play_track_command(
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
                            ), config.output_routing_mode, room, track), start_seconds, group_id))
                            .map_err(|e| AtmosError {
                                message: e.to_string(),
                            })?;
                        return Ok(());
                    }

                    // 읽기 잠금은 찾아본 뒤 바로 놓는다. 예전에는 아래 else 가지(캐시에 없어 파일을 읽는 경로)까지
                    // 쥐고 있다가 같은 잠금의 쓰기를 잡아 이 스레드가 영원히 멈췄다(tests/test_play_cache_miss.rs).
                    // 빈 항목은 미리 읽기가 실패했다는 표시다(api_preload_all_sounds가 디스크를 반복해 읽지 않으려 넣음).
                    // 예전에는 그것을 그대로 틀어 Ok·무음이 되었다(검토 A5). 없는 것으로 보고 아래에서 다시 읽어 보며,
                    // 그래도 못 읽으면 Err를 돌려준다(OSC는 rejected play_failed, tests/test_preload_failure.rs).
                    let cached_data = {
                        let cache_guard = GLOBAL_STATE
                            .sound_cache
                            .read()
                            .unwrap_or_else(|e| e.into_inner());
                        cache_guard
                            .get(&track.file_path)
                            .filter(|d| !d.samples.is_empty())
                            .cloned()
                    };
                    if let Some(data) = cached_data {
                        GLOBAL_STATE.note_track_source_rate(&track_id, &track.file_path, None);
                        reject_if_engine_down()?; // 파일을 읽는 동안 대기로 바뀌었으면 보내지 않는다
                        GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                        GLOBAL_STATE
                            .command_sender
                            .send(with_start(with_routes(build_play_track_command(
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
                            ), config.output_routing_mode, room, track), start_seconds, group_id))
                            .map_err(|e| AtmosError {
                                message: e.to_string(),
                            })?;
                        return Ok(());
                    } else {
                        // Cache miss -> Load into RAM dynamically (obeys 100% RAM rule for SFX)
                        if let Some(err) = missing_file() {
                            return Err(err);
                        }
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
                        match crate::audio::player::SoundData::load_from_file_with_source(path, target_sr) {
                            Ok((data, source_rate)) => {
                                let arc_data = std::sync::Arc::new(data);
                                {
                                    let mut cache = GLOBAL_STATE
                                        .sound_cache
                                        .write()
                                        .unwrap_or_else(|e| e.into_inner());
                                    cache.insert(track.file_path.clone(), arc_data.clone());
                                }
                                GLOBAL_STATE.note_file_source_rate(&track.file_path, source_rate);
                                GLOBAL_STATE.note_track_source_rate(&track_id, &track.file_path, Some(source_rate));
                                reject_if_engine_down()?; // 파일을 읽는 동안 대기로 바뀌었으면 보내지 않는다
                                GLOBAL_STATE.add_playing_track(instance_id, track_id.clone());
                                GLOBAL_STATE
                                    .command_sender
                                    .send(with_start(with_routes(build_play_track_command(
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
                                    ), config.output_routing_mode, room, track), start_seconds, group_id))
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
    crate::core::event_log::record_request("stop", &format!("{room_id}/{track_id}"));
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
    stop_all().map(|_| ())
}

/// 전체 정지하고, 이 정지가 올린 정지 순번을 돌려준다. 바로 뒤에 그룹을 트는 테마 시작이 기준으로 쓴다
/// (그 사이에 들어온 다른 정지가 있으면 그룹을 시작하지 않는다, core::group_start).
pub(crate) fn stop_all() -> Result<u64, AtmosError> {
    crate::core::event_log::record_request("stop_all", "-");
    let epoch = crate::core::restart_resume::cancel_all();
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
    GLOBAL_STATE.set_theme_started(false);
    GLOBAL_STATE.broadcast_state();
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::StopAll)
        .map_err(|e| AtmosError {
            message: e.to_string(),
        })?;
    Ok(epoch)
}

pub fn api_set_active_room(room_id: Option<String>) -> Result<(), AtmosError> {
    GLOBAL_STATE.set_active_room(room_id);
    Ok(())
}

pub fn api_clear_room(room_id: String) -> Result<(), AtmosError> {
    crate::core::event_log::record_request("clear_room", &room_id);
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
    GLOBAL_STATE.mark_room_cleared(&room_id);
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

/// 방 클리어 결과(대시보드 클리어 버튼). 화면이 실패 알림에 쓴다. 활성 방은 상태 알림으로 함께 간다.
#[derive(Debug, Clone)]
pub struct ClearRoomResult {
    /// 새로 활성이 된 다음 방. 마지막 방이었으면 None(활성 방 없음).
    pub next_room_id: Option<String>,
    /// 다음 방 루프 가운데 틀지 못한 트랙("트랙 이름: 이유").
    pub failed_tracks: Vec<String>,
}

/// 대시보드 방 카드의 클리어 버튼. OSC 방 클리어와 같은 공통 함수([`clear_room_and_advance`])로 이 방을 비우고, 다음
/// 방을 활성으로 올려 그 방의 루프를 한 그룹으로 같은 샘플에 함께 튼다. 예전에는 화면(Dart)이 다음 방 루프를 하나씩
/// 틀어 트랙끼리 어긋날 수 있었다(그룹 시작 설계). 전시 모드이거나 이 방이 활성 방이 아니면 아무것도 하지 않고 Err.
/// 다음 방으로 넘기지 않는 비우기(설정에서 활성 방이 지워졌을 때)는 [`api_clear_room`]이다.
pub fn api_clear_room_and_advance(room_id: String) -> Result<ClearRoomResult, AtmosError> {
    crate::core::event_log::record_request("clear_room_and_advance", &room_id);
    let config = GLOBAL_STATE
        .config
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .ok_or_else(|| AtmosError {
            message: "설정이 아직 없습니다".to_string(),
        })?;
    // 전시 모드는 방마다 루프가 계속 도는 운영이라 클리어하지 않는다(OSC 클리어와 같음, 2026-10-12 사용자 결정).
    if config.is_exhibition_mode {
        return Err(AtmosError {
            message: "전시 모드에서는 방 클리어를 하지 않습니다".to_string(),
        });
    }
    {
        // 확인과 넘김을 한 번에 한다. 버튼을 두 번 눌러도 두 번째는 여기서 막혀 다음 방까지 비우지 않는다.
        let mut guard = GLOBAL_STATE
            .active_room_id
            .write()
            .unwrap_or_else(|e| e.into_inner());
        if guard.as_ref() != Some(&room_id) {
            return Err(AtmosError {
                message: "Room is not active or already cleared".to_string(),
            });
        }
        *guard = room_after(&config, &room_id).map(|r| r.id.clone());
    }
    Ok(clear_room_and_advance(&room_id, &config))
}

/// 설정의 방 순서에서 `room_id` 다음 방. 마지막 방이거나 없는 방이면 None.
fn room_after<'a>(config: &'a AppConfig, room_id: &str) -> Option<&'a crate::common::config::RoomConfig> {
    config.rooms.iter().skip_while(|r| r.id != room_id).nth(1)
}

/// 방 클리어의 공통 본체(OSC 방 클리어, 대시보드 클리어 버튼). 그 방의 재시작 대기를 빼고(정지 순번이 오른다),
/// 재생 목록에서 빼고, 엔진에 ClearRoom을 보내 그 방 소리를 함께 줄여 끈다. 다음 방이 있으면 활성으로 올리고 그
/// 방의 루프를 한 그룹으로 같은 샘플에 함께 튼다(core::group_start). 전시 모드·활성 방 확인은 부르는 쪽이 한다.
pub(crate) fn clear_room_and_advance(room_id: &str, config: &AppConfig) -> ClearRoomResult {
    // 이 비우기가 올린 정지 순번. 아래에서 다음 방 루프를 그룹으로 틀 때 기준이 된다.
    let stop_epoch = crate::core::restart_resume::cancel_room(room_id);
    GLOBAL_STATE.mark_room_cleared(room_id);
    GLOBAL_STATE.remove_playing_tracks_of_room(room_id);
    let _ = GLOBAL_STATE.command_sender.send(AudioCommand::ClearRoom {
        room_id: hash_id(room_id),
    });

    let Some(next_room) = room_after(config, room_id) else {
        GLOBAL_STATE.set_active_room(None);
        return ClearRoomResult {
            next_room_id: None,
            failed_tracks: Vec::new(),
        };
    };
    GLOBAL_STATE.set_active_room(Some(next_room.id.clone()));
    GLOBAL_STATE.log(format!("Interlock: Auto-promoted room {} to active", next_room.id));

    // 다음 방 BGM(루프)은 대시보드와 같은 재생 경로로 튼다. 예전에는 RAM 캐시(sound_cache)에서만 찾았는데 루프
    // 트랙은 스트리밍이라 거기 없어서 아무것도 나오지 않았다. 이미 재생 중인 루프는 다시 틀지 않는다. 한 그룹으로
    // 같은 샘플에 함께 시작한다(core::group_start).
    let plan: Vec<(String, String, f64)> = next_room
        .tracks
        .iter()
        .filter(|t| t.is_loop)
        .map(|t| (next_room.id.clone(), t.id.clone(), 0.0))
        .collect();
    let failed_tracks = crate::core::group_start::play_group_since(&plan, stop_epoch)
        .into_iter()
        .map(|(track_id, e)| {
            let name = next_room
                .tracks
                .iter()
                .find(|t| t.id == track_id)
                .map_or(track_id.as_str(), |t| t.name.as_str());
            GLOBAL_STATE.log(format!("방 클리어: 다음 방 루프 {name} 재생 실패: {}", e.message));
            format!("{name}: {}", e.message)
        })
        .collect();
    ClearRoomResult {
        next_room_id: Some(next_room.id.clone()),
        failed_tracks,
    }
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
    // 매트릭스 방식이고 이 트랙에 표가 있으면 매핑 경로로 바꾸지 않는다(두 방식이 섞이지 않게, 매트릭스 라우팅 6.6).
    if track_uses_table(&room_id, &track_id) {
        GLOBAL_STATE.log(format!(
            "트랙 출력 변경을 보내지 않음: 매트릭스 방식이고 연결 표가 있는 트랙이다 — {room_id}/{track_id}"
        ));
        return Ok(());
    }
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

/// 지금 설정이 매트릭스 방식이고 이 트랙에 연결 표가 있는가.
fn track_uses_table(room_id: &str, track_id: &str) -> bool {
    let guard = GLOBAL_STATE.config.read().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().is_some_and(|c| {
        c.output_routing_mode == crate::common::config::OutputRoutingMode::Matrix
            && c.rooms
                .iter()
                .filter(|r| r.id == room_id)
                .flat_map(|r| r.tracks.iter())
                .any(|t| t.id == track_id && !t.routes.is_empty())
    })
}

/// 재생 중인 트랙의 연결 표를 바꾼다(화면이 표를 고칠 때, 매트릭스 라우팅 6.6). 매트릭스 방식이 아니거나 표가 비어 있으면
/// 오류이고 명령을 보내지 않는다(매핑 경로로 돌아갈 때는 api_set_track_output). 설정 저장은 화면이 따로 한다.
pub fn api_set_track_routes(
    room_id: String,
    track_id: String,
    routes: Vec<crate::common::config::TrackRoute>,
) -> Result<(), AtmosError> {
    let matrix = GLOBAL_STATE
        .config
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .is_some_and(|c| c.output_routing_mode == crate::common::config::OutputRoutingMode::Matrix);
    if !matrix {
        return Err(AtmosError { message: "매트릭스 라우팅 방식이 아니라 연결 표를 보낼 수 없습니다".to_string() });
    }
    if routes.is_empty() {
        return Err(AtmosError { message: "연결 표가 비어 있습니다(연결이 하나는 있어야 합니다)".to_string() });
    }
    let entries = crate::audio::route::resolve_routes(&routes, None).entries;
    GLOBAL_STATE
        .command_sender
        .send(AudioCommand::SetTrackRoutes { room_id: hash_id(&room_id), track_id: hash_id(&track_id), entries })
        .map_err(|e| AtmosError { message: e.to_string() })?;
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

    // 마지막으로 보낸 값. 값이 그대로면(엔진이 꺼졌거나 재생이 없음) 보내지 않는다 — 예전에는 장치가 없어도 16ms마다
    // 같은 값을 보내 Dart가 매번 받아 풀었다(2026-11-19 대기 중 CPU 조사). 새 구독은 새 스레드라 첫 값은 늘 간다.
    let mut last_sent: Option<Vec<f32>> = None;
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
        // 마지막 값: 피크 리미터의 마스터 게인 리덕션(dB, 0 이상). 엔진 상태는 재생·정지 같은 사건 때만 가서
        // 미터가 멈춰 있었다(2026-10-24 기능 점검). 화면 미터가 이 16ms 주기 값을 읽는다.
        levels.push(f32::from_bits(
            GLOBAL_STATE
                .current_gain_reduction_db
                .load(std::sync::atomic::Ordering::Relaxed),
        ));
        if last_sent.as_ref() != Some(&levels) {
            last_sent = Some(levels.clone());
            if sink.add(levels).is_err() {
                break; // Stop thread if port is closed
            }
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
        let _ = sink.add("Running".to_string());
    } else if let Some(status) = crate::core::engine_keeper::current_status_string() {
        // 장치를 기다리거나 실패한 상태면 그 배너를 바로 보이게 한다(엔진 유지자).
        let _ = sink.add(status);
    } else {
        let _ = sink.add("Stopped".to_string());
    }
    // 구독 전에 설정한 OSC 수신 포트가 막혔으면 처음 값으로 알린다(osc::listener, 엔진 상태가 아님).
    if let Some(status) = crate::osc::listener::current_port_status() {
        let _ = sink.add(status);
    }
    // 구독 전부터 음원 파일을 다시 열지 못해 무음인 트랙이 있으면 처음 값으로 알린다(core::read_failures, 검토 A6).
    if let Some(status) = crate::core::read_failures::current_status() {
        let _ = sink.add(status);
    }
}

pub fn api_init_audio_system(device_name: Option<String>) -> Result<(), AtmosError> {
    // 엔진 유지자(core::engine_keeper)의 재시도와 겹치지 않게 열기 잠금을 쥔다. 결과는 유지자에 알린다 — 실패해도 장치가
    // 나타날 수 있으면 유지자가 계속 찾는다(안정성 ③).
    let _open = crate::core::engine_keeper::open_lock();
    let result = init_audio_system_for_user(device_name.clone());
    let outcome = match &result {
        Ok(()) => Ok(()),
        Err(e) => Err(classify_start_error(device_name.as_deref(), &e.message)),
    };
    crate::core::engine_keeper::user_started(device_name, &outcome);
    result
}

/// 사용자 시작·장치 변경의 본체(예전 `api_init_audio_system`).
///
/// 설정의 장치를 열지 못해도 기본 장치로 바꿔 열지 않는다(안정성 ⑤, 엔진 유지자 설계 15절). 실패는 그대로 돌려주고,
/// `api_init_audio_system`이 유지자에 알려 장치가 나타날 수 있으면 유지자가 무음으로 기다리며 다시 찾는다.
fn init_audio_system_for_user(device_name: Option<String>) -> Result<(), AtmosError> {
    // 사용자가 엔진을 직접 다시 띄우면 재시작 복원 대기는 버린다(core::restart_resume).
    // 세대를 먼저 올려야 감시 루프의 스냅샷이 버린 뒤에 끼어들지 못한다.
    stop_audio_engine();
    crate::core::restart_resume::cancel_all();
    start_audio_system(device_name)
}

/// [`init_audio_system`]과 같고 장치 찾기 시간을 정하며, 실패를 분류해 돌려준다(엔진 유지자용).
fn init_audio_system_with(
    device_name: Option<String>,
    search: std::time::Duration,
) -> Result<(), crate::core::engine_keeper::StartFailure> {
    stop_audio_engine();
    start_audio_system_with(device_name, search)
}

/// 엔진 시작 결과를 기다리는 시간 = 장치 찾기 시간 + 이 여유. 넘으면 시작 호출이 멈춘 것으로 본다(유지 스레드 설계 D5).
const START_REPLY_MARGIN: std::time::Duration = std::time::Duration::from_secs(15);

/// 엔진 시작 오류 문구를 유지자의 분류로 바꾼다.
fn classify_start_error(device_name: Option<&str>, message: &str) -> crate::core::engine_keeper::StartFailure {
    use crate::core::engine_keeper::StartFailure;
    if crate::core::engine_keeper::is_hung() {
        return StartFailure::Hung;
    }
    if device_name.is_some_and(|name| !crate::core::device_search::may_appear_later(name)) {
        return StartFailure::Unavailable(message.to_string());
    }
    if message.contains(crate::audio::engine::DEVICE_NOT_FOUND_MARK) || message.contains("No default output device") {
        StartFailure::NotFound(message.to_string())
    } else {
        StartFailure::OpenFailed(message.to_string())
    }
}

/// 엔진 유지자가 엔진을 여는 함수(core::engine_keeper의 기본 열기 함수, 열기 잠금 안에서 불린다).
///
/// 자기 재시작이면 예전 일회성 재연결 스레드와 같은 순서다: "HotReloading"을 알리고 짧은 부팅 램프로 같은 장치를
/// 연다(장치 찾기 30초 그대로). 장치를 기다리는 재시도면 원하는 장치를 짧게(`engine_keeper::RETRY_SEARCH`) 찾는다.
/// 못 열면 기본 장치로 바꾸지 않고 실패를 그대로 돌려준다 — 유지자가 무음으로 기다리며 다시 찾는다(안정성 ⑤).
pub(crate) fn keeper_open(
    req: &crate::core::engine_keeper::OpenRequest,
) -> Result<(), crate::core::engine_keeper::StartFailure> {
    use crate::core::engine_keeper::StartFailure;
    let search = if req.self_restart {
        println!("🔄 [디버깅] 자동 재연결(Hot-Reload) 수행!");
        broadcast_stream_status("HotReloading".to_string());
        // 이 재시작이 만드는 믹서는 부팅 뮤트 램프를 짧게 쓴다(core::restart_resume).
        crate::core::restart_resume::set_short_startup_ramp(true);
        crate::audio::engine::DEVICE_SEARCH_TIMEOUT
    } else {
        crate::core::engine_keeper::RETRY_SEARCH
    };
    let result = match req.device.as_deref() {
        Some(name) if !crate::core::device_search::may_appear_later(name) => {
            Err(StartFailure::Unavailable(format!("'{name}'는 이 컴퓨터에서 열 수 없는 장치입니다")))
        }
        _ => init_audio_system_with(req.device.clone(), search),
    };
    if req.self_restart {
        crate::core::restart_resume::set_short_startup_ramp(false);
    }
    result
}

/// 엔진 유지자가 연 뒤 사용자가 이미 정지했으면 곧바로 내린다(세대를 올려 엔진 스레드가 끝나게 한다).
pub(crate) fn keeper_stop_engine() {
    stop_audio_engine();
}

/// 엔진 기동 본체. 부르기 전에 `stop_audio_engine`으로 세대를 올려 둔다.
fn start_audio_system(device_name: Option<String>) -> Result<(), AtmosError> {
    start_audio_system_with(device_name, crate::audio::engine::DEVICE_SEARCH_TIMEOUT)
        .map_err(|failure| AtmosError { message: failure.message() })
}

/// [`start_audio_system`]과 같고 장치 찾기 시간을 정한다. 시작 결과를 시간 제한(찾기 + 15초)으로 기다리고, 실패를
/// 장치 없음·열기 실패·멈춤으로 나눠 돌려준다.
fn start_audio_system_with(
    device_name: Option<String>,
    search: std::time::Duration,
) -> Result<(), crate::core::engine_keeper::StartFailure> {
    use crate::core::engine_keeper::StartFailure;
    // 앞선 시작 호출이 멈춰 엔진 스레드가 돌아오지 않았다. 새 스레드가 그것을 join하면 같이 멈추므로 바로 거절한다.
    if crate::core::engine_keeper::is_hung() {
        return Err(StartFailure::Hung);
    }
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

        // 이름 없이 여는 엔진(설정에 장치가 없을 때의 기본 장치)은 ASIO가 아니다 — WASAPI 장치 목록 감시가 돈다.
        let is_asio = device_name_clone.as_deref().is_some_and(|name| name.starts_with("[ASIO]"));

        match engine.start_with_search(device_name_clone, rx, gen, search) {
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

                broadcast_stream_status("Running".to_string());

                // 앞 엔진들이 남긴 부하 집계를 버린다. 10초를 못 채우고 끝난 엔진(장치 전환·재시도 중)은 집계를
                // 비우지 않아, 새 엔진의 첫 보고에 몇 분치가 섞여 "부하 높음"으로 잘못 남았다(2026-10-21 18:34:
                // 버퍼 1024프레임인데 10초에 콜백 10480회, 늦은 콜백 20회).
                {
                    let state = &crate::core::state::GLOBAL_STATE;
                    for counter in [
                        &state.callback_max_us,
                        &state.callback_overruns,
                        &state.callback_total_us,
                        &state.callback_count,
                        &state.callback_gap_max_us,
                        &state.callback_heavy,
                        &state.callback_late,
                    ] {
                        counter.store(0, std::sync::atomic::Ordering::Relaxed);
                    }
                }

                // Keep thread alive until next generation, and watch for DeviceNotAvailable
                let mut last_device_count: Option<usize> = None;
                let mut last_device_names: Vec<String> = Vec::new();
                let mut monitor_timer = 0;
                let mut stop_reason = "generation_changed";
                let mut load_timer = 0;
                let mut last_load_log: Option<std::time::Instant> = None;
                let mut load_warning = crate::core::load_warning::LoadWarningState::default();

                loop {
                    if ENGINE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != gen {
                        break;
                    }

                    if crate::core::state::GLOBAL_STATE
                        .device_needs_reset
                        .load(std::sync::atomic::Ordering::Acquire)
                    {
                        println!("⚠️ [디버깅] 장치 유실·재설정 요청 수신 (DeviceNotAvailable / kAsioResetRequest)");
                        stop_reason = "device_reset_request";
                        crate::core::state::GLOBAL_STATE
                            .device_needs_reset
                            .store(false, std::sync::atomic::Ordering::Release);
                        break;
                    }

                    // 처리 폭 재시작 요청: 이미 원하는 폭으로 열린 엔진이면 건너뛴다(core::processing_channels). 장치 신호를 먼저
                    // 본다 — 둘 다 섰으면 장치 쪽으로 다시 열고, 새 엔진은 지금 설정으로 폭을 정하므로 남은 폭 요청은 건너뛴다.
                    if crate::core::processing_channels::take_width_restart_request() {
                        stop_reason = "processing_width_changed";
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
                            stop_reason = "device_not_available";
                            break;
                        }
                    }

                    // Watchdog: Check if callback hasn't fired in >1000ms
                    // 콜백과 같은 단조 시계로 잰다(벽시계는 시간 동기화로 뛰면 오발한다, 검토 A2).
                    let now_ms = crate::core::state::GLOBAL_STATE.monotonic_ms();
                    let last_cb = crate::core::state::GLOBAL_STATE
                        .watchdog_last_callback
                        .load(std::sync::atomic::Ordering::Relaxed);
                    if last_cb > 0 && now_ms.saturating_sub(last_cb) > 1000 {
                        // 다음 발생 때 트리거를 좁히려면 실제 공백 길이를 남겨야 한다.
                        // 장치 열거로 인한 일시 정지(수백 ms~수 초)와 스레드 사망은
                        // 공백 길이가 다르게 찍힌다. println!은 앱 로그에 남지 않아
                        // 사후 분석이 불가능했으므로 파일 로그로 남긴다.
                        crate::core::state::GLOBAL_STATE.log(format!(
                            "🚨 [Watchdog] 오디오 콜백 공백 {}ms (임계 1000ms)! 엔진 강제 재기동...",
                            now_ms.saturating_sub(last_cb)
                        ));
                        stop_reason = "callback_gap_watchdog";
                        break;
                    }

                    // 콜백 간격이 100ms를 넘은 적이 있으면 사건 로그에 남긴다(콜백은 원자형만 저장한다).
                    {
                        let state = &crate::core::state::GLOBAL_STATE;
                        let gap_us = state
                            .callback_big_gap_us
                            .swap(0, std::sync::atomic::Ordering::Relaxed);
                        if gap_us > 0 {
                            let at_ms = state
                                .callback_big_gap_at_ms
                                .load(std::sync::atomic::Ordering::Relaxed);
                            crate::core::event_log::log_event(
                                "callback_gap",
                                &[
                                    ("gap_ms", &(gap_us / 1000).to_string()),
                                    ("at_unix_ms", &at_ms.to_string()),
                                ],
                            );
                        }
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
                        // 화면 부하 배너(core::load_warning): 예산 초과·늦은 콜백이 있으면 경고, 1분 정상이면 해제.
                        let window = crate::core::load_warning::LoadWindow {
                            max_us,
                            budget_us,
                            overruns,
                            late,
                            // 엔진 시작 로그의 "M채널 처리"와 같은 값(측정 결과와 바로 비교된다).
                            processing_channels: state.output_processing_channels.load(relaxed),
                            avg_us,
                            heavy,
                        };
                        if let Some(status) = load_warning.update(window) {
                            broadcast_stream_status(status);
                        }
                        // 정리 채널이 가득 차 믹서가 맡아 두거나 풀지 못한 연결 표(매트릭스 라우팅 6.4). 0이 아니면 남긴다.
                        let parked = state.route_garbage_parked.swap(0, relaxed);
                        let forgotten = state.route_garbage_forgotten.swap(0, relaxed);
                        if parked > 0 || forgotten > 0 {
                            state.log(format!(
                                "연결 표 정리 채널이 가득 찼다(최근 10초): 맡아 둠 {parked}개, 풀지 못하고 잃음 {forgotten}개"
                            ));
                        }
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
                                if warn { crate::core::load_warning::log_hint(crate::core::load_warning::load_cause(&window)) } else { "" },
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
                                        stop_reason = "device_list_changed";
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
                let self_restart = ENGINE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) == gen;
                crate::core::event_log::log_engine_stop(stop_reason, self_restart);
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
                    // 유지자 스레드에 넘기고 즉시 반환하면 이 스레드가 종료되므로
                    // 새 스레드의 join이 풀린다.
                    // 다시 열기는 엔진 유지자(core::engine_keeper)의 스레드가 한다. 설정 장치·기본 장치가 모두 실패해도
                    // 포기하지 않고 장치를 계속 찾는다(안정성 ③). 예전에는 여기서 일회성 스레드를 띄워 한 번만 시도했다.
                    crate::core::engine_keeper::report_self_restart(gen, device_name.clone());
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

    let limit = search + START_REPLY_MARGIN;
    match rx_init.recv_timeout(limit) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            let message = format!("Failed to start audio engine: {}", e);
            if e.contains(crate::audio::engine::DEVICE_NOT_FOUND_MARK) || e.contains("No default output device") {
                Err(StartFailure::NotFound(message))
            } else {
                Err(StartFailure::OpenFailed(message))
            }
        }
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            crate::core::state::GLOBAL_STATE.log(format!(
                "엔진 시작 호출이 {}초 안에 돌아오지 않았다(오디오 드라이버 멈춤으로 본다)",
                limit.as_secs()
            ));
            Err(StartFailure::Hung)
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(StartFailure::OpenFailed(
            "Failed to start audio engine: Failed to communicate with audio thread".to_string(),
        )),
    }
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
    // 사용자 정지: 유지자는 다시 시도하지 않는다(상태 StoppedByUser).
    crate::core::engine_keeper::user_stopped();
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
    /// 테마 화면 상태(GLOBAL_STATE.theme_started·cleared_room_ids). Start 버튼과 OSC 테마 시작이 같게 보인다.
    pub theme_started: bool,
    pub cleared_room_ids: Vec<String>,
    /// 장치 샘플레이트(Hz)와, 재생 중인 트랙 중 그와 달라 HQ sinc로 바꾸고 있는 것의 원래 샘플레이트들
    /// (작은 것부터)·트랙 수. 화면의 리샘플링 배지가 쓴다(core::resample_status).
    pub device_sample_rate: u32,
    pub resampled_source_rates: Vec<u32>,
    pub resampled_track_count: u32,
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
    let (device_sample_rate, resample) = GLOBAL_STATE.resample_summary(&playing_track_ids);

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
        theme_started: GLOBAL_STATE.theme_started.load(std::sync::atomic::Ordering::Relaxed),
        cleared_room_ids: GLOBAL_STATE.cleared_room_ids.read().unwrap_or_else(|e| e.into_inner()).clone(),
        device_sample_rate,
        resampled_source_rates: resample.source_rates,
        resampled_track_count: resample.track_count,
    };
    let _ = sink.add(initial_state);
    *GLOBAL_STATE
        .state_sink
        .write()
        .unwrap_or_else(|e| e.into_inner()) = Some(sink);
}

pub fn api_preload_all_sounds(config: AppConfig) -> Result<(), AtmosError> {
    let mut needed_files = std::collections::HashSet::new();
    let target_sr = GLOBAL_STATE
        .engine_sample_rate
        .load(std::sync::atomic::Ordering::Relaxed);
    for room in &config.rooms {
        for track in &room.tracks {
            if !track.is_loop && !track.is_streaming {
                if crate::audio::player::too_big_for_memory(std::path::Path::new(&track.file_path), target_sr) {
                    GLOBAL_STATE.log(format!(
                        "미리 읽기 한도(약 100MB)를 넘어 재생할 때 디스크에서 읽는다 — 방 '{}' 트랙 '{}': {}",
                        room.name, track.name, track.file_path
                    ));
                    continue;
                }
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
        match crate::audio::player::SoundData::load_from_file_with_source(path, target_sr) {
            Ok((data, source_rate)) => {
                GLOBAL_STATE.note_file_source_rate(&file, source_rate);
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
    let old = global_config.take();
    *global_config = Some(config.clone());
    // 방식이나 표가 바뀐 트랙의 경로를 맞춘다(매트릭스 라우팅 6.6).
    resync_routing(old.as_ref(), &config);
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
            // 설치된 다른 ASIO 장치도 이름만 보인다. 예전에는 지금 쓰는 ASIO 장치만 있어서, 다른 ASIO 장치를 고르려면
            // 재스캔(엔진 정지)을 알아야 했다(2026-10-21: RME가 연결돼 있는데 목록에 없다고 보임). 설치 목록은 드라이버를
            // 열지 않고 읽으니 지금 드라이버와 부딪치지 않는다. 채널 수는 열기 전에는 몰라서, 엔진이 도는 동안 다른 ASIO
            // 장치에 쓰는 값(2, api_get_device_channel_count)과 같다. 고르면 엔진이 지금 드라이버를 닫고 그 장치를 연다.
            #[cfg(target_os = "windows")]
            for driver in asio_sys::Asio::new().driver_names() {
                let driver = driver.replace('\0', "").trim().to_string();
                let name = format!("[ASIO] {driver}");
                if driver.is_empty() || device_info_list.iter().any(|d| d.name == name) {
                    continue;
                }
                device_info_list.push(OutputDeviceInfo {
                    channel_names: crate::audio::channel_names::get_channel_names_win(&driver, 2),
                    name,
                    max_channels: 2,
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
        // 모든 방의 루프를 한 그룹으로 같은 샘플에 함께 시작한다(그룹 시작 설계 3.1).
        let plan: Vec<(String, String, f64)> = config
            .rooms
            .iter()
            .flat_map(|room| {
                room.tracks.iter().filter(|t| t.is_loop).map(|t| (room.id.clone(), t.id.clone(), 0.0))
            })
            .collect();
        let _ = crate::core::group_start::play_group(&plan);
    }
    Ok(())
}

pub fn api_load_preset(config: AppConfig) -> Result<(), AtmosError> {
    api_stop_all()?;

    GLOBAL_STATE.is_exhibition_mode.store(
        config.is_exhibition_mode,
        std::sync::atomic::Ordering::Relaxed,
    );
    crate::core::ducking::apply_settings(&config);

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
    /// 옛 3D Object 경로가 쓰던 칸. 엔진은 읽지 않는다. FRB가 만든 Dart 클래스에 실려 있어(API 계약) 칸은 남긴다.
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
pub fn api_set_channel_early_ref_mix(channel: usize, mix: f32) {
    // 명령 추적은 오디오 스레드 밖(보내는 쪽)에서 찍는다. 콜백 안의 eprintln!은 블로킹이다(3법칙 2).
    if crate::core::state::debug_flags::trace_cmd() {
        eprintln!("[CMD] SetChannelEarlyRefMix ch={} mix={}", channel, mix);
    }
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
    // 명령 추적은 오디오 스레드 밖(보내는 쪽)에서 찍는다. 콜백 안의 eprintln!은 블로킹이다(3법칙 2).
    if crate::core::state::debug_flags::trace_cmd() {
        eprintln!("[CMD] SetChannelReverbSend ch={} send={}", channel, send);
    }
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
    // 명령 추적은 오디오 스레드 밖(보내는 쪽)에서 찍는다. 콜백 안의 eprintln!은 블로킹이다(3법칙 2).
    if crate::core::state::debug_flags::trace_cmd() {
        eprintln!("[CMD] SetChannelSpatialReverb ch={} (0이면 전체 채널에 적용)", channel);
    }
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
