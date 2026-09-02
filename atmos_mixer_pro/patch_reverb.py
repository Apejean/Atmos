with open('rust/src/audio/reverb.rs', 'r') as f:
    content = f.read()

content = content.replace('''        let base_lengths = [
            0.0297 * sample_rate, 
            0.0371 * sample_rate, 
            0.0411 * sample_rate, 
            0.0437 * sample_rate, 
        ];''', '''        let base_lengths = [
            0.0297 * sample_rate, 
            0.0371 * sample_rate, 
            0.0411 * sample_rate, 
            0.0437 * sample_rate
        ];''')

with open('rust/src/audio/reverb.rs', 'w') as f:
    f.write(content)
