#!/bin/sh
# Release 빌드에서 앱 번들 Contents/Resources/Legal에 'Third-party License Information.txt'를 넣는다(Windows 설치 폴더의
# Legal과 같은 문서, macOS 대상이라 ASIO·Microsoft 항목은 자동으로 빠진다). 서명(CodeSign)은 모든 빌드 단계 뒤에
# 이루어지므로 여기서 넣은 파일도 서명에 포함된다. 문서를 만들지 못하면 빌드를 멈춘다(고지 없는 앱을 막는다).
# EULA PDF는 Windows 것이 ASIO 로고·문구를 담고 있어(ASIO 계약 3조 1항 g) macOS 판이 따로 확정될 때까지 넣지 않는다.
set -e
[ "$CONFIGURATION" = "Release" ] || exit 0

app="$TARGET_BUILD_DIR/$FULL_PRODUCT_NAME"
dest="$app/Contents/Resources/Legal"
assets="$app/Contents/Frameworks/App.framework/Resources/flutter_assets"

case "${ARCHS%% *}" in
  arm64) target=aarch64-apple-darwin ;;
  *) target=x86_64-apple-darwin ;;
esac

cd "$SRCROOT/.."
mkdir -p "$dest"
"$FLUTTER_ROOT/bin/dart" run tool/legal/generate_third_party_licenses.dart \
  --target "$target" \
  --flutter-assets "$assets" \
  --out "$dest/Third-party License Information.txt"
