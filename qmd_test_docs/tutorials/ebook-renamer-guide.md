# Ebook Renamer: Multi-Language Implementation Guide

## Project Overview

A cross-language ebook file organization tool with perfect behavioral parity across Rust, Go, Python, Ruby, Haskell, OCaml, and Zig implementations.

## Core Features

### File Scanning
- Recursive directory traversal with depth control
- Extension detection (handles `.tar.gz`, `.download`, `.crdownload`)
- Filters hidden files and specific directories (`node_modules`, `.git`, `__pycache__`, `Xcode`)
- Classifies: failed downloads, undersized files, normal files

### Filename Normalization

**Pattern:** `Author - Title (Year).ext`

**Removes:**
- Series prefixes (Graduate Texts in Mathematics)
- Source indicators (libgen, Z-Library, Anna's Archive)
- Noise characters and brackets

**Extracts:**
- Year (rightmost `19XX` or `20XX`)
- Author/Title using `" - "`, `":"`, or trailing `(Author)` patterns

**Example:**
```
Before: [libgen] Graduate Texts in Mathematics - Linear Algebra (2018) (Smith, John).pdf
After:  John Smith - Linear Algebra (2018).pdf
```

### Duplicate Detection

**Strategy:**
- MD5-based hashing
- Processes `.pdf`, `.epub`, `.txt` (NOT `.mobi`)
- Priority: normalized > shallowest path > newest modified time

### Cross-Language Testing

**Critical:** All implementations must produce byte-for-byte identical JSON output.

```bash
./tests/tools/test_cross_language.sh /path/to/test/files
```

## Implementation Modules

### 1. Scanner Module
```rust
// src/scanner.rs
pub struct ScanResult {
    pub files: Vec<FileInfo>,
    pub failed_downloads: Vec<PathBuf>,
    pub small_files: Vec<PathBuf>,
}
```

### 2. Normalizer Module
```go
// source_go/internal/normalizer/normalizer.go
type NormalizedName struct {
    Author string
    Title  string
    Year   string
}

func Normalize(filename string) NormalizedName
```

### 3. Duplicates Module
```python
# source_py/ebook_renamer/duplicates.py
def find_duplicates(files: List[FileInfo]) -> List[DuplicateGroup]:
    # MD5-based duplicate detection
    pass
```

## Usage Examples

### Rust (Primary)
```bash
# TUI mode
cargo run -- /path/to/books

# JSON output
cargo run -- --dry-run --json /path/to/books

# With deletion of small files
cargo run -- --delete-small /path/to/books
```

### Go
```bash
cd source_go
go build -o ebook-renamer ./cmd/ebook-renamer
./ebook-renamer --dry-run --json /path/to/books
```

### Python
```bash
python3 source_py/ebook-renamer.py --dry-run --json /path/to/books
```

## JSON Output Schema

```json
{
  "renames": [
    {"from": "old/path.pdf", "to": "new/path.pdf"}
  ],
  "duplicate_deletes": [
    {
      "keep": "best/version.pdf",
      "delete": ["duplicate1.pdf", "duplicate2.pdf"]
    }
  ],
  "small_or_corrupted_deletes": [
    {"path": "tiny.pdf", "size": 512}
  ],
  "todo_items": [
    {
      "category": "failed_download",
      "file": "incomplete.download",
      "reason": "Incomplete download"
    }
  ]
}
```

## Deterministic Behavior Rules

### Path Handling
- JSON uses POSIX separators (`/`), never backslashes
- Paths relative to target directory
- Empty string for non-relative paths

### Array Sorting
```rust
// Critical for cross-language parity
renames.sort_by(|a, b| a.from.cmp(&b.from));
groups.sort_by(|a, b| a.keep.cmp(&b.keep));
deletes.sort_by(|a, b| a.path.cmp(&b.path));
```

### Year Extraction
- Pattern: `\b(19|20)\d{2}\b`
- Returns **rightmost** match only
- Example: "Title (2018, Publisher) (2020)" → 2020

## Testing Strategy

### Unit Tests
```bash
# Rust
cargo test

# Go
go test ./...

# Python
cd source_py && python3 -m pytest
```

### Integration Tests
```bash
# Generate test data
python3 tests/tools/generate_noise.py --clean-dir fixtures/clean --output-dir fixtures/noisy

# Build golden reference
python3 tests/tools/build_golden_from_rust.py --target-dir fixtures/noisy

# Cross-language validation
./tests/tools/test_cross_language.sh fixtures/noisy
```

## Performance Characteristics

| Language | Build Time | Runtime | Binary Size | TUI Library |
|----------|-----------|---------|-------------|-------------|
| Rust     | ~30s      | Fastest | ~8MB        | Ratatui     |
| Go       | ~5s       | Fast    | ~15MB       | Bubble Tea  |
| Python   | N/A       | Moderate| N/A         | Rich        |

## Common Edge Cases

### Extension Detection
- `.tar.gz` treated as single extension
- `.download`/`.crdownload` mark failed downloads
- No extension = empty string, not error

### Unicode Handling
- All filenames must be valid UTF-8
- `--preserve-unicode` flag exists but currently unused
- No transliteration performed

### Small File Threshold
- `< 1KB` for `.pdf` or `.epub`
- `--delete-small`: immediate deletion vs todo list
- Failed downloads never count as "small files"

## Best Practices

1. **Always run `--dry-run` first** to preview changes
2. **Use `--json` for scripting** to avoid TUI interference
3. **Test with `test_cross_language.sh`** after core logic changes
4. **Update all implementations simultaneously** when changing normalization
5. **Check `docs/spec.md`** as source of truth for behavior
