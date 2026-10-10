//! 엔진이 설정의 장치를 찾는 규칙(HANDOFF 남은 일 14). 장치가 없으면 엔진은 30초 동안 찾는다 — 부팅 직후 USB
//! 인터페이스가 늦게 올라오는 경우 때문이다. 다만 기다려도 나타날 수 없는 이름은 찾지 않고 바로 실패시킨다.
//! 실제로 찾는 일(장치 목록, ASIO 드라이버 불러 보기)은 audio::engine이 한다.

/// 찾기 전에 바로 실패시킬 이유. None이면 찾는다(최대 30초).
/// - 다른 OS에서 저장한 이름: 맥의 `[CoreAudio] …`를 Windows에서, Windows의 `[ASIO]`·`[WASAPI] …`를 macOS에서.
/// - 이 PC에 설치되지 않은 ASIO 드라이버. [installed_asio]는 드라이버를 불러오지 않고 읽은 설치 목록이다(Windows의
///   ASIO 이름일 때만 넘긴다). 드라이버는 있는데 인터페이스가 아직 안 켜졌으면 기다린다.
pub fn reject_before_search(requested: &str, installed_asio: Option<&[String]>) -> Option<String> {
    let foreign = if cfg!(target_os = "windows") {
        requested.starts_with("[CoreAudio]")
    } else {
        requested.starts_with("[ASIO]") || requested.starts_with("[WASAPI]")
    };
    if foreign {
        return Some(format!("'{requested}'는 다른 OS의 오디오 장치라 이 컴퓨터에서 열 수 없습니다"));
    }
    let driver = requested.strip_prefix("[ASIO]").map(clean_name)?;
    let installed = installed_asio?;
    if installed.iter().any(|name| clean_name(name) == driver) {
        return None;
    }
    Some(format!("ASIO 드라이버 '{driver}'가 이 컴퓨터에 설치돼 있지 않습니다"))
}

/// 장치 이름 비교용: 널 문자와 앞뒤 공백을 뗀다(엔진이 장치 목록과 견줄 때와 같다).
pub fn clean_name(name: &str) -> String {
    name.replace('\0', "").trim().to_string()
}
