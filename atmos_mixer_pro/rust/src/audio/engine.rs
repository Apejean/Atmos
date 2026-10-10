use crate::audio::mixer::AudioMixer;
use crate::common::commands::AudioCommand;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{OutputCallbackInfo, SampleFormat, Stream, StreamConfig};

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

lazy_static::lazy_static! {
    pub static ref ENGINE_INIT_SIGNAL: AtomicBool = AtomicBool::new(false);
}

/// 엔진 세대. `api_init_audio_system`·`api_stop_audio_engine`이 올리고, 스트림 콜백은 자기가
/// 만들어진 세대일 때만 일한다([`AudioEngine::begin_callback`]).
pub static ENGINE_GENERATION: AtomicU64 = AtomicU64::new(0);

pub struct AudioEngine {
    stream: Option<Stream>,
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        if let Some(stream) = self.stream.take() {
            println!("🔥 [디버깅] 오디오 스트림 명시적 Pause 및 Drop 수행...");
            let _ = stream.pause();
            drop(stream);
            println!("✅ [디버깅] 오디오 스트림 Drop 완료!");
        }
    }
}

impl AudioEngine {
    pub fn new() -> Self {
        Self { stream: None }
    }
}

#[cfg(target_os = "windows")]
pub fn get_hosts(target_prefix: Option<&str>) -> Result<Vec<cpal::Host>, String> {
    let mut hosts = Vec::new();

    let req_asio = target_prefix.map_or(true, |p| p == "[ASIO]");
    let req_wasapi = target_prefix.map_or(true, |p| p == "[WASAPI]");

    if req_asio {
        match cpal::host_from_id(cpal::HostId::Asio) {
            Ok(host) => hosts.push(host),
            Err(e) => eprintln!("ASIO Load Error: {:?}", e),
        }
    }

    if req_wasapi {
        hosts.push(cpal::default_host()); // WASAPI is default on Windows
    }

    if hosts.is_empty() {
        hosts.push(cpal::default_host());
    }

    Ok(hosts)
}

#[cfg(not(target_os = "windows"))]
pub fn get_hosts(_target_prefix: Option<&str>) -> Result<Vec<cpal::Host>, String> {
    Ok(vec![cpal::default_host()])
}

/// ASIO 드라이버 하나만 불러 초기화해 보고 바로 내린다(인터페이스가 꺼져 있으면 초기화가 실패한다).
/// cpal의 장치 목록과 달리 다른 드라이버는 건드리지 않는다.
#[cfg(target_os = "windows")]
fn asio_driver_opens(driver: &str) -> bool {
    asio_sys::Asio::new().load_driver(driver).is_ok()
}

#[cfg(not(target_os = "windows"))]
fn asio_driver_opens(_driver: &str) -> bool {
    true
}

#[cfg(target_os = "windows")]
pub fn apply_windows_admin_optimizations() {
    use windows::Win32::UI::Shell::IsUserAnAdmin;

    unsafe {
        if IsUserAnAdmin().as_bool() {
            println!("🔥 [디버깅] 관리자 권한 확인됨. MMCSS 스레드 승격 & RAM 고정 시도.");
            // 1. MMCSS (Pro Audio) 승격
            let mut task_index = 0;
            let class_name: Vec<u16> = "Pro Audio\0".encode_utf16().collect();
            let _handle = windows::Win32::System::Threading::AvSetMmThreadCharacteristicsW(
                windows::core::PCWSTR(class_name.as_ptr()),
                &mut task_index,
            );

            // 2. RAM Working Set 고정
            let process = windows::Win32::System::Threading::GetCurrentProcess();
            // 최소 500MB, 최대 2GB Working Set
            let min_size = 500 * 1024 * 1024;
            let max_size = 2000 * 1024 * 1024;
            let _ = windows::Win32::System::Memory::SetProcessWorkingSetSizeEx(
                process,
                min_size,
                max_size,
                windows::Win32::System::Memory::SETPROCESSWORKINGSETSIZEEX_FLAGS(0),
            );
        } else {
            // 일반 계정인 경우: 튕김(크래시) 없이 표준 프로세스로 Graceful Fallback
            println!("⚠️ 일반 사용자 계정으로 로그인되었습니다. 표준 스케줄링 모드로 구동합니다.");
        }
    }
}

