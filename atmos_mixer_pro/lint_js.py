import re

def check_brackets(code):
    brackets = []
    for line_num, line in enumerate(code.split('\n'), 1):
        for char in line:
            if char in '{[(':
                brackets.append((char, line_num))
            elif char in '}])':
                if not brackets:
                    return f"Extra closing bracket '{char}' at line {line_num}"
                last, last_line = brackets.pop()
                if (last == '{' and char != '}') or \
                   (last == '[' and char != ']') or \
                   (last == '(' and char != ')'):
                    return f"Mismatched bracket '{char}' at line {line_num}, expected closing for '{last}' from line {last_line}"
    if brackets:
        return f"Unclosed brackets: {brackets}"
    return "OK"

with open("/var/folders/ql/2vxg05fj5fb4t49qdv74j0hm0000gp/T/tmp480ozf6r.js") as f:
    print(check_brackets(f.read()))
