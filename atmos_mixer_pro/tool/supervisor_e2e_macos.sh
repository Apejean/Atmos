#!/usr/bin/env bash
# 감시 프로그램 끝까지 확인(macOS, 사람이 옆에서 돌린다). 실제 앱을 감시 프로그램으로 띄워
# 강제 종료·일시 정지·두 번째 감시·앱 직접 두 번째 실행·넘겨받기·창 닫기를 차례로 본다.
# 판단 시간은 짧게 준다(멈춤 10초, 첫 하트비트 60초, 오디오 멈춤 30초). 실제 config로 소리가 난다.
#
# 사용법(atmos_mixer_pro 폴더에서): tool/supervisor_e2e_macos.sh [앱 실행 파일]
#   기본 앱: build/macos/Build/Products/Release/atmos_mixer_pro.app/Contents/MacOS/atmos_mixer_pro
#   먼저 `flutter build macos --release`로 앱을 빌드한다.
set -u
cd "$(dirname "$0")/.." || exit 1

APP="${1:-build/macos/Build/Products/Release/atmos_mixer_pro.app/Contents/MacOS/atmos_mixer_pro}"
LOG_DIR="${TMPDIR:-/tmp}"
LOG_DIR="${LOG_DIR%/}/atmos_mixer_pro_logs"
SUP=supervisor/target/release/atmos_supervisor
SUP_ARGS=(--app "$APP" --hang-secs 10 --first-heartbeat-secs 60 --audio-stall-secs 30)
APP_LOG="$LOG_DIR/atmos_mixer_pro.log"
SUP_LOG="$LOG_DIR/supervisor.log"
PASS=0
FAIL=0

ok() { echo "  통과: $1"; PASS=$((PASS + 1)); }
bad() { echo "  실패: $1"; FAIL=$((FAIL + 1)); }
alive() { kill -0 "$1" 2>/dev/null; }
gone() { ! alive "$1"; }
app_pid() { tr -d '[:space:]' <"$LOG_DIR/app.pid" 2>/dev/null; }
args_of() { ps -o args= -p "$1" 2>/dev/null; }
# 명령줄이 앱 실행 파일로 시작하는 프로세스만 센다(감시의 명령줄에도 --app 경로가 들어 있다).
app_count() { pgrep -f '^[^ ]*Contents/MacOS/atmos_mixer_pro( |$)' | wc -l | tr -d ' '; }
size_of() { stat -f%z "$1" 2>/dev/null || echo 0; }
# [secs]초 안에 명령이 성공할 때까지 1초마다 다시 본다.
wait_for() {
  local secs=$1
  shift
  local end=$((SECONDS + secs))
  until "$@"; do
    [ "$SECONDS" -ge "$end" ] && return 1
    sleep 1
  done
}
app_running() { local p; p=$(app_pid); [ -n "$p" ] && alive "$p"; }
heartbeat_from() { grep -q "^pid=$1\$" "$LOG_DIR/heartbeat" 2>/dev/null; }
new_app_since() { local p; p=$(app_pid); [ -n "$p" ] && [ "$p" != "$1" ] && alive "$p"; }
# [file]의 [offset] 바이트 뒤에 [text]가 나왔나
logged_since() { tail -c +"$(($2 + 1))" "$1" 2>/dev/null | grep -q "$3"; }

cleanup() {
  [ -n "${SUP_PID:-}" ] && alive "$SUP_PID" && kill "$SUP_PID" 2>/dev/null
  local p
  p=$(app_pid)
  [ -n "$p" ] && alive "$p" && kill "$p" 2>/dev/null
}
trap cleanup EXIT

[ -x "$APP" ] || { echo "앱이 없다: $APP — 먼저 flutter build macos --release"; exit 1; }
if [ "$(app_count)" != "0" ]; then
  echo "다른 Atmos 앱이 떠 있다. 닫고 다시 실행한다."
  exit 1
fi
cargo build --release --manifest-path supervisor/Cargo.toml || exit 1
mkdir -p "$LOG_DIR"

echo "== 0. 감시가 앱을 띄운다"
SUP_OFF=$(size_of "$SUP_LOG")
"$SUP" "${SUP_ARGS[@]}" &
SUP_PID=$!
if wait_for 60 app_running && wait_for 90 heartbeat_from "$(app_pid)"; then
  ok "앱이 떴고 하트비트가 온다(pid $(app_pid), 인자: $(args_of "$(app_pid)"))"
else
  bad "앱이 뜨지 않았거나 하트비트가 오지 않았다"
  exit 1
fi
echo "   대시보드에서 루프 트랙 하나를 재생한다(소리가 나는지 듣는다). 준비되면 Enter."
read -r _
sleep 6 # 공연 상태가 한 번 이상 저장되게(5초 간격)

