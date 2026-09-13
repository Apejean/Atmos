#[cfg(target_os = "macos")]
pub fn get_channel_names_mac(device_name_target: &str, num_channels: u32) -> Vec<String> {
    use coreaudio_sys::*;
    use std::ffi::CStr;
    use std::ptr;

    let fallback = (1..=num_channels)
        .map(|i| format!("Channel {}", i))
        .collect::<Vec<_>>();

    unsafe {
        let property_address = AudioObjectPropertyAddress {
            mSelector: kAudioHardwarePropertyDevices,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain,
        };

        let mut data_size: u32 = 0;
        let status = AudioObjectGetPropertyDataSize(
            kAudioObjectSystemObject,
            &property_address,
            0,
            ptr::null(),
            &mut data_size,
        );

        if status != 0 {
            return fallback;
        }

        let num_devices = data_size as usize / std::mem::size_of::<AudioObjectID>();
        let mut devices: Vec<AudioObjectID> = vec![0; num_devices];

        let status = AudioObjectGetPropertyData(
            kAudioObjectSystemObject,
            &property_address,
            0,
            ptr::null(),
            &mut data_size,
            devices.as_mut_ptr() as *mut _,
        );

        if status != 0 {
            return fallback;
        }

        for &device_id in &devices {
            // Get device name
            let name_addr = AudioObjectPropertyAddress {
                mSelector: kAudioDevicePropertyDeviceNameCFString,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };
            let mut name_ref: CFStringRef = ptr::null();
            let mut name_size = std::mem::size_of::<CFStringRef>() as u32;

            let status = AudioObjectGetPropertyData(
                device_id,
                &name_addr,
                0,
                ptr::null(),
                &mut name_size,
                &mut name_ref as *mut _ as *mut _,
            );

            if status == 0 && !name_ref.is_null() {
                let length = CFStringGetLength(name_ref);
                let mut buffer: Vec<u8> = vec![0; (length * 4 + 1) as usize];
                if CFStringGetCString(
                    name_ref,
                    buffer.as_mut_ptr() as *mut i8,
                    buffer.len() as i64,
                    kCFStringEncodingUTF8,
                ) != 0
                {
                    let c_str = CStr::from_ptr(buffer.as_ptr() as *const i8);
                    if let Ok(str_slice) = c_str.to_str() {
                        if str_slice == device_name_target {
                            // Match found! Get channel names.
                            let mut names = Vec::new();
                            for i in 1..=num_channels {
                                let ch_name_addr = AudioObjectPropertyAddress {
                                    mSelector: kAudioObjectPropertyElementName,
                                    mScope: kAudioDevicePropertyScopeOutput,
                                    mElement: i,
                                };
                                let mut ch_name_ref: CFStringRef = ptr::null();
                                let mut ch_name_size = std::mem::size_of::<CFStringRef>() as u32;

                                let ch_status = AudioObjectGetPropertyData(
                                    device_id,
                                    &ch_name_addr,
                                    0,
                                    ptr::null(),
                                    &mut ch_name_size,
                                    &mut ch_name_ref as *mut _ as *mut _,
                                );

                                if ch_status == 0 && !ch_name_ref.is_null() {
                                    let ch_length = CFStringGetLength(ch_name_ref);
                                    let mut ch_buffer: Vec<u8> =
                                        vec![0; (ch_length * 4 + 1) as usize];
                                    if CFStringGetCString(
                                        ch_name_ref,
                                        ch_buffer.as_mut_ptr() as *mut i8,
                                        ch_buffer.len() as i64,
                                        kCFStringEncodingUTF8,
                                    ) != 0
                                    {
                                        let ch_c_str =
                                            CStr::from_ptr(ch_buffer.as_ptr() as *const i8);
                                        if let Ok(ch_str) = ch_c_str.to_str() {
                                            names.push(ch_str.to_string());
                                            continue;
                                        }
                                    }
                                }
                                names.push(format!("Channel {}", i));
                            }
                            return names;
                        }
                    }
                }
            }
        }
    }
    fallback
}

