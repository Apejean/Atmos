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

    // 현재 Windows 경로는 채널 이름을 조회하지 않고 "Channel N"만 돌려준다.
    //
    // 이것이 갖는 의미(중요): Dart의 `isPhysicalOutputChannel`
    // (lib/core/utils/channel_routing.dart)은 채널 이름으로 드라이버 내부
    // 가상 채널(DAW 리턴, 루프백 등)을 걸러낸다. macOS(CoreAudio)는
    // "Mon 1"/"DAW 7" 같은 실제 이름을 주므로 걸러지지만, Windows는 이름이
    // 없어 모든 채널이 물리로 판정된다(판별 불가 시 열어두는 설계).
    // 규칙 자체는 두 OS에서 같지만 입력 데이터가 없어 결과가 달라진다.
    //
    // 동일한 동작을 얻으려면 여기서 실제 채널 이름을 채워야 한다.
    // - ASIO: `ASIOGetChannelInfo`의 `name` 필드 (드라이버가 "Analogue 1",
    //   "SPDIF L", "Loopback 1" 등을 준다)
    // - WASAPI: 엔드포인트 단위라 채널별 이름이 사실상 없다. 공유 모드에서는
    //   보통 2채널이므로 실익이 크지 않다.
    //
    // 미구현으로 두는 이유는 검증 수단이 없어서다. Windows/ASIO 실기 없이
    // 작성한 FFI 코드를 "동작한다"고 넣는 것보다, 이름이 없다는 사실을
    // 드러내고 안전한 쪽(전부 표시)으로 두는 편이 낫다.
    fallback
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn get_channel_names_fallback(num_channels: u32) -> Vec<String> {
    (1..=num_channels)
        .map(|i| format!("Channel {}", i))
        .collect()
}
