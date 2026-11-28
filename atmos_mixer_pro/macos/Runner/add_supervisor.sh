#!/bin/sh
# Release 빌드에서 감시 프로그램(atmos_supervisor, ../supervisor)을 앱 번들 Contents/MacOS에 넣는다. macOS 로그인 자동
# 실행(환경설정 "macOS 시작 시 자동 실행")은 LaunchAgent가 이것을 --logon으로 띄우고, 감시가 같은 폴더의 앱을 띄운다
# (Windows의 작업 스케줄러 → 감시 → 앱과 같은 흐름). 앱이 유니버설이라 ARCHS마다 빌드해 lipo로 합친다.
# 서명은 앱 서명보다 먼저 이 파일에 따로 한다(안에 든 실행 파일이 서명되지 않으면 앱 서명 확인이 깨진다).
set -e
[ "$CONFIGURATION" = "Release" ] || exit 0

export PATH="$HOME/.cargo/bin:$PATH"
app="$TARGET_BUILD_DIR/$FULL_PRODUCT_NAME"
dest="$app/Contents/MacOS/atmos_supervisor"
manifest="$SRCROOT/../supervisor/Cargo.toml"
target_dir="$SRCROOT/../supervisor/target"

parts=""
for arch in $ARCHS; do
  case "$arch" in
    arm64) triple=aarch64-apple-darwin ;;
    x86_64) triple=x86_64-apple-darwin ;;
    *) echo "add_supervisor: 모르는 아키텍처 $arch" >&2; exit 1 ;;
  esac
  cargo build --release --manifest-path "$manifest" --target "$triple" --target-dir "$target_dir"
  parts="$parts $target_dir/$triple/release/atmos_supervisor"
done

# shellcheck disable=SC2086
lipo -create $parts -output "$dest"
codesign --force --sign "${EXPANDED_CODE_SIGN_IDENTITY:--}" --timestamp=none "$dest"
