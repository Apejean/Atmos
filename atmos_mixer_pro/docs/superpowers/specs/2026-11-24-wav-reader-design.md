# 자체 WAV 리더 — 27채널 이상·RF64 파일 열기 (안정성 ⑨ 설계)

> 상태: **설계 초안(2026-11-24, Windows Main). 아래 "정할 것" D1~D4는 사용자 결정 뒤 구현한다.** 범위는 HANDOFF 확정된 결정 "한 파일 27채널 이상을 지원해야 한다(채널 수는 현장에 따라 무한)"와 남은 일 16 ⑨를 따른다. 경로는 `atmos_mixer_pro/` 기준. 엔진 구조(파일 읽기 경로)를 바꾸므로 저장소 CLAUDE.md의 에스컬레이션 대상이다. FRB API 시그니처는 바꾸지 않는다.

## 1. 왜

- 지금은 모든 음원을 symphonia 0.5.5로 연다. symphonia의 WAV 읽개는 채널 수를 채널 위치 플래그(`Channels`, 26개)로 바꾸는데, 27채널 이상이면 `riff: invalid channel count`로 열기 자체가 실패한다(`symphonia-format-riff-0.5.5/src/common.rs` `try_channel_count_to_mask`).
- symphonia의 WAV 읽개는 `RIFF` 머리만 안다. 4GB를 넘는 WAV의 표준인 **RF64**(EBU Tech 3306, `RF64`·`ds64` 덩어리)는 열지 못한다. 24채널 24bit 48kHz 파일은 약 20분이면 4GB에 닿는다(이 PC 시험 파일 `long_24ch_10min.wav`가 10분에 2.07GB). 64채널이면 약 7분이다.
- 현장에는 MADI(64채널)·Dante처럼 채널이 많은 시스템이 오고, 콘텐츠 제작 쪽은 채널마다 한 파일로 주거나 다채널 한 파일로 준다. 한 파일 다채널을 받으려면 둘 다 필요하다.

## 2. 지금 27채널 이상 파일을 넣으면

코드로 확인했다(2026-11-24, main 087d44f). 앱으로 열어 보지는 않았다.

| 자리 | 지금 동작 |
|---|---|
| 트랙 카드·출력 목록(`api_get_audio_file_channels` → `SoundData::probe_channels`) | 열기 실패를 **2채널로** 돌려준다. 그래서 화면은 스테레오 파일로 보고 스테레오 자리를 권한다. |
| 처리 폭(`output_groups::cached_file_channels` → `probe_channels`) | 같은 이유로 2채널로 계산한다. 멀티 줄이 열 채널 수가 틀린다. |
| 재생: 스트리밍(`DiskStreamer::new_at`) | 열기 오류로 재생이 시작되지 않는다. |
| 재생: 메모리(`api_preload_sound` → `SoundData::load_from_file_with_source`) | 미리 읽기 실패 → 재생 때 다시 읽어 보고 실패(A5 규칙). |
| 이어 가기 길이(`restart_resume` → `streaming::file_frames`) | 길이를 몰라 "길이 모름"으로 이어 간다. |

RF64는 채널 수와 상관없이 위와 같다.

## 3. 원칙

1. **27채널 이상이거나 RF64일 때만** 자체 리더를 탄다(확정된 결정). 그 밖의 파일(지금 쓰는 모든 파일)은 symphonia 경로를 그대로 둔다 — 회귀를 막는다.
2. 형식은 **PCM 정수 16·24·32bit와 32bit 부동소수**부터(확정된 결정 "S16/S24/S32/F32부터"). 그 밖은 분명한 오류 문구로 거절한다(D2).
3. 채널 마스크(`WAVE_FORMAT_EXTENSIBLE`의 `dwChannelMask`)는 **개수 확인용으로만** 본다. 채널 순서는 파일에 들어 있는 순서 그대로다(파일 채널 n → 출력 시작 + n). 마스크로 채널을 재배치하지 않는다.
4. 파일 읽기는 디코더 스레드·미리 읽기에서만 한다. **오디오 콜백은 바뀌지 않는다**(3법칙 영향 없음). 믹서가 받는 것은 지금과 같은 "인터리브 f32 묶음"이다.
5. 같은 파일을 두 길로 읽지 않는다: 머리말을 읽어 자체 리더 대상이면 끝까지 그 리더만 쓴다(열기·다시 열기·길이·채널 수 모두).