impl AudioEngine {
    /// `generation`: 이 엔진의 세대(ENGINE_GENERATION). 콜백은 이 세대가 지금 세대일 때만 일한다.
    pub fn start(
        &mut self,
        device_name: Option<String>,
        cmd_receiver: crossbeam_channel::Receiver<AudioCommand>,
        generation: u64,
    ) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
            apply_windows_admin_optimizations();
        }

        let target_prefix = device_name.as_ref().and_then(|n| {
            if n.starts_with("[ASIO]") {
                Some("[ASIO]")
            } else if n.starts_with("[WASAPI]") {
                Some("[WASAPI]")
            } else {
                None
            }
        });

        let hosts = get_hosts(target_prefix)?;

        let device = if let Some(ref name) = device_name {
            let mut found_device = None;
            let target_name = name
                .replace("[ASIO] ", "")
                .replace("[WASAPI] ", "")
                .replace("[CoreAudio] ", "");
            let target_name = target_name.replace('\0', "").trim().to_string();

            println!("🔥 [디버깅] 플러터 원본 요청: '{}'", name);
            println!("🔥 [디버깅] 공백 제거 후 타겟: '{}'", target_name);

            // 기다려도 나타날 수 없는 이름은 찾지 않는다(core::device_search). ASIO 설치 목록은 드라이버를
            // 불러오지 않고 읽는다.
            #[cfg(target_os = "windows")]
            let installed_asio = name.starts_with("[ASIO]").then(|| asio_sys::Asio::new().driver_names());
            #[cfg(not(target_os = "windows"))]
            let installed_asio: Option<Vec<String>> = None;
            if let Some(reason) =
                crate::core::device_search::reject_before_search(name, installed_asio.as_deref())
            {
                crate::core::state::GLOBAL_STATE.log(format!("장치를 찾지 않고 실패: {reason}"));
                return Err(reason);
            }
            let asio_driver = installed_asio.as_ref().and_then(|names| {
                names
                    .iter()
                    .find(|n| crate::core::device_search::clean_name(n) == target_name)
                    .cloned()
            });

            let start_time = std::time::Instant::now();
            let timeout_secs = 30;
            let mut first_try = true;
            loop {
                // 처음 한 번은 목록을 그대로 훑는다(장치가 있으면 바로 열린다). 못 찾아 다시 볼 때 ASIO는 찾는
                // 드라이버 하나만 먼저 불러 보고, 열릴 때만 목록을 훑는다 — 목록은 설치된 드라이버를 모두 불러와
                // Generic Low Latency 같은 드라이버가 0.5초마다 창을 띄웠다.
                let scan = first_try || asio_driver.as_deref().is_none_or(asio_driver_opens);
                first_try = false;
                for host in hosts.iter().filter(|_| scan) {
                    if let Ok(devices) = host.output_devices() {
                        for d in devices {
                            if let Ok(d_name) = d.name() {
                                let clean_d_name = d_name.replace('\0', "").trim().to_string();
                                if clean_d_name == target_name {
                                    found_device = Some(d);
                                    break;
                                }
                            }
                        }
                    }
                    // cpal 0.15.3(macOS)의 output_devices()는 출력 전용 장치를 빠뜨린다
                    // (audio::coreaudio_fallback 문서 참고).
                    #[cfg(target_os = "macos")]
                    if found_device.is_none() {
                        found_device =
                            crate::audio::coreaudio_fallback::find_output_device(host, &target_name);
                    }
                    if found_device.is_some() {
                        break;
                    }
                }

                if found_device.is_some() {
                    println!(
                        "✅ [디버깅] ASIO 오디오 인터페이스 인식 성공: {}",
                        target_name
                    );
                    break;
                }

                if start_time.elapsed().as_secs() >= timeout_secs {
                    break;
                }
                println!(
                    "⚠️ 장치를 찾는 중... ({} / 30초)",
                    start_time.elapsed().as_secs()
                );
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

            if let Some(d) = found_device {
                d
            } else {
                let error_msg = format!("Requested device '{}' not found after 30s. Available devices were not matched.", name);
                eprintln!("{}", error_msg);
                crate::core::state::GLOBAL_STATE.log(format!("장치를 30초 동안 찾지 못했다: {name}"));
                return Err(error_msg);
            }
        } else {
            cpal::default_host()
                .default_output_device()
                .ok_or("No default output device".to_string())?
        };

        println!("Using output device: {}", device.name().unwrap_or_default());

        let default_config_result = device.default_output_config().ok();
        let mut best_config = default_config_result.clone();
        let mut max_ch = best_config.as_ref().map(|c| c.channels()).unwrap_or(0);
        let default_sample_rate = best_config
            .as_ref()
            .map(|c| c.sample_rate())
            .unwrap_or(cpal::SampleRate(48000));

        let mut supported_configs_result = device.supported_output_configs();
        for _ in 0..3 {
            if supported_configs_result.is_ok() {
                break;
            }
            println!("⏳ [ASIO Lock Retry] COM object might be busy. Waiting 500ms...");
            std::thread::sleep(std::time::Duration::from_millis(500));
            supported_configs_result = device.supported_output_configs();
        }

        if let Ok(supported_configs) = supported_configs_result {
            for c in supported_configs {
                if c.channels() > max_ch {
                    max_ch = c.channels();
                    if c.min_sample_rate() <= default_sample_rate
                        && c.max_sample_rate() >= default_sample_rate
                    {
                        best_config = Some(c.with_sample_rate(default_sample_rate));
                    } else {
                        best_config = Some(c.with_max_sample_rate());
                    }
                }
            }
        }

        // cpal 0.15.3(macOS)은 출력 전용 장치의 설정을 읽지 못한다(audio::coreaudio_fallback
        // 문서 참고). 스트림은 열리므로 CoreAudio에서 직접 읽은 설정으로 채운다.
        // 맥북 내장 스피커가 OS 기본 출력일 때 "기본 장치"로 시작하는 경로도 여기를 지난다.
        #[cfg(target_os = "macos")]
        if best_config.is_none() {
            best_config = device.name().ok().and_then(|n| {
                crate::audio::coreaudio_fallback::output_config(n.replace('\0', "").trim())
            });
        }

        // 설정을 하나도 얻지 못하면 패닉하지 않고 오류로 돌려준다.
        let supported_config = best_config.ok_or_else(|| "No output configs found".to_string())?;
        let sample_format = supported_config.sample_format();
        let mut config: StreamConfig = supported_config.clone().into();

        let mut target_buffer_size = 2048; // Force a safe, large default to prevent dropouts
        if let Some(app_config) = crate::core::state::GLOBAL_STATE
            .config
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            if app_config.buffer_size > 0 {
                target_buffer_size = app_config.buffer_size;
            }
        }

        // 실제로 콜백에 넘어올 프레임 수. AudioMixer::new()에 그대로 전달해
        // 바이노럴 FFT/오버랩-애드 버퍼를 정확한 크기로 사전 할당한다(위
        // AudioMixer::new 문서 참고). Unknown인 경우 정확한 값을 알 수
        // 없으므로 클램프 전 목표치를 최선의 추정치로 쓴다.
        let mut resolved_buffer_size: usize = target_buffer_size as usize;

        match supported_config.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => {
                let clamped = target_buffer_size.clamp(*min, *max);
                config.buffer_size = cpal::BufferSize::Fixed(clamped);
                resolved_buffer_size = clamped as usize;
                println!(
                    "🔥 [디버깅] Buffer size clamped to {} (Range: {} - {})",
                    clamped, min, max
                );
            }
            cpal::SupportedBufferSize::Unknown => {
                config.buffer_size = cpal::BufferSize::Default;
                println!("⚠️ [디버깅] SupportedBufferSize::Unknown -> Using Default");
            }
        }

        config.sample_rate = supported_config.sample_rate();

        println!("Stream config: {:?}", config);

        crate::core::state::GLOBAL_STATE
            .active_device_channels
            .store(config.channels as u32, std::sync::atomic::Ordering::SeqCst);

        crate::core::state::GLOBAL_STATE
            .engine_sample_rate
            .store(config.sample_rate.0, std::sync::atomic::Ordering::SeqCst);

        // 믹서 처리 폭: 장치는 전체 채널로 열고, 믹서는 출력 설정에서 켠 가장 높은 채널까지만
        // 처리한다(core::processing_channels). 나머지 출력에는 무음을 넣는다.
        let processing = match crate::core::state::GLOBAL_STATE
            .config
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            Some(app_config) => crate::core::processing_channels::processing_channel_count(
                app_config,
                config.channels as usize,
            ),
            None => config.channels as usize,
        };
        let virtual_channels = crate::core::processing_channels::mixer_width(processing);
        crate::core::state::GLOBAL_STATE
            .processing_channels
            .store(virtual_channels as u32, std::sync::atomic::Ordering::SeqCst);
        crate::core::state::GLOBAL_STATE.log(format!(
            "오디오 엔진: 장치 출력 {}채널 중 {}채널 처리(믹서 폭 {}), 버퍼 {}프레임, {}Hz",
            config.channels, processing, virtual_channels, resolved_buffer_size, config.sample_rate.0
        ));

        *crate::core::state::GLOBAL_STATE
            .engine_error
            .write()
            .unwrap_or_else(|e| e.into_inner()) = None;

        let (gc_tx, gc_rx) =
            crossbeam_channel::bounded::<crate::audio::player::SoundInstance>(8192);
        std::thread::spawn(move || {
            while let Ok(dropped) = gc_rx.recv() {
                // Instance is dropped here in a background thread, preventing GC in audio thread.
                if crate::core::state::debug_flags::trace_cmd() {
                    eprintln!(
                        "[CMD] 인스턴스 정리 track={} instance={}",
                        dropped.track_id_str, dropped.instance_id
                    );
                }
                crate::core::state::GLOBAL_STATE.remove_playing_track(dropped.instance_id);
            }
        });

        // 분석 스레드(analysis_thread)는 프로듀서(mixer.process)가 실제로 링버퍼에 쓰는
        // 인터리빙 폭인 virtual_channels(위 믹서 폭) 기준으로 프레임 경계를 계산해야 한다.
        // config.channels(하드웨어 채널 수)를 넘기면 EBU R128/RTA가 잘못된 프레임 경계로
        // 데이터를 재해석하여 미터가 실제 오디오와 무관한 값을 표시한다.

        let (analysis_tx, analysis_rx) = rtrb::RingBuffer::new(65536);
        crate::audio::analysis::start_analysis_thread(
            analysis_rx,
            config.sample_rate.0,
            virtual_channels,
        );

        let mut mixer = AudioMixer::new(
            config.sample_rate.0,
            virtual_channels,
            resolved_buffer_size,
            gc_tx,
            Some(analysis_tx),
        );

        let err_fn = |err: cpal::StreamError| {
            eprintln!("an error occurred on stream: {}", err);
            let err_str = err.to_string();
            // 장치 유실은 오류 종류로 본다. 표시 문구("The requested device is no longer available…")에는
            // "DeviceNotAvailable"이 없어서, 문자열만 보면 놓치고 1초 뒤 워치독이 대신 잡았다(실기 2026-10-07).
            let is_disconnect = matches!(err, cpal::StreamError::DeviceNotAvailable)
                || err_str.contains("DeviceNotAvailable")
                || err_str.contains("kAsioResetRequest");
            if is_disconnect {
                crate::core::state::GLOBAL_STATE
                    .device_needs_reset
                    .store(true, Ordering::Release);
            } else {
                *crate::core::state::GLOBAL_STATE
                    .engine_error
                    .write()
                    .unwrap_or_else(|e| e.into_inner()) = Some(err_str);
                crate::core::state::GLOBAL_STATE.broadcast_state();
            }
        };

        let cmd_receiver_f32 = cmd_receiver;

        let stream = match sample_format {
            SampleFormat::F32 => {
                let copy_channels = (config.channels as usize).min(virtual_channels);
                let sample_rate = config.sample_rate.0;
                let mut temp_buf: Vec<f32> = vec![0.0; 65536];
                let hw_channels = config.channels as usize;
                let mut last_started: Option<std::time::Instant> = None;
                device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &OutputCallbackInfo| {
                        // 비정규 실수를 0으로 다룬다(audio::denormal 참고). 콜백이 끝나면 원래대로 돌린다.
                        let _denormals = crate::audio::denormal::DenormalGuard::new();
                        // 처리 시간은 명령 처리(begin_callback)부터 잰다.
                        let started = std::time::Instant::now();
                        // 옛 세대 스트림은 무음만 낸다(begin_callback 참고).
                        if !Self::begin_callback(&mut mixer, &cmd_receiver_f32, generation) {
                            data.fill(0.0);
                            return;
                        }
                        Self::record_callback_gap(
                            &mut last_started,
                            started,
                            resolved_buffer_size,
                            sample_rate,
                        );
                        // 처리 폭 밖의 출력 채널은 무음이다(core::processing_channels).
                        if copy_channels < hw_channels {
                            data.fill(0.0);
                        }

                        let frames = data.len() / hw_channels;
                        let temp_len = frames * virtual_channels;
                        if temp_buf.len() < temp_len {
                            temp_buf.resize(temp_len, 0.0);
                        }
                        let temp = &mut temp_buf[..temp_len];
                        temp.fill(0.0);

                        mixer.process(temp, virtual_channels);

                        for frame in 0..frames {
                            // 디스크리트 라우팅: 가상 채널 N -> 물리 출력 N.
                            // 예전에는 hw_channels == 2일 때 가상 채널에 5.1 역할
                            // (2=센터, 3=LFE, 4/5=서라운드)을 가정해 L/R로 접었다.
                            // 이 앱은 "채널 = 물리 스피커" 모델이라 그 가정이 라우팅을
                            // 왜곡했다(Ch-3을 고르면 양쪽에서 나는 등). 폴드다운을 제거해
                            // 장치에 실재하는 채널만 그대로 내보낸다. 2채널 장치에서
                            // 3번 이상 채널은 들리지 않으며, 다채널 청음은 실기로 하거나
                            // 추후 바이노럴 모니터링(binaural)을 켜서 확인한다.
                            for ch in 0..copy_channels {
                                data[frame * hw_channels + ch] =
                                    temp[frame * virtual_channels + ch];
                            }
                        }
                        Self::record_callback_time(started, frames, sample_rate);
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::I16 => {
                let copy_channels = (config.channels as usize).min(virtual_channels);
                let sample_rate = config.sample_rate.0;
                let mut temp_buf: Vec<f32> = vec![0.0; 65536];
                let hw_channels = config.channels as usize;
                let mut last_started: Option<std::time::Instant> = None;
                device.build_output_stream(
                    &config,
                    move |data: &mut [i16], _: &OutputCallbackInfo| {
                        // 비정규 실수를 0으로 다룬다(audio::denormal 참고). 콜백이 끝나면 원래대로 돌린다.
                        let _denormals = crate::audio::denormal::DenormalGuard::new();
                        // 처리 시간은 명령 처리(begin_callback)부터 잰다.
                        let started = std::time::Instant::now();
                        // 옛 세대 스트림은 무음만 낸다(begin_callback 참고).
                        if !Self::begin_callback(&mut mixer, &cmd_receiver_f32, generation) {
                            data.fill(<i16 as cpal::Sample>::EQUILIBRIUM);
                            return;
                        }
                        Self::record_callback_gap(
                            &mut last_started,
                            started,
                            resolved_buffer_size,
                            sample_rate,
                        );
                        // 처리 폭 밖의 출력 채널은 무음이다(core::processing_channels).
                        if copy_channels < hw_channels {
                            data.fill(<i16 as cpal::Sample>::EQUILIBRIUM);
                        }

                        let frames = data.len() / hw_channels;
                        let temp_len = frames * virtual_channels;
                        if temp_buf.len() < temp_len {
                            temp_buf.resize(temp_len, 0.0);
                        }
                        let temp = &mut temp_buf[..temp_len];
                        temp.fill(0.0);

                        mixer.process(temp, virtual_channels);

                        for frame in 0..frames {
                            // 디스크리트 라우팅: 가상 채널 N -> 물리 출력 N.
                            // 예전에는 hw_channels == 2일 때 가상 채널에 5.1 역할
                            // (2=센터, 3=LFE, 4/5=서라운드)을 가정해 L/R로 접었다.
                            // 이 앱은 "채널 = 물리 스피커" 모델이라 그 가정이 라우팅을
                            // 왜곡했다(Ch-3을 고르면 양쪽에서 나는 등). 폴드다운을 제거해
                            // 장치에 실재하는 채널만 그대로 내보낸다. 2채널 장치에서
                            // 3번 이상 채널은 들리지 않으며, 다채널 청음은 실기로 하거나
                            // 추후 바이노럴 모니터링(binaural)을 켜서 확인한다.
                            for ch in 0..copy_channels {
                                data[frame * hw_channels + ch] =
                                    cpal::Sample::from_sample(temp[frame * virtual_channels + ch]);
                            }
                        }
                        Self::record_callback_time(started, frames, sample_rate);
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::I32 => {
                let copy_channels = (config.channels as usize).min(virtual_channels);
                let sample_rate = config.sample_rate.0;
                let mut temp_buf: Vec<f32> = vec![0.0; 65536];
                let hw_channels = config.channels as usize;
                let mut last_started: Option<std::time::Instant> = None;
                device.build_output_stream(
                    &config,
                    move |data: &mut [i32], _: &OutputCallbackInfo| {
                        // 비정규 실수를 0으로 다룬다(audio::denormal 참고). 콜백이 끝나면 원래대로 돌린다.
                        let _denormals = crate::audio::denormal::DenormalGuard::new();
                        // 처리 시간은 명령 처리(begin_callback)부터 잰다.
                        let started = std::time::Instant::now();
                        // 옛 세대 스트림은 무음만 낸다(begin_callback 참고).
                        if !Self::begin_callback(&mut mixer, &cmd_receiver_f32, generation) {
                            data.fill(<i32 as cpal::Sample>::EQUILIBRIUM);
                            return;
                        }
                        Self::record_callback_gap(
                            &mut last_started,
                            started,
                            resolved_buffer_size,
                            sample_rate,
                        );
                        // 처리 폭 밖의 출력 채널은 무음이다(core::processing_channels).
                        if copy_channels < hw_channels {
                            data.fill(<i32 as cpal::Sample>::EQUILIBRIUM);
                        }

                        let frames = data.len() / hw_channels;
                        let temp_len = frames * virtual_channels;
                        if temp_buf.len() < temp_len {
                            temp_buf.resize(temp_len, 0.0);
                        }
                        let temp = &mut temp_buf[..temp_len];
                        temp.fill(0.0);

                        mixer.process(temp, virtual_channels);

                        for frame in 0..frames {
                            // 디스크리트 라우팅: 가상 채널 N -> 물리 출력 N.
                            // 예전에는 hw_channels == 2일 때 가상 채널에 5.1 역할
                            // (2=센터, 3=LFE, 4/5=서라운드)을 가정해 L/R로 접었다.
                            // 이 앱은 "채널 = 물리 스피커" 모델이라 그 가정이 라우팅을
                            // 왜곡했다(Ch-3을 고르면 양쪽에서 나는 등). 폴드다운을 제거해
                            // 장치에 실재하는 채널만 그대로 내보낸다. 2채널 장치에서
                            // 3번 이상 채널은 들리지 않으며, 다채널 청음은 실기로 하거나
                            // 추후 바이노럴 모니터링(binaural)을 켜서 확인한다.
                            for ch in 0..copy_channels {
                                data[frame * hw_channels + ch] =
                                    cpal::Sample::from_sample(temp[frame * virtual_channels + ch]);
                            }
                        }
                        Self::record_callback_time(started, frames, sample_rate);
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::U16 => {
                let copy_channels = (config.channels as usize).min(virtual_channels);
                let sample_rate = config.sample_rate.0;
                let mut temp_buf: Vec<f32> = vec![0.0; 65536];
                let hw_channels = config.channels as usize;
                let mut last_started: Option<std::time::Instant> = None;
                device.build_output_stream(
                    &config,
                    move |data: &mut [u16], _: &OutputCallbackInfo| {
                        // 비정규 실수를 0으로 다룬다(audio::denormal 참고). 콜백이 끝나면 원래대로 돌린다.
                        let _denormals = crate::audio::denormal::DenormalGuard::new();
                        // 처리 시간은 명령 처리(begin_callback)부터 잰다.
                        let started = std::time::Instant::now();
                        // 옛 세대 스트림은 무음만 낸다(begin_callback 참고).
                        if !Self::begin_callback(&mut mixer, &cmd_receiver_f32, generation) {
                            data.fill(<u16 as cpal::Sample>::EQUILIBRIUM);
                            return;
                        }
                        Self::record_callback_gap(
                            &mut last_started,
                            started,
                            resolved_buffer_size,
                            sample_rate,
                        );
                        // 처리 폭 밖의 출력 채널은 무음이다(core::processing_channels).
                        if copy_channels < hw_channels {
                            data.fill(<u16 as cpal::Sample>::EQUILIBRIUM);
                        }

                        let frames = data.len() / hw_channels;
                        let temp_len = frames * virtual_channels;
                        if temp_buf.len() < temp_len {
                            temp_buf.resize(temp_len, 0.0);
                        }
                        let temp = &mut temp_buf[..temp_len];
                        temp.fill(0.0);

                        mixer.process(temp, virtual_channels);

                        for frame in 0..frames {
                            // 디스크리트 라우팅: 가상 채널 N -> 물리 출력 N.
                            // 예전에는 hw_channels == 2일 때 가상 채널에 5.1 역할
                            // (2=센터, 3=LFE, 4/5=서라운드)을 가정해 L/R로 접었다.
                            // 이 앱은 "채널 = 물리 스피커" 모델이라 그 가정이 라우팅을
                            // 왜곡했다(Ch-3을 고르면 양쪽에서 나는 등). 폴드다운을 제거해
                            // 장치에 실재하는 채널만 그대로 내보낸다. 2채널 장치에서
                            // 3번 이상 채널은 들리지 않으며, 다채널 청음은 실기로 하거나
                            // 추후 바이노럴 모니터링(binaural)을 켜서 확인한다.
                            for ch in 0..copy_channels {
                                data[frame * hw_channels + ch] =
                                    cpal::Sample::from_sample(temp[frame * virtual_channels + ch]);
                            }
                        }
                        Self::record_callback_time(started, frames, sample_rate);
                    },
                    err_fn,
                    None,
                )
            }
            _ => return Err("Unsupported format".to_string()),
        };

        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                // Task 3: 버퍼 사이즈 및 최대 채널 초과 시 폴백 처리
                eprintln!("Failed to build stream with config {:?}: {}", config, e);
                return Err(format!("Failed to build stream: {}", e));
            }
        };

        stream.play().map_err(|e| e.to_string())?;

        self.stream = Some(stream);

        Ok(())
    }

    /// 오디오 콜백의 앞부분. 이 스트림이 지금 세대면 초기화 신호·워치독 시각을 갱신하고 명령을
    /// 처리한 뒤 true를 돌려준다. 옛 세대면 아무것도 건드리지 않고 false — 콜백은 무음만 낸다.
    ///
    /// cpal 0.15.3(macOS)은 기본 장치가 아닌 장치의 스트림에 장치 분리 리스너를 달면서 그
    /// 리스너 안에 스트림 자신(Arc)을 넣는다. 순환 참조라 drop해도 해제되지 않고, 워치독 재시작처럼
    /// pause가 실패하는 상황이면 옛 스트림이 계속 돈다. 그러면 옛 콜백과 새 콜백이 같은 명령 큐를
    /// 나눠 먹어서 재생은 한쪽, 정지는 다른 쪽으로 가 소리가 멈추지 않았다(실기 로그: 워치독 재시작
    /// 뒤에만 정지 실패, 전체 정지도 일부만 멈춤, 살아 있는 믹서 3개). 옛 세대는 명령에 손대지 않는다.
    /// cpal 0.16.0에서 순환 참조는 없어졌지만(test_engine_drop_releases_stream), 세대를 올린 뒤 옛
    /// 엔진이 drop되기까지(엔진 스레드는 100ms마다 세대를 본다) 옛 콜백이 공용 명령 큐를 가져가지 않게 둔다.
    pub fn begin_callback(
        mixer: &mut AudioMixer,
        rx: &crossbeam_channel::Receiver<AudioCommand>,
        generation: u64,
    ) -> bool {
        if ENGINE_GENERATION.load(Ordering::Acquire) != generation {
            return false;
        }
        if !ENGINE_INIT_SIGNAL.load(Ordering::Acquire) {
            ENGINE_INIT_SIGNAL.store(true, Ordering::Release);
        }
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_millis() as u64;
        crate::core::state::GLOBAL_STATE
            .watchdog_last_callback
            .store(now_ms, Ordering::Relaxed);
        Self::process_commands(mixer, rx);
        true
    }

    /// 오디오 콜백 한 번의 처리 시간(명령 처리+믹서+장치 버퍼 채우기)을 남긴다. 원자형 저장만 한다
    /// (할당·잠금 없음). 엔진 스레드의 점검 루프가 평균·최댓값·예산 초과 횟수를 읽어 비우고, 부하가
    /// 높으면 앱 로그에 남긴다.
    #[inline]
    fn record_callback_time(started: std::time::Instant, frames: usize, sample_rate: u32) {
        let elapsed_us = started.elapsed().as_micros().min(u32::MAX as u128) as u32;
        let budget_us = (frames as u64 * 1_000_000 / u64::from(sample_rate.max(1))) as u32;
        let state = &crate::core::state::GLOBAL_STATE;
        state.callback_budget_us.store(budget_us, Ordering::Relaxed);
        state.callback_max_us.fetch_max(elapsed_us, Ordering::Relaxed);
        state.callback_total_us.fetch_add(elapsed_us, Ordering::Relaxed);
        state.callback_count.fetch_add(1, Ordering::Relaxed);
        if u64::from(elapsed_us) * 10 > u64::from(budget_us) * 7 {
            state.callback_heavy.fetch_add(1, Ordering::Relaxed);
        }
        if elapsed_us > budget_us {
            state.callback_overruns.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// 이 스트림의 앞 콜백 시작부터 이번 콜백 시작까지의 간격을 남긴다(원자형 저장만). 간격이 장치를
    /// 연 버퍼 길이(`frames`)의 1.5배를 넘으면 늦은 콜백으로 센다. 콜백마다 받은 프레임 수와 비교하지
    /// 않는다. WASAPI는 약 10ms마다 남은 만큼(때로 수백 프레임 미만)을 채워 짧은 콜백 뒤의 보통 간격도
    /// 늦은 것으로 셌다(2026-10-09 Windows: 정상 재생에서 10초에 26회).
    #[inline]
    fn record_callback_gap(
        last_started: &mut Option<std::time::Instant>,
        started: std::time::Instant,
        frames: usize,
        sample_rate: u32,
    ) {
        if let Some(prev) = *last_started {
            let gap_us = started.duration_since(prev).as_micros().min(u32::MAX as u128) as u32;
            let budget_us = frames as u64 * 1_000_000 / u64::from(sample_rate.max(1));
            let state = &crate::core::state::GLOBAL_STATE;
            state.callback_gap_max_us.fetch_max(gap_us, Ordering::Relaxed);
            if u64::from(gap_us) * 2 > budget_us * 3 {
                state.callback_late.fetch_add(1, Ordering::Relaxed);
            }
        }
        *last_started = Some(started);
    }

    fn process_commands(mixer: &mut AudioMixer, rx: &crossbeam_channel::Receiver<AudioCommand>) {
        // Lock-free pop from command queue
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                AudioCommand::PlayTrack {
                    instance,
                    room_volume,
                } => {
                    // 인스턴스는 api_play_track(비-오디오 스레드)에서 이미 생성되어
                    // 왔다. 여기서는 풀 슬롯에 옮겨 담기만 한다(할당 없음).
                    let instance = *instance;

                    if let Some(i) = mixer.instances.iter().position(|slot| slot.is_none()) {
                        mixer.instances[i] = Some(instance);
                        mixer.temp_room_vols_target[i] = room_volume;
                        mixer.temp_room_vols[i] = room_volume;
                    } else {
                        // 풀이 가득 찼다. 울리지 못하는 인스턴스는 정리 스레드로 넘긴다(거기서 해제하고 재생 목록의
                        // id도 지운다). 여기서 버리면 메모리 해제와 디코더 스레드 join이 콜백 안에서 일어난다.
                        // 정리 채널까지 가득 찬 극단적인 경우에만 돌려받은 인스턴스가 여기서 해제된다.
                        let _ = mixer.gc_sender.try_send(instance);
                    }
                }
                AudioCommand::StopTrack { room_id, track_id } => {
                    for inst in mixer.instances.iter_mut().flatten() {
                        if inst.room_id == room_id && inst.id == track_id {
                            inst.is_stopping = true;
                        }
                    }
                }
                AudioCommand::StopAll => {
                    for inst in mixer.instances.iter_mut().flatten() {
                        inst.is_stopping = true;
                    }
                }
                AudioCommand::SetBinauralEnabled { enabled } => {
                    mixer.binaural.enabled = enabled;
                    // 방 시뮬레이션은 헤드폰 미리듣기에서만 걸린다(mixer::refresh_early_ref_mix).
                    mixer.refresh_all_early_ref_mixes();
                }
                AudioCommand::SetReverbParams { mix, decay } => {
                    mixer.reverb.set_params(decay, mix);
                }
                AudioCommand::SetMasterMute { muted } => {
                    mixer.master_mute = muted;
                }
                AudioCommand::ClearRoom { room_id } => {
                    for inst in mixer.instances.iter_mut().flatten() {
                        if inst.room_id == room_id {
                            inst.is_stopping = true;
                        }
                    }
                }
                AudioCommand::SetMasterVolume { room_id, volume } => {
                    if let Some(slot) = mixer
                        .room_volumes
                        .iter_mut()
                        .find(|s| s.as_ref().is_some_and(|(id, _)| *id == room_id))
                    {
                        if let Some((_, v)) = slot.as_mut() {
                            *v = volume;
                        }
                    } else if let Some(empty_slot) =
                        mixer.room_volumes.iter_mut().find(|s| s.is_none())
                    {
                        *empty_slot = Some((room_id, volume));
                    }
                }
                AudioCommand::SetTrackVolume {
                    room_id,
                    track_id,
                    volume,
                } => {
                    for inst in mixer.instances.iter_mut().flatten() {
                        if inst.room_id == room_id && inst.id == track_id {
                            inst.volume = volume;
                            inst.volume_smoother.set_target(volume);
                        }
                    }
                }
                AudioCommand::SetTrackOutput {
                    room_id,
                    track_id,
                    output_channel,
                    output_stereo,
                } => {
                    for inst in mixer.instances.iter_mut().flatten() {
                        if inst.room_id == room_id && inst.id == track_id {
                            inst.output_channel = output_channel;
                            inst.output_stereo = output_stereo;
                        }
                    }
                }
                AudioCommand::SetChannelReverbSend { channel, send } => {
                    if crate::core::state::debug_flags::trace_cmd() {
                        eprintln!("[CMD] SetChannelReverbSend ch={} send={}", channel, send);
                    }
                    if channel < mixer.channel_dsp.len() {
                        mixer.channel_dsp[channel].target_reverb_send = send.clamp(0.0, 1.0);
                    }
                }
                AudioCommand::SetSpatialReverb {
                    is_enabled,
                    room_size,
                    decay_time,
                    pre_delay_ms,
                    damp,
                    density,
                    dry_wet,
                } => {
                    mixer.reverb.set_full_params(
                        is_enabled, room_size, decay_time, pre_delay_ms, damp, density, dry_wet,
                    );
                }
                AudioCommand::SetChannelSpatialReverb {
                    channel,
                    is_enabled,
                    room_size,
                    decay_time,
                    pre_delay_ms,
                    damp,
                    density,
                    dry_wet,
                } => {
                    if crate::core::state::debug_flags::trace_cmd() {
                        eprintln!("[CMD] SetChannelSpatialReverb ch={} (0이면 전체 채널에 적용)", channel);
                    }
                    if channel == 0 {
                        mixer.reverb.set_full_params(
                            is_enabled, room_size, decay_time, pre_delay_ms, damp, density,
                            dry_wet,
                        );
                        for dsp in &mut mixer.channel_dsp {
                            dsp.reverb.set_full_params(
                                is_enabled, room_size, decay_time, pre_delay_ms, damp, density,
                                dry_wet,
                            );
                        }
                    } else if (channel - 1) < mixer.channel_dsp.len() {
                        let ch = channel - 1;
                        mixer.channel_dsp[ch].reverb.set_full_params(
                            is_enabled, room_size, decay_time, pre_delay_ms, damp, density,
                            dry_wet,
                        );
                    }
                }
                AudioCommand::SetChannelDelay { channel, delay_ms } => {
                    if channel < mixer.channel_dsp.len() {
                        mixer.channel_dsp[channel].update_delay_target(delay_ms);
                    }
                }
                AudioCommand::SetChannelEq { channel, bands } => {
                    if channel < mixer.channel_dsp.len() {
                        mixer.channel_dsp[channel]
                            .update_eq_targets(&bands, mixer.sample_rate as f32);
                    }
                    let _ = mixer
                        .spatial_gc_tx
                        .try_send(crate::audio::mixer::SpatialGarbage::EqBands(bands));
                }
                AudioCommand::ApplyChannelTuning {
                    channel,
                    delay_ms,
                    eq_bands,
                    phase_invert,
                    gain_db,
                } => {
                    if channel < mixer.channel_dsp.len() {
                        mixer.channel_dsp[channel].update_delay_target(delay_ms);
                        mixer.channel_dsp[channel]
                            .update_eq_targets(&eq_bands, mixer.sample_rate as f32);
                        mixer.channel_dsp[channel].phase_invert = phase_invert;
                        mixer.channel_dsp[channel].set_gain_db(gain_db);
                    }
                    let _ = mixer
                        .spatial_gc_tx
                        .try_send(crate::audio::mixer::SpatialGarbage::EqBands(eq_bands));
                }
                AudioCommand::UpdateSpatialConfig {
                    listener_position,
                    channel_positions,
                    channel_room_ids,
                    active_room_id,
                    room_zones,
                    trajectory,
                    track_positions,
                    early_reflection_taps,
                    channel_is_sub,
                    bass_route,
                    channel_sim_bands,
                } => {
                    mixer.listener_position = listener_position;
                    // 헤드폰 미리듣기의 현장 물리 밴드: 목표값만 바꾸고(할당 없음) 다 쓴 벡터는
                    // GC 스레드로 보낸다(Law 1).
                    for (ch, bands) in channel_sim_bands.iter().enumerate() {
                        mixer.binaural.set_channel_sim_bands(ch, bands);
                    }
                    let _ = mixer.spatial_gc_tx.try_send(
                        crate::audio::mixer::SpatialGarbage::SimBands(channel_sim_bands),
                    );
                    // 방별 서브 라우팅: 새 표는 대기석에 두고, 믹서가 저역 믹스를 0까지
                    // 내린 순간에 바꿔 끼운다(클릭 없음). 밀려난 대기 표는 GC로 보낸다.
                    let (old_route, old_is_sub) = mixer.set_bass_routing(bass_route, channel_is_sub);
                    let _ = mixer.spatial_gc_tx.try_send(
                        crate::audio::mixer::SpatialGarbage::BassRouting(old_route, old_is_sub),
                    );
                    let old_positions =
                        std::mem::replace(&mut mixer.channel_positions, channel_positions);
                    let old_room_ids =
                        std::mem::replace(&mut mixer.channel_room_ids, channel_room_ids);
                    // 채널별 방 ID가 바뀐 뒤에 불러야 마스크가 새 배치로 계산된다.
                    mixer.set_binaural_room(active_room_id);
                    let _ = mixer.spatial_gc_tx.try_send(
                        crate::audio::mixer::SpatialGarbage::ChannelRoomIds(old_room_ids),
                    );
                    let old_zones = std::mem::replace(&mut mixer.room_zones, room_zones);
                    let old_traj = std::mem::replace(&mut mixer.trajectory, trajectory);
                    let old_taps =
                        std::mem::replace(&mut mixer.channel_early_ref_taps, early_reflection_taps);

                    // 채널 위치/룸이 바뀌었으니 바이노럴 렌더러의 채널별 기준
                    // 방위각도 다시 계산한다(BINAURAL_SPATIAL_RENDERING_SPEC.md).
                    // channel_positions/room_zones가 이미 교체된 뒤에 호출해야
                    // 새 값을 읽는다.
                    mixer.recalculate_binaural_channel_azimuths();
                    let _ = mixer.spatial_gc_tx.try_send(
                        crate::audio::mixer::SpatialGarbage::ChannelPositions(old_positions),
                    );
                    let _ = mixer
                        .spatial_gc_tx
                        .try_send(crate::audio::mixer::SpatialGarbage::RoomZones(old_zones));
                    let _ = mixer
                        .spatial_gc_tx
                        .try_send(crate::audio::mixer::SpatialGarbage::Trajectory(old_traj));
                    let _ = mixer.spatial_gc_tx.try_send(
                        crate::audio::mixer::SpatialGarbage::EarlyReflectionTaps(old_taps),
                    );

                    // 새 초기반사 탭을 채널별 DSP 스무딩 타겟으로 이관(Law 1: 고정 배열 대입만, 힙 할당 없음)
                    let n_ch = mixer
                        .channel_dsp
                        .len()
                        .min(mixer.channel_early_ref_taps.len());
                    for ch in 0..n_ch {
                        let taps = mixer.channel_early_ref_taps[ch];
                        mixer.apply_early_reflection_taps(ch, &taps);
                    }

                    for inst in mixer.instances.iter_mut().flatten() {
                        if let Some(pos) = track_positions.get(&inst.track_id_str) {
                            inst.current_position = Some(pos.clone());
                        }
                    }
                    mixer.recalculate_spatial_dsp();
                    let _ = mixer.spatial_gc_tx.try_send(
                        crate::audio::mixer::SpatialGarbage::TrackPositions(track_positions),
                    );
                }
                AudioCommand::SetChannelPanDeg { channel, pan_deg } => {
                    if crate::core::state::debug_flags::trace_cmd() {
                        eprintln!("[CMD] SetChannelPanDeg ch={} deg={}", channel, pan_deg);
                    }
                    if channel < mixer.channel_pan_deg.len() {
                        mixer.channel_pan_deg[channel] = pan_deg;
                    }
                }
                AudioCommand::SetChannelEarlyRefMix { channel, mix } => {
                    if crate::core::state::debug_flags::trace_cmd() {
                        eprintln!("[CMD] SetChannelEarlyRefMix ch={} mix={}", channel, mix);
                    }
                    // 실제 적용량은 믹서가 방 시뮬레이션 몫과 합쳐서 계산한다
                    // (mixer::refresh_early_ref_mix).
                    mixer.set_channel_early_ref_mix(channel, mix);
                }
                AudioCommand::UpdateTrajectoryPosition { position } => {
                    if let Some(traj) = &mut mixer.trajectory {
                        traj.current_position = position;
                    } else {
                        mixer.trajectory = Some(crate::common::config::Trajectory {
                            waypoints: vec![],
                            current_position: position,
                            ..Default::default()
                        });
                    }
                    mixer.recalculate_spatial_dsp();
                }
                AudioCommand::UpdateSingleBandEq {
                    channel,
                    band,
                    freq,
                    gain_db,
                    q_factor,
                    filter_type_idx,
                } => {
                    if channel < mixer.channel_dsp.len()
                        && band < mixer.channel_dsp[channel].target_bands.len()
                    {
                        let filter_type = match filter_type_idx {
                            0 => crate::common::config::EqType::LowCut,
                            1 => crate::common::config::EqType::LowShelf,
                            2 => crate::common::config::EqType::Bell,
                            3 => crate::common::config::EqType::Notch,
                            4 => crate::common::config::EqType::HighShelf,
                            5 => crate::common::config::EqType::HighCut,
                            _ => crate::common::config::EqType::Bell,
                        };
                        let b = &mut mixer.channel_dsp[channel].target_bands[band];
                        b.freq = freq;
                        b.gain = gain_db;
                        b.q_factor = q_factor;
                        b.filter_type = filter_type;
                        // trigger recount/rebuild
                        let all_bands = mixer.channel_dsp[channel].target_bands.clone();
                        mixer.channel_dsp[channel]
                            .update_eq_targets(&all_bands, mixer.sample_rate as f32);
                    }
                }
                AudioCommand::UpdateSoundSourcePosition { sound_id, x, y, z } => {
                    let point = crate::common::config::Point3D {
                        x,
                        y,
                        z,
                        ..Default::default()
                    };
                    // If it's the global trajectory ID we update it
                    if sound_id == "global_trajectory" || sound_id == "trajectory" {
                        if let Some(traj) = &mut mixer.trajectory {
                            traj.current_position = point;
                        } else {
                            mixer.trajectory = Some(crate::common::config::Trajectory {
                                waypoints: vec![],
                                current_position: point,
                                ..Default::default()
                            });
                        }
                    } else {
                        // Or if we map sound objects directly, we'd update their positions here.
                        // For now we'll support global trajectory.
                        if let Some(traj) = &mut mixer.trajectory {
                            traj.current_position = point;
                        }
                    }
                    mixer.recalculate_spatial_dsp();
                }
                AudioCommand::ApplyGlobalTuning {
                    master_headroom_db,
                    peak_limiter_enabled,
                } => {
                    mixer.master_headroom_db = master_headroom_db;
                    mixer.peak_limiter_enabled = peak_limiter_enabled;
                }
                AudioCommand::SetLfeBoostEnabled { enabled } => {
                    // 실제 게인 변화는 믹서가 램프로 따라간다(Law 3).
                    mixer.lfe_boost_enabled = enabled;
                }
                AudioCommand::SetCrossoverFrequency { freq } => {
                    let fs = mixer.sample_rate as f32;
                    for c in mixer.crossovers.iter_mut() {
                        c.set_target_freq(freq, fs);
                    }
                }
                AudioCommand::ApplyAllChannelTunings { tunings } => {
                    for (channel, delay_ms, eq_bands, phase_invert, gain_db) in tunings {
                        if channel < mixer.channel_dsp.len() {
                            mixer.channel_dsp[channel].update_delay_target(delay_ms);
                            mixer.channel_dsp[channel]
                                .update_eq_targets(&eq_bands, mixer.sample_rate as f32);
                            mixer.channel_dsp[channel].phase_invert = phase_invert;
                            mixer.channel_dsp[channel].set_gain_db(gain_db);
                        }
                    }
                }
                // catch-all(`_ => {}`)을 두지 않는다.
                //
                // 예전에는 여기에 catch-all이 있어서 처리부가 없는 커맨드가
                // 조용히 버려졌다. PlayTestNoise가 그 상태로 오래 남아 있었고
                // (커맨드 정의와 FFI 바인딩은 있는데 핸들러가 없어 아무 일도
                // 일어나지 않음), 컴파일러도 아무 경고를 주지 않았다.
                // 이제 match가 모든 variant를 열거하므로, 새 커맨드를 추가하고
                // 처리를 빠뜨리면 컴파일 에러로 즉시 드러난다.
            }
        }
    }
}
