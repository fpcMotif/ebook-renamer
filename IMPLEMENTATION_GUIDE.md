# Implementation Guide: Series/Edition/Volume Detection

This document guides implementation of series, edition, and volume detection across all language implementations to maintain cross-language behavioral parity.

## Overview

The Rust implementation (primary reference) has been updated with:
1. Series detection and preservation (e.g., `[GTM 52]`)
2. Edition detection and normalization (e.g., `2nd ed`)
3. Volume detection and normalization (e.g., `Vol 2`)

**Reference Implementation**: `src/normalizer.rs` (Rust)

---

## Changes Required

### 1. Update Parsed Metadata Structure

**Rust** (`src/normalizer.rs:6-13`):
```rust
pub struct ParsedMetadata {
    pub authors: Option<String>,
    pub title: String,
    pub year: Option<u16>,
    pub series: Option<String>,      // NEW
    pub edition: Option<String>,     // NEW
    pub volume: Option<String>,      // NEW
}
```

**Go** (`source_go/internal/types/types.go`):
```go
type ParsedMetadata struct {
    Authors *string
    Title   string
    Year    *int
    Series  *string  // NEW
    Edition *string  // NEW
    Volume  *string  // NEW
}
```

**Python** (`source_py/ebook_renamer/types.py`):
```python
@dataclass
class ParsedMetadata:
    title: str
    authors: Optional[str] = None
    year: Optional[int] = None
    series: Optional[str] = None    # NEW
    edition: Optional[str] = None   # NEW
    volume: Optional[str] = None    # NEW
```

---

### 2. Add Series Extraction Function

**Function**: `extract_series_info(s: &str) -> (Option<String>, String)`

**Logic**:
1. Define series mappings (name → abbreviation):
   ```
   Graduate Texts in Mathematics → GTM
   Cambridge Studies in Advanced Mathematics → CSAM
   London Mathematical Society Lecture Note Series → LMSLN
   Progress in Mathematics → PM
   Springer Undergraduate Mathematics Series → SUMS
   Graduate Studies in Mathematics → GSM
   AMS Mathematical Surveys and Monographs → AMS-MSM
   Oxford Graduate Texts in Mathematics → OGTM
   Springer Monographs in Mathematics → SMM
   ```

2. Match patterns (in order):
   - **Pattern 1**: `^Series Name (\d+) -` → Extract `Abbr Volume`, remove from string
   - **Pattern 2**: `^Series Name -` (no volume) → Remove series name only, no series field
   - **Pattern 3**: `^\(Series Name (\d+)\)` → Extract `Abbr Volume`
   - **Pattern 4**: `\[Series Name (\d+)\]` → Extract `Abbr Volume`

3. Return `(Option<series_info>, cleaned_string)`

**Rust Reference**: `src/normalizer.rs:89-164`

**Key Points**:
- Series name without volume → remove but don't set series field
- Series name with volume → extract as `[Abbr Vol]`
- Case-insensitive matching for series names

---

### 3. Add Edition Extraction Function

**Function**: `extract_edition(s: &str) -> (Option<String>, String)`

**Logic**:
1. Match patterns:
   - `(\d+)(?:st|nd|rd|th)\s+[Ee]dition`
   - `(\d+)(?:st|nd|rd|th)\s+[Ee]d\.?`
   - `[Ee]dition\s+(\d+)`

2. Extract edition number, normalize to `Nth ed`:
   - 1 → `1st ed`
   - 2 → `2nd ed`
   - 3 → `3rd ed`
   - 4+ → `4th ed`, `5th ed`, etc.

3. Remove edition text from string

4. Return `(Option<edition_info>, cleaned_string)`

**Rust Reference**: `src/normalizer.rs:166-196`

**Examples**:
- `"Topology - 2nd Edition"` → `("2nd ed", "Topology -")`
- `"Book Title 3rd ed."` → `("3rd ed", "Book Title")`

---

### 4. Add Volume Extraction Function

**Function**: `extract_volume(s: &str) -> (Option<String>, String)`

**Logic**:
1. Match patterns (with normalization flag):
   - `Vol\.?\s+(\d+)` → already normalized
   - `Volume\s+(\d+)` → needs normalization
   - `Part\s+(\d+)` → needs normalization

2. Extract volume number

3. If needs normalization, replace in string with `Vol N`

4. Return `(Option<"Vol N">, normalized_string)`

**Rust Reference**: `src/normalizer.rs:198-224`

