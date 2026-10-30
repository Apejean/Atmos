import sys, os, json, shutil, zipfile, re, glob
sys.path.insert(0,'work')
import numpy as np, cv2
from PIL import Image
import pipe
from pipe import ORDER

ROOT='The_Awesome_Logo_Final'
def P(*a): return os.path.join(ROOT,*a)
M=json.load(open('work/metrics2.json')); VO=json.load(open('work/verify_outputs.json')); GEO=json.load(open('work/geo2.json'))

# ---- 01 originals + analysis drawings ----
BOARD=r"C:/Users/USER/AppData/Local/Temp/claude/C--Users-USER-AppData-Local-Claude-Data-claude-pzs8sxrjxfjjc-scratch-workspaces-5ac7278c-2ac4-4e46-b5fe-c62ad6c72539-f6ebfe5f-de5d-4ccf-a05e-89d6cb5b7a1c-scratch-2026-10-30-49f901/b10274b4-537c-489c-9636-091feff359c0/images/1.webp"
try: Image.open(BOARD).convert('RGB').save(P('01_Original_Reference','Original_Board_First_Reference.png'))
except Exception as e: print('board copy skipped',e)
shutil.copy('work/new_dark.png',P('01_Original_Reference','Original_Dark_2000px.png'))
shutil.copy('work/new_light.png',P('01_Original_Reference','Original_Light_2000px.png'))

R={}
cols={'leg_right':(255,80,80),'leg_left':(80,255,80),'arch_front':(90,140,255),'arch_right_low':(255,255,0),'arch_right_band':(255,0,255)}
for nm in ('dark','light'):
    v,G,faces,fits,wm=pipe.run(nm); R[nm]=(v,G,faces,fits,wm)
    ov=v.frame.copy()
    for k,p in faces.items(): cv2.polylines(ov,[p.poly().round().astype(np.int32)],True,cols[k],2,cv2.LINE_AA)
    for name in ('TL','A_apex','inner_top','BLc','BRL','BRR'):
        p=tuple(int(round(x)) for x in G[name]); cv2.circle(ov,p,7,(255,160,0),-1); cv2.putText(ov,'%s (%d,%d)'%(name,p[0],p[1]),(p[0]+10,p[1]-10),0,0.6,(255,160,0),2,cv2.LINE_AA)
    x0,y0,w,h=[int(x) for x in M[nm]['symbol_source_bbox']]
    Image.fromarray(ov[100:1500,300:1800]).save(P('01_Original_Reference','Analysis_Reference_Drawing_%s.png'%nm.capitalize()))

def wm_metrics(wm):
    hb=np.array(wm['head']['bbox']); tb=np.array(wm['tag']['bbox'])
    def one(bb):
        w=bb[:,1,0]-bb[:,0,0]; gaps=bb[1:,0,0]-bb[:-1,1,0]; cap=float(np.median(bb[:,1,1]-bb[:,0,1])); return w,gaps,cap,bb[0,0,0],bb[-1,1,0],bb[:,0,1].mean(),bb[:,1,1].mean()
    return one(hb),one(tb)
L=lambda xs:', '.join('%.0f'%x for x in xs)
def blk(nm):
    v,G,faces,fits,wm=R[nm]; (hw,hg,hc,hx0,hx1,hy0,hy1),(tw,tg,tc,tx0,tx1,ty0,ty1)=wm_metrics(wm); sb=np.array(GEO[nm]['symbb'])
    sw=sb[1][0]-sb[0][0]; sh=sb[1][1]-sb[0][1]; scx=(sb[0][0]+sb[1][0])/2
    info=wm['info']
    return dict(sw=sw,sh=sh,asp=sw/sh,hw=hw,hg=hg,hc=hc,hx0=hx0 if False else hx0,hx1=hx1,hy0=hy0,hy1=hy1,tw=tw,tg=tg,tc=tc,tx0=tx0,tx1=tx1,ty0=ty0,ty1=ty1,scx=scx,info=info,sy1=sb[1][1],v=v,G=G)
