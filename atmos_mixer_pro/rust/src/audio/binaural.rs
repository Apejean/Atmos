use realfft::RealFftPlanner;
use rustfft::num_complex::Complex;
use sofa_reader::{SofaFile, SourcePosition};

pub struct BinauralChannel {
    ir_left_freq: Vec<Complex<f32>>,
    ir_right_freq: Vec<Complex<f32>>,
    target_ir_left_freq: Vec<Complex<f32>>,
    target_ir_right_freq: Vec<Complex<f32>>,
    fft_size: usize,
    input_buffer: Vec<f32>,
    overlap_add_left: Vec<f32>,
    overlap_add_right: Vec<f32>,
    r2c: std::sync::Arc<dyn realfft::RealToComplex<f32>>,
    c2r: std::sync::Arc<dyn realfft::ComplexToReal<f32>>,
    input_freq: Vec<Complex<f32>>,
    out_left_freq: Vec<Complex<f32>>,
    out_right_freq: Vec<Complex<f32>>,
    out_left_time: Vec<f32>,
    out_right_time: Vec<f32>,
    out_left_time_target: Vec<f32>,
    out_right_time_target: Vec<f32>,
    crossfade_phase: f32,
    is_switching: bool,

    // update_hrtf()에서 재사용하는 임시 패딩 버퍼. 오디오 콜백(process_interleaved)에서
    // 호출될 수 있으므로 매번 vec![]로 새로 할당하지 않고 사전 할당된 버퍼를 덮어쓴다(Law1 준수).
    scratch_pad_left: Vec<f32>,
    scratch_pad_right: Vec<f32>,
}

impl BinauralChannel {
    pub fn new(ir_left: &[f32], ir_right: &[f32], block_size: usize) -> Self {
        let ir_len = ir_left.len().max(ir_right.len());
        let fft_size = (block_size + ir_len - 1).next_power_of_two();
        
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(fft_size);
        let c2r = planner.plan_fft_inverse(fft_size);
        
        let mut ir_left_padded = vec![0.0; fft_size];
        ir_left_padded[..ir_left.len()].copy_from_slice(ir_left);
        let mut ir_left_freq = r2c.make_output_vec();
        let _ = r2c.process(&mut ir_left_padded, &mut ir_left_freq);

        let mut ir_right_padded = vec![0.0; fft_size];
        ir_right_padded[..ir_right.len()].copy_from_slice(ir_right);
        let mut ir_right_freq = r2c.make_output_vec();
        let _ = r2c.process(&mut ir_right_padded, &mut ir_right_freq);

        let target_ir_left_freq = ir_left_freq.clone();
        let target_ir_right_freq = ir_right_freq.clone();

        let input_freq = vec![Complex::new(0.0, 0.0); ir_left_freq.len()];
        let out_left_freq = vec![Complex::new(0.0, 0.0); ir_left_freq.len()];
        let out_right_freq = vec![Complex::new(0.0, 0.0); ir_left_freq.len()];
        
        let out_left_time = vec![0.0; fft_size];
        let out_right_time = vec![0.0; fft_size];
        let out_left_time_target = vec![0.0; fft_size];
        let out_right_time_target = vec![0.0; fft_size];

        Self {
            ir_left_freq,
            ir_right_freq,
            target_ir_left_freq,
            target_ir_right_freq,
            fft_size,
            input_buffer: vec![0.0; fft_size],
            overlap_add_left: vec![0.0; fft_size],
            overlap_add_right: vec![0.0; fft_size],
            r2c,
            c2r,
            input_freq,
            out_left_freq,
            out_right_freq,
            out_left_time,
            out_right_time,
            out_left_time_target,
            out_right_time_target,
            crossfade_phase: 1.0,
            is_switching: false,

            scratch_pad_left: vec![0.0; fft_size],
            scratch_pad_right: vec![0.0; fft_size],
        }
    }

