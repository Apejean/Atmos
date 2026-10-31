import sys, os, json, shutil, zipfile, re
sys.path.insert(0,'work')
import numpy as np, cv2
from PIL import Image
import sym2
from sym2 import *

ROOT='The_Awesome_Logo_Final'
def P(*a): return os.path.join(ROOT,*a)
M=json.load(open('work/metrics.json')); WM=json.load(open('work/wordmark.json')); VO=json.load(open('work/verify_outputs.json'))
G=geometry(); faces=build(G); fits=fit_all(faces)

# ---- 01: reference + analysis drawing ----
shutil.copy('work/dark_panel.png',P('01_Original_Reference','Original_Reference_DarkPanel.png'))
big=cv2.imread('work/sym_big.png'); dr=big.copy()
cols={'leg_right':(80,200,255),'leg_left':(255,200,80),'arch_front':(120,255,120),'arch_right_low':(255,120,200),'arch_right_band':(0,0,255)}
for k in ORDER:
    pts=faces[k].poly().round().astype(np.int32); cv2.polylines(dr,[pts],True,cols[k],2,cv2.LINE_AA)
    for v in pts[::24]: pass
for name,pt in (('TL',G['TL']),('apex',G['A_apex']),('inner',G['inner_top']),('BL',G['BLc']),('BRL',G['BRL']),('BRR',G['BRR'])):
    p=tuple(int(round(v)) for v in pt); cv2.circle(dr,p,6,(255,255,255),-1); cv2.putText(dr,'%s (%d,%d)'%(name,p[0],p[1]),(p[0]+8,p[1]-8),0,0.5,(255,255,255),1,cv2.LINE_AA)
cv2.rectangle(dr,(42,43),(1112,860),(0,255,255),1)
cv2.putText(dr,'bbox 42,43 - 1112,860 (big px = 4x panel px)',(50,35),0,0.55,(0,255,255),1,cv2.LINE_AA)
cv2.imwrite(P('01_Original_Reference','Analysis_Reference_Drawing.png'),dr)

# ---- measurements ----
head=[np.array(b) for b in WM['head']['bbox_panel_px']]; tag=[np.array(b) for b in WM['tag']['bbox_panel_px']]
def metr(bbs):
    w=[b[1][0]-b[0][0] for b in bbs]; gaps=[bbs[i+1][0][0]-bbs[i][1][0] for i in range(len(bbs)-1)]
    top=np.mean([b[0][1] for b in bbs]); bot=np.mean([b[1][1] for b in bbs])
    return w,gaps,top,bot