B={nm:blk(nm) for nm in R}
d,l=B['dark'],B['light']
def row(nm,b):
    x=M[nm]; v=b['v']
    return f"| {nm} | {x['source_size_px'][0]}×{x['source_size_px'][1]} | {x['symbol_source_bbox'][2]}×{x['symbol_source_bbox'][3]} ({x['symbol_source_bbox'][2]/x['symbol_source_bbox'][3]:.3f}) | {x['frame_scale']} | {b['sw']:.0f}×{b['sh']:.0f} |"

rep=f'''# 원본 분석 보고서 (The Awesome 로고)

## 0. 기준 이미지
- 이번 작업의 기준은 새로 받은 **2000×1399px 이미지 2장**이다(`Original_Dark_2000px.png`, `Original_Light_2000px.png`). 처음에 받은 시안 보드(`Original_Board_First_Reference.png`)는 심볼 폭이 약 268px라서 참고용으로만 보관한다.
- 요청서의 `reference/logo_original.png`는 제공되지 않았다.
- 두 이미지는 **같은 로고의 두 버전이지만 서로 동일한 도형이 아니다**. 아래 표처럼 가로세로 비와 음영, 안쪽 삼각 음각의 꼭짓점 위치, 글자 폭이 서로 다르다. 그래서 **다크/라이트를 각각 따로 추적**했다. 즉 Full_Logo_Dark는 다크 이미지에서, Full_Logo_Light는 라이트 이미지에서 만들었다.
- 두 이미지 모두 가장자리가 부드럽다(작은 이미지를 키운 흔적). 이 이상의 정밀도는 원본에서 얻을 수 없다.

## 1. 측정 (도구: Python 3.15, OpenCV 5.0, NumPy, SciPy, Shapely)
| 버전 | 원본 크기 | 원본에서 심볼 외곽 (가로비) | 작업 좌표 배율 | 작업 좌표 심볼 크기 |
|---|---|---|---|---|
{row('dark',d)}
{row('light',l)}

작업 좌표는 심볼 폭이 약 1070이 되도록 원본을 균일 축소(배율 위 표)한 좌표계다. 좌표 원점 위치만 다르고 형태는 바뀌지 않는다.

### 심볼 구조 (다크/라이트 공통)
앞뒤 순서(뒤→앞): `leg_right` → `leg_left` → `arch_front` → `arch_right_low` → `arch_right_band`.
- `leg_right` : 오른쪽 다리 전체. 안쪽 가장자리는 **안쪽 삼각 음각의 꼭짓점에서 오른쪽 발끝까지 하나의 직선**이다.
- `leg_left` : 왼쪽 다리. 오른쪽 가장자리는 상단 꼭짓점에서 안쪽 삼각 꼭짓점까지, 거기서 아치 뒤로 이어진다.
- `arch_front` : 곡선 리본 앞면. 왼쪽 다리 앞을 지나 오른쪽 다리의 안쪽 직선에서 끝난다.
- `arch_right_low` / `arch_right_band` : 아치가 오른쪽 다리 앞으로 이어지는 부분(아래 면, 위쪽 어두운 띠).
- 중앙 삼각형 음각은 두 다리와 아치 사이의 **빈 공간(투명)** 이다.

### 주요 꼭짓점 (작업 좌표)
| 점 | 다크 | 라이트 |
|---|---|---|
| 상단 왼쪽 | {np.round(d['G']['TL']).astype(int).tolist()} | {np.round(l['G']['TL']).astype(int).tolist()} |
| 상단 오른쪽(정점) | {np.round(d['G']['A_apex']).astype(int).tolist()} | {np.round(l['G']['A_apex']).astype(int).tolist()} |
| 안쪽 삼각 꼭짓점 | {np.round(d['G']['inner_top']).astype(int).tolist()} | {np.round(l['G']['inner_top']).astype(int).tolist()} |
| 왼쪽 아래 끝 | {np.round(d['G']['BLc']).astype(int).tolist()} | {np.round(l['G']['BLc']).astype(int).tolist()} |
| 오른쪽 아래 끝(바깥/안쪽) | {np.round(d['G']['BRL']).astype(int).tolist()} / {np.round(d['G']['BRR']).astype(int).tolist()} | {np.round(l['G']['BRL']).astype(int).tolist()} / {np.round(l['G']['BRR']).astype(int).tolist()} |

기준 도면: `Analysis_Reference_Drawing_Dark.png`, `Analysis_Reference_Drawing_Light.png` (면 윤곽과 꼭짓점 좌표 표시).

### 금속 명암
면마다 (1) 면 안 밝기 회귀로 정한 주 방향의 선형 그라디언트(12스톱) + (2) 직교 방향의 흑/백 반투명 오버레이(12스톱)로 표현했다. 스톱 수(기본+오버레이): {', '.join(f"{k} {a}+{b}" for k,(a,b) in GEO['dark']['fits'].items())}.

## 2. 워드마크 측정 (작업 좌표, 다크 / 라이트)
| 항목 | 다크 | 라이트 |
|---|---|---|
| THE AWESOME 폭 | {d['hx1']-d['hx0']:.0f} | {l['hx1']-l['hx0']:.0f} |
| 대문자 높이 | {d['hc']:.0f} | {l['hc']:.0f} |
| 획 두께(측정) | {d['info']['w_head']:.1f} | {l['info']['w_head']:.1f} |
| 글자 폭 | {L(d['hw'])} | {L(l['hw'])} |
| 글자 사이 간격 | {L(d['hg'])} | {L(l['hg'])} |
| TECHNOLOGY × ARTISTRY 폭 | {d['tx1']-d['tx0']:.0f} | {l['tx1']-l['tx0']:.0f} |
| 태그라인 대문자 높이 / 획 두께 | {d['tc']:.0f} / {d['info']['w_tag']:.1f} | {l['tc']:.0f} / {l['info']['w_tag']:.1f} |
| 태그라인/제목 크기비 | {d['info']['scale']:.3f} | {l['info']['scale']:.3f} |
| 심볼 중심 x / 제목 중심 x | {d['scx']:.0f} / {(d['hx0']+d['hx1'])/2:.0f} | {l['scx']:.0f} / {(l['hx0']+l['hx1'])/2:.0f} |
| 심볼 아래 → 제목 윗선 | {d['hy0']-d['sy1']:.0f} | {l['hy0']-l['sy1']:.0f} |
| 제목 아랫선 → 태그라인 윗선 | {d['ty0']-d['hy1']:.0f} | {l['ty0']-l['hy1']:.0f} |

## 3. 벡터화 방법
1. **심볼 외곽 직선**: 법선 방향 최대 기울기로 가장자리 위치를 잡고(스냅) 직선 맞춤. 코너는 인접 직선의 교점으로 계산해 모서리가 정확히 만난다.
2. **아치 곡선**: 스냅한 가장자리 점들에 강건한 다항식(4차/3차)을 맞추어 매끈한 호로 바꾼 뒤 제어점 8~13개의 Bézier로 만들었다(점 단위 떨림 제거). 곡선과 직선은 교점에서 정확히 만난다.
3. **경계 겹침**: 앞면이 뒷면을 약간 덮도록 이어 붙여 면 사이의 틈(검은 실선)이 생기지 않게 했다.
4. **글자**: 8배/3배 확대한 윤곽을 추적한 뒤 **획이 직선인 글자는 직선 맞춤+교점 모서리, O는 타원 맞춤, 나머지 곡선은 평활화+Bézier 맞춤**으로 정리했다. 제목의 A·W·M, 태그라인의 N·L·I·Y·×·C·G·R은 추적 윤곽이 번져서 지저분해 **측정한 획 두께와 글자 상자로 직선·원호 획을 다시 구성**했다(아래 차이점 문서 참고).
'''
open(P('01_Original_Reference','Original_Analysis_Report.md'),'w',encoding='utf-8').write(rep)

