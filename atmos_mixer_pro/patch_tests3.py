import re

with open("rust/tests/stress_test.rs", "r") as f:
    stress = f.read()

stress = stress.replace("""                    volume: 1.0,
                    output_channel: 0,
                    output_stereo: true,
                    current_position: None,""", """                    volume: 1.0,
                    output_channel: 0,
                    output_stereo: true,
                    current_position: None,
                    streamer: None,
                    room_volume: 1.0,""")

with open("rust/tests/stress_test.rs", "w") as f:
    f.write(stress)
