//! 그룹 시작(docs/superpowers/specs/2026-11-12-group-start-design.md, HANDOFF 확정된 결정 "동기").
//! 같은 그룹의 대기 인스턴스는 StartGroup이 든 블록의 같은 샘플에서 함께 시작하고(서로 다른 블록에 들어왔어도),
//! 첫 묶음이 늦거나 재생 중 디스크가 밀린 인스턴스는 밀린 만큼 건너뛰어 그룹 위치로 합류한다.
//! 대기 중에 정지가 오면 들린 적 없이 정리된다.
use rust_lib_atmos_mixer_pro::audio::engine::{AudioEngine, ENGINE_GENERATION};
use rust_lib_atmos_mixer_pro::audio::mixer::AudioMixer;
use rust_lib_atmos_mixer_pro::audio::player::SoundInstance;
use rust_lib_atmos_mixer_pro::common::commands::AudioCommand;
use rust_lib_atmos_mixer_pro::core::state::GLOBAL_STATE;
use std::sync::atomic::Ordering;

const FS: u32 = 48_000;
const CH: usize = 4;
const BLOCK: usize = 480;
const GROUP: u64 = 77;
/// 디코더 묶음 크기(WAV 해독 한 번 분량과 비슷하게).
const CHUNK: usize = 1152;
/// 위치 비교 허용 오차(초): 반 샘플.
const HALF_SAMPLE: f64 = 0.5 / FS as f64;

/// 장치 콜백 대신 시험이 블록을 하나씩 돌린다. 명령은 시험 전용 큐로 보낸다.
struct Rig {
    mixer: AudioMixer,
    tx: crossbeam_channel::Sender<AudioCommand>,
    rx: crossbeam_channel::Receiver<AudioCommand>,
    temp: Vec<f32>,
    out: Vec<f32>,
    /// 출력 채널별로 지금까지 낸 샘플.
    rendered: Vec<Vec<f32>>,
    _gc_rx: crossbeam_channel::Receiver<SoundInstance>,
}

impl Rig {
    fn new() -> Self {
        for ch in 0..CH {
            GLOBAL_STATE.enabled_channels[ch].store(true, Ordering::SeqCst);
        }
        let (gc_tx, gc_rx) = crossbeam_channel::unbounded();
        let mut mixer = AudioMixer::new(FS, CH, BLOCK, gc_tx, None);
        mixer.startup_ramp.current_gain = 1.0;
        let (tx, rx) = crossbeam_channel::unbounded();
        Rig {
            mixer,
            tx,
            rx,
            temp: vec![0.0; BLOCK * CH],
            out: vec![0.0; BLOCK * CH],
            rendered: vec![Vec::new(); CH],
            _gc_rx: gc_rx,
        }
    }

    fn send(&self, cmd: AudioCommand) {
        self.tx.send(cmd).expect("명령 큐");
    }

    /// 콜백 한 번: 명령 처리(begin_callback) + 한 블록(render_block).
    fn block(&mut self) {
        let generation = ENGINE_GENERATION.load(Ordering::SeqCst);
        assert!(AudioEngine::begin_callback(&mut self.mixer, &self.rx, generation));
        AudioEngine::render_block(&mut self.mixer, &mut self.temp, &mut self.out, CH, CH);
        for f in 0..BLOCK {
            for (c, chan) in self.rendered.iter_mut().enumerate() {
                chan.push(self.out[f * CH + c]);
            }
        }
    }

    fn first_sound(&self, ch: usize) -> Option<usize> {
        self.rendered[ch].iter().position(|v| v.abs() > 1e-6)
    }

    fn position(&self, id: u64) -> f64 {
        self.mixer
            .instances
            .iter()
            .flatten()
            .find(|i| i.instance_id == id)
            .map(|i| i.position_seconds())
            .expect("인스턴스가 풀에 없다")
    }

    fn has(&self, id: u64) -> bool {
        self.mixer.instances.iter().flatten().any(|i| i.instance_id == id)
    }
}

/// 출력 `out`(0부터)으로 나가는 모노 스트리밍 루프 인스턴스와 그 묶음을 넣는 생산자. 그룹 GROUP에 대기 표시.
/// 생산자를 버리면 스트림이 끝난 것으로 보므로 시험이 들고 있어야 한다.
fn armed_stream(id: u64, out: usize) -> (SoundInstance, rtrb::Producer<Vec<f32>>) {
    let (tx, rx) = rtrb::RingBuffer::<Vec<f32>>::new(256);
    let mut inst = SoundInstance::new(
        id, id as u32, 1, format!("t{id}"), None, None, FS, 1, true, 1.0, out, false, None,
        GLOBAL_STATE.enabled_channels.len(),
    );
    inst.stream_receiver = Some(rx);
    inst.arm_for_group(GROUP);
    (inst, tx)
}

/// 파일 위치 `from`부터 `frames`개를 `CHUNK` 묶음으로 넣는다. 값은 0이 아니다.
fn feed(tx: &mut rtrb::Producer<Vec<f32>>, from: usize, frames: usize) {
    let mut at = from;
    while at < from + frames {
        let n = CHUNK.min(from + frames - at);
        let chunk: Vec<f32> = (at..at + n).map(|i| 0.2 + 0.1 * ((i % 97) as f32 / 97.0)).collect();
        assert!(tx.push(chunk).is_ok(), "링이 가득 찼다");
        at += n;
    }
}

