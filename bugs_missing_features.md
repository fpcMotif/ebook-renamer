# Bugs and Missing Features Analysis

## Bugs Found

### Bug 1: Test Assert Uses All Caps Title (CRITICAL)
**Location**: `src/normalizer.rs` test at line 1747

**Issue**: Test `test_all_caps_title_with_author_suffix` expects specific behavior:
```rust
assert_eq!(metadata.authors, Some("A. B. PIPPARD".to_string()));
assert_eq!(metadata.title, "THE PHYSICS OF VIBRATIONS");
```

**Problem**: If the title is all-caps, the parser may not correctly identify the author. The test expects it to work but the parser might fail on all-caps titles with dash separators.

**Status**: Tests pass, but logic may be fragile.

**Fix**: Enhance author detection to handle all-caps titles better.

### Bug 2: Empty Filename Edge Case (MEDIUM)
**Location**: `src/scanner.rs` test at line 1333

**Issue**: Empty filename (`.pdf`) is accepted:
```rust
let metadata = parse_filename(".pdf", ".pdf").unwrap();
assert!(metadata.title.is_empty() || metadata.title == "");
```

**Problem**: Processing files with empty names could cause issues with file operations.

**Status**: Handled but should probably reject as invalid input.

**Fix**: Add validation to reject empty filenames.

### Bug 3: Missing `tui` CLI Argument (MEDIUM)
**Location**: `src/cli.rs`

**Issue**: The `Args` struct has `pub tui: bool` but there's no actual CLI argument to enable it in `#[command]` attributes.

**Problem**: User cannot enable TUI mode via command line.

**Status**: The field exists but cannot be set from CLI.

**Fix**: Add `#[arg(long, help = "Run with TUI interface")]` to the `tui` field.

### Bug 4: No Actual TUI Implementation (HIGH)
**Location**: `src/tui.rs`

**Issue**: The `run_process` function in TUI doesn't implement full main.rs logic:
```rust
fn run_process(mut args: Args, tx: mpsc::Sender<AppEvent>) -> Result<()> {
    // ... (Simplified logic for TUI demo, ideally copy full logic)
```

**Problem**: Comment admits it's a demo/simplified version. The actual main.rs logic includes:
- JSON output generation
- Small file deletion
- Failed download cleanup
- All detailed operations

These are missing from TUI version.

**Status**: TUI mode is incomplete/feature-incomplete.

**Fix**: Copy full logic from `main.rs` into `run_process()`.

## Missing Features

### Feature 1: Undo Script (HIGH)
**Status**: Partially implemented in TUI only

**Description**: Undo script generation exists in TUI (`src/tui.rs` lines 193-240) but not in main CLI mode.

**Use Case**: Users want to revert changes after running the tool.

**Implementation Needed**: Add undo script generation to main.rs flow.

### Feature 2: arXiv Metadata Fetching (HIGH)
**Status**: Placeholder only

**Location**: `src/cli.rs` lines 81-86
```rust
#[arg(
    long,
    help = "Fetch arXiv metadata via API (not implemented yet)"
)]
pub fetch_arxiv: bool,
```

**Use Case**: Automatically fetch correct titles/author names from arXiv.org for academic papers.

**Implementation Needed**:
1. Parse arXiv IDs from filenames (e.g., `2012.08669v1.pdf`)
2. Call arXiv API: `https://export.arxiv.org/api/query?search_list=id&search_query=2012.08669v1`
3. Parse XML/JSON response
4. Extract title, author, year
5. Use metadata for renaming

### Feature 3: Config File Support (MEDIUM)
**Status**: Not implemented

**Use Case**: Users want to save preferences (default directory, max depth, etc.) without specifying flags every time.

**Implementation Needed**: Support `~/.ebook-renamer/config.toml` or similar.

### Feature 4: Progress Bar for CLI (MEDIUM)
**Status**: Not implemented

**Use Case**: Users want visual feedback during long-running operations.

**Implementation Needed**: Use `indicatif` crate for progress bars.

### Feature 5: Interactive Mode (MEDIUM)
**Status**: Not implemented

**Use Case**: Users want to confirm each operation before it executes.

**Implementation Needed**: Add `--interactive` flag that pauses before each rename/delete.

### Feature 6: Backup Creation (MEDIUM)
**Status**: Not implemented

**Use Case**: Users want automatic backups before making changes.

**Implementation Needed**: Create timestamped backup or copy originals to backup folder.

### Feature 7: Series/Volume Sorting (MEDIUM)
**Status**: Partially implemented

