import sys, os, json, zipfile, shutil
sys.path.insert(0,'work')
import numpy as np
import golden_design as gd, goldbuild

ROOT='The_Awesome_Logo_Final'; P=lambda *a: os.path.join(ROOT,*a)
PHI=gd.PHI; D=gd.design(); H=D['H']; W=D['W']
L={}
for nm in ('dark','light'):
    v,G,faces,fits,wm=goldbuild.run(nm); i=wm['info']
    hb=np.array(wm['head']['bbox']); tb=np.array(wm['tag']['bbox'])
    L[nm]=dict(u=G['u'],Ws=G['Wsym'],Hs=G['Hsym'],hw=hb[:,1,0].max()-hb[:,0,0].min(),tw=tb[:,1,0].max()-tb[:,0,0].min(),
               cap=i['cap'],tcap=i['tag_cap'],g1=i['gap1'],g2=i['gap2'],clear=G['clear'])
d=L['dark']
f2=lambda x:'%.2f'%x
pt=lambda p:'(%.2f, %.2f)'%(p[0],H-p[1])

spec=f'''# 황금비 설계 명세 (Golden_Design_Specification)

이 버전의 심볼은 원본 이미지를 따라 그린 것이 아니라, **황금비(φ = 1.618)와 피보나치 수(1, 2, 3, 5, 8, 13, 21, 34)로 정한 치수에서 직선·타원의 교점으로 작도**한 것입니다. 황금나선이나 피보나치 나선은 쓰지 않았습니다. 원본 심볼의 비율(가로÷세로 1.33)과 인상은 유지했습니다.

- 설계도: `Construction_Symbol.png/svg`(심볼 작도), `Construction_Full_Logo.png/svg`(심볼·워드마크 비례)
- 단위: **u = 심볼 높이 ÷ 26**. 아래 좌표는 (왼쪽 아래 모서리에서의 가로, 바닥에서의 높이)이고 단위는 u입니다.
- 실제 크기: 납품 파일에서 1u = {f2(d['u'])} (SVG 좌표 단위). 비율로 정의되므로 어떤 크기로 확대·축소해도 관계는 같습니다.

## 1. 기본 틀
| 항목 | 값 | 규칙 |
|---|---|---|
| 심볼 틀 | 34u × 26u (가로÷세로 {W/H:.3f}) | 34 = 피보나치, 26 = 2×13. 34:26 = φ²:2 (원본 1.331과 1.7% 차이) |
| 정점 A의 가로 위치 | {f2(D['A'][0])}u = 34u ÷ φ | 폭의 황금분할점 |
| 황금분할 보조선 | 가로 13u·21u, 높이 26/φ² = {f2(H/PHI**2)}u · 26/φ = {f2(H/PHI)}u | 설계도의 금색 선 |

## 2. 획의 두께
| 획 | 값 | 규칙 |
|---|---|---|
| 왼쪽·오른쪽 다리 가로 폭 | 둘 다 {f2(D['w'])}u (= 13/2) | **같은 높이에서 좌우 가로 폭이 같음.** 안쪽 삼각 꼭짓점이 황금 높이(아래 표)에 오도록 정한 값 |
| 다리 두께(획에 수직) | 왼쪽 {f2(D['t_L'])}u / 오른쪽 {f2(D['t_right'])}u | 기울기가 달라 수직 두께만 약간 다름 |
| 아치 리본 두께(정점) | {f2(D['t_crest'])}u | = 다리 가로 폭 ÷ φ |

## 3. 주요 점 (교점으로 결정)
| 점 | 좌표 (u) | 정의 |
|---|---|---|
| A 정점 | {pt(D['A'])} | 34/φ, 꼭대기 |
| TL 왼쪽 위 모서리 | {pt(D['TL'])} | 윗변(1u 오를 때 8u: 피보나치 1:8) × 왼쪽 바깥선 |
| P_hole 안쪽 삼각 꼭짓점 | {pt(D['inner'])} | 접힘선 × 오른쪽 다리 안쪽선, 높이 {D['h_hole']/H:.3f}H = 1 − 1/φ³ |
| 아치 정점 | {pt((D['xc'],H-D['ch']))} | 높이 = 안쪽 삼각 꼭짓점 높이 ÷ φ |
| E 리본 끝 | {pt(D['E'])} | 오른쪽 바깥선 위, 바닥에서 5u (피보나치) |
| F 아치 발 안쪽 | {pt(D['F'])} | 아래 타원이 바닥과 만나는 점(정점 두께 규칙에서 결정) |
| BRR 오른쪽 발 안쪽 | {pt(D['BRR'])} | 34 − 다리 폭 |
| pa / lb 아치 × 오른쪽 다리 안쪽선 | {pt(D['pa'])} / {pt(D['lb'])} | 교점 |

## 4. 곡선 (황금 타원)
- 아치의 위·아래 가장자리는 **크기가 같은 황금 타원 두 개**입니다(세로 반지름 = 가로 반지름 × φ; 가로 {f2(D['ell1'][1])}u, 세로 {f2(D['ell1'][2])}u). 중심만 다릅니다.
- 위 가장자리: 왼쪽 발 모서리와 리본 끝 E를 지나고, 정점 높이 = 안쪽 삼각 꼭짓점 높이 ÷ φ.
- 아래 가장자리: 같은 타원을 옮겨 **같은 점 E를 지나게** 했습니다. 그래서 두 곡선이 E에서 정확히 만나 리본이 한 점으로 모이고, 오른쪽 다리를 지나는 앞뒤 부분이 끊김 없이 한 곡선으로 이어집니다.
- 아래 타원의 위치(아치 발 F)는 정점에서의 리본 두께가 다리 가로 폭 ÷ φ가 되도록 정했습니다.
- 윗변(파란 선)은 직선, 접힘선(초록 선)은 왼쪽 다리 바깥선과 평행한 직선입니다.

## 5. 심볼과 워드마크의 비례 (다크 기준 실제 치수, 라이트도 같은 규칙)
| 관계 | 값 | 규칙 |
|---|---|---|
| 제목 폭 ÷ 심볼 폭 | {d['hw']:.0f} ÷ {d['Ws']:.0f} = {d['hw']/d['Ws']:.3f} | φ |
| 제목 폭 ÷ (심볼 꼭대기~제목 아래) | {d['hw']:.0f} ÷ {d['Hs']+d['g1']+d['cap']:.0f} = {d['hw']/(d['Hs']+d['g1']+d['cap']):.3f} | φ (황금사각형 블록) |
| 심볼~제목 간격 ÷ 제목~태그라인 간격 | {d['g1']:.0f} ÷ {d['g2']:.0f} = {d['g1']/d['g2']:.3f} | φ |
| 제목 대문자 높이 ÷ 태그라인 대문자 높이 | {d['cap']:.1f} ÷ {d['tcap']:.1f} = {d['cap']/d['tcap']:.3f} | φ² |
| 여백(클리어 스페이스) | {d['clear']:.0f} | 심볼 높이 ÷ φ³ |

글자 모양과 자간은 다크 원본에서 추적한 것을 쓰고, 크기와 위치만 위 규칙으로 정했습니다. 라이트 버전도 같은 글자·같은 배치를 씁니다(음영과 글자색만 다름).

## 6. 원본에서 가져온 것과 새로 정한 것
- **새로 작도:** 심볼의 모든 윤곽(직선 5종, 타원 2개, 교점), 심볼과 글자의 크기·간격·여백.
- **원본에서 가져옴:** 금속 음영(원본에서 맞춘 그라디언트를 새 면에 맞게 옮김), 글자 윤곽과 자간.
- 원본 대비 변화: 비교 이미지 `06_Comparison/Original_vs_Golden*.png`. 전체 인상은 같지만 다리 두께·아치 곡선·정점 위치 등이 위 규칙에 맞게 바뀌었습니다.
'''
os.makedirs(P('08_Golden_Design'),exist_ok=True)
# 01: original images + the original analysis (unchanged from the traced v1 package)
with zipfile.ZipFile('The_Awesome_Logo_Final_v1_원본비율.zip') as z:
    for n in z.namelist():
        if n.startswith('The_Awesome_Logo_Final/01_Original_Reference/'):
            z.extract(n,'.')
