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

/// 엔진이 지금 연 ASIO 드라이버의 출력 채널 이름: (드라이버 이름, 채널 0부터의 이름). 엔진 스레드가 스트림을
/// 만든 뒤 채우고(`remember_asio_output_names`), 장치 목록 API가 읽는다. 오디오 콜백은 쓰지 않는다.
#[cfg(target_os = "windows")]
static ASIO_OUTPUT_NAMES: std::sync::Mutex<Option<(String, Vec<String>)>> = std::sync::Mutex::new(None);

/// cpal이 지금 불러 둔 ASIO 드라이버에 출력 채널 이름을 묻고 기억한다(`ASIOGetChannelInfo`, 이름은 31자까지).
/// 드라이버를 새로 부르거나 내리지 않는다: asio-sys는 불러 둔 드라이버를 `Asio` 객체마다 따로 기억해서, 다른
/// `Asio`로 `load_driver`하면 ASIO SDK가 지금 드라이버를 먼저 내려(removeCurrentDriver) 재생이 끊긴다. 그래서
/// 엔진 스레드에서, 그 드라이버로 스트림을 만든 뒤 재생 전에 부른다. asio-sys는 cpal과 같은 하나다(Cargo.lock).
#[cfg(target_os = "windows")]
pub fn remember_asio_output_names(driver_name: &str) {
    use asio_sys::bindings::asio_import as ai;
    use std::os::raw::c_long;

    let (mut ins, mut outs): (c_long, c_long) = (0, 0);
    // SAFETY: cpal이 이 드라이버를 불러 ASIOInit과 버퍼 생성을 마친 뒤, 같은 엔진 스레드에서 묻기만 한다.
    // 0은 ASE_OK.
    let rc = unsafe { ai::ASIOGetChannels(&mut ins, &mut outs) };
    if rc != 0 {
        crate::core::state::GLOBAL_STATE
            .log(format!("ASIO 출력 채널 이름을 묻지 못했다({driver_name}): ASIOGetChannels {rc}"));
        return;
    }
    let names: Vec<String> = (0..outs)
        .map(|channel| {
            let mut info = ai::ASIOChannelInfo {
                channel,
                isInput: 0,
                isActive: 0,
                channelGroup: 0,
                type_: 0,
                name: [0; 32],
            };
            // SAFETY: 위와 같다.
            if unsafe { ai::ASIOGetChannelInfo(&mut info) } != 0 {
                return String::new();
            }
            let bytes: Vec<u8> = info.name.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
            String::from_utf8_lossy(&bytes).trim().to_string()
        })
        .collect();
    let mut names = names;
    let stripped = strip_common_suffix(&mut names)
        .map(|s| format!(", 모든 채널에 같은 꼬리 \"{}\" 뗌", s.trim()))
        .unwrap_or_default();
    let named = names.iter().filter(|n| !n.is_empty()).count();
    crate::core::state::GLOBAL_STATE.log(format!(
        "ASIO 출력 채널 이름 {named}/{outs}개({driver_name}): {} … {}{stripped}",
        names.first().map_or("", String::as_str),
        names.last().map_or("", String::as_str)
    ));
    if let Ok(mut remembered) = ASIO_OUTPUT_NAMES.lock() {
        *remembered = Some((driver_name.to_string(), names));
    }
}

/// 채널 이름 num_channels개. 엔진이 연 ASIO 드라이버(`remember_asio_output_names`)면 그 드라이버가 준 이름을,
/// 아니면(WASAPI, 엔진이 열지 않은 ASIO 드라이버) "Channel N"을 쓴다. Dart의 `isPhysicalOutputChannel`이 이
/// 이름으로 드라이버 내부 가상 채널을 거른다(RME의 `AN 1`·`ADAT 3`·`AES 1`은 물리로 판정, channel_routing_test).
#[cfg(target_os = "windows")]
pub fn get_channel_names_win(device_name_target: &str, num_channels: u32) -> Vec<String> {
    let remembered = ASIO_OUTPUT_NAMES.lock().ok().and_then(|r| r.clone());
    names_or_fallback(remembered.as_ref(), device_name_target, num_channels)
}

/// 모든 채널 이름 끝에 같은 괄호 꼬리가 붙어 있으면 떼고 그 꼬리를 돌려준다. RME는 장치 번호를 이렇게
/// 붙인다(`Analog 1 (1)`, 2026-10-11 실기). 여러 대를 묶어 꼬리가 서로 다르면(`(1)`·`(2)`) 장치를 가려야
/// 하니 그대로 둔다. 꼬리가 없는 이름이 하나라도 있거나, 떼면 빈 이름이 되면 손대지 않는다. 빈 이름은 본다.
pub fn strip_common_suffix(names: &mut [String]) -> Option<String> {
    fn suffix_of(name: &str) -> Option<&str> {
        let name = name.trim_end();
        let open = name.rfind(" (")?;
        (name.ends_with(')') && open > 0).then(|| &name[open..])
    }
    let mut common: Option<&str> = None;
    for name in names.iter().filter(|n| !n.is_empty()) {
        let suffix = suffix_of(name)?;
        if common.is_some_and(|c| c != suffix) {
            return None;
        }
        common = Some(suffix);
    }
    let common = common?.to_string();
    for name in names.iter_mut().filter(|n| !n.is_empty()) {
        let keep = name.trim_end().len() - common.len();
        name.truncate(keep);
    }
    Some(common)
}

/// `get_channel_names_win`의 이름 고르기. 드라이버 이름이 같을 때만 그 이름을 쓰고, 빈 이름과 모자란 채널은
/// "Channel N"(1부터)이다. OS와 상관없이 시험할 수 있게 따로 둔다(test_channel_names).
pub fn names_or_fallback(
    remembered: Option<&(String, Vec<String>)>,
    device_name: &str,
    num_channels: u32,
) -> Vec<String> {
    (0..num_channels as usize)
        .map(|i| {
            remembered
                .filter(|(driver, _)| driver == device_name)
                .and_then(|(_, names)| names.get(i))
                .filter(|name| !name.is_empty())
                .cloned()
                .unwrap_or_else(|| format!("Channel {}", i + 1))
        })
        .collect()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn get_channel_names_fallback(num_channels: u32) -> Vec<String> {
    (1..=num_channels)
        .map(|i| format!("Channel {}", i))
        .collect()
}
