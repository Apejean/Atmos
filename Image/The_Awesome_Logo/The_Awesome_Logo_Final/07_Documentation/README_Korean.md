# The Awesome 로고 패키지 (황금비 설계 버전)

원본 로고의 형태와 인상을 유지하면서, 심볼을 **황금비와 피보나치 수로 작도**해 다시 만든 벡터 세트입니다. 황금나선은 쓰지 않았고 비율만 썼습니다. 원본을 그대로 따라 그린 이전 결과는 `The_Awesome_Logo_Final_v1_원본비율.zip`에 있습니다.

## 폴더
```
The_Awesome_Logo_Final/
 01_Original_Reference/  원본 이미지와 원본 분석 보고서
 02_SVG/                 Full_Logo / Symbol / Wordmark 각각 _Dark(검정 배경) · _Light(흰 배경) · *_Transparent(배경 없음)
   Mono/                 단색(검정/흰색) 심볼·전체 로고, 배경 있음/없음 — 도장·자수·각인·실크인쇄용
 03_PNG/                 위와 같은 구성의 PNG (전체 로고·글자 가로 3000px, 심볼 1024×1024)
   Mono/                 단색 PNG
 04_App_Icons/           Windows.ico(정사각), macOS.icns(표준 여백), 1024 PNG
   Web/                  favicon.ico/.svg, 16·32 PNG, apple-touch-icon(180), android 192/512, site.webmanifest, head_snippet.html
 05_Illustrator/         Full_Logo_Dark/Light(.pdf, 배경 포함) + _Transparent.pdf, Symbol_Dark/Light.pdf, 단색 PDF,
                         Full_Logo.eps, Illustrator_Compatible.svg
 06_Comparison/          원본과 황금비 버전 비교
 07_Documentation/       이 문서, 규격, 제작 스크립트
 08_Golden_Design/       황금비 설계도(심볼 작도, 전체 비례)와 치수 명세
```

## 핵심 규칙 (자세한 치수는 `08_Golden_Design/Golden_Design_Specification.md`)
- 심볼 틀 34u × 26u (φ²:2), 정점은 폭의 황금분할점(34/φ = 21u)
- 좌우 다리의 가로 폭이 같고(6.5u), 안쪽 삼각 꼭짓점은 높이 1 − 1/φ³ 지점
- 아치: 같은 황금 타원 두 개(세로÷가로 = φ)가 리본 끝에서 만남, 리본 두께 = 다리 폭 ÷ φ, 정점 높이 = 안쪽 삼각 꼭짓점 높이 ÷ φ
- 제목 폭 = 심볼 폭 × φ, (심볼+간격+제목) 블록은 황금사각형, 간격비 φ, 글자 크기비 φ²

## 사용 요령
- **다크 버전**(`*_Dark`)은 검정 배경에 다크용 금속 음영과 흰 글자, **라이트 버전**(`*_Light`)은 흰 배경에 라이트용 금속 음영과 검정 글자입니다. 두 버전은 같은 도형·같은 배치입니다.
- 배경을 직접 깔 때는 `*_Transparent` 파일을 쓰세요: 어두운 배경 위에는 `Dark_Transparent`, 밝은 배경 위에는 `Light_Transparent`.
- 그라디언트를 못 쓰는 곳(도장, 자수, 각인, 실크인쇄, 1도 인쇄)에는 `Mono` 폴더의 단색 로고를 쓰세요. 리본이 겹치는 곳은 얇은 틈(다리 폭 ÷ φ⁵)으로 구분했습니다.
- 웹사이트에는 `04_App_Icons/Web`의 파일을 사이트 루트에 두고 `head_snippet.html`의 태그를 `<head>`에 넣으세요.
- Illustrator에서는 `Full_Logo_*.pdf`를 여는 것을 권장합니다. `Full_Logo.eps`는 음영이 단순한 버전이고 렌더링을 확인하지 못했습니다.