hw,hg,ht,hb=metr(head); tw,tg,tt,tb=metr(tag)
cap=hb-ht; tcap=tb-tt
sym_w=(1112-42)/4; sym_h=(860-43)/4
hx0,hx1=head[0][0][0],head[-1][1][0]; tx0,tx1=tag[0][0][0],tag[-1][1][0]
sx0=160+42/4; sx1=160+1112/4; scx=(sx0+sx1)/2
L=lambda xs:', '.join('%.1f'%v for v in xs)
HL='THE AWESOME'.replace(' ','')
rep=f'''# 원본 분석 보고서 (The Awesome 로고)

## 0. 기준 이미지
- 요청서의 `reference/logo_original.png`는 제공되지 않았고, 첨부된 **브랜드 시안 보드 1장(1536×1024)** 만 있었다. 보드에서 심볼이 가장 크고 선명한 **왼쪽 위 "전체 로고 (다크 배경)" 패널**(보드 좌표 0,0–600,460)을 기준으로 삼았다. 사본: `Original_Reference_DarkPanel.png`, 보드 전체: `Original_Board.png`.
- 이 패널에서 심볼 폭은 약 {sym_w:.0f}px다. **원본 해상도 자체가 낮아서**, 그 이하의 정밀도는 측정할 수 없다(아래 오차 참고).
- 오른쪽 위 "전체 로고 (라이트 배경)" 패널은 **같은 로고가 아니다**: 심볼 외곽 폭×높이가 233×195px(비 1.19)로, 다크 패널의 {sym_w:.0f}×{sym_h:.0f}px(비 {sym_w/sym_h:.2f})와 비율이 다르고 글자 폭도 442px 대 454px로 다르다. 그래서 라이트 패널은 기준으로 쓰지 않았고, 라이트용 파일은 다크 패널 기하를 그대로 쓰고 글자색만 바꿨다.

## 1. 심볼 측정 (다크 패널, 도구: Python 3.15 / OpenCV 5.0 / NumPy)
| 항목 | 값 |
|---|---|
| 바운딩 박스(패널 px) | x {sx0:.1f} – {sx1:.1f}, y {95+0.75:.1f} – 300.0 |
| 크기 / 가로세로 비 | {sym_w:.1f} × {sym_h:.1f} px / **{sym_w/sym_h:.3f}** |
| 상단 꼭짓점(좌/우) | 큰 좌표계 ({G['TL'][0]:.0f},{G['TL'][1]:.0f}) / ({G['A_apex'][0]:.0f},{G['A_apex'][1]:.0f}) |
| 안쪽 삼각 음각 꼭짓점 | ({G['inner_top'][0]:.0f},{G['inner_top'][1]:.0f}) |
| 좌하 / 우하 끝 | ({G['BLc'][0]:.0f},{G['BLc'][1]:.0f}) / ({G['BRL'][0]:.0f},{G['BRL'][1]:.0f}), 우측 다리 안쪽 끝 ({G['BRR'][0]:.0f},{G['BRR'][1]:.0f}) |
| 좌우 하단 절단 | 두 다리 모두 수평 절단(y={G['BLc'][1]:.0f}) |

"큰 좌표계"는 패널 크롭(160,85)을 원점으로 4배 확대한 좌표다(=SVG 내부 좌표). 외곽선은 임계값 윤곽 + 법선 방향 최대 기울기 스냅(가장자리 위치 보정)으로 얻었다. 좌표는 `Analysis_Reference_Drawing.png`(기준 도면)에 표시했다.

### 구조(앞뒤 순서)
1. `leg_right` 오른쪽 다리 전체(뒤) – 위쪽이 어두운 차콜, 아래로 갈수록 은백색.
2. `leg_left` 왼쪽 다리 전체 – 위쪽 밝은 은색, 아래(아치 뒤)는 회색으로 어두워짐. 오른쪽 가장자리가 꺾이는 지점이 안쪽 삼각 음각의 꼭짓점.
3. `arch_front` 곡선 리본 앞면 – 왼쪽 다리 앞을 가로질러 오른쪽 다리 안쪽 가장자리에서 끊김.
4. `arch_right_low` 오른쪽 다리 아래쪽(아치 윗선 아래).
5. `arch_right_band` 아치가 오른쪽 다리 앞으로 이어지는 어두운 띠.
중앙의 삼각형 음각 공간은 별도 도형이 아니라 두 다리와 아치 사이의 **빈 공간(투명)** 이다.

### 금속 명암
각 면마다 (1) 면 안 밝기 회귀로 얻은 주 방향 선형 그라디언트 12스톱, (2) 직교 방향 흑/백 반투명 오버레이 12스톱으로 표현했다. 면별 스톱 수: {', '.join(f"{k} {len(v[0][2])}+{len(v[1][2])}" for k,v in fits.items())}.

## 2. 워드마크 측정 (패널 px)
- 글자 윤곽 10개 / 태그라인 19개(TECHNOLOGY 10 + × 1 + ARTISTRY 8)를 8배 확대 후 50% 임계 윤곽으로 추출.
- THE AWESOME: x {hx0:.1f}–{hx1:.1f}(폭 {hx1-hx0:.1f}), 대문자 높이 {cap:.1f}px, 글자 폭 [{L(hw)}], 글자 사이 간격 [{L(hg)}].
- TECHNOLOGY × ARTISTRY: x {tx0:.1f}–{tx1:.1f}(폭 {tx1-tx0:.1f}), 대문자 높이 {tcap:.1f}px, 글자 폭 [{L(tw)}], 간격 [{L(tg)}].
- 상대 위치: 두 줄 중심 x = {(hx0+hx1)/2:.1f} / {(tx0+tx1)/2:.1f}, 심볼 중심 x = {scx:.1f}. 태그라인 대문자 높이/제목 대문자 높이 = {tcap/cap:.2f}. 제목 윗선–심볼 아랫선 간격 {ht-300:.1f}px, 두 줄 사이 {tt-hb:.1f}px.

## 3. 벡터화 방법
1. 심볼: 윤곽/경계선 직선·곡선을 가장자리 스냅으로 맞춘 뒤 면별 Bézier(Catmull-Rom→cubic, 곡선당 9–16개 제어점)로 구성. PNG/JPEG 삽입 없음.
2. 워드마크: 8배 확대 윤곽을 Schneider 곡선 맞춤으로 Bézier화. 직선 획 글자(T,H,E,A,W,M 등)는 번진 모서리를 직선 맞춤 후 교점으로 다시 날카롭게 만들었다. 글꼴은 대입하지 않았다.
3. 렌더링(resvg)해 원본 패널과 같은 크기로 비교(06_Comparison).
'''
open(P('01_Original_Reference','Original_Analysis_Report.md'),'w',encoding='utf-8').write(rep)

