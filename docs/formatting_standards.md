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

### 3. What to Remove

#### Bracketed Annotations (Remove ALL)
- `[Lecture notes]` → remove
- `[Series info]` → remove
- `[Graduate Texts in Mathematics]` → remove
- Any content in square brackets `[...]`

#### Publisher/Series Info (Remove ALL)
- `(Pure and Applied Mathematics (Academic Press))` → remove
- `(Springer)` → remove
- `(Cambridge University Press)` → remove
- `(Foundations of Computer Science)` → remove
- Any parenthetical containing publisher keywords
- **NOTE:** Matching is case-insensitive.

#### Source Markers (Remove ALL)
- `- Z-Library`
- `- libgen.li`
- `- Anna's Archive`
- `(Z-Library)`
- `(libgen)`
- Any variation of library/source names

#### Trailing ID Noise (Remove ALL)
- Amazon ASINs: `-B0F5TFL6ZQ` → remove
- ISBN-like: `-9780262046305` → remove
- Pattern: `[-_]` followed by 8+ alphanumeric characters at end of filename
- Only strip if it appears **after** the title/author portion

#### Other Patterns to Remove
- **Version Info:** `v1.0`, `version 2.0`, `3rd Edition`
- **Page Counts:** `500 pages`, `200pp`
- **Language Tags:** `English Edition`, `Chinese Edition`
- `(auth.)` or `(Auth.)` → remove
- `.download` suffix → remove
- Multiple spaces → single space
- Leading/trailing punctuation (dash, colon, comma, semicolon, period)

### 4. Publisher/Series Detection Keywords

If parenthetical content contains any of these keywords (case-insensitive), remove it:

**Publishers:**
```
Press, Publishing, Academic Press, Springer, Cambridge, Oxford, MIT Press,
Wiley, Pearson, McGraw-Hill, Elsevier, Taylor & Francis
```

**General Types:**
```
Fiction, Novel, Handbook, Manual, Guide, Reference,
Cookbook, Workbook, Encyclopedia, Dictionary, Atlas, Anthology,
Biography, Memoir, Essay, Poetry, Drama, Short Stories
```

**Academic Types:**
```
Thesis, Dissertation, Proceedings, Conference, Symposium, Workshop,
Report, Technical Report, White Paper, Preprint, Manuscript,
Lecture, Course Notes, Study Guide, Solutions Manual
```

**Series/Editions:**
```
Series, Textbook Series, Graduate Texts, Graduate Studies, Lecture Notes,
Pure and Applied, Mathematics, Foundations of, Monographs, Studies, Collection,
Textbook, Edition, Vol., Volume, No., Part,
Revised Edition, Updated Edition, Expanded Edition,
Abridged, Unabridged, Complete Edition, Anniversary Edition,
Collector's Edition, Special Edition, 1st ed, 2nd ed, 3rd ed
```

**Format/Quality:**
```
OCR, Scanned, Retail, Searchable, Bookmarked, Optimized,
Compressed, High Quality, HQ, DRM-free, No DRM, Cracked,
Kindle Edition, PDF version, EPUB version, MOBI version
```

**Languages (Chinese/Japanese):**
```
理工, 出版社, 小说, 教材, 教程, 手册, 指南, 参考书, 文集, 论文集,
丛书, 系列, 修订版, 第二版, 第三版, 增订版
小説, 教科書, テキスト, ハンドブック, マニュアル, ガイド,
講義, シリーズ, 改訂版, 第2版, 第3版, の
English, Chinese, Japanese
```

Also remove if:
- Contains numbers with multiple non-letter characters (likely series info)
- Matches pattern: `(YYYY, Publisher)` where YYYY is a year
- Matches Version pattern (`v1.0`) or Page Count pattern (`500 pages`)

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

1. **Remove extension** (.pdf, .epub, .txt, .download)
2. **Clean noise sources** (Z-Library, libgen, Anna's Archive patterns)
3. **Remove ALL bracketed annotations** `[...]`
4. **Extract year** (find last occurrence of 19xx/20xx)
5. **Remove parentheticals** containing:
   - Year patterns: `(YYYY, Publisher)` or `(YYYY)`
   - Publisher/series keywords (expanded list)
   - But preserve author names at the end
6. **Parse author and title** using smart pattern matching
7. **Clean author name** (handle commas, remove (auth.) patterns)
8. **Clean title** (remove orphaned brackets, multiple spaces, trailing punctuation, version info, page counts)
9. **Generate final filename**: `Author - Title (Year).ext`

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
- [ ] Publisher keyword detection (Comprehensive list, Case-insensitive)
- [ ] Author validation logic
- [ ] Comma handling for author names
- [ ] Multiple space cleanup
- [ ] Trailing punctuation removal
- [ ] Version and Page Count removal

## Test Cases

Use these examples to validate implementation:

### Basic Cases
1. `Differential Geometry (Paulo Ventura Araujo).pdf`
   → `Paulo Ventura Araujo - Differential Geometry.pdf`

2. `Category Theory Course [Lecture notes] (John Baez).pdf`
   → `John Baez - Category Theory Course.pdf`

3. `Algebraic Topology - A Structural Introduction (Marco Grandis).pdf`
   → `Marco Grandis - Algebraic Topology - A Structural Introduction.pdf`

### Improvements Verification (New Cases)
4. `Great Novel (Fiction) (John Doe).pdf`
   → `John Doe - Great Novel.pdf` (Type removal)

5. `Learn Python (3rd Edition) (2023).pdf`
   → `Learn Python (2023).pdf` (Edition removal)

6. `Book Title (OCR) (Searchable) (Author).pdf`
   → `Author - Book Title.pdf` (Format removal)

7. `故事集 (小说) (作者).pdf`
   → `作者 - 故事集.pdf` (Chinese type removal)

8. `Title libgen.li.pdf`
   → `Title.pdf` (Noise cleanup)

9. `Software Manual v2.0.pdf`
   → `Software Manual.pdf` (Version pattern)

10. `Huge Book 500 pages.pdf`
    → `Huge Book.pdf` (Page count pattern)

## Notes

- **Preserve non-Latin scripts** (CJK, Arabic, etc.) as-is when using `--preserve-unicode` flag
- **Year is optional** - only include if found in original filename
- **Author is optional** - if no clear author pattern, use title only
- **Case sensitivity:** Preserve original capitalization of author names and titles
- **Whitespace:** Normalize multiple spaces to single space, trim leading/trailing
