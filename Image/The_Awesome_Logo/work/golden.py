import sys, io, os, json
sys.path.insert(0,'work')
import numpy as np, cv2, resvg_py
from PIL import Image
import pipe, logo
phi=(1+5**.5)/2
ROOT='The_Awesome_Logo_Final'
def spiral_pts(x,y,w,h,n=9):
    pts=[]
    for k in range(n):
        st=k%4
        if st==0: s=h; c=(x+s,y+s); a0,a1=np.pi,1.5*np.pi; x+=s; w-=s
        elif st==1: s=w; c=(x,y+s); a0,a1=1.5*np.pi,2*np.pi; y+=s; h-=s
        elif st==2: s=h; c=(x+w-s,y); a0,a1=0,0.5*np.pi; w-=s
        else: s=w; c=(x+w,y+h-s); a0,a1=0.5*np.pi,np.pi; h-=s
        a=np.linspace(a0,a1,60); pts.append(np.c_[c[0]+s*np.cos(a),c[1]+s*np.sin(a)])
    return pts
def squares(x,y,w,h,n=9):
    sq=[]
    for k in range(n):
        st=k%4
        if st==0: s=h; sq.append((x,y,s)); x+=s; w-=s
        elif st==1: s=w; sq.append((x,y,s)); y+=s; h-=s
        elif st==2: s=h; sq.append((x+w-s,y,s)); w-=s
        else: s=w; sq.append((x,y+h-s,s)); h-=s
    return sq
rep=[]
for nm,suf,bgc in (('dark','Dark',(0,0,0)),('light','Light',(255,255,255))):
    v,G,faces,fits,wm=pipe.run(nm)
    defs,sb=logo.sym_defs_and_body(faces,fits)
    logo.WM={'head':{'paths':wm['head']['paths']},'tag':{'paths':wm['tag']['paths']}}
    col='#ffffff' if nm=='dark' else '#000000'
    svg=logo.wrap((0,0,pipe.FW,pipe.FH),defs,'<g>'+sb+'</g>'+logo.wm_body(col),bg='#%02x%02x%02x'%bgc)
    img=np.array(Image.open(io.BytesIO(bytes(resvg_py.svg_to_bytes(svg_string=svg)))).convert('RGB'))
    sym=np.vstack([faces[k].poly() for k in faces]); hb=np.array(wm['head']['bbox']); tb=np.array(wm['tag']['bbox'])
    hx0=hb[:,0,0].min(); hx1=hb[:,1,0].max(); Wr=hx1-hx0; y0=sym[:,1].min(); Hr=Wr/phi
    Hreal=hb[:,1,1].max()-y0
    # golden rectangle: headline width x (width/phi), anchored at symbol top; spiral drawn inside
    gold=(255,190,0) if nm=='dark' else (200,120,0)
    ov=img.copy()
    for (sx,sy,s) in squares(hx0,y0,Wr,Hr): cv2.rectangle(ov,(int(sx),int(sy)),(int(sx+s),int(sy+s)),gold,1,cv2.LINE_AA)
    for P in spiral_pts(hx0,y0,Wr,Hr): cv2.polylines(ov,[P.round().astype(np.int32)],False,(255,60,60),3,cv2.LINE_AA)
    # actual block (symbol top -> headline bottom)
    cv2.rectangle(ov,(int(hx0),int(y0)),(int(hx1),int(y0+Hreal)),(60,160,255),2,cv2.LINE_AA)
    out=cv2.addWeighted(ov,0.85,img,0.15,0)
    X0,Y0,X1,Y1=int(hx0-80),int(y0-60),int(hx1+80),int(tb[:,1,1].max()+60)
    Image.fromarray(out[Y0:Y1,X0:X1]).save(os.path.join(ROOT,'06_Comparison','Golden_Ratio_Overlay_%s.png'%suf))
    hw=hb[:,1,0].max()-hb[:,0,0].min(); capH=np.median(hb[:,1,1]-hb[:,0,1]); capT=np.median(tb[:,1,1]-tb[:,0,1])
    gap1=hb[:,0,1].min()-sym[:,1].max(); gap2=tb[:,0,1].min()-hb[:,1,1].max()
    rep.append((suf,dict(W=Wr,Hreal=Hreal,Hgold=Hr,ratio=Wr/Hreal,cap=capH/capT,gap=gap1/gap2,hw_sw=hw/(sym[:,0].max()-sym[:,0].min()),symWH=(sym[:,0].max()-sym[:,0].min())/(sym[:,1].max()-sym[:,1].min()))))
def pct(x,t): return '%+.1f%%'%(100*(x/t-1))
d=dict(rep)['Dark']; l=dict(rep)['Light']
gv={}
for nm in ('dark','light'):
    v_,G_,f_,fi_,w_=pipe.run(nm); gv[nm]=G_
