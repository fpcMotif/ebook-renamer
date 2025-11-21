# Ebook Filename Formatting Standards

This document outlines the key formatting rules and patterns used to normalize ebook filenames. These standards should be implemented consistently across all language implementations (Rust, Go, Python, Zig, etc.).

## Output Format

**Standard Format:** `Author, Author - Title (Year).pdf`

Examples:
- `Paulo Ventura Araujo - Differential Geometry.pdf`
- `John Baez - Category Theory Course.pdf`
- `Barry Mitchell - Theory of Categories.pdf`
- `Marco Grandis - Higher Dimensional Categories From Double To Multiple Categories.pdf`
- `Wolfgang Bietenholz, Uwe-Jens Wiese - Uncovering Quantum Field Theory and the Standard Model.pdf`
- `Jean-Pierre Serre - Local Fields (1979).pdf`

## Key Rules

### 1. Author Formatting

- **Single Author:** `FirstName LastName` or `LastName, FirstName` (keep as-is if already formatted)
- **Multiple Authors:** `Author1, Author2, Author3` (comma-separated, keep commas!)
- **CJK/Non-Latin Authors:** Recognized by non-Latin characters (Chinese, Japanese, Arabic, Cyrillic, etc.)
- **Smart Comma Handling:**
  - `Marco, Grandis` → `Marco Grandis` (**ONLY** if single word each side)
  - `Thomas H. Wolff, Izabella Aba, Carol Shubin` → `Thomas H. Wolff, Izabella Aba, Carol Shubin` (keep all commas)
  - `Smith, John` → `Smith, John` (keep if likely Lastname, Firstname format with 2+ words)
  - Multiple commas with multi-word parts: keep ALL commas
- **Author Detection:** Must have uppercase Latin letter OR non-Latin alphabetic characters (not just digits/punctuation)

### 2. Year Formatting

- **Format:** `(YYYY)` only - no publisher info
- **Extraction:** Find **last occurrence** of 19xx or 20xx pattern in filename
- **Removal:** Remove `(YYYY, Publisher)` → keep only year in final format
- **Placement:** Year goes **after title**, before extension
- **Examples:**
  - `(2005, Birkhäuser)` → `(2005)`
  - `(2013)` → `(2013)`
  - `2020, Publisher` → `(2020)`
  - `Deadly Decision in Beijing. ... (1989 Tiananmen Massacre)` → include `(1989)` at end

### 3. What to Remove（按类别）

#### 3.1 Bracket Normalization
- 先清理孤立 `(`、`)`、`[`、`]`，必要时直接删除无法闭合的部分
- 将 `_` 视为空格，之后再压缩多余空格

#### 3.2 Bracketed Annotations
- `[Lecture notes]`, `[Series info]`, `[Graduate Texts in Mathematics]`
- `[英文版]`, `[中文版]`, `[高清扫描]` 等语言/画质标记

#### 3.3 Source Markers
- `Z-Library`, `libgen`, `Anna's Archive`, `pdfdrive`, `ebook-dl`, `BookZZ`, `VK`, `mega`, `Calibre`, `Kindle`, `Scribd`
- 任意 `(<source>)`、`- <source>`、`<source>.pdf` 变体

#### 3.4 ID / Hash / 追踪信息
- Amazon ASINs：`-B0F5TFL6ZQ`
- ISBN：`-9780262046305`、`ISBN 978...`
- `-- 32位hash --`、末尾 8+ 位十六进制/字母数字 token

#### 3.5 Duplicate/Format Markers
- `Copy`, `copy`, `副本`, `(1)`, `(2)` 等重复下载序号
- `(scan)`, `(scanned)`, `(ocr)`, `(color)`, `(bw)`
- `(English Edition)`, `(Kindle Edition)`, `(EPUB)`, `(PDF)`, `(中文版)`

