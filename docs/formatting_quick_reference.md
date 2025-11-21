# Formatting Quick Reference

## Output Format
```
Author, Author - Title (Year).ext
```

## Processing Pipeline

1. Remove extension & `.download` suffix
2. Remove series prefixes (London Mathematical Society Lecture Note Series, etc.)
3. Remove ALL `[...]` bracketed content
4. Remove source markers (Z-Library, libgen, Anna's Archive)
5. Remove duplicate markers (-2, -3, (1), (2), etc.)
6. Extract year (last 19xx/20xx found)
7. **Clean orphaned brackets** (remove unclosed parentheses/brackets and their content)
8. Remove parentheticals with:
   - Year patterns: `(YYYY, Publisher)` or `(YYYY)`
   - Publisher keywords (Press, Series, Academic Press, Textbook Series, etc.)
   - Nested parentheticals: `(Pure and Applied (Academic Press))`
   - Unclosed parentheses at the end
   - Keep author names at end
9. Parse author/title using patterns:
   - `Title (Author)` → trailing author
   - `Author - Title` → dash separator
   - `Author: Title` → colon separator
   - `Author1, Author2 - Title` → multi-author
10. Clean author: smart comma handling
   - `Marco, Grandis` → `Marco Grandis` (join if both single words)
   - `Thomas H. Wolff, Izabella Aba, Carol Shubin` → keep commas (multi-author)
11. Clean title: remove orphaned brackets, source markers, publisher keywords, multiple spaces, trailing punctuation
12. Generate: `Author - Title (Year).ext`

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

