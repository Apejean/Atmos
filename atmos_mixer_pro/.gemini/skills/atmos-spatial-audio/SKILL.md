---
name: atmos-spatial-audio
description: Implements and audits 3D Spatial Audio algorithms (DBAP multi-channel panning, Catmull-Rom 3D trajectory automation, 1.2m mannequin ear aiming, LR4 crossover, SOFA HRTF).
---

# 🌐 Skill: 3D Spatial Audio & Acoustic Algorithms

This skill provides mathematical formulas, coordinates, and implementation standards for 3D spatial sound rendering in Atmos Mixer Pro.

---

## 1. 📐 3D DBAP (Distance-Based Amplitude Panning)

Used for irregular multi-channel speaker layouts ($1 \sim N$ channels).

### Mathematical Model
Given virtual source position $\mathbf{S} = (x, y, z)$ and speaker position $\mathbf{P}_i = (x_i, y_i, z_i)$:
1. Euclidean distance:
   $$d_i = \sqrt{(x - x_i)^2 + (y - y_i)^2 + (z - z_i)^2}$$
2. Raw distance weight (with rolloff factor $a$, typically $a = 1.0 \sim 2.0$):
   $$w_i = d_i^{-a}$$
3. Energy Conservation Normalization ($\sum_{i=1}^N g_i^2 = 1$):
   $$g_i = \frac{w_i}{\sqrt{\sum_{k=1}^N w_k^2}}$$

---

## 2. 👤 3D Listener & Mannequin Standards

### Coordinate Standards
- **3D Model Asset:** `assets/models/listener_head.glb` (or `listener_mannequin.glb`)
- **Center Listener Placement:** $(W/2,\; D/2,\; 1.2\text{m})$
  - $1.2\text{m}$ is the international acoustic standard seated human ear height.
- **Speaker Aiming Vector (Pitch/Yaw):**
  - All ceiling/wall speakers must compute orientation vectors aimed directly at the listener's ears (`listenerGroup` at $1.2\text{m}$ height).

---

## 3. 🎢 Catmull-Rom Spline 3D Trajectory Automation

Used for smooth 3D sound trajectory pathing without acceleration artifacts.

### Centripetal Catmull-Rom Formula
Given 4 control points $\mathbf{P}_0, \mathbf{P}_1, \mathbf{P}_2, \mathbf{P}_3$ and parameter $t \in [0, 1]$:
$$\mathbf{C}(t) = 0.5 \cdot \begin{bmatrix} 1 & t & t^2 & t^3 \end{bmatrix} \begin{bmatrix} 0 & 2 & 0 & 0 \\ -1 & 0 & 1 & 0 \\ 2 & -5 & 4 & -1 \\ -1 & 3 & -3 & 1 \end{bmatrix} \begin{bmatrix} \mathbf{P}_0 \\ \mathbf{P}_1 \\ \mathbf{P}_2 \\ \mathbf{P}_3 \end{bmatrix}$$
- **Runtime Requirement:** Interpolation must be computed per audio buffer in Rust without allocations.

---

## 4. 🎛️ Linkwitz-Riley 24dB/oct (LR4) Crossover

- **Structure:** 2 cascaded 2nd-order Butterworth filters.
- **Magnitude:** Completely flat ($0\text{dB}$ bump) at crossover frequency ($f_c \in [60\text{Hz}, 120\text{Hz}]$).
- **Phase:** $360^\circ$ difference ($= 0^\circ$ in-phase) across bands.
- **Routing:**
  - Satellites ($1 \sim N$): High-Pass Filter ($f_c$)
  - Subwoofer (LFE, .1): Low-Pass Filter ($f_c$) summing low frequencies from all satellite channels.