open(P('08_Golden_Design','Golden_Design_Specification.md'),'w',encoding='utf-8').write(spec)

readme=f'''# The Awesome 로고 패키지 (황금비 설계 버전)

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
'''
open(P('07_Documentation','README_Korean.md'),'w',encoding='utf-8').write(readme)

specs=f'''# 로고 규격 (Logo_Specifications)

| 항목 | 값 |
|---|---|
| 심볼 | 은색 금속 리본 "A", 5개 면(`leg_right`, `leg_left`, `arch_front`, `arch_right_low`, `arch_right_band`) |
| 심볼 비율 | 34 : 26 (= φ² : 2 = 1.309) |
| 전체 로고 | 제목 폭 = 심볼 폭 × φ, 블록(심볼 꼭대기~제목 아래)은 황금사각형 |
| 여백 | 심볼 높이 ÷ φ³ (모든 방향) |
| 글자색 | 다크 배경용 #FFFFFF, 라이트 배경용 #000000 |
| 배경 | 다크 버전 #000000, 라이트 버전 #FFFFFF (배경 없는 `_Transparent` 파일도 함께 제공) |

## 앱 아이콘
- `Windows.ico` : 모서리까지 꽉 채운 검정 정사각형, 다크 심볼(폭 66%), 16~256px
- `macOS.icns` : 1024 캔버스에 824px 둥근 사각형(여백 100px)과 그림자, 다크 심볼, 16~1024px
아이콘 안 심볼 크기·둥글기·그림자는 일반적인 앱 아이콘 템플릿에 맞춘 값입니다(황금비 규칙 밖).

## SVG 편집
각 금속 면은 `<g id="면이름">` 아래 두 path(기본 그라디언트, 오버레이)로 되어 있습니다. 글자는 글자별 path입니다.
'''
open(P('07_Documentation','Logo_Specifications.md'),'w',encoding='utf-8').write(specs)

