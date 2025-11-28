# Naming Rules Analysis and Improvement Proposal

## Current Implementation Analysis

### Strengths ✓
1. **Good Source Cleaning**: Effectively removes Z-Library, libgen, Anna's Archive markers
2. **Year Extraction**: Finds rightmost year (usually publication year)
3. **Multi-author Support**: Preserves comma-separated author lists
4. **CJK Support**: Recognizes non-Latin author names
5. **Nested Cleanup**: Removes nested publisher information

### Critical Issues ❌

#### 1. **Series Information Loss**
**Problem**: Series metadata is completely discarded
```
Input:  "Graduate Texts in Mathematics 52 - Saunders Mac Lane - Categories for the Working Mathematician (1978).pdf"
Output: "Saunders Mac Lane - Categories for the Working Mathematician (1978).pdf"
Lost:   Series name AND volume number
```

**Why this matters**:
- Series volume numbers are important for reading order
- Users organize collections by series
- Calibre and other ebook managers use series metadata
- Examples: GTM 52, Springer Undergraduate Mathematics Series, etc.

**Proposed Solution**:
```
Option A (Recommended): Author - Title [Series Volume] (Year).ext
  "Saunders Mac Lane - Categories for the Working Mathematician [GTM 52] (1978).pdf"

Option B (Calibre-style): Author - Title (Year) [Series Volume].ext
  "Saunders Mac Lane - Categories for the Working Mathematician (1978) [GTM 52].pdf"

Option C (Verbose): Author - Title [Series Name Volume] (Year).ext
  "Saunders Mac Lane - Categories for the Working Mathematician [Graduate Texts in Mathematics 52] (1978).pdf"
```

#### 2. **No Edition Detection**
**Problem**: Edition information not preserved or standardized
```
Input:  "Advanced Calculus - 2nd Edition - James Munkres (2018).pdf"
Output: "James Munkres - Advanced Calculus (2018).pdf"
Lost:   Edition number
```

**Proposed Solution**:
- Detect: "2nd edition", "Second Edition", "3rd ed.", etc.
- Normalize to: `(Year, 2nd ed)` or `(2nd ed, Year)` or `[2nd ed]`
- Example: `"James Munkres - Advanced Calculus (2018, 2nd ed).pdf"`

#### 3. **Volume/Part Numbers Lost**
**Problem**: Multi-volume works lose volume information
```
Input:  "Spivak - Differential Geometry Vol 2.pdf"
Output: "Spivak - Differential Geometry.pdf"  (Volume 2 lost!)
```

**Proposed Solution**:
- Preserve "Vol N", "Volume N", "Part N" in title or as suffix
- Example: `"Spivak - Differential Geometry Vol 2.pdf"`
- Or: `"Spivak - Differential Geometry [Vol 2].pdf"`

#### 4. **Ambiguous Multi-Author Comma Handling**
**Problem**: Current logic is complex and error-prone
- "Marco, Grandis" → "Marco Grandis" (single-word comma removal)
- "Smith, John" → "Smith, John" (lastname-firstname preservation)
- Logic depends on word count, which can fail

**Edge Cases**:
```
"Smith, J." → "Smith J." (incorrect - should keep comma)
"von Neumann, John" → "von Neumann John" (incorrect!)
```

**Proposed Solution**:
- **Keep ALL commas** for simplicity and accuracy
- Let users manually fix obvious typos if needed
- OR: Use more sophisticated name parsing library

#### 5. **Year Ambiguity**
**Problem**: Always takes LAST year, but this can be wrong
```
Input:  "Foundations of Mathematics (1934) - 2020 Reprint.pdf"
Current: Extracts 2020 (reprint year, not original publication)
Wanted:  Should probably use 1934 (original publication)
```

**Mitigation**: No perfect solution, but could:
- Prioritize years in parentheses over bare years
- Check for "reprint", "revised" keywords
- Default to earliest year if multiple in parentheses

#### 6. **Publisher Suffix Removal Can Break Titles**
**Problem**: Aggressive publisher removal can corrupt titles
```
Input:  "Machine Learning - Stanford Approach.pdf"
Current: Might remove "Stanford" if detected as publisher keyword
```

**Mitigation**: Use stricter publisher detection at title end

---

## Proposed Naming Convention

### Standard Format (Recommended)
```
Author(s) - Title [Series Volume] (Year, Edition).ext
```

### Examples
```
✓ Simple:
  "Terry Tao - Analysis I (2016).pdf"

✓ With series:
  "Saunders Mac Lane - Categories for the Working Mathematician [GTM 52] (1978).pdf"

✓ With edition:
  "James Munkres - Topology (2000, 2nd ed).pdf"

✓ Multi-author:
  "Ernst Kunz, Richard G. Belshoff - Introduction to Plane Algebraic Curves (2005).pdf"

✓ With series AND edition:
  "John Lee - Introduction to Smooth Manifolds [GTM 218] (2012, 2nd ed).pdf"

✓ Multi-volume:
  "Michael Spivak - Differential Geometry Vol 2 (1979).pdf"

✓ CJK author:
  "苏阳 - 文革时期中国农村的集体杀戮 (1989).pdf"

✓ No author:
  "Algebraic Geometry Notes [Lecture Notes] (2020).pdf"
```

