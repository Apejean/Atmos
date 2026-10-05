//! 앱 로그 파일 회전. 크기를 넘으면 뒤로 밀고(.1, .2 …) 정한 개수만 남긴다. 내보내기는 밀린 파일까지 복사한다.
//! Windows의 %TEMP%는 저절로 비워지지 않아, 회전이 없으면 무인 운영에서 로그가 끝없이 커진다.
use rust_lib_atmos_mixer_pro::core::log_file::{append_line, export_to, LOG_FILE_NAME};
use std::path::{Path, PathBuf};

fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("atmos_log_test_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn 크기를_넘으면_뒤로_밀고_정한_개수만_남기며_내보내기는_전부_복사한다() {
    let dir = fresh_dir("rotate");
    // 한 줄 10바이트("line-0000" + 줄바꿈), 파일당 30바이트를 넘으면 민다, 밀린 파일은 2개까지
    for i in 0..20 {
        append_line(&dir, &format!("line-{i:04}"), 30, 2).unwrap();
    }
    assert_eq!(
        names(&dir),
        vec![LOG_FILE_NAME.to_string(), format!("{LOG_FILE_NAME}.1"), format!("{LOG_FILE_NAME}.2")],
        "지금 파일 + 밀린 파일 2개만 남아야 한다"
    );
    for name in names(&dir) {
        let len = std::fs::metadata(dir.join(&name)).unwrap().len();
        assert!(len <= 40, "{name}이 {len}바이트 — 회전 크기(30)를 한 줄 넘게 넘었다");
    }
    // 최신 줄은 지금 파일 끝에, 바로 앞 줄들은 .1에 있다
    let current = std::fs::read_to_string(dir.join(LOG_FILE_NAME)).unwrap();
    assert!(current.ends_with("line-0019\n"), "지금 파일 끝: {current:?}");
    let older = std::fs::read_to_string(dir.join(format!("{LOG_FILE_NAME}.1"))).unwrap();
    assert!(older.contains("line-0015"), ".1 내용: {older:?}");

    let dest = fresh_dir("export");
    std::fs::create_dir_all(&dest).unwrap();
    assert_eq!(export_to(&dir, &dest).unwrap(), 3, "지금 파일과 밀린 파일을 모두 복사해야 한다");
    assert_eq!(names(&dest), names(&dir));

    // 로그가 하나도 없으면 0개
    let empty = fresh_dir("empty");
    std::fs::create_dir_all(&empty).unwrap();
    assert_eq!(export_to(&empty, &dest).unwrap(), 0);

    for d in [dir, dest, empty] {
        let _ = std::fs::remove_dir_all(d);
    }
}
