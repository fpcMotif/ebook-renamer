# Formatting Quick Reference

## Output Format
```
Author, Author - Title (Year).ext
```

## Processing Pipeline

1. Remove extension & `.download` suffix
2. Remove series prefixes (early, before other cleaning)
3. Remove ALL `[...]` bracketed content
4. Remove source markers (Z-Library, libgen, Anna's Archive) - **case insensitive**
5. Remove extended noise:
   - Editions: `2nd Edition`, `Revised Edition`
   - Versions: `v1.0`, `Version 2`
   - Language tags: `(English)`, `(中文版)`
   - Quality markers: `OCR`, `Scanned`
   - Academic IDs: `arXiv:1234.5678`, `doi:...`, `ISBN...`
6. Remove duplicate markers: `-2`, `(1)`, `Copy 1`
7. Extract year (last 19xx/20xx found)
8. Remove parentheticals with:
   - Year patterns: `(YYYY, Publisher)` or `(YYYY)`
   - Publisher keywords - **case insensitive**
   - Nested parentheticals: `(Pure and Applied (Academic Press))`
   - Keep author names at end
9. **Validate and fix brackets** (ensure all brackets properly paired)
10. Parse author/title using patterns:
    - `Title (Author)` → trailing author
    - `Author - Title` → dash separator
    - `Author: Title` → colon separator
    - `Author1, Author2 - Title` → multi-author
11. Clean author: smart comma handling
    - `Marco, Grandis` → `Marco Grandis` (join if both single words)
    - `Thomas H. Wolff, Izabella Aba, Carol Shubin` → keep ALL commas
12. Clean title: remove orphaned brackets, multiple spaces, trailing punctuation
13. Generate: `Author - Title (Year).ext`

## Key Removals

**Remove (Case Insensitive):**
- `[Lecture notes]`, `[Series info]` (all brackets)
- `(Pure and Applied Mathematics)`, `(Academic Press)`, `(springer)` (publisher info)
- `- Z-Library`, `- z-library`, `(LIBGEN)` (source markers)
- `-B0F5TFL6ZQ`, `-9780262046305` (trailing ID noise)
- `(auth.)` patterns
- `(YYYY, Publisher)` → keep only `(YYYY)`
- **NEW:** `2nd Edition`, `v1.0` (editions/versions)
- **NEW:** `(English)`, `(中文版)` (language tags)
- **NEW:** `OCR`, `Scanned` (quality markers)
- **NEW:** `arXiv:1234.5678`, `ISBN...` (academic IDs)
- **NEW:** Orphaned/unmatched brackets

**Keep:**
- Author names in parentheses at end
- CJK/non-Latin author names (detected by Unicode letters)
- Year in format `(YYYY)`
- Original capitalization
- Commas in multi-author lists (ALL commas preserved)

## Publisher Keywords (Case Insensitive)
```
press, publishing, publisher,
springer, cambridge, oxford, mit press, elsevier, wiley, pearson,
series, textbook series, lecture notes,
graduate texts, graduate studies,
pure and applied, foundations of,
monographs, studies, collection,
textbook, edition, revised, reprint,
vol., volume, no., part,
出版社, 出版, 教材, 系列, 丛书, 讲义, 版, 修订版 (Chinese)
の (Japanese)
```

**Edition Pattern:** `\d+(st|nd|rd|th)\s+ed(ition)?` (auto-detected)

## Author Rules
- **Detection:** Uppercase Latin letter OR non-Latin alphabetic characters (not just digits)
- **Multi-author:** Keep ALL commas: `Author1, Author2, Author3`
- **Single-word comma:** Join only if both sides are single words: `Marco, Grandis` → `Marco Grandis`
- **Multi-word comma:** Keep as-is: `Smith, John` or `Thomas H. Wolff, Izabella Aba`
- **CJK authors:** Recognized by non-Latin characters: 苏阳, 傅柯, etc.

## Regex Patterns

```regex
Year: \b(19|20)\d{2}\b
Brackets: \[[^\]]*\]
Trailing ID: [-_][A-Za-z0-9]{8,}$
Parentheticals (simple): \([^)]+\)
Parentheticals (nested): \([^()]*(?:\([^()]*\)[^()]*)*\)
Source markers: [zZ]-?Library|libgen|Anna'?s?\s+Archive
Multiple spaces: \s{2,}
```

## Quick Examples

| Input | Output |
|-------|--------|
| `Title (Author).pdf` | `Author - Title.pdf` |
| `Title [Notes] (Author).pdf` | `Author - Title.pdf` |
| `Title (A1, A2, A3).pdf` | `A1, A2, A3 - Title.pdf` |
| `Title (Marco, Grandis).pdf` | `Marco Grandis - Title.pdf` |
| `Title (Series) (2020).pdf` | `Title (2020).pdf` |
| `Title (springer press).pdf` | `Title.pdf` |
| `Title-ASIN123.pdf` | `Title.pdf` |
| `Title 2nd Edition.pdf` | `Title.pdf` |
| `Title (中文版).pdf` | `Title.pdf` |
| `Title with ( orphan.pdf` | `Title with orphan.pdf` |
| `标题 (作者).pdf` | `作者 - 标题.pdf` |

