use crate::audio::mixer::AudioMixer;
use std::time::Instant;

pub struct OfflineRenderer {
    pub target_sample_rate: u32,
    pub target_channels: usize,
    pub duration_seconds: f32,
}

impl OfflineRenderer {
    pub fn new(target_sample_rate: u32, target_channels: usize, duration_seconds: f32) -> Self {
        Self {
            target_sample_rate,
            target_channels,
            duration_seconds,
        }
    }

    pub fn render_to_wav(&self, mut mixer: AudioMixer, out_path: &str) -> Result<(), String> {
        let total_frames = (self.target_sample_rate as f32 * self.duration_seconds) as usize;
        let mut frame_count = 0;
        let block_size = 8192; 
        
        let (tx, rx) = crossbeam_channel::bounded::<Vec<f32>>(32);
        
        let out_path_owned = out_path.to_string();
        let target_channels = self.target_channels;
        let target_sample_rate = self.target_sample_rate;
        
        let writer_thread = std::thread::spawn(move || {
            let spec = hound::WavSpec {
                channels: target_channels as u16,
                sample_rate: target_sample_rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };
            
            match hound::WavWriter::create(&out_path_owned, spec) {
                Ok(mut writer) => {
                    while let Ok(buffer) = rx.recv() {
                        for &sample in &buffer {
                            let _ = writer.write_sample(sample);
                        }
                    }
                    let _ = writer.finalize();
                }
                Err(e) => {
                    eprintln!("Failed to create WavWriter at {}: {:?}", out_path_owned, e);
                }
            }
        });
        
        let start_time = Instant::now();
        
        while frame_count < total_frames {
            let frames_to_process = (total_frames - frame_count).min(block_size);
            let mut buffer = vec![0.0; frames_to_process * self.target_channels]; 
            
            mixer.process(&mut buffer, self.target_channels);
            
            if tx.send(buffer).is_err() {
                break;
            }
            
            frame_count += frames_to_process;
        }
        
        drop(tx);
        
        if writer_thread.join().is_err() {
            return Err("Failed to join writer thread".to_string());
        }
        
        let elapsed = start_time.elapsed();
        println!("Offline render complete. Took {:?}", elapsed);
        
        Ok(())
    }
}
