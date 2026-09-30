//! 장치 선택/재스캔 때마다 오디오 엔진은 믹서를 새로 만든다. 예전에는 바이노럴
//! 켜짐 여부가 믹서 필드에만 있어서 새 믹서가 `enabled: false`로 시작했다.
//! UI 배지는 켜짐으로 남는데 실제로는 바이노럴 없이 ch0 원본이 왼쪽 출력으로만
//! 나갔다(실기 보고: "Ch1이 왼쪽에서만 들린다", "재스캔하고 다시 켜야 동작한다").

use rust_lib_atmos_mixer_pro::api::simple::api_set_binaural_enabled;
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;

#[test]
fn new_mixer_inherits_user_binaural_setting() {
    api_set_binaural_enabled(true);
    let (gc_tx, _gc_rx) = crossbeam_channel::bounded(16);
    let restarted = AudioMixer::new(48_000, 16, 1024, gc_tx, None);
    assert!(
        restarted.binaural.enabled,
        "사용자가 바이노럴을 켰는데 엔진 재시작 후 새 믹서에서 꺼져 있다"
    );

    api_set_binaural_enabled(false);
    let (gc_tx2, _gc_rx2) = crossbeam_channel::bounded(16);
    let restarted_off = AudioMixer::new(48_000, 16, 1024, gc_tx2, None);
    assert!(
        !restarted_off.binaural.enabled,
        "사용자가 바이노럴을 껐는데 새 믹서에서 켜져 있다"
    );
}