def mt(nm):
    x=M[nm]; o=x['symbol_outline_distance']
    return f"""- 심볼 외곽선: 벡터 외곽선에서 법선 방향 ±8px 안의 가장 강한 원본 가장자리까지 거리 — 평균 **{o['mean_px']}px**, 95% {o['p95_px']}px, 최대 {o['max_px']}px (측정 점의 {o['points_used_pct']}%; 심볼 폭 약 1070 기준)
- 심볼 평균 절대 밝기 오차(0–255): {x['symbol']['mean_abs_diff']}
- 제목: 글자 윤곽 평균 거리 {x['headline']['outline_distance']['mean_px']}px (95% {x['headline']['outline_distance']['p95_px']}px), 영역 일치도(IoU) {x['headline']['IoU']}
- 태그라인: 글자 윤곽 평균 거리 {x['tagline']['outline_distance']['mean_px']}px (95% {x['tagline']['outline_distance']['p95_px']}px), IoU {x['tagline']['IoU']}"""
diffs=f'''# 벡터화 차이점 (Vectorization_Differences)

**이 벡터는 원본과 "완전히 동일"하지 않다. 특히 이번 버전은 요청에 따라 심볼을 황금비로 바꿨기 때문에 원본 이미지의 심볼과 모양이 의도적으로 다르다**(폭 약 +22~28%, 정점 위치·안쪽 삼각 꼭짓점·아치 높이 이동; 자세한 내용은 `Golden_Ratio_Analysis.md`). 원본 비율 그대로의 이전 결과는 `The_Awesome_Logo_Final_v1_원본비율.zip`에 있다. 그래서 `06_Comparison`의 원본과의 비교(차이 지도 등)에는 이 의도적 변경이 그대로 차이로 나타나며, 아래 '심볼 외곽선' 수치는 **황금비 조정을 가한 기준 이미지**와의 거리다. 실제 렌더링을 원본과 같은 크기로 겹쳐 비교한 값과 알려진 차이를 적는다. 비교 이미지는 `06_Comparison`.

## 측정값
### 다크 (기준: Original_Dark_2000px.png)
{mt('dark')}

### 라이트 (기준: Original_Light_2000px.png)
{mt('light')}

주의: 원본의 가장자리는 약 3px 번져 있고 글자 주변에 빛 번짐(광륜)이 있다. 벡터는 일부러 그 번짐 가운데의 선명한 선에 맞추었기 때문에 글자 IoU 수치는 낮게 나온다(특히 가는 태그라인). 수치보다 `Outline_Comparison*.png`(빨강=원본, 하늘색=벡터)를 같이 보는 편이 정확하다.

## 가장자리를 깔끔하게 한 부분 (의도적인 차이)
- 원본의 번진 가장자리를 그대로 따라가지 않고 **직선은 직선, 곡선은 매끈한 호**로 정리했다. 원본의 미세한 요철/번짐은 재현하지 않는다.
- 제목의 **T H E**는 추적 윤곽을 직선 맞춤한 것이고, **A W M**은 측정한 획 두께로 직선 획을 다시 구성했다. 그래서 원본 글자의 모서리 둥글림(번짐으로 생긴 것으로 보임)이 없다.
- 태그라인은 원본 획이 약 {d['info']['w_tag']:.1f}px로 너무 가늘어 추적이 불안정했다. T·E·H·O·A·S는 제목 글자를 태그라인 크기로 줄여 쓰고(같은 글꼴로 보임), N·L·I·Y·×·C·G·R은 측정한 획 두께와 글자 상자로 구성했다. **이 글자들의 세부 모양(예: G의 가로획 길이, R의 다리 각도)은 원본에서 직접 추적한 것이 아니라 추정**이다.
- 반복되는 E(제목 3개)는 하나의 윤곽을 공유한다.

## 그 밖의 알려진 차이
1. **그라디언트는 근사**: 면마다 선형 그라디언트 1장 + 직교 방향 투명도 오버레이 1장이다. 원본의 2차원 반사(왼쪽 다리 아래의 어두운 띠, 아치 중앙의 어두운 띠 등)는 완전히 재현되지 않는다. `Difference_Map*.png`의 밝은 부분이 해당 차이다.
2. **가장자리 반사광**(얇은 밝은 선)은 별도 스트로크로 만들지 않았다.
3. **아치 오른쪽 띠의 아래 경계**는 원본에서 흐려 추정한 곡선이다.
4. **다크와 라이트가 서로 다른 도형**이라 두 버전을 각각 추적했다. 앱 아이콘의 다크는 다크 심볼, 라이트는 라이트 심볼을 쓴다. `Symbol_Only.svg`는 다크 버전, `Symbol_Only_Light.svg`는 라이트 버전이다.
5. 원본 배경은 순수 검정/흰색이 아니라 약간의 명암 변화가 있지만 납품 파일은 투명이다(앱 아이콘 제외).

## 파일별 검증 상태
| 파일 | 상태 |
|---|---|
| SVG 6종 | resvg로 렌더링 확인(원본 대조 포함). 내부에 `<image>` 없음 |
| PNG, ICO, ICNS | 생성 후 다시 열어 크기 목록 확인(ICO 16–256, ICNS 16–1024). ICO는 형식상 256px까지 |
| PDF 2종 | PyMuPDF로 다시 열어 렌더링 확인, 포함 이미지 0개(순수 벡터): {json.dumps(VO,ensure_ascii=False)} |
| Illustrator_Compatible.svg | 일반 SVG 1.1 구조. **실제 Adobe Illustrator에서 열어보지 못했다**(이 PC에 없음) |
| Full_Logo.eps | **렌더링 검증 못 함**(PostScript 인터프리터 없음). 투명도를 지원하지 않아 오버레이 층이 빠진 **축소 충실도** 버전(라이트 버전 기준). PDF/SVG를 권장 |
| .ai | 네이티브 AI 파일은 만들 수 없어 PDF와 SVG를 제공. Illustrator에서 열어 AI로 저장 |
'''
open(P('07_Documentation','Vectorization_Differences.md'),'w',encoding='utf-8').write(diffs)