    pub fn update_hrtf(&mut self, new_ir_left: &[f32], new_ir_right: &[f32]) {
        let len_l = new_ir_left.len().min(self.fft_size);
        self.scratch_pad_left.fill(0.0);
        self.scratch_pad_left[..len_l].copy_from_slice(&new_ir_left[..len_l]);
        let _ = self.r2c.process(&mut self.scratch_pad_left, &mut self.target_ir_left_freq);

        let len_r = new_ir_right.len().min(self.fft_size);
        self.scratch_pad_right.fill(0.0);
        self.scratch_pad_right[..len_r].copy_from_slice(&new_ir_right[..len_r]);
        let _ = self.r2c.process(&mut self.scratch_pad_right, &mut self.target_ir_right_freq);

        self.is_switching = true;
        self.crossfade_phase = 0.0;
    }

        pub fn process_block(&mut self, input: &[f32], out_left: &mut [f32], out_right: &mut [f32]) {
        let block_size = input.len().min(self.fft_size);
        self.input_buffer.fill(0.0);
        self.input_buffer[..block_size].copy_from_slice(&input[..block_size]);

        let _ = self.r2c.process(&mut self.input_buffer, &mut self.input_freq);

        // Convolve with current IR
        for i in 0..self.input_freq.len() {
            self.out_left_freq[i] = self.input_freq[i] * self.ir_left_freq[i];
            self.out_right_freq[i] = self.input_freq[i] * self.ir_right_freq[i];
        }
        let _ = self.c2r.process(&mut self.out_left_freq, &mut self.out_left_time);
        let _ = self.c2r.process(&mut self.out_right_freq, &mut self.out_right_time);

        let scale = 1.0 / self.fft_size as f32;
        
        // If switching, convolve with target IR and crossfade
        if self.is_switching {
            for i in 0..self.input_freq.len() {
                self.out_left_freq[i] = self.input_freq[i] * self.target_ir_left_freq[i];
                self.out_right_freq[i] = self.input_freq[i] * self.target_ir_right_freq[i];
            }
            let _ = self.c2r.process(&mut self.out_left_freq, &mut self.out_left_time_target);
            let _ = self.c2r.process(&mut self.out_right_freq, &mut self.out_right_time_target);
            
            let fade_step = 1.0 / block_size as f32;
            
            for i in 0..block_size {
                self.crossfade_phase += fade_step;
                if self.crossfade_phase >= 1.0 {
                    self.crossfade_phase = 1.0;
                    self.is_switching = false;
                    self.ir_left_freq.copy_from_slice(&self.target_ir_left_freq);
                    self.ir_right_freq.copy_from_slice(&self.target_ir_right_freq);
                }
                
                let cur_l = self.out_left_time[i] * scale;
                let cur_r = self.out_right_time[i] * scale;
                let tar_l = self.out_left_time_target[i] * scale;
                let tar_r = self.out_right_time_target[i] * scale;
                
                let mix_l = cur_l * (1.0 - self.crossfade_phase) + tar_l * self.crossfade_phase;
                let mix_r = cur_r * (1.0 - self.crossfade_phase) + tar_r * self.crossfade_phase;
                
                out_left[i] = mix_l + self.overlap_add_left[i];
                out_right[i] = mix_r + self.overlap_add_right[i];
            }
        } else {
            for i in 0..block_size {
                out_left[i] = self.out_left_time[i] * scale + self.overlap_add_left[i];
                out_right[i] = self.out_right_time[i] * scale + self.overlap_add_right[i];
            }
        }
        
        // Update overlap buffers
        self.overlap_add_left.fill(0.0);
        self.overlap_add_right.fill(0.0);
        for i in block_size..self.fft_size {
            let src_i = i;
            let dst_i = i - block_size;
            if self.is_switching {
                let mix_l = (self.out_left_time[src_i] * (1.0 - self.crossfade_phase) + self.out_left_time_target[src_i] * self.crossfade_phase) * scale;
                let mix_r = (self.out_right_time[src_i] * (1.0 - self.crossfade_phase) + self.out_right_time_target[src_i] * self.crossfade_phase) * scale;
                self.overlap_add_left[dst_i] = mix_l;
                self.overlap_add_right[dst_i] = mix_r;
            } else {
                self.overlap_add_left[dst_i] = self.out_left_time[src_i] * scale;
                self.overlap_add_right[dst_i] = self.out_right_time[src_i] * scale;
            }
        }
    }
}

