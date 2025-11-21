# Formatting Quick Reference

## Output Format
```
Author, Author - Title (Year).ext
```

## Processing Pipeline

1. Remove `.download` + duplicate extensions, trim whitespace
2. Normalize brackets（平衡 `()`、`[]`，把 `_` 变空格）确保结构健康
3. Remove series prefixes
4. Clean structured noise（来源/ID/复制与格式标签）
5. Remove ALL `[...]` bracketed content
6. Extract year（最后一个 19xx/20xx）
7. Remove parentheticals with:
   - Year patterns: `(YYYY, Publisher)` or `(YYYY)`
   - Publisher keywords（需命中关键词且不是作者括号）
   - Nested publisher blocks: `(Pure and Applied (Academic Press))`
8. Parse author/title using patterns:
   - `Title (Author)` → trailing author
   - `Author - Title` → dash separator
   - `Author: Title` → colon separator
   - `Author1, Author2 - Title` → multi-author
9. Clean author: smart comma handling
   - `Marco, Grandis` → `Marco Grandis` (join if both single words)
   - `Thomas H. Wolff, Izabella Aba, Carol Shubin` → keep commas (multi-author)
10. Clean title: 再次 bracket 规范化、去 `(auth.)`、ID、尾部标点
11. Generate: `Author - Title (Year).ext`

## Key Removals

**Remove:**
- `[Lecture notes]`, `[Series info]`, `[中文版]`（全部方括号说明）
- `(Pure and Applied Mathematics (Academic Press))`, `(Springer)`, `(English Edition)`
- `- Z-Library`, `(libgen)`, `-- hash --`, `-B0F5TFL6ZQ`, `ISBN 978...`
- `(auth.)` / `(translator)`、`Copy (1)`、`(scan)` 等噪声
- `(YYYY, Publisher)` → 只保留 `(YYYY)`

**Keep:**
- Author names in parentheses at end
- CJK/non-Latin author names (detected by Unicode letters)
- Year in format `(YYYY)`
- Original capitalization
- Commas in multi-author lists

## Publisher Keywords（命中任一 + 非作者括号才删除）
```
Press, Publishing, Academic Press, Springer, Cambridge, Oxford, MIT Press,
Series, Textbook Series, Graduate Texts, Graduate Studies, Lecture Notes,
Pure and Applied, Monographs, Collection, Textbook, Edition, Vol., Volume, No.,
Part, Verlag, Universitätsverlag, Université, 学, 出版社, 版社
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