## 4. 구조

### 4.1 새 모듈 `rust/src/audio/wav_reader.rs`

```rust
/// 머리말에서 읽은 것.
pub struct WavInfo {
    pub channels: u16,          // 1..=MAX_FILE_CHANNELS
    pub sample_rate: u32,
    pub format: PcmFormat,      // I16 | I24 | I32 | F32
    pub frames: u64,            // data 바이트 / block_align
    pub data_offset: u64,       // 파일 안 data 시작
    pub rf64: bool,
}

/// 이 파일을 자체 리더로 읽어야 하나. WAV가 아니거나 머리말이 깨졌으면 None(지금 길 그대로 — symphonia가 오류를 낸다).
pub fn sniff(path: &Path) -> Option<WavInfo>;   // Some이면 channels > 26 || rf64

pub struct WavReader { /* BufReader<File>, info, 다음 프레임 */ }
impl WavReader {
    pub fn open(path: &Path) -> anyhow::Result<WavReader>;
    pub fn info(&self) -> &WavInfo;
    /// 최대 `max_frames`를 f32 인터리브로 `out`에 채운다(out을 비우고 다시 씀). 0이면 파일 끝.
    pub fn read(&mut self, max_frames: usize, out: &mut Vec<f32>) -> std::io::Result<usize>;
    /// 정확히 그 프레임으로 간다(PCM이라 샘플 단위로 정확).
    pub fn seek(&mut self, frame: u64) -> std::io::Result<()>;
}
```

- **머리말 읽기**
  - `RIFF`/`RF64` + `WAVE`. RF64면 바로 뒤 `ds64`의 64비트 크기(RIFF 크기, data 크기, 표본 수)를 쓴다. `data` 크기가 `0xFFFFFFFF`일 때 ds64 값을 쓴다.
  - 덩어리를 차례로 건너뛰며 `fmt `과 `data`를 찾는다(LIST·bext·JUNK·iXML 등은 건너뜀, 홀수 크기는 1바이트 채움).
  - `fmt `: 형식 1(PCM)·3(IEEE float)·0xFFFE(EXTENSIBLE, 하위 형식 GUID가 PCM이나 float). 비트 수와 `block_align = channels × bytes`가 맞는지 본다. 24bit는 3바이트로 꽉 채운 것만(`wValidBitsPerSample` < 컨테이너는 컨테이너 크기로 읽는다).
  - `data`가 파일 끝보다 길면(녹음 중 끊긴 파일) 실제 파일 길이까지로 줄이고 로그를 남긴다.
- **변환**(symphonia와 같은 값): I16 `/32768`, I24 `/8388608`(부호 확장), I32 `/2147483648`, F32 그대로.
- 읽기는 묶음 단위(예: 1024프레임, 채널 수와 상관없이)로 큰 `BufReader`(1MB)에서 한 번에 읽어 바꾼다. 버퍼는 리더가 들고 다시 쓴다.

### 4.2 끼울 자리

| 자리 | 바꾸는 것 |
|---|---|
| `audio/streaming.rs` `DiskStreamer::new_at` | 처음에 `wav_reader::sniff`. Some이면 채널·샘플레이트·길이를 거기서 얻는다(symphonia probe를 건너뜀). |
| `audio/streaming.rs` `Source`·`open_source`·`open_source_at` | `Source`를 둘 중 하나로: `Symphonia { format, decoder, track_id }` / `Wav(WavReader)`. 다시 열기(A6 규칙: 루프 끝·읽기 오류·연속 오류, 1·2·5·10초)와 사건 로그는 그대로 쓴다. Wav의 찾기는 늘 정확하다(`seeked = true`). |
| `audio/streaming.rs` 디코더 루프 | 패킷 → 디코드 → `SampleBuffer` 자리에서 Wav면 `read()`로 바로 인터리브 f32를 얻는다. 그 뒤(다시 연 자리 앞 버리기, 회복 기록, 리샘플러, `send_chunk`)는 공용이다. 파일 끝 = `read()`가 0. |
| `audio/streaming.rs` `file_frames` | sniff가 Some이면 `(frames, sample_rate)`. |
| `audio/player.rs` `SoundData::probe_channels` | sniff가 Some이면 그 채널 수. **열기 실패 때 2를 돌려주는 것은 그대로 둔다**(다른 형식의 지금 동작). |
| `audio/player.rs` `load_from_file_with_source` | sniff가 Some이면 `WavReader`로 끝까지 읽는다(리샘플·96kHz 초과 다운샘플은 지금과 같은 함수). |