#[cfg(target_os = "windows")]
pub fn get_channel_names_win(_device_name_target: &str, num_channels: u32) -> Vec<String> {
    let fallback = (1..=num_channels)
        .map(|i| format!("Channel {}", i))
        .collect::<Vec<_>>();

    // ============================================================
    // TODO(windows-asio-channel-names): 실기(RME UFX+ + Windows) 필요
    // ============================================================
    // 현재 이 경로는 채널 이름을 조회하지 않고 "Channel N"만 돌려준다.
    //
    // 영향: Dart의 `isPhysicalOutputChannel`(lib/core/utils/channel_routing.dart)
    // 은 채널 "이름"으로 드라이버 내부 가상 채널(예: Focusrite의 DAW 리턴)을
    // 걸러낸다. 이름이 없으면 판별 불가로 보고 전부 물리로 취급하므로(안전
    // 쪽으로 열어두는 설계), Windows에서는 아무것도 걸러지지 않는다. 판별
    // "규칙"은 두 OS가 동일하고, RME 명명 규칙(`AN 1`, `ADAT 3` 등)이 그
    // 규칙과 충돌하지 않는 것도 이미 테스트로 확인했다
    // (test/core/utils/channel_routing_test.dart). 부족한 건 "이름을 가져오는
    // 코드" 뿐이다.
    //
    // ## 왜 지금 구현하지 않았는가
    // 1. 이 함수를 쓰는 cpal 0.15.x의 ASIO 백엔드(asio-sys 0.2.6)는 채널
    //    "개수"만 공개 API로 노출한다(`Driver::channels()` ->
    //    `ASIOGetChannels`). 이름을 주는 `ASIOGetChannelInfo`는 크레이트
    //    내부에 비공개 함수(`asio_channel_info`, bindings/mod.rs:908)로만
    //    존재하고 밖으로 노출되지 않는다.
    // 2. 이 macOS 개발 환경에서는 Windows 타겟으로 크로스 컴파일 자체가
    //    안 된다(`cargo check --target x86_64-pc-windows-msvc`가 우리 코드와
    //    무관한 dart-sys 빌드 스크립트에서 시스템 헤더 부재로 즉시 실패하는
    //    것을 확인함, 2026-09-13). 즉 여기서 Windows 코드를 작성해도 문법
    //    오류 여부조차 컴파일러가 확인해 줄 수 없다. 검증 수단이 전혀 없는
    //    unsafe FFI 코드를 "구현 완료"로 남기지 않기 위해, 코드 대신 아래의
    //    실행 가능한 계획을 남긴다.
    //
    // ## 구현 경로 (권장 순서)
    // **A안(권장): asio-sys를 패치해 채널 이름을 공개 API로 노출한다.**
    //
    // 1. `asio-sys` 0.2.6 소스를 로컬로 vendor하거나 fork한다
    //    (crates.io 소스 경로 예:
    //    `~/.cargo/registry/src/*/asio-sys-0.2.6/src/bindings/mod.rs`를
    //    참고해 동일 구조로 복제).
    // 2. `bindings/mod.rs`의 비공개 `asio_channel_info(channel, is_input)`
    //    (908행 부근)를 그대로 두고, `impl Driver`에 다음을 추가한다:
    //    ```rust
    //    pub fn channel_name(&self, channel: c_long, is_input: bool)
    //        -> Result<String, AsioError>
    //    {
    //        let info = asio_channel_info(channel, is_input)?;
    //        // info.name: [c_char; 32], null-terminated.
    //        let cstr = unsafe { std::ffi::CStr::from_ptr(info.name.as_ptr()) };
    //        Ok(cstr.to_string_lossy().into_owned())
    //    }
    //    ```
    //    **동시성 근거**: 같은 파일의 기존 공개 메서드 `Driver::channels()`
    //    (451행)도 `ASIOGetChannels`를 `lock_state()` 없이 곧바로 호출한다.
    //    즉 이 크레이트는 메타데이터 조회(개수 조회)에 스트림 상태 락을
    //    쓰지 않는 것이 기존 관례다. 위 `channel_name()`도 같은 관례를 따라
    //    락 없이 구현하면 되고, 이는 크레이트 저자의 기존 설계를 벗어나지
    //    않는 안전한 확장이다(스트림 시작/정지/파괴처럼 드라이버 내부 상태를
    //    바꾸는 연산만 `lock_state()`를 쓴다, 793~860행 참고).
    // 3. 루트 `Cargo.toml`에 `[patch.crates-io]`로 로컬 경로를 지정해 cpal이
    //    이 fork를 쓰도록 한다.
    // 4. 이 함수에서 `driver.channels()`로 얻은 개수만큼 반복 호출해
    //    `Vec<String>`을 만든다. output이므로 `is_input = false`.
    //
    // **B안(비권장): 여기서 raw `ai::ASIOGetChannelInfo`를 직접 부른다.**
    // asio-sys를 건드리지 않고 `asio-sys`가 재노출하는 `bindings::asio_import`
    // 타입을 직접 써서 이 파일에서 unsafe FFI를 짠다. 크레이트를 패치하지
    // 않아도 되지만, 크레이트의 내부 구현 세부(예: 드라이버가 이미 로드되어
    // 있어야 함, `ASIOInit` 호출 시점)에 이 파일이 직접 의존하게 되어
    // 크레이트 버전이 바뀌면 조용히 깨질 수 있다. A안이 그 결합을 크레이트
    // 경계 안으로 가둔다.
    //
    // ## 검증 순서 (Windows + RME UFX+ 실기에서)
    // 1. `cargo check` (Windows 네이티브 환경에서) — 타입 체크.
    // 2. `cargo test --test test_analysis_channel_width` 등 기존 스위트가
    //    여전히 통과하는지(회귀 없음 확인).
    // 3. 이 함수가 실제로 반환하는 문자열을 눈으로 확인한다. RME는 보통
    //    `AN 1`, `ADAT 3`, `AES 1` 같은 이름을 준다 — 전부
    //    `isPhysicalOutputChannel`이 이미 물리로 판정하도록 테스트되어
    //    있다. 만약 RME가 예상 밖의 접두어(`Loopback`, `DAW`, `Virtual`을
    //    포함하는 이름)를 실제로 반환한다면, 그건 새 사실이므로
    //    `test/core/utils/channel_routing_test.dart`에 그 정확한 문자열로
    //    새 테스트 케이스를 추가하고 나서 판정을 신뢰한다. 추측으로 넘기지
    //    않는다.
    // 4. 앱을 실행해 환경설정/메인화면/스피커 레이아웃/FX 네 화면에 같은
    //    채널 수·같은 이름이 뜨는지 확인한다(macOS에서 한 것과 동일한
    //    수동 QA, task.md 5절 패턴 참고).
    //
    fallback
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn get_channel_names_fallback(num_channels: u32) -> Vec<String> {
    (1..=num_channels)
        .map(|i| format!("Channel {}", i))
        .collect()
}
