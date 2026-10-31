# 청취자 흉상(`assets/models/listener_bust_white.obj`) 재생성

3D 화면의 청취자 흉상을 저장소 안에서 다시 만드는 스크립트다(2026-10-31, 사용자 결정).

## 출처

| 부분 | 만든 방법 | 권리 |
|---|---|---|
| 머리·목·어깨 | 이전 흉상(ChatGPT로 만든 모델, 2026-10-24 채택)을 높이별 단면으로 재고, 세부를 흐린 뒤 매끈한 곡면으로 새로 짰다. 꼭짓점은 복사하지 않았다. | 이전 흉상과 같은 출처 상태 |
| 귀 | MakeHuman 기본 메시(MPFB2 `data/3dobjs/base.obj`)와 형태 데이터(`data/targets`)로 만든 남성 흉상의 귀 | CC0 1.0. MakeHuman LICENSE의 "D. Concerning the output"에 따라 결과물에 제한이 없다 |
| 도구 | Blender(`bpy`), numpy, scipy, scikit-image | 결과물에 권리를 주장하지 않는다 |

이미지→3D 생성 모델(TRELLIS 등)은 쓰지 않았다. 원문을 확인해 보니 GLB 내보내기에 쓰는 nvdiffrast(NVIDIA Source Code License 3.3조)와 공식 데모의 배경 제거 모델 RMBG-2.0(CC BY-NC 4.0)이 비상업 전용이었다.

## 결과물 규격

- Y가 위, 바닥 y 0(평평하게 막힘), 정수리 y 3.97, 얼굴 +Z
- 외이도 y 2.50. 귀는 y 2.21~2.77, z 평균 -0.146(`studio_engine.html`의 `rawEarY`·`rawEarZ`)
- 꼭짓점 45,002개, 삼각형 90,000개, 정점 법선 포함(`f a//a`), 닫힌 메시, 재질 `PureWhite`

## 다시 만들기

준비물:

- MPFB2 저장소: `git clone --depth 1 https://github.com/makehumancommunity/mpfb2.git` → `MPFB_DATA=<경로>/mpfb2/src/mpfb/data`
- numpy·scipy·scikit-image가 있는 파이썬(`PY`)
- `bpy`(Blender 5.2, `pip install bpy`)와 numpy가 있는 파이썬(`BPY`). 같은 파이썬이어도 된다
- 기준 흉상: 교체 전 파일 `git show 347f42d:atmos_mixer_pro/assets/models/listener_bust_white.obj > ref.obj`
- 메모리 8GB 이상. CPU 4코어에서 약 30~40분 걸린다

```bash
cd atmos_mixer_pro/tool/listener_bust
MPFB_DATA=/path/to/mpfb2/src/mpfb/data PY=python3 BPY=python3 \
  ./run.sh ref.obj ../../assets/models/listener_bust_white.obj
```

중간 산출물은 `BUST_WORK`(기본 `./work`)에 생긴다. 커밋하지 않는다.

## 단계 (`run.sh`)

1. `morph.py` → `cutlevel.py` → `bust_std.py` → `align.py`: MakeHuman 남성 흉상(`configs/std8.json`)을 만들어 외이도·정수리를 맞춘다. 귀를 가져올 원본이다.
2. `mh_head.py` → `mh_align.py`: 같은 머리를 고운 복셀(0.005)로 다시 만든다. 거친 복셀로 만들면 귓바퀴 가장자리가 들쭉날쭉하다.
3. `prep_reference.py` → `measure.py` → `build.py` → `earfill.py`: 기준 흉상을 단면으로 재서 매끈한 몸통을 만들고(`configs/body.json`), 옛 귀 자리를 주변 곡면에서 이어지는 면으로 메운다(`configs/earfill.json`).
4. `corr.py` → `mh_smooth.py`: MakeHuman 머리 피부 높이를 새 몸통에 맞추고(`configs/corr.json`), 귓바퀴를 뺀 피부를 매끈하게 한다.
5. `merge4.py` → `finalize.py`: 귀 둘레에서 몸통 피부를 MakeHuman 피부에 서서히 감싸 붙이고 귓바퀴를 합친 뒤, 귀 둘레를 다듬고 감축해 OBJ로 쓴다(`configs/merge.json`).