fn play(inst: SoundInstance) -> AudioCommand {
    AudioCommand::PlayTrack { instance: inst, room_volume: 1.0 }
}

#[test]
fn 서로_다른_블록에_들어온_대기_인스턴스도_startgroup_블록의_같은_샘플에서_시작한다() {
    let mut rig = Rig::new();
    let (a, mut ta) = armed_stream(1, 0);
    let (b, mut tb) = armed_stream(2, 1);
    feed(&mut ta, 0, BLOCK * 10);
    feed(&mut tb, 0, BLOCK * 10);
    rig.send(play(a));
    rig.block();
    rig.send(play(b));
    rig.block();
    assert!(rig.first_sound(0).is_none() && rig.first_sound(1).is_none(), "StartGroup 전에 소리가 났다");
    assert!(rig.position(1).abs() < HALF_SAMPLE, "대기 중에 위치가 움직였다");

    rig.send(AudioCommand::StartGroup { group_id: GROUP });
    for _ in 0..4 {
        rig.block();
    }
    let fa = rig.first_sound(0).expect("A 소리가 없다");
    let fb = rig.first_sound(1).expect("B 소리가 없다");
    assert_eq!(fa, fb, "같은 그룹이 다른 샘플에 시작했다");
    assert!((2 * BLOCK..3 * BLOCK).contains(&fa), "StartGroup이 든 블록에서 시작해야 한다: {fa}");
    assert!((rig.position(1) - rig.position(2)).abs() < HALF_SAMPLE);
}

#[test]
fn 첫_묶음이_늦은_인스턴스는_밀린_만큼_건너뛰어_그룹_위치로_합류한다() {
    let mut rig = Rig::new();
    let (a, mut ta) = armed_stream(1, 0);
    let (b, mut tb) = armed_stream(2, 1);
    feed(&mut ta, 0, BLOCK * 20);
    rig.send(play(a));
    rig.send(play(b));
    rig.send(AudioCommand::StartGroup { group_id: GROUP });
    for _ in 0..3 {
        rig.block(); // B는 아직 첫 묶음이 없다
    }
    assert!(rig.first_sound(1).is_none());
    feed(&mut tb, 0, BLOCK * 20); // 디코더가 이제야 파일 처음부터 보낸다
    for _ in 0..3 {
        rig.block();
    }
    let (pa, pb) = (rig.position(1), rig.position(2));
    assert!((pa - pb).abs() < HALF_SAMPLE, "B가 그룹 위치로 합류하지 못했다: A {pa} B {pb}");
    let fb = rig.first_sound(1).expect("B 소리가 없다");
    assert!(fb >= 3 * BLOCK, "B는 묶음이 온 블록에서 소리가 나야 한다: {fb}");
}

#[test]
fn 재생_중_묶음이_비면_밀린_만큼_건너뛰어_다시_맞춘다() {
    let mut rig = Rig::new();
    let (a, mut ta) = armed_stream(1, 0);
    let (b, mut tb) = armed_stream(2, 1);
    feed(&mut ta, 0, BLOCK * 30);
    feed(&mut tb, 0, BLOCK * 2);
    rig.send(play(a));
    rig.send(play(b));
    rig.send(AudioCommand::StartGroup { group_id: GROUP });
    for _ in 0..5 {
        rig.block(); // B는 2블록 뒤 묶음이 바닥난다
    }
    // 디코더는 B가 밀린 줄 모르고 이어서 보낸다
    feed(&mut tb, BLOCK * 2, BLOCK * 28);
    for _ in 0..2 {
        rig.block();
    }
    let (pa, pb) = (rig.position(1), rig.position(2));
    assert!((pa - pb).abs() < HALF_SAMPLE, "밀린 B가 다시 맞춰지지 않았다: A {pa} B {pb}");
}

#[test]
fn 대기_중에_전체_정지가_오면_들리지_않고_정리된다() {
    let mut rig = Rig::new();
    let (a, mut ta) = armed_stream(1, 0);
    feed(&mut ta, 0, BLOCK * 10);
    rig.send(play(a));
    rig.block();
    rig.send(AudioCommand::StopAll);
    rig.block();
    assert!(!rig.has(1), "대기 중 정지된 인스턴스가 풀에 남았다");
    rig.send(AudioCommand::StartGroup { group_id: GROUP });
    for _ in 0..3 {
        rig.block();
    }
    assert!(rig.first_sound(0).is_none(), "정지 뒤에 온 StartGroup으로 소리가 났다");
}

#[test]
fn cancelgroup은_대기_인스턴스를_들리지_않게_정리한다() {
    let mut rig = Rig::new();
    let (a, mut ta) = armed_stream(1, 0);
    feed(&mut ta, 0, BLOCK * 10);
    rig.send(play(a));
    rig.send(AudioCommand::CancelGroup { group_id: GROUP });
    rig.block();
    assert!(!rig.has(1), "취소한 그룹의 인스턴스가 풀에 남았다");
    rig.send(AudioCommand::StartGroup { group_id: GROUP });
    for _ in 0..3 {
        rig.block();
    }
    assert!(rig.first_sound(0).is_none(), "취소한 그룹이 소리를 냈다");
}