spec=f'''# 로고 규격 (Logo_Specifications)

| 항목 | 값 |
|---|---|
| 심볼 | 은색 금속 리본 "A" — 5개 면(`leg_right`, `leg_left`, `arch_front`, `arch_right_low`, `arch_right_band`) |
| 심볼 가로세로 비 | 다크 {d['asp']:.3f} : 1, 라이트 {l['asp']:.3f} : 1 (원본 이미지 2장이 서로 다름) |
| 전체 로고 viewBox | 다크 `{' '.join(str(x) for x in GEO['dark']['full_vb'])}`, 라이트 `{' '.join(str(x) for x in GEO['light']['full_vb'])}` |
| 워드마크 | THE AWESOME / TECHNOLOGY × ARTISTRY — 윤곽선 패스(글꼴 의존 없음) |
| 글자색 | 다크 배경용 #FFFFFF, 라이트 배경용 #000000 |
| 금속 면 색 | 면별 그라디언트(회색~백색~차콜). SVG의 `linearGradient g_*`(기본), `h_*`(오버레이) 참조 |
| 배경 | 모든 로고 파일 투명(앱 아이콘 제외) |

## 파일
- `02_SVG` : Symbol_Only(다크 버전), Symbol_Only_Light(추가), Wordmark_Tagline(검정), Wordmark_Tagline_White(추가), Full_Logo_Light, Full_Logo_Dark
- `03_PNG` : 투명 배경. 심볼 1024×1024 캔버스(다크 버전 심볼), 전체 로고 가로 3000px
- `04_App_Icons` : Windows.ico(정사각 꽉 채움), macOS.icns(표준 여백·그림자), 각 1024 PNG, 라이트/다크 둥근 PNG
- `05_Illustrator` : PDF(라이트/다크, 순수 벡터·소프트마스크), EPS(축소 충실도), Illustrator_Compatible.svg
- `06_Comparison` : 원본 대조(다크 기준은 접미사 없음, 라이트는 `_Light`)
- `07_Documentation/source_scripts` : 제작에 쓴 Python 스크립트(재현용)

## 앱 아이콘 (04_App_Icons)
- `Windows.ico` / `AppIcon_Windows_Square_1024.png` : **모서리까지 꽉 채운 검정 정사각형** 바탕에 다크 심볼(폭 66%). 16·24·32·48·64·128·256px 포함(ICO 형식 한계로 256px까지).
- `macOS.icns` / `AppIcon_macOS_1024.png` : macOS 표준 형태. 1024 캔버스에 824px 둥근 사각형(여백 100px, 모서리 반경 185)과 부드러운 그림자, 다크 심볼(타일 폭의 62%). 16–1024px 포함.
- `AppIcon_Dark_1024.png`, `AppIcon_Light_1024.png` : 이전 둥근 사각형 PNG(참고용).
치수(심볼 비율, 둥근 정도, 그림자)는 Apple의 일반적인 아이콘 템플릿을 기준으로 눈대중으로 맞춘 값이며 원본에서 측정한 값이 아니다.

## SVG 편집
각 금속 면은 `<g id="면이름">` 아래 두 path(기본 그라디언트, 오버레이)로 되어 있고 같은 `d`를 쓰므로 모양을 바꿀 때 함께 수정한다. 글자는 글자별 path(`THE_AWESOME_n`, `TECHNOLOGY_ARTISTRY_n`).
'''
open(P('07_Documentation','Logo_Specifications.md'),'w',encoding='utf-8').write(spec)