echo "== 1. 앱 강제 종료(kill -9) → --auto-relaunched로 다시 뜨고 멈춘 위치부터 이어 간다"
P=$(app_pid)
APP_OFF=$(size_of "$APP_LOG")
kill -9 "$P"
if wait_for 30 new_app_since "$P"; then
  NEW=$(app_pid)
  case "$(args_of "$NEW")" in
    *--auto-relaunched*) ok "다시 떴다(pid $NEW, 인자: $(args_of "$NEW"))" ;;
    *) bad "다시 뜬 앱의 인자에 --auto-relaunched가 없다: $(args_of "$NEW")" ;;
  esac
  if wait_for 90 logged_since "$APP_LOG" "$APP_OFF" "공연 이어 가기"; then
    ok "앱 로그: $(tail -c +"$((APP_OFF + 1))" "$APP_LOG" | grep "공연 이어 가기" | tail -1)"
  else
    bad "앱 로그에 이어 가기 결과가 없다"
  fi
else
  bad "강제 종료한 앱이 30초 안에 다시 뜨지 않았다"
fi
echo "   소리가 멈춘 위치 근처부터 다시 나는지 듣는다. 확인했으면 Enter."
read -r _

echo "== 2. 앱 일시 정지(kill -STOP) → 멈춤 판단(10초) 뒤 강제 종료되고 다시 뜬다"
wait_for 90 heartbeat_from "$(app_pid)"
P=$(app_pid)
kill -STOP "$P"
if wait_for 40 new_app_since "$P"; then
  ok "멈춘 앱 대신 새 앱이 떴다(pid $(app_pid))"
  logged_since "$SUP_LOG" "$SUP_OFF" "UI 멈춤" && ok "감시 기록에 UI 멈춤" || bad "감시 기록에 UI 멈춤이 없다"
else
  bad "멈춘 앱을 40초 안에 다시 띄우지 않았다"
  kill -CONT "$P" 2>/dev/null
fi
wait_for 90 heartbeat_from "$(app_pid)"

echo "== 3. 감시를 하나 더 실행하면 바로 끝나고, 앱은 하나만 남는다"
"$SUP" "${SUP_ARGS[@]}"
RC=$?
[ "$RC" = "0" ] && alive "$SUP_PID" && ok "두 번째 감시가 끝났고 첫 감시는 돈다" || bad "두 번째 감시 종료 코드 $RC"
sleep 10
[ "$(app_count)" = "1" ] && ok "앱은 하나다" || bad "앱이 $(app_count)개다"

echo "== 4. 앱을 직접 하나 더 실행하면 종료 코드 3으로 바로 끝난다"
"$APP" >/dev/null 2>&1 &
DUP=$!
if wait_for 30 gone "$DUP"; then
  wait "$DUP"
  RC=$?
  [ "$RC" = "3" ] && ok "중복 실행이 종료 코드 3으로 끝났다" || bad "중복 실행 종료 코드 $RC"
else
  bad "직접 실행한 두 번째 앱이 30초 안에 끝나지 않았다"
  kill "$DUP" 2>/dev/null
fi

echo "== 5. 감시만 끝낸 뒤 감시를 다시 띄우면 떠 있는 앱을 넘겨받는다"
P=$(app_pid)
kill "$SUP_PID"
wait "$SUP_PID" 2>/dev/null
alive "$P" && ok "감시가 끝나도 앱은 돈다" || bad "감시를 끝냈더니 앱도 끝났다"
SUP_OFF=$(size_of "$SUP_LOG")
"$SUP" "${SUP_ARGS[@]}" &
SUP_PID=$!
if wait_for 20 logged_since "$SUP_LOG" "$SUP_OFF" "넘겨받았다(pid $P)"; then
  ok "떠 있는 앱을 넘겨받았다(pid $P)"
else
  bad "떠 있는 앱을 넘겨받지 않았다"
fi
sleep 3
[ "$(app_count)" = "1" ] && ok "새로 띄우지 않았다" || bad "앱이 $(app_count)개다"

echo "== 6. 창을 닫으면 다시 뜨지 않고 감시도 끝난다"
echo "   앱 창의 닫기 버튼(빨간 원)을 누른다(⌘Q가 아니라 창 닫기). 60초 기다린다."
P=$(app_pid)
if wait_for 60 gone "$P"; then
  ok "앱이 닫혔다"
  wait_for 15 gone "$SUP_PID" && ok "감시도 끝났다" || bad "앱을 닫았는데 감시가 끝나지 않았다"
  sleep 5
  [ "$(app_count)" = "0" ] && ok "다시 뜨지 않았다" || bad "닫은 앱이 다시 떴다"
else
  bad "60초 안에 창이 닫히지 않았다"
fi

echo
echo "결과: 통과 $PASS, 실패 $FAIL"
echo "감시 기록: $SUP_LOG"
echo "앱 로그: $APP_LOG"
[ "$FAIL" = "0" ]