**Examples**:
- `"Vol 2"` → `("Vol 2", "Vol 2")` (no change)
- `"Volume 2"` → `("Vol 2", "Vol 2")` (normalized)
- `"Part 3"` → `("Vol 3", "Vol 3")` (normalized)

**Important**: Volume info stays in title, not removed!

---

### 5. Update Parsing Flow

**Old flow**:
```
1. Remove extension
2. Remove series prefixes
3. Remove brackets
4. Clean noise
5. Remove duplicates
6. Extract year
7. Clean parentheticals
8. Parse author/title
```

**New flow**:
```
1. Remove extension
2. Extract series info (BEFORE bracket removal)  // NEW
3. Remove brackets
4. Clean noise
5. Remove duplicates
6. Extract edition                               // NEW
7. Extract year
8. Clean parentheticals
9. Extract volume                                // NEW
10. Parse author/title
```

**Rust Reference**: `src/normalizer.rs:40-87`

---

### 6. Update Filename Generation

**Old format**: `Author - Title (Year).ext`

**New format**: `Author - Title [Series] (Year, Edition).ext`

**Logic**:
```rust
fn generate_new_filename(metadata, extension):
    result = ""

    // Author
    if metadata.authors:
        result += metadata.authors + " - "

    // Title (with volume if present)
    result += metadata.title

    // Series in brackets
    if metadata.series:
        result += " [" + metadata.series + "]"

    // Year and/or Edition in parentheses
    if metadata.year AND metadata.edition:
        result += " (" + year + ", " + edition + ")"
    elif metadata.year:
        result += " (" + year + ")"
    elif metadata.edition:
        result += " (" + edition + ")"

    result += extension
    return result
```

**Rust Reference**: `src/normalizer.rs:697-730`

**Examples**:
- `("Author", "Title", 2020, "GTM 52", "2nd ed", None)`
  → `"Author - Title [GTM 52] (2020, 2nd ed).pdf"`
- `("Author", "Title Vol 2", 1979, None, None, "Vol 2")`
  → `"Author - Title Vol 2 (1979).pdf"`

---

## Testing Requirements

### Unit Tests to Add

1. **Series Extraction**:
   - `Graduate Texts in Mathematics 52 - Author - Title` → series = `"GTM 52"`
   - `(Cambridge Studies in Advanced Mathematics 218) Author - Title` → series = `"CSAM 218"`
   - `Graduate Texts in Mathematics - Author - Title` (no vol) → series = `None`

2. **Edition Detection**:
   - `Title - 2nd Edition` → edition = `"2nd ed"`
   - `Title 3rd ed.` → edition = `"3rd ed"`
   - `Title Edition 1` → edition = `"1st ed"`

3. **Volume Detection**:
   - `Title Volume 2` → volume = `"Vol 2"`, title normalized to `"Title Vol 2"`
   - `Title Vol 3` → volume = `"Vol 3"`, title unchanged
   - `Title Part 1` → volume = `"Vol 1"`, title normalized to `"Title Vol 1"`

4. **Filename Generation**:
   - All fields → `"Author - Title [GTM 52] (2020, 2nd ed).pdf"`
   - Series only → `"Author - Title [GTM 52] (2020).pdf"`
   - Edition only → `"Author - Title (2020, 2nd ed).pdf"`
   - Volume in title → `"Author - Title Vol 2 (1979).pdf"`

### Cross-Language Parity Tests

**CRITICAL**: Run `./tests/tools/test_cross_language.sh` after implementation!

Add these test files to `tests/fixtures/`:
```
Graduate Texts in Mathematics 218 - John Lee - Introduction to Smooth Manifolds (2012, 2nd Edition).pdf
James Munkres - Topology - 2nd Edition (2000).pdf
Michael Spivak - Differential Geometry Volume 2 (1979).pdf
(Cambridge Studies in Advanced Mathematics 184) Ciprian Demeter - Fourier Restriction (2020).pdf
```

Expected outputs:
```
John Lee - Introduction to Smooth Manifolds [GTM 218] (2012, 2nd ed).pdf
James Munkres - Topology (2000, 2nd ed).pdf
Michael Spivak - Differential Geometry Vol 2 (1979).pdf
Ciprian Demeter - Fourier Restriction [CSAM 184] (2020).pdf
```

---

## Language-Specific Notes

### Go (`source_go/`)