readme=f'''# The Awesome 로고 패키지

제공받은 2000px 원본 이미지(다크/라이트)에서 추적한 벡터 세트이며, **심볼은 요청에 따라 황금비로 조정**했습니다(외곽 가로÷세로 1.618, 정점 가로 위치 0.618, 안쪽 삼각 꼭짓점 높이 0.764, 아치 정점 높이 0.472). 원본 비율 그대로의 이전 결과는 `The_Awesome_Logo_Final_v1_원본비율.zip`입니다. 원본과 100% 동일하다고 주장하지 않습니다. 측정된 차이와 의도적으로 정리한 부분은 `07_Documentation/Vectorization_Differences.md`, 원본 대조는 `06_Comparison`에 있습니다.

## 폴더
```
The_Awesome_Logo_Final/
 01_Original_Reference/  원본 이미지, 분석 보고서, 기준 도면
 02_SVG/                 Symbol_Only, Wordmark_Tagline, Full_Logo_Light, Full_Logo_Dark (+추가 변형)
 03_PNG/                 투명 PNG
 04_App_Icons/           Windows.ico(정사각), macOS.icns(표준 여백), 1024 PNG들
 05_Illustrator/         Full_Logo_Light/Dark.pdf, Full_Logo.eps, Illustrator_Compatible.svg
 06_Comparison/          원본/벡터/겹침/차이/외곽선 비교
 07_Documentation/       문서와 제작 스크립트
```

## 사용 요령
- 밝은 배경 → `Full_Logo_Light`, 어두운 배경 → `Full_Logo_Dark`. 이 둘은 원본 이미지가 서로 다른 도형이라 **각각 따로 추적**한 것입니다.
- SVG는 순수 벡터(PNG 삽입 없음). 면별 `<g id>`로 편집할 수 있습니다.
- Illustrator: `Full_Logo_*.pdf`를 여는 것을 권장합니다(그라디언트와 투명 마스크 유지). `.ai`로 저장하려면 열어서 "다른 이름으로 저장".
- `Full_Logo.eps`는 음영이 단순한 축소 버전입니다(투명도 미지원, 렌더링 미검증).

## 알려둘 점
- 원본이 약간 흐려서 가장자리 위치 오차는 약 {M['dark']['symbol_outline_distance']['mean_px']}~{M['light']['symbol_outline_distance']['mean_px']}px(작업 좌표, 심볼 폭 1070 기준)입니다. 음영은 근사입니다.
- 가장자리는 의도적으로 직선/호로 정리했고, 일부 글자(A·W·M, 태그라인의 N·L·I·Y·×·C·G·R)는 측정값으로 다시 구성했습니다. 자세한 내용은 차이점 문서를 보세요.
- 제작 도구: Python 3.15, OpenCV, NumPy, SciPy, Shapely, Pillow, resvg, PyMuPDF.
'''
open(P('07_Documentation','README_Korean.md'),'w',encoding='utf-8').write(readme)

os.makedirs(P('07_Documentation','source_scripts'),exist_ok=True)
for fn in ('pipe','tagfix','snap','geom','bezfit','sym2','logo','pdfw','eps','build2','icons2','docs2'):
    shutil.copy('work/%s.py'%fn,P('07_Documentation','source_scripts',fn+'.py'))

zp='The_Awesome_Logo_Final.zip'
if os.path.exists(zp): os.remove(zp)
with zipfile.ZipFile(zp,'w',zipfile.ZIP_DEFLATED) as z:
    for dp,_,fns in os.walk(ROOT):
        for fn in fns:
            full=os.path.join(dp,fn); z.write(full,os.path.relpath(full,'.'))
print('zip',os.path.getsize(zp))
