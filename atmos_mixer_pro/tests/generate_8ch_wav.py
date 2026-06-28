import wave
import math
import struct

def generate_8ch_wav(filename, duration_sec=5.0, sample_rate=44100):
    num_channels = 8
    
    # Frequencies for each channel: C major scale
    frequencies = [
        261.63, # Ch 1: C4
        293.66, # Ch 2: D4
        329.63, # Ch 3: E4
        349.23, # Ch 4: F4
        392.00, # Ch 5: G4
        440.00, # Ch 6: A4
        493.88, # Ch 7: B4
        523.25  # Ch 8: C5
    ]
    
    num_samples = int(duration_sec * sample_rate)
    
    with wave.open(filename, 'w') as wav_file:
        wav_file.setnchannels(num_channels)
        wav_file.setsampwidth(2) # 16-bit
        wav_file.setframerate(sample_rate)
        
        for i in range(num_samples):
            frame_data = bytearray()
            for ch in range(num_channels):
                t = float(i) / sample_rate
                # Generate sine wave
                sample_val = math.sin(2.0 * math.pi * frequencies[ch] * t)
                # Scale to 16-bit integer range (-32767 to 32767)
                # Lower volume to 30% to avoid deafening
                int_val = int(sample_val * 32767 * 0.3)
                frame_data.extend(struct.pack('<h', int_val))
            wav_file.writeframesraw(frame_data)
            
    print(f"Successfully generated {filename} with {num_channels} channels.")

if __name__ == "__main__":
    generate_8ch_wav("test_8ch.wav")
