# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

This is a multi-language ebook renaming tool with **perfect cross-language behavioral parity** between Rust, Go, Python, Ruby, Haskell, OCaml, and Zig implementations. All implementations produce identical JSON output for the same input files.

**NEW**: Cloud storage integration for Dropbox and Google Drive allows remote file organization without local downloads.

## Critical Architecture Principle

**Behavioral Parity is Sacred**: All language implementations must produce byte-for-byte identical JSON output when given the same input directory. This is enforced through cross-language testing. When modifying normalization logic, duplicate detection, or any core functionality, you MUST update all implementations simultaneously.

## Building and Running

### Rust (Primary Implementation)
```bash
# Build
cargo build --release

# Run with TUI (default)
cargo run -- /path/to/books

# Run with JSON output (no TUI)
cargo run -- --dry-run --json /path/to/books

# Run tests
cargo test
```

### Go
```bash
# Build
cd source_go && go build -o ebook-renamer ./cmd/ebook-renamer

# Run with TUI
./ebook-renamer /path/to/books

# Run with JSON output
./ebook-renamer --dry-run --json /path/to/books

# Run tests
go test ./...
```

### Python
```bash
# Run directly (requires rich library for TUI)
python3 source_py/ebook-renamer.py --dry-run --json /path/to/books

# Run tests
cd source_py && python3 -m pytest
```

### Other Implementations
- **Ruby**: `ruby source_rb/ebook-renamer.rb --dry-run --json /path/to/books`
- **Haskell**: `cd source_hs && stack build && stack exec ebook-renamer`
- **Zig**: `cd source_zig && zig build run`

### Cloud Storage (Rust only)
```bash
# Dropbox: Dry run
cargo run -- --cloud-provider dropbox --cloud-token YOUR_TOKEN --cloud-path /Books --dry-run

# Dropbox: Apply renames
cargo run -- --cloud-provider dropbox --cloud-token YOUR_TOKEN --cloud-path /Books

# Google Drive
cargo run -- --cloud-provider google-drive --cloud-token YOUR_TOKEN --cloud-path /MyEbooks --dry-run
```

## Cross-Language Testing

**CRITICAL**: Run this after ANY changes to core logic:
```bash
./tests/tools/test_cross_language.sh /path/to/test/files
```

This compares JSON output from all implementations. If they differ, the change is WRONG and must be fixed.

## Module Architecture

All implementations follow this modular structure:

1. **Scanner Module** (`src/scanner.rs`, `source_go/internal/scanner/`, etc.)
   - File system traversal with configurable depth
   - Extension detection (handles `.tar.gz`, `.download`, `.crdownload`)
   - Skips hidden files and specific directories (`node_modules`, `.git`, `__pycache__`, `Xcode`)
   - Classifies files: failed downloads, too small files, normal files

