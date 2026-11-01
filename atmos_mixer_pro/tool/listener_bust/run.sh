#!/bin/bash
# listener_bust_white.obj 재생성. 사용: ./run.sh <기준 OBJ> <출력 OBJ>
#   기준 OBJ: 교체 전 흉상(git show 347f42d:atmos_mixer_pro/assets/models/listener_bust_white.obj)
# 환경 변수: MPFB_DATA(MPFB2 data 폴더), BUST_WORK(중간 산출물, 기본 ./work), PY(numpy·scipy·scikit-image), BPY(bpy가 있는 파이썬)
set -euo pipefail
cd "$(dirname "$0")"
REF=$1; OUT=$2
export BUST_WORK=${BUST_WORK:-work}; export MPFB_DATA=${MPFB_DATA:-/tmp/mpfb2/src/mpfb/data}
PY=${PY:-python3}; BPY=${BPY:-python3}; W=$BUST_WORK; S=scripts; C=configs
mkdir -p "$W"
# 1) MakeHuman 흉상(귀를 가져올 원본): 모프 → 절단 높이 → 복셀 흉상 → 외이도·정수리 정렬
$PY $S/morph.py $C/std8.json $W/morph_std8.obj
echo '{"voxel": 0.012, "target_tris": 89000}' > $W/std8.bust.json
$PY $S/cutlevel.py $W/morph_std8.obj $W/std8.bust.json
$BPY $S/bust_std.py $W/morph_std8.obj $W/std8.obj $W/std8.bust.json
$PY $S/align.py $W/morph_std8.obj $W/std8.obj
# 2) 같은 MakeHuman 머리를 고운 복셀(0.005)로 다시 만들어 같은 좌표로 정렬
$BPY $S/mh_head.py $W/morph_std8.obj $W/mhhead 6.9 0.005
$PY $S/mh_align.py $W/morph_std8.obj $W/std8.obj.raw.v.npy $W/mhhead
# 3) 기준 흉상 치수 측정 → 매끈한 로프트 몸통 → 옛 귀 자리 메우기
$PY $S/prep_reference.py "$REF"
$PY $S/measure.py
$PY $S/build.py $C/body.json $W/body.obj
$PY $S/earfill.py $W/body.obj $W/body_fill.obj $C/earfill.json
# 4) MakeHuman 피부를 로프트 머리 높이에 맞추고 귀 둘레 피부를 매끈하게
$PY $S/corr.py $W/body.obj.prof.npy $W/mhhead.v.npy $W/mhhead.f.npy $W/mhhead_corr.obj $C/corr.json
$BPY $S/mh_smooth.py $W/mhhead_corr.obj $W/mhhead_smooth.obj 40 0.6
# 5) 귀 주변 피부 감싸기 + 귓바퀴 합치기 + 다듬기 + 감축 → OBJ
$BPY $S/merge4.py $W/body_fill.obj $W/mhhead_smooth.obj $W/bust.obj $C/merge.json
$PY $S/finalize.py $W/bust.obj "$OUT"
