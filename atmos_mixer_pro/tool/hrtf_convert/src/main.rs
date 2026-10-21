//! 개발 전용 변환 도구: 앱에 내장하는 SOFA HRTF(MIT KEMAR)를 앱이 읽는 HrtfSet 바이너리로 옮긴다.
//!
//! 제품(rust/ 크레이트, 설치 파일)에는 들어가지 않는다. sofa-reader(GPL-3.0-or-later)는 이 도구에서만
//! 쓰고, 개발 PC에서 한 번 실행해 결과 파일만 저장소에 둔다. 숫자는 바꾸지 않는다: strict_load가 읽은
//! f32를 비트 그대로 쓴다. 형식은 rust/src/audio/hrtf_data.rs 머리 주석과 같다.
//!
//! 사용: cargo run --release -- <입력 .sofa> <출력 .hrtfset>
//! 출력: SOFA 전역 속성(출처 기록용), 측정점 수·IR 길이·샘플레이트, 다시 읽어 비교한 결과.

use std::path::Path;
use std::process::ExitCode;

use sofa_reader::{SofaFile, SofaReader};

/// rust/src/audio/hrtf_data.rs의 HRTF_SET_MAGIC·HRTF_SET_VERSION과 같아야 한다.
const MAGIC: [u8; 8] = *b"ATMSHRTF";
const VERSION: u32 = 1;
const RECEIVERS: usize = 2;
const HEADER_LEN: usize = 28;

/// 출처 기록에 남길 SOFA 전역 속성.
const ATTRIBUTES: &[&str] = &[
    "Conventions",
    "Version",
    "SOFAConventions",
    "SOFAConventionsVersion",
    "APIName",
    "APIVersion",
    "ApplicationName",
    "ApplicationVersion",
    "DataType",
    "RoomType",
    "Title",
    "DatabaseName",
    "ListenerShortName",
    "Organization",
    "AuthorContact",
    "Comment",
    "History",
    "License",
    "References",
    "Origin",
    "DateCreated",
    "DateModified",
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("사용: hrtf_convert <입력 .sofa> <출력 .hrtfset>");
        return ExitCode::from(2);
    }
    match run(Path::new(&args[1]), Path::new(&args[2])) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("실패: {e}");
            ExitCode::FAILURE
        }
    }
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_bits().to_le_bytes());
}

fn run(input: &Path, output: &Path) -> Result<(), String> {
    let reader = SofaReader::open(input).map_err(|e| format!("SOFA 열기 실패: {e}"))?;
    for name in ATTRIBUTES {
        match reader.attribute_string(name) {
            Ok(v) => println!("attr {name} = {v}"),
            Err(_) => println!("attr {name} = (없음)"),
        }
    }

    let db = SofaFile::strict_load(input).map_err(|e| format!("SOFA strict_load 실패: {e}"))?;
    let (m, n) = (db.num_measurements, db.ir_length);
    if db.positions.len() != m || db.impulse_responses.len() != m * RECEIVERS * n {
        return Err(format!(
            "SOFA 크기가 맞지 않음: 위치 {}개, IR {}개, M={m}, N={n}",
            db.positions.len(),
            db.impulse_responses.len()
        ));
    }
    let m32 = u32::try_from(m).map_err(|_| format!("측정점 수가 너무 큼: {m}"))?;
    let n32 = u32::try_from(n).map_err(|_| format!("IR 길이가 너무 큼: {n}"))?;

    let mut bytes = Vec::with_capacity(HEADER_LEN + 4 * (m * 3 + m * RECEIVERS * n));
    bytes.extend_from_slice(&MAGIC);
    put_u32(&mut bytes, VERSION);
    put_f32(&mut bytes, db.sample_rate);
    put_u32(&mut bytes, m32);
    put_u32(&mut bytes, RECEIVERS as u32);
    put_u32(&mut bytes, n32);
    for p in &db.positions {
        put_f32(&mut bytes, p.azimuth);
        put_f32(&mut bytes, p.elevation);
        put_f32(&mut bytes, p.distance);
    }
    for &v in &db.impulse_responses {
        put_f32(&mut bytes, v);
    }
    std::fs::write(output, &bytes).map_err(|e| format!("쓰기 실패({}): {e}", output.display()))?;

    // 디스크에서 다시 읽어 SOFA 값과 비트 단위로 맞춰 본다.
    let back = std::fs::read(output).map_err(|e| format!("다시 읽기 실패: {e}"))?;
    if back != bytes {
        return Err("다시 읽은 파일이 쓴 바이트와 다름".to_string());
    }
    verify(&back, &db)?;

    println!("M(측정점) = {m}, R(귀) = {RECEIVERS}, N(IR 길이) = {n}");
    println!("sample_rate = {} (비트 {:#010x})", db.sample_rate, db.sample_rate.to_bits());
    println!("출력 {} = {}바이트", output.display(), back.len());
    println!("검증: 위치 {}개·IR {}개 값이 SOFA 읽기 결과와 비트 단위로 같음", m * 3, m * RECEIVERS * n);
    Ok(())
}

/// 쓴 파일을 형식대로 다시 풀어 SOFA 읽기 결과와 값마다 비트를 비교한다.
fn verify(bytes: &[u8], db: &SofaFile) -> Result<(), String> {
    let word = |i: usize| -> Result<u32, String> {
        let b = bytes.get(i..i + 4).ok_or_else(|| format!("{i}번 바이트에서 파일이 끝남"))?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if bytes.get(0..8) != Some(&MAGIC[..]) || word(8)? != VERSION {
        return Err("머리(매직·판)가 다름".to_string());
    }
    if word(12)? != db.sample_rate.to_bits() {
        return Err("샘플레이트 비트가 다름".to_string());
    }
    let (m, n) = (db.num_measurements, db.ir_length);
    if word(16)? as usize != m || word(20)? as usize != RECEIVERS || word(24)? as usize != n {
        return Err("머리의 M·R·N이 다름".to_string());
    }
    let mut at = HEADER_LEN;
    for (i, p) in db.positions.iter().enumerate() {
        for (k, v) in [p.azimuth, p.elevation, p.distance].into_iter().enumerate() {
            if word(at)? != v.to_bits() {
                return Err(format!("위치 {i}의 {k}번째 값 비트가 다름"));
            }
            at += 4;
        }
    }
    for (i, v) in db.impulse_responses.iter().enumerate() {
        if word(at)? != v.to_bits() {
            return Err(format!("IR {i}번째 값 비트가 다름"));
        }
        at += 4;
    }
    if at != bytes.len() {
        return Err(format!("파일 끝에 남는 바이트가 있음({at} / {})", bytes.len()));
    }
    Ok(())
}
