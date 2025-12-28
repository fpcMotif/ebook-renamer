with open('src/tui.rs', 'r') as f:
    lines = f.readlines()

# Find and fix the broken lines
new_lines = []
for i, line in enumerate(lines):
    # Fix line 347 - the debug output loop
    if '.map(|x| buffer.cell(x, y).map(|x| buffer.cell(x, y).map(|c| c.symbol()).unwrap_or(" "))' in line:
        new_lines.append('                .map(|x| buffer.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))\n')
    # Fix line 376 - assert_area_contains_str
    elif '.map(|x| buffer.cell(x, y).map(|c| c.symbol()).unwrap_or(" ").symbol())' in line:
        new_lines.append('                .map(|x| buffer.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))\n')
    # Fix line 391 - assert_line_style
    elif '.map(|x| buffer.cell(x, y).map(|c| c.symbol()).unwrap_or(" ')) in line:
        new_lines.append('                .map(|x| buffer.cell((x, y)).unwrap_or(&ratatui::buffer::Cell::default()))\n')
    else:
        new_lines.append(line)

with open('src/tui.rs', 'w') as f:
    f.writelines(new_lines)

print("Fixed tui.rs properly")
