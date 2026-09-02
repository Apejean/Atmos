# Linkwitz-Riley 24dB/oct (LR4) Crossover Architecture

## 1. Overview
To support Bass Management and Subwoofer (LFE) integration in the 3D Atmos Mixer, we need to implement Linkwitz-Riley 24dB/octave (LR4) crossover filters in the Rust DSP backend.

## 2. DSP Implementation (Rust)
The LR4 filter is formed by cascading two 2nd-order Butterworth filters.
- **Low-Pass Filter (LPF) for Subwoofer:** `Butterworth_LPF(fc) -> Butterworth_LPF(fc)`
- **High-Pass Filter (HPF) for Main Speakers:** `Butterworth_HPF(fc) -> Butterworth_HPF(fc)`

When these two outputs are summed acoustically, the magnitude response is completely flat (0dB bump at crossover frequency), and the phase response of both bands remains in-phase (360-degree difference = 0-degree).

## 3. Bass Management Routing (Dart -> Rust)
- Define a global `crossoverFrequency` (e.g., 80Hz).
- **Satellite Channels (1-7):** Route audio through the HPF.
- **Subwoofer Channel (LFE, .1):** Route the low-frequency content (below 80Hz) from ALL satellite channels, sum them, and pass them through the LPF.

## 4. UI Integration
- Add a "Bass Management" toggle in the Tuning Modal.
- Add a Crossover Frequency slider (60Hz ~ 120Hz).