그 밖(믹서, 처리 폭, 출력 줄 게이트, 그룹 시작, 이어 가기 규칙)은 채널 수만 맞게 받으면 그대로 동작한다.

- **믹서**: 인스턴스 채널 수만큼 `temp_vals`(믹서 폭) 안에서 읽는다(`ch_limit = channels.min(temp_vals.len())`). 처리 폭이 "시작 채널 + 파일 채널 수 − 1"까지 열리므로(`processing_channel_count`, 멀티 줄 규칙) 64채널 파일을 CH1에 두면 폭 64다. 장치 채널보다 많은 파일 채널은 지금처럼 버려진다(D4).
- **리샘플러**(rubato `SincFixedOut`)는 채널 수 제한이 없다. 64채널 리샘플은 무거우므로(디코더 스레드) 시험에서 처리 시간을 잰다.

### 4.3 채널 상한 `MAX_FILE_CHANNELS`

- 파일 채널 수에 상한을 둔다(D1, 권장 256). 이보다 많으면 "채널이 너무 많습니다(최대 256)"로 거절한다.
- 매트릭스 라우팅 설계(`2026-11-14-matrix-routing-design.md` 6.5)가 "⑨가 정한 파일 채널 최대 수"를 함께 쓰기로 했다. 같은 상수를 쓴다(그 문서의 `FILE_CHANNEL_READ_CAP` 64 대신).
- 장치 쪽 출력 줄 표는 이미 4096채널이다(`*_rows_open`). 상한은 메모리·묶음 크기 보호용이다.

### 4.4 메모리에 올리는 트랙(스트리밍 끔)의 한도

- 지금 미리 읽기 한도는 2천5백만 샘플(약 100MB)이다(`api_preload_sound`). 64채널 48kHz면 약 8초, 256채널이면 약 2초다. 넘으면 미리 읽기가 실패하고 그 트랙은 재생되지 않는다(지금도 ≤26채널 큰 파일에 같은 일이 있다).
- 결정 D3: 한도를 넘는 파일은 재생할 때 **자동으로 스트리밍**하게 할지.

## 5. 정할 것 (사용자 결정)

| # | 질문 | 권장 | 이유 |
|---|---|---|---|
| D1 | 한 파일 최대 채널 수 | **256** | MADI 64·Dante 대형 구성(128~256)을 넘는다. 상한이 있어야 묶음·버퍼 크기를 미리 잡는다. 더 필요하면 상수 하나만 바꾼다. |
| D2 | 처음 지원할 형식 | **PCM 16·24·32bit, float 32bit** | 확정된 결정 그대로. float 64bit·8bit·압축 WAV(ADPCM 등)는 거절하고 문구로 알린다. 필요해지면 더한다. |
| D3 | 미리 읽기 한도를 넘는 파일 | **재생할 때 자동 스트리밍**(모든 파일에 적용, 앱 로그 한 줄) | 지금은 그 트랙이 그냥 안 난다(무음·거절). 현장에서 큰 다채널 파일을 "스트리밍 끔"으로 둔 실수가 무음이 되지 않게. 다른 안: 지금처럼 거절하고 트랙 카드에 경고만. |
| D4 | 파일 채널이 장치 출력보다 많을 때 | **지금처럼 남는 채널은 버리고, 트랙 카드에 "파일 N채널 중 M채널만 나감"** | 장치가 바뀌어도 공연은 나야 한다. 버려지는 것을 화면에 알린다(지금은 조용히 버린다). |

