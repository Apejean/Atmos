//! All Mute는 엔진(믹서)을 새로 만들어도 유지된다. 음소거 상태는 Dart 화면에만 있고 재동기화가
//! 다시 보내지 않으므로, 새 믹서가 음소거를 잃으면 재시작 복원이 다시 튼 소리가 그대로 나간다.
use rust_lib_atmos_mixer_pro::api::simple::api_set_master_mute;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;

fn new_mixer() -> AudioMixer {
    let (gc_tx, _gc_rx) = crossbeam_channel::unbounded();
    AudioMixer::new(48_000, 2, 480, gc_tx, None)
}

#[test]
fn 음소거한_뒤_새로_만든_믹서도_음소거로_시작한다() {
    api_set_master_mute(true).unwrap();
    assert!(new_mixer().master_mute, "새 믹서가 All Mute를 잃었다(재시작 뒤 소리가 되살아난다)");

    api_set_master_mute(false).unwrap();
    assert!(!new_mixer().master_mute, "음소거를 풀었는데 새 믹서가 음소거로 시작한다");
}
