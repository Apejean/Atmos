//! cpal 0.15.3 macOS 버그 보완용 CoreAudio 직접 조회.
//!
//! cpal 0.15.3은 출력 장치의 설정을 조회할 때(`supported_output_configs`,
//! `default_output_config`) 오디오 유닛을 **입력 모드**로 만든다
//! (`audio_unit_from_device(self, true)`). 입력이 없는 출력 전용 장치 — Apple Silicon
//! 맥북의 내장 스피커("MacBook Pro 스피커")가 대표적이다 — 는 여기서
//! `Invalid property value`로 실패하고, 그 결과 `output_devices()` 목록에서도 빠진다.
//! 반면 실제 스트림은 출력 모드로 열기 때문에 정상 재생된다.
//!
//! 그래서 cpal이 설정을 읽지 못한 장치에 한해, 스트림을 여는 데 필요한 값
//! (출력 채널 수, 현재 샘플레이트, 버퍼 크기 범위)을 CoreAudio 속성에서 직접 읽는다.
//! 장치 열거와 엔진 기동 경로에서만 호출되며 오디오 스레드와는 무관하다.

use coreaudio_sys::*;
use std::ffi::CStr;
use std::mem::size_of;
use std::ptr;

/// 이름이 `device_name`인 장치의 출력 스트림 설정.
/// 장치가 없거나 출력 채널이 0이면(입력 전용 장치) None.
pub fn output_config(device_name: &str) -> Option<cpal::SupportedStreamConfig> {
    let device_id = find_device_id(device_name)?;
    let channels = output_channel_count(device_id)?;
    if channels == 0 {
        return None;
    }
    let sample_rate: f64 = get_property(device_id, kAudioDevicePropertyNominalSampleRate)?;
    let buffer_size =
        match get_property::<AudioValueRange>(device_id, kAudioDevicePropertyBufferFrameSizeRange)
        {
            Some(r) => cpal::SupportedBufferSize::Range {
                min: r.mMinimum as u32,
                max: r.mMaximum as u32,
            },
            None => cpal::SupportedBufferSize::Unknown,
        };
    Some(cpal::SupportedStreamConfig::new(
        channels.min(u16::MAX as u32) as u16,
        cpal::SampleRate(sample_rate as u32),
        buffer_size,
        // cpal CoreAudio 백엔드는 F32 스트림을 장치 고유 포맷으로 변환해서 연다.
        cpal::SampleFormat::F32,
    ))
}

/// cpal `output_devices()`에서 빠진 출력 전용 장치를 이름으로 찾는다.
/// CoreAudio가 출력 채널을 보고하는 경우에만 돌려준다.
pub fn find_output_device(host: &cpal::Host, device_name: &str) -> Option<cpal::Device> {
    use cpal::traits::{DeviceTrait, HostTrait};
    output_config(device_name)?;
    host.devices().ok()?.find(|d| {
        d.name()
            .map(|n| n.replace('\0', "").trim() == device_name)
            .unwrap_or(false)
    })
}

/// 이름으로 CoreAudio 장치 ID를 찾는다(cpal `Device::name()`과 같은 속성으로 비교).
fn find_device_id(device_name: &str) -> Option<AudioObjectID> {
    let addr = AudioObjectPropertyAddress {
        mSelector: kAudioHardwarePropertyDevices,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    unsafe {
        let mut size: u32 = 0;
        if AudioObjectGetPropertyDataSize(kAudioObjectSystemObject, &addr, 0, ptr::null(), &mut size)
            != 0
        {
            return None;
        }
        let mut ids: Vec<AudioObjectID> = vec![0; size as usize / size_of::<AudioObjectID>()];
        if AudioObjectGetPropertyData(
            kAudioObjectSystemObject,
            &addr,
            0,
            ptr::null(),
            &mut size,
            ids.as_mut_ptr() as *mut _,
        ) != 0
        {
            return None;
        }
        // 두 호출 사이에 장치가 빠졌을 수 있으니 실제로 채워진 개수만 본다.
        ids.truncate(size as usize / size_of::<AudioObjectID>());
        ids.into_iter()
            .find(|&id| device_name_of(id).as_deref() == Some(device_name))
    }
}

fn device_name_of(id: AudioObjectID) -> Option<String> {
    let addr = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyDeviceNameCFString,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    unsafe {
        let mut name_ref: CFStringRef = ptr::null();
        let mut size = size_of::<CFStringRef>() as u32;
        if AudioObjectGetPropertyData(
            id,
            &addr,
            0,
            ptr::null(),
            &mut size,
            &mut name_ref as *mut _ as *mut _,
        ) != 0
            || name_ref.is_null()
        {
            return None;
        }
        let len = CFStringGetLength(name_ref);
        let mut buf: Vec<u8> = vec![0; (len * 4 + 1) as usize];
        let ok = CFStringGetCString(
            name_ref,
            buf.as_mut_ptr() as *mut _,
            buf.len() as _,
            kCFStringEncodingUTF8,
        ) != 0;
        // 이 속성은 호출자가 해제해야 하는 CFString을 돌려준다.
        CFRelease(name_ref as CFTypeRef);
        if !ok {
            return None;
        }
        let name = CStr::from_bytes_until_nul(&buf).ok()?.to_string_lossy();
        Some(name.replace('\0', "").trim().to_string())
    }
}

/// 출력 스트림들의 채널 수 합계(kAudioDevicePropertyStreamConfiguration, 출력 스코프).
fn output_channel_count(id: AudioObjectID) -> Option<u32> {
    let addr = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyStreamConfiguration,
        mScope: kAudioObjectPropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };
    unsafe {
        let mut size: u32 = 0;
        if AudioObjectGetPropertyDataSize(id, &addr, 0, ptr::null(), &mut size) != 0
            || (size as usize) < size_of::<u32>()
        {
            return None;
        }
        // AudioBufferList 안에 포인터가 있어 8바이트 정렬 버퍼로 받는다.
        let mut buf: Vec<u64> = vec![0; (size as usize).div_ceil(8)];
        if AudioObjectGetPropertyData(id, &addr, 0, ptr::null(), &mut size, buf.as_mut_ptr() as *mut _)
            != 0
        {
            return None;
        }
        let list = buf.as_ptr() as *const AudioBufferList;
        let n = ptr::addr_of!((*list).mNumberBuffers).read() as usize;
        let first = ptr::addr_of!((*list).mBuffers) as *const AudioBuffer;
        // 받은 크기를 넘어 읽지 않도록 확인한다.
        let needed = (first as usize - list as usize) + n * size_of::<AudioBuffer>();
        if needed > size as usize {
            return None;
        }
        Some((0..n).map(|i| (*first.add(i)).mNumberChannels).sum())
    }
}

/// 장치의 전역 스코프 고정 크기 속성 하나를 읽는다.
fn get_property<T: Copy>(id: AudioObjectID, selector: AudioObjectPropertySelector) -> Option<T> {
    let addr = AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut value = std::mem::MaybeUninit::<T>::uninit();
    let mut size = size_of::<T>() as u32;
    unsafe {
        let status = AudioObjectGetPropertyData(
            id,
            &addr,
            0,
            ptr::null(),
            &mut size,
            value.as_mut_ptr() as *mut _,
        );
        if status != 0 || size as usize != size_of::<T>() {
            return None;
        }
        Some(value.assume_init())
    }
}