## 6. 시험

`rust/tests/test_wav_reader.rs`(시험 안에서 작은 WAV를 만들어 쓴다, 저장소에 큰 파일을 넣지 않는다):
- 머리말: RIFF 32·64채널 × I16/I24/I32/F32, EXTENSIBLE, 앞에 LIST·JUNK가 있는 파일, 홀수 크기 덩어리, RF64(`ds64`, data 크기 0xFFFFFFFF), 끊긴 data, 26채널 이하 RIFF는 sniff가 None.
- 값: 채널마다 다른 값(채널 번호 × 작은 상수)을 써서 읽은 인터리브 순서·변환 값이 맞는지. 24bit 음수 부호 확장.
- 찾기: 임의 프레임으로 찾은 뒤 읽은 값이 처음부터 읽은 값과 같다.
- 거절: 형식 2(ADPCM)·float 64bit·채널 257개 → 분명한 오류.
- 스트리밍: 32채널 루프 파일을 `DiskStreamer`로 열어 두 바퀴 묶음이 이어지는지, `new_at` 시작 위치가 샘플 단위로 맞는지, 이름을 바꿔 다시 열기 실패 → 되돌려 회복(A6 시험과 같은 방식).
- 메모리 읽기·채널 수·길이: `load_from_file_with_source`, `probe_channels` = 32, `file_frames`.
- 회귀: 지금 시험 전체(`cargo test`)가 그대로 통과(≤26채널은 symphonia 길).
- 부하(조사용, `#[ignore]`): 64채널 48kHz 파일 스트리밍의 디코더 스레드 처리 시간, 64채널 리샘플(44.1→48kHz) 처리 시간.

장치 확인(사람이 들음): 32채널·64채널 시험 파일을 멀티 CH1 시작으로 MADIface에 내보내 채널마다 다른 톤이 제 채널에서 나는지(R3 시험 파일과 같은 방식: 채널 n은 200 + 20n Hz, 매초 클릭).

## 7. 범위 밖

- 샘플레이트가 다른 파일(리샘플)을 먼 위치에서 이어 틀 때 처음부터 해독하는 것(HANDOFF 남은 일 8 "일부 해결")은 symphonia 길의 문제라 따로 다룬다. 자체 리더 파일은 리샘플이어도 원본 프레임으로 정확히 찾은 뒤 리샘플러를 다시 시작하므로 이 문제가 없다(리샘플러 앞부분 몇 프레임의 과도 응답은 시험에서 본다).
- WAV가 아닌 다채널 형식(CAF, W64, FLAC 8채널 초과 등)은 다루지 않는다.
- 매트릭스 라우팅(파일 채널을 여러 출력으로 나누기)은 별도 설계 문서다. 이 문서는 "파일 채널 n → 시작 + n"까지만이다.

## 8. 작업 순서

1. `wav_reader.rs` 머리말·읽기·찾기 + 단위 시험(실패 → 구현 → 통과).
2. `probe_channels`·`file_frames`·`load_from_file_with_source`에 끼우기 + 시험.
3. `streaming.rs` `Source` 두 갈래 + 디코더 루프 + 다시 열기 시험.
4. D3·D4(결정에 따라) + 화면 문구.
5. 전체 `cargo test`·`flutter test`, 장치 확인(32·64채널), 릴리스.

## 9. 위험

| 위험 | 막는 법 |
|---|---|
| 지금 쓰는 파일의 동작이 바뀜 | 27채널 이상·RF64만 새 길. 26채널 이하 RIFF는 sniff가 None을 주는 시험. |
| 머리말 해석 실수로 잡음(채널이 밀림) | 채널마다 다른 값을 쓴 시험 파일, 사람이 채널별 톤 확인. |
| 64채널 이상 리샘플이 디코더 스레드에서 느림 | 부하 시험으로 재고, 느리면 리샘플 파일은 미리 바꿔 두라고 안내(현장은 48kHz 원본 권장). |
| 큰 파일 읽기로 디스크가 밀림 | R3 측정(2026-11-24)에서 24채널 2분 따라잡기(약 440MB)도 부하 경고 0. 64채널은 시험에서 다시 잰다. |