#### 3.6 Publisher/Series Parentheses
- 仅当括号内包含出版社关键词（见下）或“年份 + 出版社”模式才移除
- 纯作者括号或与标题含义强相关的内容必须保留

#### 3.7 其他噪声
- `(auth.)` / `(Auth.)`
- `.download` 或重复扩展名
- 多余空格、前后 `- : , ; .`

### 4. Publisher/Series Detection Keywords

移除括号内容需满足以下任一条件：
1. 含下列关键词之一并且**不是**结尾作者括号：
   ```
   Press, Publishing, Academic Press, Springer, Cambridge, Oxford, MIT Press,
   Series, Graduate Texts, Graduate Studies, Lecture Notes, Pure and Applied,
   Monographs, Collection, Textbook, Edition, Vol., Volume, No., Part,
   Verlag, Universitätsverlag, Université, 学, 出版社, 版社
   ```
2. 匹配 `(YYYY, <text>)` 或 `<text> (YYYY, <text>)`
3. 以明显的卷/册/编号结构存在（含数字且非字母字符 ≥2）

注意：一般词如 “Mathematics”、“Studies” 不再单独触发删除，除非与其他检测条件同时满足。

### 5. Author Detection Patterns

#### Pattern 1: Trailing Author in Parentheses
```
"Title (Author Name)" → Author: "Author Name", Title: "Title"
```

#### Pattern 2: Dash Separator
```
"Author Name - Title" → Author: "Author Name", Title: "Title"
```

#### Pattern 3: Colon Separator
```
"Author Name: Title" → Author: "Author Name", Title: "Title"
```

#### Pattern 4: Multiple Authors
```
"Author1, Author2 - Title" → Authors: "Author1, Author2", Title: "Title"
```

### 6. Author Validation Rules

An author string is valid if:
- Length >= 2 characters
- Contains at least one uppercase letter
- Does NOT contain: "auth.", "translator", "Z-Library", "libgen", "Anna's Archive"
- Does NOT match publisher/series keywords

### 7. Processing Order

1. Remove `.download` / 重复扩展名并裁空白
2. Normalize brackets（平衡括号、替换 `_`、压缩多余空格）
3. Remove series prefixes
4. Clean structured noise（来源、ID、复制/格式标签）
5. Remove ALL bracketed annotations `[...]`
6. Extract year（最后一个 19xx/20xx）
7. Remove含年份或 publisher 关键词的括号（保留作者括号）
8. Parse author & title（含 trailing author、dash/colon、multiple author、semicolon）
9. Clean author（逗号策略、去 `(auth.)`）
10. Clean title（再次 bracket 规范化、去 ID、裁标点）
11. Generate final filename

### 8. Edge Cases

#### Nested Parentheticals
```
"Theory (Pure and Applied (Academic Press)) (Author)"
→ Remove nested publisher info, keep author
→ Result: "Author - Theory"
```

#### Multiple Years
```
"Title (1995) (2005, Publisher)"
→ Use last year found: (2005)
```

#### Author vs Publisher Ambiguity
```
"Title (John Smith)" → Keep if looks like author name
"Title (Academic Press)" → Remove if contains publisher keywords
```

#### Comma Handling
```
"Marco, Grandis" → "Marco Grandis" (both single words)
"Smith, John" → "Smith, John" (likely Lastname, Firstname - keep comma)
"Author1, Author2, Author3" → "Author1 Author2 Author3" (multiple commas)
```

## Implementation Checklist

When implementing in other languages, ensure:

- [ ] Regex support for pattern matching
- [ ] String manipulation functions
- [ ] Year extraction (19xx/20xx pattern)
- [ ] Parenthetical matching (handles nested)
- [ ] Publisher keyword detection
- [ ] Author validation logic
- [ ] Comma handling for author names
- [ ] Multiple space cleanup
- [ ] Trailing punctuation removal

## Test Cases

Use these examples to validate implementation:

