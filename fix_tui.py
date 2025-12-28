import re

with open('src/tui.rs', 'r') as f:
    content = f.read()

# Fix the broken replacements
content = re.sub(
    r'\.map\(\|c\| c\.symbol\(\)\)\.unwrap_or\(" "\)\.symbol\(\)',
    r'.map(|x| buffer.cell(x, y).map(|c| c.symbol()).unwrap_or(" "))',
    content
)

# Fix line with double .unwrap_or
content = re.sub(
    r'\.map\(\|x\| buffer\.cell\(x, y\)\.map\(\|c\| c\.symbol\(\)\)\.unwrap_or\(" "\)\)',
    r'.map(|x| buffer.cell(x, y).map(|c| c.symbol()).unwrap_or(" "))',
    content
)

with open('src/tui.rs', 'w') as f:
    f.write(content)

print("Fixed tui.rs")