**Use Case**: Users want to organize multi-volume books into folders.

**Implementation Needed**: Option to create folders like `Series Name/Vol 1/`, `Series Name/Vol 2/`.

### Feature 8: EPUB Metadata Extraction (MEDIUM)
**Status**: Not implemented

**Current State**: Only parses filename for EPUB files.

**Use Case**: EPUB files contain rich metadata (title, author, cover) that can be used for renaming.

**Implementation Needed**: Parse EPUB structure and extract `metadata.xml` or `opf` files.

### Feature 9: PDF Metadata Extraction (MEDIUM)
**Status**: Not implemented

**Current State**: Only parses filename for PDF files.

**Use Case**: PDF files contain metadata (DocumentInfo dictionary) with author, title, etc.

**Implementation Needed**: Parse PDF internal metadata using a PDF parsing crate (e.g., `lopdf`).

### Feature 10: Duplicate Resolution Strategies (LOW)
**Status**: Not implemented

**Current State**: Always keeps newest/shallowest file.

**Use Case**: Users might want to:
- Keep largest file
- Keep highest resolution file (for images)
- Manually choose which to keep

**Implementation Needed**: Add CLI flag `--duplicate-strategy` with options: newest, largest, manual.

### Feature 11: Dry Run CSV/JSON Export (LOW)
**Status**: JSON implemented, CSV not

**Current State**: `--json` flag exists for machine-readable output.

**Use Case**: CSV format for spreadsheet analysis.

**Implementation Needed**: Add `--csv` flag and CSV writer.

### Feature 12: File Association (LOW)
**Status**: Not implemented

**Use Case**: Recognize file formats and handle appropriately (e.g., open DJVU with different tools).

**Implementation Needed**: MIME type detection and format-specific handling.

### Feature 13: Parallel Processing (LOW)
**Status**: Not implemented

**Current State**: Sequential file processing.

**Use Case**: Speed up operations on large directories.

**Implementation Needed**: Use `rayon` for parallel file operations.

## Code Quality Issues

### Issue 1: Dead Code
**Location**: Multiple modules

**Examples**:
- `src/duplicates.rs`: `detect_name_variants` and `strip_variant_suffix` marked `#[allow(dead_code)]`
- `src/todo.rs`: `InvalidExtension` variant marked `#[allow(dead_code)]`
- `src/json_output.rs`: Unused import `use std::path::Path;`

**Fix**: Remove dead code or implement features using it.

### Issue 2: Magic Numbers
**Location**: Throughout codebase

**Examples**:
- `1024` bytes (file size threshold)
- `0.85` (similarity threshold)
- `5` (PDF header size)
- `18446744073709551615` (default max depth - usize::MAX)

**Fix**: Define constants at top of each module.

### Issue 3: Inconsistent Error Handling
**Location**: Throughout

**Issue**: Some functions return `anyhow::Result`, some return `std::io::Result`.

**Fix**: Standardize on `anyhow::Result`.

### Issue 4: Large Function Complexity
**Location**: `src/normalizer.rs` `parse_filename` function

**Issue**: Function is very long (200+ lines) with multiple concerns.

**Fix**: Refactor into smaller helper functions.

## Performance Concerns

### Concern 1: Hash Computation Without Streaming
**Location**: `src/duplicates.rs` lines 264-281

**Issue**: Reads entire file into memory for MD5:
```rust
let mut buffer = [0u8; BUFFER_SIZE];
loop {
    let bytes_read = file.read(&mut buffer)?;
    // ...
    hasher.consume(&buffer[..bytes_read]);
}
```

**Impact**: Large files (100MB+) will consume significant memory.

**Fix**: Already uses streaming! This is actually well-implemented. ✅

### Concern 2: Regex Compilation in Hot Path
**Location**: `src/normalizer.rs` line 255

**Issue**: `regex::Regex::new()` is called inside function.

**Impact**: If called many times, recompiles regex each time.

**Fix**: Use `lazy_static` or `once_cell` for compiled regexes.

## Documentation Gaps

### Gap 1: No Architecture Documentation
**Status**: Missing

**Need**: High-level overview of how modules interact.

### Gap 2: No API Documentation
**Status**: Missing `///` doc comments on public functions.

**Need**: Rustdoc comments for all public APIs.

### Gap 3: No Usage Examples
**Status**: Missing

**Need**: README examples for common use cases.

### Gap 4: No CONTRIBUTING Guide
**Status**: Missing

**Need**: Guidelines for contributing code/tests.

### Gap 5: No Changelog
**Status**: Missing

**Need**: Document version history and changes.