pub struct VirtualMixRoomBinaural {
    channels: Vec<BinauralChannel>,
    pub enabled: bool,
    temp_channel_buffers: Vec<Vec<f32>>,
    mix_left: Vec<f32>,
    mix_right: Vec<f32>,
    current_yaw: f32,
    current_pitch: f32,
    current_roll: f32,

    // MIT KEMAR SOFA HRTF 데이터셋. new()에서 1회만 로드하며, 오디오 콜백에서는
    // 이 데이터를 읽기만 한다(신규 할당/파일 I/O 절대 금지, Law1 준수).
    hrtf_db: Option<SofaFile>,
    // 데이터셋 측정 거리(m). 방위각 보간 시 타깃 위치의 반경 성분으로 사용.
    nominal_distance: f32,
    // 방위각 보간 결과를 저장하는 사전 할당 스크래치 버퍼(Law1 준수).
    interp_ir_left: Vec<f32>,
    interp_ir_right: Vec<f32>,
}

impl VirtualMixRoomBinaural {
    pub fn new(num_channels: usize, block_size: usize) -> Self {
        // SOFA HRTF 파일은 엔진 초기화 시점(new())에서 1회만 로드한다.
        let sofa_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/hrtf/mit_kemar_normal_pinna.sofa"
        );
        let hrtf_db = match SofaFile::strict_load(sofa_path) {
            Ok(db) => Some(db),
            Err(e) => {
                eprintln!(
                    "[Binaural] SOFA HRTF 로드 실패({}): {}. 무필터 폴백 IR을 사용합니다.",
                    sofa_path, e
                );
                None
            }
        };

        let nominal_distance = hrtf_db
            .as_ref()
            .and_then(|db| db.positions.first())
            .map(|p| p.distance)
            .unwrap_or(1.0);
        let ir_len = hrtf_db.as_ref().map(|db| db.ir_length).unwrap_or(2);

        // 정면(0° azimuth, 0° elevation) 실측 HRIR로 초기 채널을 구성한다. 로드 실패 시 무필터 폴백.
        let (init_ir_left, init_ir_right): (Vec<f32>, Vec<f32>) = match &hrtf_db {
            Some(db) => {
                let target = SourcePosition::new(0.0, 0.0, nominal_distance);
                let (idx, _dist) = db.find_nearest(&target);
                match db.get_hrtf_slices(idx) {
                    Some((_pos, left, right)) => (left.to_vec(), right.to_vec()),
                    None => (vec![1.0, 0.0], vec![0.0, 1.0]),
                }
            }
            None => (vec![1.0, 0.0], vec![0.0, 1.0]),
        };

        let mut channels = Vec::new();
        let mut temp_channel_buffers = Vec::new();
        for _ in 0..num_channels {
            channels.push(BinauralChannel::new(&init_ir_left, &init_ir_right, block_size));
            temp_channel_buffers.push(vec![0.0; 8192]);
        }