def row(label,fn,target):
    return '| %s | %s | %s | %s |'%(label,fn('dark'),fn('light'),target)
md=f'''# 황금비(피보나치 황금나선) 적용 — Golden_Ratio_Analysis

기준값: φ = 1.618, 1/φ = 0.618, 1/φ² = 0.382, 1−1/φ³ = 0.764. 겹침 이미지: `06_Comparison/Golden_Ratio_Overlay_Dark.png`, `..._Light.png`
(금색 칸 = 피보나치 사각형 분할, 빨간 선 = 황금나선, 파란 사각형 = 제목 폭 × 실제 로고 블록).

## 이번에 **심볼을 황금비로 바꾼 부분** (요청에 따른 의도적 변경)
원본 이미지의 심볼과 달라집니다. 되돌리려면 `The_Awesome_Logo_Final_v1_원본비율.zip`(원본 비율 그대로의 이전 결과)을 쓰세요.

| 항목 | 다크 (전→후) | 라이트 (전→후) | 목표 |
|---|---|---|---|
| 심볼 외곽 가로÷세로 | {gv['dark']['WH_before']:.3f} → {gv['dark']['golden']['bbox_W']/gv['dark']['golden']['bbox_H']:.3f} | {gv['light']['WH_before']:.3f} → {gv['light']['golden']['bbox_W']/gv['light']['golden']['bbox_H']:.3f} | 1.618 (황금사각형) |
| 정점의 가로 위치 (왼쪽 발~오른쪽 발 기준) | {gv['dark']['apex_before']:.3f} → 0.618 | {gv['light']['apex_before']:.3f} → 0.618 | 0.618 |
| 안쪽 삼각 꼭짓점 높이 (바닥~정점 기준) | {gv['dark']['hole_before']:.3f} → 0.764 | {gv['light']['hole_before']:.3f} → 0.764 | 0.764 |
| 아치 정점 높이 | {gv['dark']['crest_before']:.3f} → 0.472 | {gv['light']['crest_before']:.3f} → 0.472 | 0.764×0.618 = 0.472 |

적용 방법: ① 안쪽 삼각 꼭짓점이 0.764가 되도록 오른쪽 다리 안쪽 직선을 오른쪽 발을 축으로 살짝 회전, ② 아치를 위로 끌어올려 정점 높이를 0.472로, ③ 심볼을 가로로 늘려 가로÷세로=φ, ④ 발은 고정한 채 위쪽을 밀어(전단) 정점을 가로 0.618 위치로. 직선은 그대로 직선, 곡선은 매끈한 곡선으로 유지됩니다. 높이는 그대로이고 폭이 다크 약 +22%, 라이트 약 +28% 넓어졌습니다. 음영은 같은 변환을 가한 원본 이미지로 다시 맞췄습니다.

## 글자·간격 (바꾸지 않음) — 대신 일부 관계가 달라짐
글자 크기와 간격은 그대로 두었습니다. 심볼이 넓어졌으므로 다음 관계는 변했습니다.
| 항목 | 다크 | 라이트 | 황금비 목표 |
|---|---|---|---|
| 제목 폭 ÷ 심볼 폭 | {d['hw_sw']:.3f} | {l['hw_sw']:.3f} | 1.618 (원래 다크 1.645/라이트 1.689였으나 심볼 확대로 **이 관계는 깨짐**) |
| (제목 폭) ÷ (심볼 꼭대기~제목 아래) | {d['ratio']:.3f} | {l['ratio']:.3f} | 1.618 (유지) |
| 간격 비 (심볼~제목 ÷ 제목~태그라인) | {d['gap']:.3f} | {l['gap']:.3f} | 1.618 (유지) |
| 제목÷태그라인 대문자 높이 | {d['cap']:.3f} | {l['cap']:.3f} | 2.618 (다크 일치, 라이트는 원본이 3.05) |

심볼 폭과 제목 폭의 황금비도 다시 맞추려면 글자를 약 20% 키우거나 줄여야 하는데, 그러면 블록 전체 비율과 간격 비율이 어긋나므로 이번에는 건드리지 않았습니다.

## 황금나선과 아치 곡선
아치 아래쪽 가장자리에 황금나선과 원호를 각각 맞춰 보았더니 오차가 같아서(다크 2.7 대 2.8px, 라이트 3.6 대 3.6px) 나선이 더 낫다는 근거가 없었습니다. 위쪽 가장자리는 나선이 원보다 약간 가깝지만(6.6~8.2px 대 8.4~9.9px) 현재 쓰는 맞춤(2~3px)보다 커서 쓰지 않았습니다. 그래서 곡선 종류는 바꾸지 않고, **위치와 비율만** 황금분할에 맞췄습니다.
'''
open(os.path.join(ROOT,'07_Documentation','Golden_Ratio_Analysis.md'),'w',encoding='utf-8').write(md)
print(json.dumps(dict(rep),indent=1,default=float))