### Series Abbreviation Table
Common series should use short abbreviations:
```
GTM  = Graduate Texts in Mathematics
SUMS = Springer Undergraduate Mathematics Series
LMSLN = London Mathematical Society Lecture Note Series
PM   = Progress in Mathematics
GSAM = Cambridge Studies in Advanced Mathematics
AMS  = American Mathematical Society
```

---

## Implementation Strategy

### Phase 1: Core Improvements (High Priority)
1. ✅ **Series Detection & Preservation**
   - Regex patterns for common series
   - Extract series name + volume number
   - Map to abbreviations
   - Append as `[Series Vol]`

2. ✅ **Edition Detection**
   - Patterns: `\d+(st|nd|rd|th)\s+(ed\.|edition)`
   - Normalize: `"2nd ed"`
   - Append to year: `(Year, Edition)`

3. ✅ **Volume/Part Preservation**
   - Keep "Vol N", "Volume N", "Part N" in title
   - Alternative: Move to suffix `[Vol N]`

### Phase 2: Edge Case Handling (Medium Priority)
4. 🔄 **Smarter Year Selection**
   - Prioritize parenthesized years
   - Detect reprint/revised keywords
   - Fallback to rightmost year

5. 🔄 **Improved Author Name Parsing**
   - Keep all commas by default
   - Better lastname-firstname detection
   - Handle "von", "de", "van" prefixes

### Phase 3: Advanced Features (Low Priority)
6. 🔮 **Publisher Retention (Optional)**
   - Flag: `--keep-publisher`
   - Format: `Author - Title (Year, Publisher).ext`

7. 🔮 **Translator Information (Optional)**
   - Detect "translated by X"
   - Format: `Author - Title (trans. X) (Year).ext`

---

## Calibre Integration

If you use Calibre, consider these mappings:
```
Filename:        "Author - Title [Series Vol] (Year, Edition).ext"
Calibre import:  Author → Author
                 Title → Title
                 Series → Extracted from [Series Vol]
                 Year → Year
                 Comments → Edition
```

---

## Configuration Options (Future)

```toml
[naming]
format = "author-title-series-year"  # or custom template

[series]
abbreviate = true
abbreviations = { "Graduate Texts in Mathematics" = "GTM" }

[edition]
include = true
format = "short"  # "2nd ed" vs "Second Edition"

[year]
prefer_earliest = false  # Use rightmost (default) or leftmost

[author]
preserve_all_commas = true
transliterate_cjk = false
```

---

## Cross-Language Implementation Checklist

For behavioral parity, ALL implementations must:
- [ ] Use SAME series abbreviation table
- [ ] Use SAME regex patterns for detection
- [ ] Use SAME ordering: `[Series]` before `(Year)`
- [ ] Sort JSON output identically
- [ ] Handle edge cases identically

### Test Cases Required
```rust
// Series detection
"Graduate Texts in Mathematics 218 - John Lee - Introduction to Smooth Manifolds.pdf"
  → "John Lee - Introduction to Smooth Manifolds [GTM 218].pdf"

// Edition detection
"Topology - 2nd Edition - James Munkres (2000).pdf"
  → "James Munkres - Topology (2000, 2nd ed).pdf"

// Volume preservation
"Spivak - Differential Geometry Vol 2.pdf"
  → "Michael Spivak - Differential Geometry Vol 2.pdf"

// Combined
"Graduate Studies in Mathematics Vol. 147 - John Roe - Lectures on Coarse Geometry (2nd Edition, 2020).pdf"
  → "John Roe - Lectures on Coarse Geometry [GSM 147] (2020, 2nd ed).pdf"
```

---

## Migration Path

**Option A: Breaking Change**
- Implement all improvements at once
- Bump version to 2.0
- Document migration in CHANGELOG
- Provide script to rename existing libraries

**Option B: Gradual Rollout**
- Add `--naming-style v2` flag
- Default to v1 for backward compatibility
- Deprecate v1 in future version

**Option C: Configurable (Recommended)**
- Add config file support
- Let users choose features:
  - `series_preservation = true/false`
  - `edition_detection = true/false`
  - `volume_preservation = true/false`
- Default: preserve all metadata

---

## Questions for User

Before implementing, please decide:

1. **Series format preference?**
   - A: `[GTM 52]` (abbreviated, compact)
   - B: `[Graduate Texts in Mathematics 52]` (full name)
   - C: User-configurable

2. **Edition format preference?**
   - A: `(2020, 2nd ed)`
   - B: `(2nd ed, 2020)`
   - C: `[2nd ed] (2020)`

3. **Volume handling?**
   - A: Keep in title: `"Title Vol 2"`
   - B: Move to suffix: `"Title [Vol 2]"`

4. **Breaking change acceptable?**
   - Yes: Implement immediately, bump to v2.0
   - No: Add as opt-in feature with config

5. **Comma preservation?**
   - A: Keep all commas (simple, safe)
   - B: Keep current smart logic
   - C: Make configurable