### Basic Cases
1. `Differential Geometry (Paulo Ventura Araujo).pdf`
   → `Paulo Ventura Araujo - Differential Geometry.pdf`

2. `Category Theory Course [Lecture notes] (John Baez).pdf`
   → `John Baez - Category Theory Course.pdf`

3. `Algebraic Topology - A Structural Introduction (Marco Grandis).pdf`
   → `Marco Grandis - Algebraic Topology - A Structural Introduction.pdf`

### Multi-Author Cases
4. `Lectures on harmonic analysis (Thomas H. Wolff, Izabella Aba, Carol Shubin).pdf`
   → `Thomas H. Wolff, Izabella Aba, Carol Shubin - Lectures on harmonic analysis.pdf`
   **Note:** Commas preserved for multi-author lists

5. `Uncovering Quantum Field Theory and the Standard Model (Wolfgang Bietenholz, Uwe-Jens Wiese).pdf`
   → `Wolfgang Bietenholz, Uwe-Jens Wiese - Uncovering Quantum Field Theory and the Standard Model.pdf`

6. `A supplement for Category theory for computing science (Michael Barr, Charles Wells).pdf`
   → `Michael Barr, Charles Wells - A supplement for Category theory for computing science.pdf`

### Single-Word Comma Case
7. `Higher Dimensional Categories From Double To Multiple Categories (Marco, Grandis).pdf`
   → `Marco Grandis - Higher Dimensional Categories From Double To Multiple Categories.pdf`
   **Note:** Single-word comma case is joined (unlike multi-author)

### Nested Publisher Info
8. `Theory of Categories (Pure and Applied Mathematics (Academic Press)) (Barry Mitchell).pdf`
   → `Barry Mitchell - Theory of Categories.pdf`
   **Note:** Nested publisher info removed

### Trailing ID Noise
9. `Math History A Long-Form Mathematics Textbook (The Long-Form Math Textbook Series)-B0F5TFL6ZQ.pdf`
   → `Math History A Long-Form Mathematics Textbook.pdf`
   **Note:** Series info and ASIN removed

### Non-Latin Authors (CJK)
10. `文革时期中国农村的集体杀戮 Collective Killings in Rural China during the Cultural Revolution (苏阳).pdf`
    → `苏阳 - 文革时期中国农村的集体杀戮 Collective Killings in Rural China during the Cultural Revolution.pdf`
    **Note:** CJK author recognized and positioned correctly

### Complex Cases with Years
11. `Deadly Decision in Beijing. Succession Politics, Protest Repression, and the 1989 Tiananmen Massacre (Yang Su).pdf`
    → `Yang Su - Deadly Decision in Beijing. Succession Politics, Protest Repression, and the 1989 Tiananmen Massacre (1989).pdf`
    **Note:** Year extracted from title context

12. `Local Fields (Jean-Pierre Serre).pdf`
    → `Jean-Pierre Serre - Local Fields.pdf`

13. `Tools for PDE Pseudodifferential Operators, Paradifferential Operators, and Layer Potentials (Michael E. Taylor).pdf`
    → `Michael E. Taylor - Tools for PDE Pseudodifferential Operators, Paradifferential Operators, and Layer Potentials.pdf`

14. `From Quantum Cohomology to Integrable Systems (Martin A. Guest).pdf`
    → `Martin A. Guest - From Quantum Cohomology to Integrable Systems.pdf`

15. `Bases cristallines des groupes quantiques (Masaki Kashiwara).pdf`
    → `Masaki Kashiwara - Bases cristallines des groupes quantiques.pdf`

## Notes

- **Preserve non-Latin scripts** (CJK, Arabic, etc.) as-is when using `--preserve-unicode` flag
- **Year is optional** - only include if found in original filename
- **Author is optional** - if no clear author pattern, use title only
- **Case sensitivity:** Preserve original capitalization of author names and titles
- **Whitespace:** Normalize multiple spaces to single space, trim leading/trailing