# ---- docs ----
def fs(x): return json.dumps(x,ensure_ascii=False)
spec=f'''# 로고 규격 (Logo_Specifications)

| 항목 | 값 |
|---|---|
| 심볼 | 은색 금속 리본 "A" — 5개 면(`leg_right`, `leg_left`, `arch_front`, `arch_right_low`, `arch_right_band`) |
| 심볼 가로세로 비 | {sym_w/sym_h:.3f} : 1 (기준 패널 실측 {sym_w:.1f}×{sym_h:.1f}px) |
| 전체 로고 viewBox | `-453 -107 2119 1525` (SVG 내부 좌표, 심볼 폭 1070 단위) |
| 심볼 viewBox | `12 13 1130 877` |
| 워드마크 | THE AWESOME / TECHNOLOGY × ARTISTRY — 윤곽선 패스(글꼴 의존 없음) |
| 색(글자) | 다크 배경용 #FFFFFF, 라이트 배경용 #000000 |
| 금속 면 색 | 면별 그라디언트로 정의(회색~백색~차콜). 대표: 은백색 약 #F2F2F2, 중간 회색 #9DA3AA 부근, 그림자 #2A2D30 부근 (실측 스톱은 SVG 참조) |
| 배경 | 모든 로고 파일 투명(앱 아이콘 제외) |

## 파일 구성
- `02_SVG` : Symbol_Only, Wordmark_Tagline(검정), Wordmark_Tagline_White(추가), Full_Logo_Light/Dark
- `03_PNG` : 투명 배경. 심볼 1024×1024 캔버스, 전체 로고 가로 3000px
- `04_App_Icons` : 1024 라이트/다크 PNG, Windows.ico(16·24·32·48·64·128·256), macOS.icns(16–1024)
- `05_Illustrator` : PDF(라이트/다크, 순수 벡터·소프트마스크), EPS, Illustrator_Compatible.svg
- `06_Comparison` : 원본 대조 자료

## 앱 아이콘
둥근 사각형(반경 22.5%), 다크=검정 바탕, 라이트=흰 바탕(옅은 회색 테두리). 심볼은 폭 62%로 가운데 배치. 보드의 아이콘 시안을 눈대중으로 맞춘 것이며 원본에서 치수를 측정한 값이 아니다. 보드의 macOS 아이콘은 원형 테두리가 있는 시안이지만 장식 추가를 피하려고 같은 둥근 사각형 아이콘을 사용했다.

## SVG 편집 방법
각 금속 면은 `<g id="면이름">` 아래 2개 path(기본 그라디언트 `g_*`, 직교 오버레이 `h_*`)로 되어 있다. 면 모양은 두 path가 같은 `d`를 쓰므로 함께 수정해야 한다. 글자는 글자별 path(`THE_AWESOME_n`, `TECHNOLOGY_ARTISTRY_n`).
'''
open(P('07_Documentation','Logo_Specifications.md'),'w',encoding='utf-8').write(spec)

