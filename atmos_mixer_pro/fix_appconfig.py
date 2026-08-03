import re
import os
import glob

def find_dart_files(directory):
    return glob.glob(os.path.join(directory, '**/*.dart'), recursive=True)

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    if 'AppConfig(' not in content:
        return

    # Print the lines containing AppConfig instantiations and following lines
    lines = content.split('\n')
    for i, line in enumerate(lines):
        if 'AppConfig(' in line:
            print(f"--- {filepath}:{i+1} ---")
            for j in range(i, min(i+25, len(lines))):
                print(f"{j+1}: {lines[j]}")
                if ');' in lines[j] or '), ' in lines[j] or ')' in lines[j] and 'AppConfig' not in lines[j]:
                    break
            print("-----------------------")

if __name__ == '__main__':
    for f in find_dart_files('lib'):
        process_file(f)
    for f in find_dart_files('test'):
        process_file(f)
