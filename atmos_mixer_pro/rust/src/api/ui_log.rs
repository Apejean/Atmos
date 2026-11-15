//! 화면(Dart) 쪽 예외를 앱 로그에 남긴다(검토 보고서 A8). 기록만 하고 엔진·상태 방송·설정은 건드리지 않는다.

/// 한 줄에 남기는 최대 글자 수(바이트가 아니라 글자).
pub const MAX_CHARS: usize = 4000;
/// 상세 줄에 넣는 스택의 앞 줄 수.
pub const STACK_LINES: usize = 12;

/// 줄바꿈·제어 문자를 공백으로 바꾸고 글자 경계로 자른다(측정 스크립트가 로그를 줄 단위로 읽는다).
fn one_line(text: &str, max_chars: usize) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let trimmed = flat.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let mut cut: String = trimmed.chars().take(max_chars).collect();
        cut.push('…');
        cut
    }
}

/// 로그 두 줄: `사건: ui_error kind=… summary=…`(첫 줄 요약)과 상세(메시지 전체 + 스택 앞 줄, ` | `로 이음).
#[flutter_rust_bridge::frb(ignore)]
pub fn format_ui_error(kind: &str, message: &str, stack: &str) -> (String, String) {
    let kind = one_line(kind, 40).replace(' ', "_");
    let first = message.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let summary = format!("사건: ui_error kind={kind} summary={}", one_line(first, 300));
    let stack_part: Vec<&str> = stack
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .take(STACK_LINES)
        .collect();
    let detail_raw = if stack_part.is_empty() {
        message.to_string()
    } else {
        format!("{message} | {}", stack_part.join(" | "))
    };
    let detail = format!("화면 오류 상세({kind}): {}", one_line(&detail_raw, MAX_CHARS));
    (summary, detail)
}

/// 화면 쪽 예외 하나를 앱 로그에 두 줄로 남긴다.
pub fn api_log_ui_error(kind: String, message: String, stack: String) {
    let (summary, detail) = format_ui_error(&kind, &message, &stack);
    crate::core::state::GLOBAL_STATE.log(summary);
    crate::core::state::GLOBAL_STATE.log(detail);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_line_message_becomes_one_line_summary() {
        let (summary, detail) =
            format_ui_error("flutter", "A RenderFlex overflowed\nby 12 pixels\r\non the right", "#0 a\n#1 b\n");
        assert_eq!(summary, "사건: ui_error kind=flutter summary=A RenderFlex overflowed");
        assert!(!summary.contains('\n') && !detail.contains('\n') && !detail.contains('\r'));
        assert!(detail.starts_with("화면 오류 상세(flutter): A RenderFlex overflowed by 12 pixels"));
        assert!(detail.ends_with("| #0 a | #1 b"));
    }

    #[test]
    fn korean_and_long_messages_are_cut_on_char_boundaries() {
        let long_korean = "한글오류".repeat(1500); // 6000글자, 바이트로는 18000
        let (summary, detail) = format_ui_error("platform", &long_korean, "");
        // 자르다 패닉이 나지 않고, 글자 수 상한 + 말줄임표 하나다.
        let body = detail.trim_start_matches("화면 오류 상세(platform): ");
        assert_eq!(body.chars().count(), MAX_CHARS + 1);
        assert!(body.ends_with('…'));
        assert!(summary.chars().count() < 400);
    }

    #[test]
    fn stack_is_limited_to_first_lines_and_control_chars_are_spaces() {
        let stack: String = (0..30).map(|i| format!("#{i} frame\n")).collect();
        let (_, detail) = format_ui_error("flutter", "bad\u{0007}bell", &stack);
        assert!(detail.contains("bad bell"));
        assert!(detail.contains("#11 frame"));
        assert!(!detail.contains("#12 frame"));
    }

    #[test]
    fn empty_inputs_do_not_panic() {
        let (summary, detail) = format_ui_error("", "", "");
        assert_eq!(summary, "사건: ui_error kind= summary=");
        assert_eq!(detail, "화면 오류 상세(): ");
    }
}