2. **Normalizer Module** (`src/normalizer.rs`, `source_go/internal/normalizer/`, etc.)
   - Deterministic filename parsing
   - Removes series prefixes (Graduate Texts in Mathematics, etc.)
   - Removes source indicators (libgen, Z-Library, Anna's Archive)
   - Extracts year (rightmost `19XX` or `20XX`)
   - Splits author/title using `" - "`, `":"`, or trailing `(Author)` patterns
   - Output format: `Author - Title (Year).ext`

3. **Duplicates Module** (`src/duplicates.rs`, `source_go/internal/duplicates/`, etc.)
   - MD5-based duplicate detection
   - Only processes `.pdf`, `.epub`, `.txt`, `.djvu` (NOT `.mobi`)
   - Retention priority: normalized files > shallowest path > newest modified time
   - Skipped in cloud storage mode to avoid downloading files

4. **Todo Module** (`src/todo.rs`, `source_go/internal/todo/`, etc.)
   - Generates `todo.md` in Chinese
   - Categories: failed downloads, small files, corrupted PDFs
   - Prevents duplicate entries
   - Updates timestamp

5. **JSON Output Module** (`src/json_output.rs`, `source_go/internal/jsonoutput/`, etc.)
   - **Deterministic array sorting** (critical for cross-language parity)
   - POSIX-style path separators
   - Schema: `renames`, `duplicate_deletes`, `small_or_corrupted_deletes`, `todo_items`

6. **TUI Module** (`src/tui.rs`, `source_go/internal/tui/`, `source_py/ebook_renamer/tui.py`)
   - Rust: Ratatui with progress gauge and scrolling logs
   - Go: Bubble Tea with spinners and viewport
   - Python: Rich with progress bars
   - Zig: ANSI escape codes

7. **Cloud Storage Module** (`src/cloud_storage.rs`, Rust only for now)
   - Trait-based abstraction for cloud storage backends
   - Dropbox backend: List, rename, delete files via Dropbox API
   - Google Drive backend: List, rename, delete files via Google Drive API
   - Skips MD5 hash computation to avoid downloading files
   - Supports PDF, EPUB, DJVU, TXT, and MOBI files

## Deterministic Behavior Rules

### Path Handling
- JSON output uses POSIX separators (`/`), never backslashes
- Paths are relative to target directory
- Empty string for paths that can't be made relative

### Array Sorting (CRITICAL)
```rust
// renames: sort by `from` field
renames.sort_by(|a, b| a.from.cmp(&b.from));

// duplicate_deletes: sort by `keep` field, sort `delete` arrays internally
groups.sort_by(|a, b| a.keep.cmp(&b.keep));

// small_or_corrupted_deletes: sort by `path`
deletes.sort_by(|a, b| a.path.cmp(&b.path));

// todo_items: sort by category, then file
items.sort_by(|a, b| a.category.cmp(&b.category).then(a.file.cmp(&b.file)));
```

### Year Extraction
- Pattern: `\b(19|20)\d{2}\b`
- Returns **rightmost** (last) match only
- Example: "Title (2018, Publisher) (2020)" → extracts 2020

### Duplicate Detection Priority
1. Files with `new_name` set (already normalized)
2. Shallowest directory depth
3. Most recent modification time

## File Processing Edge Cases

### Extension Detection
- `.tar.gz` is treated as single extension
- `.download` and `.crdownload` mark failed downloads
- No extension = empty string, not error

### Small File Handling
- Threshold: < 1KB for `.pdf`, `.epub`, or `.djvu`
- `--delete-small` flag: deletes immediately instead of adding to todo
- Failed downloads never count as "small files"

### Unicode and Non-Latin Scripts
- `--preserve-unicode` flag exists but currently unused
- No transliteration performed
- Filenames must be valid UTF-8

## Testing Infrastructure

### Test Data Generation
```bash
# Import real files from Downloads
python3 tests/tools/import_from_downloads.py --downloads ~/Downloads --output test_fixtures

# Generate noisy variations
python3 tests/tools/generate_noise.py --clean-dir test_fixtures/clean --output-dir test_fixtures/noisy

# Build golden reference from Rust
python3 tests/tools/build_golden_from_rust.py --target-dir test_fixtures/noisy --output-dir test_results
```

## Common Development Tasks

### Adding New Normalization Rule
1. Update `docs/spec.md` with exact regex and behavior
2. Implement in Rust `src/normalizer.rs`
3. Implement in Go `source_go/internal/normalizer/normalizer.go`
4. Implement in Python `source_py/ebook_renamer/normalizer.py`
5. Run cross-language test to verify parity
6. Update `docs/formatting_standards.md`

### Adding New CLI Flag
1. Add to `src/cli.rs` (Rust)
2. Add to `source_go/internal/cli/cli.go` (Go)
3. Add to `source_py/ebook_renamer/cli.py` (Python)
4. Update README.md CLI reference section
5. Update `docs/spec.md` if it affects output

### Fixing Duplicate Detection Bug
1. Write failing test in Rust (`cargo test`)
2. Write failing test in Go (`go test ./internal/duplicates/`)
3. Fix in all implementations
4. Verify cross-language test passes

## Key Files

- `docs/spec.md` - **Canonical behavior specification** (source of truth)
- `docs/formatting_standards.md` - Detailed normalization rules with examples
- `Cargo.toml` - Rust dependencies
- `source_go/go.mod` - Go dependencies
- `tests/tools/test_cross_language.sh` - Cross-language validation

## Performance Characteristics

| Implementation | Build Time | Runtime | Binary Size | TUI Library |
|---|---|---|---|---|
| Rust | ~30s | Fastest | ~8MB | Ratatui |
| Go | ~5s | Fast | ~15MB | Bubble Tea |
| Python | N/A | Moderate | N/A | Rich |

## Important Notes

- JSON output suppresses ALL other stdout messages (for piping)
- `todo.md` is always written, even in dry-run and JSON modes
- Hidden directories (starting with `.`) are skipped entirely
- MD5 calculation uses 8KB streaming buffer
- Download recovery extracts PDFs from `.download`/`.crdownload` folders before processing