diffs=f'''# 벡터화 차이점 (Vectorization_Differences)

**이 벡터는 원본과 "완전히 동일"하지 않다.** 아래는 실제 비교로 측정된 값과 알려진 차이다. 비교 이미지는 `06_Comparison`.

## 측정값 (원본 패널 600×460과 같은 크기로 렌더링해 비교, 배경은 비교용으로만 #0F0F0F)
- 심볼 외곽선(밝기 30 임계 윤곽) 거리: 평균 {M['symbol_outline_distance']['mean_px']} px, 95% {M['symbol_outline_distance']['p95_px']} px, 최대 {M['symbol_outline_distance']['max_px']} px
- 심볼 영역 일치도(밝기 60 임계 IoU): {M['symbol']['IoU_threshold_60']}
- 심볼 평균 절대 밝기 오차(0–255): 영역 전체 {M['symbol']['mean_abs_diff_all']}, 합집합 내부 {M['symbol']['mean_abs_diff_inside_union']}
- 사분면별 평균 오차: {fs(M['symbol_quadrant_mean_abs_diff'])}
- 제목 글자 윤곽 거리: 평균 {M['wordmark_outline_distance']['mean_px']} px, 최대 {M['wordmark_outline_distance']['max_px']} px; 글자 IoU {M['wordmark']['IoU_threshold_128']}
- 태그라인 글자 IoU {M['tagline']['IoU_threshold_128']} (글자 두께 약 2px로 작아서 오차 비율이 큼)

## 알려진 차이
1. **원본 해상도 한계**: 기준 심볼 폭이 약 {sym_w:.0f}px뿐이라 경계 위치는 ±1px 내외로만 맞출 수 있고, 원본의 미세한 곡률은 확인할 수 없다.
2. **그라디언트는 근사**: 면마다 선형 그라디언트 1장 + 직교 방향 투명도 오버레이 1장으로 표현했다. 원본의 2차원 반사 분포(특히 왼쪽 다리 아래 회색 띠, 오른쪽 다리 상단 어두운 부분, 아치 중앙의 어두운 띠)는 완전히 재현되지 않는다. 비교 이미지의 Difference_Map에서 밝은 부분이 이 차이다.
3. **가장자리 반사광**: 원본에 보이는 얇은 가장자리 밝은 선(왼쪽 다리 바깥 가장자리, 두 다리가 접히는 선)은 별도 스트로크로 만들지 않았다. 면 그라디언트에 포함된 정도만 반영된다.
4. **아치 오른쪽 띠(`arch_right_band`)의 아래 경계**는 원본에서 흐릿해 추정 곡선이다.
5. **글자**: 원본 글꼴을 알 수 없어 윤곽을 추적했다. 원본이 약 3px 두께의 래스터라서 곡선 글자(S, O, C, G, R 등)는 약간 불규칙할 수 있고, 모서리는 직선 맞춤으로 선명하게 보정했다(직선 글자만). 태그라인 'A'는 직선 보정이 불안정해 곡선 맞춤 그대로 두었다.
6. **라이트 배경 시안**은 기준 패널과 비율이 달라(위 분석 보고서 0항) 다크 패널 기하를 공통으로 사용했다. 라이트 시안의 심볼 음영도 별도로 맞추지 않았다.
7. **원본 배경**은 순수 검정이 아니라 약 RGB(15,15,15)였지만, 납품 파일은 투명이다.

## 파일별 검증 상태
| 파일 | 상태 |
|---|---|
| SVG 5종 | resvg로 렌더링 확인(원본 대조 포함). 내부에 `<image>` 없음 |
| PNG, ICO, ICNS | 생성 후 다시 열어 크기 목록 확인(ICO: 16–256, ICNS: 16–1024). ICO는 형식상 256px까지 |
| PDF 2종 | PyMuPDF로 다시 열어 렌더링 확인, 포함 이미지 0개 = 벡터 {fs(VO['Full_Logo_Light.pdf'])} |
| Illustrator_Compatible.svg | 형식은 일반 SVG 1.1 구조. **실제 Adobe Illustrator에서 열어보지 못했다**(이 PC에 없음) |
| Full_Logo.eps | **렌더링 검증 못 함**(PostScript 인터프리터 없음). 투명도를 지원하지 않아 직교 오버레이 층이 빠진 **축소 충실도** 버전(면 기본 그라디언트 + 글자). PDF/SVG보다 음영이 단순함 |
| .ai | 네이티브 AI 파일은 만들 수 없어 PDF와 SVG를 제공. Illustrator에서 열어 AI로 저장하면 된다 |
'''
open(P('07_Documentation','Vectorization_Differences.md'),'w',encoding='utf-8').write(diffs)

