# Formatting Quick Reference

## Output Format
```
Author, Author - Title (Year).ext
```

## Processing Pipeline

1. Remove extension & `.download` suffix
2. Remove source markers (Z-Library, libgen, Anna's Archive)
3. Remove ALL `[...]` bracketed content
4. Extract year (last 19xx/20xx found)
5. Remove trailing ID noise (Amazon ASINs: `-B0F5TFL6ZQ`, `-9780262046305`, etc.)
6. Remove parentheticals with:
   - Year patterns: `(YYYY, Publisher)` or `(YYYY)`
   - Publisher keywords (Press, Series, Academic Press, Textbook Series, etc.)
   - Nested parentheticals: `(Pure and Applied (Academic Press))`
   - Keep author names at end
7. Parse author/title using patterns:
   - `Title (Author)` → trailing author
   - `Author - Title` → dash separator
   - `Author: Title` → colon separator
   - `Author1, Author2 - Title` → multi-author
8. Clean author: smart comma handling
   - `Marco, Grandis` → `Marco Grandis` (join if both single words)
   - `Thomas H. Wolff, Izabella Aba, Carol Shubin` → keep commas (multi-author)
9. Clean title: remove orphaned brackets, multiple spaces, trailing punctuation
10. Generate: `Author - Title (Year).ext`

## Key Removals

**Remove:**
- `[Lecture notes]`, `[Series info]` (all brackets)
- `(Pure and Applied Mathematics)`, `(Academic Press)` (publisher info)
- `- Z-Library`, `(libgen)` (source markers)
- `-B0F5TFL6ZQ`, `-9780262046305` (trailing ID noise)
- `(auth.)` patterns
- `(YYYY, Publisher)` → keep only `(YYYY)`

**Keep:**
- Author names in parentheses at end
- CJK/non-Latin author names (detected by Unicode letters)
- Year in format `(YYYY)`
- Original capitalization
- Commas in multi-author lists

## Publisher Keywords
```
Press, Publishing, Academic Press, Springer, Cambridge, Oxford, MIT Press,
Series, Textbook Series, Graduate Texts, Graduate Studies, Lecture Notes,
Pure and Applied, Mathematics, Foundations of, Monographs, Studies, Collection,
Textbook, Edition, Vol., Volume, No., Part
```

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
| `Title-ASIN123.pdf` | `Title.pdf` |
| `标题 (作者).pdf` | `作者 - 标题.pdf` |

