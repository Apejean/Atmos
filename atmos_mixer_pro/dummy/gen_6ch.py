import wave
import struct
import math

# Audio parameters
num_channels = 6
sample_width = 2 # 16-bit
frame_rate = 48000
duration = 5.0 # seconds

num_frames = int(frame_rate * duration)

output_file = "/Users/Allweno/Projects/GitHub/atmos/atmos_mixer_pro/dummy/6ch_test.wav"

with wave.open(output_file, 'w') as wav_file:
    wav_file.setnchannels(num_channels)
    wav_file.setsampwidth(sample_width)
    wav_file.setframerate(frame_rate)
    
    for i in range(num_frames):
        # Generate 6 different frequencies for each channel
        data = []
        for ch in range(num_channels):
            freq = 440.0 + ch * 110.0 # 440, 550, 660, 770, 880, 990
            val = math.sin(2.0 * math.pi * freq * (i / frame_rate))
            # amplitude 0.5 to avoid clipping
            sample = int(val * 0.5 * 32767)
            data.append(struct.pack('<h', sample))
        
        wav_file.writeframes(b''.join(data))

print(f"Created {output_file}")