        Self {
            channels,
            enabled: false,
            temp_channel_buffers,
            mix_left: vec![0.0; 8192],
            mix_right: vec![0.0; 8192],
            current_yaw: 0.0,
            current_pitch: 0.0,
            current_roll: 0.0,
            hrtf_db,
            nominal_distance,
            interp_ir_left: vec![0.0; ir_len],
            interp_ir_right: vec![0.0; ir_len],
        }
    }

    pub fn process_interleaved(&mut self, output: &mut [f32], out_channels: usize) {
        if !self.enabled || out_channels < 2 { return; }
        
        // Handle 3-DoF Head Tracking via GLOBAL_STATE
        let yaw = f32::from_bits(crate::core::state::GLOBAL_STATE.hrtf_yaw.load(std::sync::atomic::Ordering::Relaxed));
        let pitch = f32::from_bits(crate::core::state::GLOBAL_STATE.hrtf_pitch.load(std::sync::atomic::Ordering::Relaxed));
        let roll = f32::from_bits(crate::core::state::GLOBAL_STATE.hrtf_roll.load(std::sync::atomic::Ordering::Relaxed));
        
        if (yaw - self.current_yaw).abs() > 0.01 || (pitch - self.current_pitch).abs() > 0.01 || (roll - self.current_roll).abs() > 0.01 {
            self.current_yaw = yaw;
            self.current_pitch = pitch;
            self.current_roll = roll;
            
            if let Some(db) = &self.hrtf_db {
                // 실측 SOFA HRIR 룩업: 머리 yaw 회전을 음원의 상대 방위각으로 환산하고
                // 최근접 3개 실측 위치를 역거리 가중 삼선형 보간한다(무할당, 사전 할당 스크래치 버퍼 사용).
                let azimuth_deg = -yaw.to_degrees();
                let target = SourcePosition::new(azimuth_deg, 0.0, self.nominal_distance);
                let nbrs = db.find_three_nearest(&target);

                const EPS: f32 = 1e-4;
                let mut weights = [0.0f32; 3];
                let mut total = 0.0f32;
                for (slot, (_idx, dist)) in weights.iter_mut().zip(nbrs.iter()) {
                    let w = 1.0 / (*dist + EPS);
                    *slot = w;
                    total += w;
                }
                if total > 0.0 {
                    for w in weights.iter_mut() {
                        *w /= total;
                    }
                }

                self.interp_ir_left.fill(0.0);
                self.interp_ir_right.fill(0.0);
                for ((idx, _dist), w) in nbrs.iter().zip(weights.iter()) {
                    if let Some((_pos, left, right)) = db.get_hrtf_slices(*idx) {
                        for (dst, src) in self.interp_ir_left.iter_mut().zip(left.iter()) {
                            *dst += src * (*w);
                        }
                        for (dst, src) in self.interp_ir_right.iter_mut().zip(right.iter()) {
                            *dst += src * (*w);
                        }
                    }
                }

                for ch in &mut self.channels {
                    ch.update_hrtf(&self.interp_ir_left, &self.interp_ir_right);
                }
            } else {
                // SOFA 데이터셋 로드 실패 시 폴백: 기존 합성 시프트 유지
                let shift = yaw.sin() * 0.5; // -0.5 to 0.5
                let dummy_ir_left = [(0.5 - shift).max(0.0), 0.0];
                let dummy_ir_right = [(0.5 + shift).max(0.0), 1.0];

                for ch in &mut self.channels {
                    ch.update_hrtf(&dummy_ir_left, &dummy_ir_right);
                }
            }
        }
        
        let frames = output.len() / out_channels;
        let num_ch = self.channels.len().min(out_channels);
        
        let frames = frames.min(8192);
        
        // De-interleave
        for ch in 0..num_ch {
            for frame in 0..frames {
                self.temp_channel_buffers[ch][frame] = output[frame * out_channels + ch];
            }
        }
        
        self.mix_left[..frames].fill(0.0);
        self.mix_right[..frames].fill(0.0);
        
        // Process each channel
        for ch in 0..num_ch {
            self.channels[ch].process_block(&self.temp_channel_buffers[ch][..frames], &mut self.mix_left[..frames], &mut self.mix_right[..frames]);
        }
        
        // Re-interleave
        for frame in 0..frames {
            output[frame * out_channels] = self.mix_left[frame];
            output[frame * out_channels + 1] = self.mix_right[frame];
        }
    }
}