1. Update `internal/types/types.go` with new fields
2. Add functions to `internal/normalizer/normalizer.go`
3. Use `regexp.MustCompile()` for patterns
4. Pointer types: `*string` for optional fields
5. Test file: `internal/normalizer/normalizer_test.go`

**Pattern matching**:
```go
seriesRegex := regexp.MustCompile(`^Graduate Texts in Mathematics\s+(\d+)\s*[-\s]`)
if match := seriesRegex.FindStringSubmatch(s); match != nil {
    volume := match[1]
    series := "GTM " + volume
    // ...
}
```

### Python (`source_py/`)

1. Update `ebook_renamer/types.py` dataclass
2. Add functions to `ebook_renamer/normalizer.py`
3. Use `re` module for regex
4. Optional fields: `Optional[str]` with default `None`
5. Test file: `tests/test_normalizer.py`

**Pattern matching**:
```python
import re

series_pattern = r'^Graduate Texts in Mathematics\s+(\d+)\s*[-\s]'
match = re.match(series_pattern, s)
if match:
    volume = match.group(1)
    series = f"GTM {volume}"
    # ...
```

### Ruby, Haskell, OCaml, Zig

Follow same logic as above, adapting to language idioms.

**Ruby** (`source_rb/ebook-renamer.rb`):
- Use `Struct` for ParsedMetadata
- Regex with `/pattern/`
- Optional: `nil` values

**Haskell** (`source_hs/`):
- Use `Maybe` for optional fields
- Pattern matching for extraction
- Pure functions

**OCaml** (`source_ocaml/`):
- Use `option` type
- Pattern matching
- Immutable strings

**Zig** (`source_zig/`):
- Use `?[]const u8` for optional strings
- Manual string manipulation
- Allocator management

---

## Migration Checklist

For each language implementation:

- [ ] Update ParsedMetadata structure with 3 new fields
- [ ] Implement `extract_series_info()` function
- [ ] Implement `extract_edition()` function
- [ ] Implement `extract_volume()` function
- [ ] Update parsing flow to call new functions
- [ ] Update filename generation to use new format
- [ ] Add unit tests for series detection (3+ tests)
- [ ] Add unit tests for edition detection (3+ tests)
- [ ] Add unit tests for volume detection (3+ tests)
- [ ] Add unit tests for filename generation (4+ tests)
- [ ] Run language-specific test suite (all pass)
- [ ] Run cross-language parity test (JSON outputs match)
- [ ] Update language README if exists

---

## Debugging Cross-Language Differences

If cross-language tests fail:

1. **Check Regex Behavior**: Different regex engines may behave differently
   - Go uses RE2 (no lookahead/lookbehind)
   - Python uses full regex
   - Ensure consistent patterns

2. **Check String Trimming**: Ensure consistent whitespace handling
   - Trim after each extraction
   - Use same trim characters

3. **Check Ordering**: Functions must be called in exact same order
   - Series → Edition → Year → Volume

4. **Check Edge Cases**:
   - Empty strings
   - Strings with no matches
   - Multiple matches (should use first/last consistently)

5. **Use JSON Diff**:
   ```bash
   diff <(jq . rust_output.json) <(jq . go_output.json)
   ```

---

## Performance Considerations

- Compile regexes once (static/global)
- Avoid repeated string allocations
- Use string builders for concatenation
- Cache common patterns

---

## Documentation Updates

After implementation:

1. Update `CLAUDE.md` with v2.0 format
2. Update `README.md` examples
3. Update `CHANGELOG.md` with breaking changes
4. Update `docs/spec.md` (already done)
5. Update `docs/formatting_standards.md` (already done)

---

## Questions / Issues

If you encounter issues:

1. Check Rust implementation first (source of truth)
2. Compare JSON outputs byte-by-byte
3. Check test cases in `src/normalizer.rs` (lines 1099-1238)
4. Verify series mappings match exactly
5. Ensure edition suffix logic matches (1st, 2nd, 3rd, Nth)

---

## Version Information

- **Breaking Change**: Yes (filename format changed)
- **Version Bump**: 1.x → 2.0
- **Backward Compatibility**: No (old vs new format)
- **Migration**: Reprocess entire library

---

## Implementation Priority

**Recommended order**:
1. ✅ Rust (completed)
2. Go (most used, compiled)
3. Python (most accessible)
4. Ruby, Haskell, OCaml, Zig (as needed)

**Time Estimates**:
- Go: 2-3 hours
- Python: 2-3 hours
- Others: 1-2 hours each

**Total**: ~10-15 hours for all implementations
