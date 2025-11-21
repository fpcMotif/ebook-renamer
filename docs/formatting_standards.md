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

#### Publisher/Series Info (Remove ALL - **Case Insensitive**)
- `(Pure and Applied Mathematics (Academic Press))` → remove
- `(Springer)` → remove
- `(Cambridge University Press)` → remove
- `(springer press)` → remove (lowercase also detected)
- `(TEXTBOOK SERIES)` → remove (uppercase also detected)
- `(Foundations of Computer Science)` → remove
- Any parenthetical containing publisher keywords

#### Source Markers (Remove ALL - **Case Insensitive**)
- `- Z-Library`, `- z-library`, `- Z-LIBRARY`
- `- libgen.li`, `- LIBGEN`
- `- Anna's Archive`, `- anna's archive`
- `(Z-Library)`, `(z-Library)`
- `(libgen)`, `(LIBGEN)`
- Any variation of library/source names

#### Edition and Version Info (Remove ALL - **NEW**)
- `2nd Edition`, `3rd edition` → remove
- `(Revised Edition)` → remove
- `v1.0`, `Version 2` → remove
- `(Reprint 2020)` → remove

#### Language and Quality Markers (Remove ALL - **NEW**)
- `(English Version)`, `(Chinese Version)` → remove
- `(中文版)`, `(英文版)` → remove
- `OCR`, `Scanned`, `Watermarked` → remove

#### Academic Identifiers (Remove ALL - **NEW**)
- `arXiv:1234.5678` → remove
- `doi:10.1234/example` → remove
- `ISBN-123-456-789` → remove

#### Trailing ID Noise (Remove ALL)
- Amazon ASINs: `-B0F5TFL6ZQ` → remove
- ISBN-like: `-9780262046305` → remove
- Pattern: `[-_]` followed by 8+ alphanumeric characters at end of filename
- Only strip if it appears **after** the title/author portion

#### Duplicate Markers (Remove ALL)
- `Copy 1`, `Copy 2` → remove
- `(1)`, `(2)` at end → remove
- `-2`, `-3` at end → remove

#### Other Patterns to Remove
- `(auth.)` or `(Auth.)` → remove
- `.download` suffix → remove
- Multiple spaces → single space
- Leading/trailing punctuation (dash, colon, comma, semicolon, period)
- **Orphaned brackets** → removed or paired correctly

### 4. Publisher/Series Detection Keywords (**Case Insensitive**)

If parenthetical content contains any of these keywords, remove it:

```
press, publishing, publisher,
springer, cambridge, oxford, mit press, elsevier, wiley, pearson,
series, textbook series, lecture notes,
graduate texts, graduate studies,
pure and applied, foundations of,
monographs, studies, collection,
textbook, edition, revised, reprint,
vol., volume, no., part,
出版社, 出版, 教材, 系列, 丛书, 讲义, 版, 修订版  (Chinese)
の  (Japanese)
```

**Edition Pattern Detection:**
- `2nd ed`, `3rd edition`, `1st Edition` → automatically detected
- Pattern: `\d+(st|nd|rd|th)\s+ed(ition)?`

Also remove if:
- Contains numbers with multiple non-letter characters (likely series info)
- Matches pattern: `(YYYY, Publisher)` where YYYY is a year
- Contains edition patterns (case insensitive)

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
2. **Remove series prefixes** (early, before other cleaning)
3. **Remove ALL bracketed annotations** `[...]`
4. **Clean noise sources** (Z-Library, libgen, Anna's Archive patterns)
5. **Clean extended noise** (editions, versions, language tags, quality markers)
6. **Remove duplicate markers** (-2, -3, (1), (2))
7. **Extract year** (find last occurrence of 19xx/20xx)
8. **Remove parentheticals** containing:
   - Year patterns: `(YYYY, Publisher)` or `(YYYY)`
   - Publisher/series keywords
   - But preserve author names at the end
9. **Validate and fix brackets** (ensure all brackets are properly paired)
10. **Parse author and title** using smart pattern matching
11. **Clean author name** (handle commas, remove (auth.) patterns)
12. **Clean title** (remove orphaned brackets, multiple spaces, trailing punctuation)
13. **Generate final filename**: `Author - Title (Year).ext`

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
"Title (springer press)" → Remove (case insensitive)
```

#### Orphaned Brackets (**NEW**)
```
"Title with orphan ( bracket" → "Title with orphan bracket"
"Title (Series (Publisher)) Author)" → "Author - Title" (extra ) removed)
"Title ((nested" → "Title" (orphaned brackets removed)
```

#### Comma Handling
```
"Marco, Grandis" → "Marco Grandis" (both single words)
"Smith, John" → "Smith, John" (likely Lastname, Firstname - keep comma)
"Author1, Author2, Author3" → "Author1, Author2, Author3" (keep ALL commas)
```

#### Edition and Version Handling (**NEW**)
```
"Title - 2nd Edition (Author)" → "Author - Title"
"Title v1.0 (Author)" → "Author - Title"
"Title (Revised Edition)" → "Title"
```

#### Language Tags (**NEW**)
```
"Title (English Version) (Author)" → "Author - Title"
"Title (中文版)" → "Title"
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