diffs='''# 원본과 달라진 점 (Vectorization_Differences)

## 이 버전에서 의도적으로 바꾼 것
- 심볼 윤곽 전체를 황금비 규칙으로 새로 작도했습니다(`08_Golden_Design` 참고). 원본 이미지의 윤곽을 따라 그린 것이 아니므로 원본과 겹쳐 보면 다릅니다. 비교: `06_Comparison/Original_vs_Golden*.png`.
- 심볼과 글자의 크기·간격·여백을 황금비 규칙으로 다시 정했습니다. 라이트 버전도 다크와 같은 심볼 크기·글자·배치를 씁니다(원본 라이트 이미지는 심볼 비율과 태그라인 크기·자간이 달랐음). 라이트는 음영만 라이트 원본에서 가져왔습니다.

## 원본에서 가져와 근사한 것
- 금속 음영: 원본에서 면마다 맞춘 그라디언트(기본 + 직교 오버레이)를 새 면 모양에 맞게 옮겼습니다. 원본 음영의 근사이며, 면 모양이 바뀐 만큼 반사 위치도 조금 달라집니다.
- 글자: 원본 윤곽을 추적해 정리한 것(직선 글자는 직선 맞춤, O는 타원, 일부 글자는 측정한 획 두께로 재구성)이며 원본 글꼴이 아닙니다.

## 검증 상태
| 파일 | 상태 |
|---|---|
| SVG, PNG | resvg로 렌더링해 눈으로 확인. SVG 안에 이미지 삽입 없음 |
| ICO, ICNS | 다시 열어 크기 목록 확인 |
| PDF | PyMuPDF로 다시 열어 렌더링 확인, 이미지 삽입 없음 |
| EPS | 렌더링 미검증(인터프리터 없음), 투명 오버레이가 빠진 단순 버전 |
| Illustrator | 이 PC에 없어 열어보지 못함. 네이티브 .ai는 만들지 않음 |
'''
open(P('07_Documentation','Vectorization_Differences.md'),'w',encoding='utf-8').write(diffs)
for fn in ('Golden_Ratio_Analysis.md',):
    p=P('07_Documentation',fn)
    if os.path.exists(p): os.remove(p)
os.makedirs(P('07_Documentation','source_scripts'),exist_ok=True)
for fn in ('golden_design','goldbuild','construction','extras','docs3'):
    shutil.copy('work/%s.py'%fn,P('07_Documentation','source_scripts',fn+'.py'))
zp='The_Awesome_Logo_Final.zip'
if os.path.exists(zp): os.remove(zp)
with zipfile.ZipFile(zp,'w',zipfile.ZIP_DEFLATED) as z:
    for dp,_,fns in os.walk(ROOT):
        for fn in fns:
            full=os.path.join(dp,fn); z.write(full,os.path.relpath(full,'.'))
print('zip',os.path.getsize(zp))
print(json.dumps({k:{kk:round(float(vv),2) for kk,vv in v.items()} for k,v in L.items()},indent=1))
