//! 바이노럴 HRIR 전환 출력은 "정지 상태 렌더 두 개(이전 방향, 새 방향)를
//! 같은 비율로 섞은 결과"와 같아야 한다.
//!
//! 예전에는 오버랩 테일을 블록 끝 시점의 섞임 비율 하나로 미리 섞어 저장했다.
//! 다음 블록 앞부분은 샘플마다 비율이 달라지는데 테일만 고정이라, 직전 입력의
//! 응답이 몰려 있는 HRIR 피크 지점(블록 시작 후 약 49샘플)에서 어긋났다.
//! 실측: 이상적 섞기 2차 차분 1.21배 vs 실제 6.14배, 오차는 페이드 중인 매
//! 블록 49번째 샘플 부근(실기 보고: "드래그할 때 틱틱 끊기는 소리").

use rust_lib_atmos_mixer_pro::audio::binaural::VirtualMixRoomBinaural;

const FS: f32 = 48_000.0;
const BLOCK: usize = 1024;
const NCH: usize = 16;
const CHANGE_BLK: usize = 30;
const XFADE: usize = BLOCK * 4;

fn render(change: bool, from: f32, to: f32) -> Vec<[f32; 2]> {
    let mut b = VirtualMixRoomBinaural::new(NCH, BLOCK);
    b.enabled = true;
    b.channel_base_azimuth_mut()[0] = from;
    b.mark_base_azimuth_dirty();
    let mut ph = 0.0f32;
    let mut out = Vec::new();
    for blk in 0..40 {
        if change && blk == CHANGE_BLK {
            b.channel_base_azimuth_mut()[0] = to;
            b.mark_base_azimuth_dirty();
        }
        let mut buf = vec![0.0f32; BLOCK * NCH];
        for f in 0..BLOCK {
            buf[f * NCH] = ph.sin() * 0.3;
            ph += 2.0 * std::f32::consts::PI * 440.0 / FS;
        }
        b.process_interleaved(&mut buf, NCH);
        for f in 0..BLOCK {
            out.push([buf[f * NCH], buf[f * NCH + 1]]);
        }
    }
    out
}

#[test]
fn switching_output_equals_ideal_blend_of_static_renders() {
    for to in [10.0f32, 40.0, 90.0] {
        let y0 = render(false, 0.0, 0.0);
        let y1 = render(false, to, to);
        let ys = render(true, 0.0, to);
        let start = CHANGE_BLK * BLOCK;
        let mut worst_err = 0.0f32;
        let mut at = 0usize;
        for n in start..(start + 8 * BLOCK) {
            let k = n - start;
            let p = ((k + 1) as f32 / XFADE as f32).min(1.0);
            for c in 0..2 {
                let ideal = (1.0 - p) * y0[n][c] + p * y1[n][c];
                let e = (ys[n][c] - ideal).abs();
                if e > worst_err {
                    worst_err = e;
                    at = k;
                }
            }
        }
        println!("0 -> {to}도: 이상적 섞기와의 최대 오차 {worst_err:.6} (전환 후 {}블록 {}샘플)", at / BLOCK, at % BLOCK);
        assert!(
            worst_err < 5e-4,
            "0 -> {to}도 전환 출력이 이상적 섞기와 {worst_err:.5}만큼 어긋난다 \
             (전환 후 {}블록 {}샘플, 신호 진폭 0.3)",
            at / BLOCK,
            at % BLOCK
        );
    }
}