readme=f'''# The Awesome 로고 패키지

기존 로고를 **새로 디자인하지 않고** 첨부된 브랜드 시안 보드의 다크 패널에서 추적·재현한 벡터 파일 세트입니다. 원본과 100% 동일하다고 주장하지 않습니다 — 측정된 차이는 `07_Documentation/Vectorization_Differences.md`와 `06_Comparison`에 있습니다.

## 폴더
```
The_Awesome_Logo_Final/
 01_Original_Reference/  원본 보드, 사용한 기준 패널, 분석 보고서, 기준 도면
 02_SVG/                 Symbol_Only, Wordmark_Tagline(+White), Full_Logo_Light, Full_Logo_Dark
 03_PNG/                 투명 PNG (심볼 1024, 전체 로고 3000px 폭)
 04_App_Icons/           AppIcon_Light/Dark_1024.png, Windows.ico, macOS.icns
 05_Illustrator/         Full_Logo_Light/Dark.pdf, Full_Logo.eps, Illustrator_Compatible.svg
 06_Comparison/          Original, Vector_Render, Overlay, Difference_Map 등
 07_Documentation/       이 문서, 규격, 차이점
```

## 사용 요령
- 밝은 배경 → `Full_Logo_Light`(검정 글자), 어두운 배경 → `Full_Logo_Dark`(흰 글자). 심볼만 쓸 때는 `Symbol_Only`.
- SVG는 순수 벡터(PNG 삽입 없음). 면별 `<g id>`로 편집할 수 있습니다.
- Illustrator: `Full_Logo_*.pdf`를 여는 것을 권장합니다(그라디언트와 투명 마스크가 유지됨). AI로 저장하려면 열어서 "다른 이름으로 저장 → .ai".
- `Full_Logo.eps`는 단순화된 음영입니다(투명도 미지원). 가능하면 PDF/SVG를 쓰세요.

## 알려둘 점
- 기준 이미지는 심볼 폭 약 {sym_w:.0f}px의 보드 시안이라 정밀도에 한계가 있습니다(외곽선 평균 오차 {M['symbol_outline_distance']['mean_px']}px, 음영은 근사).
- 더 큰 원본이 있으면 같은 스크립트 방식으로 다시 맞추면 정밀도가 크게 올라갑니다.
- 파일 생성에 사용한 도구: Python 3.15, OpenCV, NumPy, SciPy, Pillow, resvg, PyMuPDF.
'''
open(P('07_Documentation','README_Korean.md'),'w',encoding='utf-8').write(readme)

# ---- zip ----
zp='The_Awesome_Logo_Final.zip'
if os.path.exists(zp): os.remove(zp)
with zipfile.ZipFile(zp,'w',zipfile.ZIP_DEFLATED) as z:
    for dp,_,fns in os.walk(ROOT):
        for fn in fns:
            full=os.path.join(dp,fn); z.write(full,os.path.relpath(full,'.'))
print('zip',os.path.getsize(zp))
