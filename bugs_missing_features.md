# Bugs and Missing Features Analysis

**Last Updated:** 2026-01-12

## ✅ FIXED Bugs

### Bug 1: TUI Runs by Default (FIXED)
**Location**: `src/main.rs` line 83
**Issue**: Non-JSON mode always ran TUI even without explicit flag.
**Fix**: Added `--tui` CLI flag. TUI is now opt-in.

### Bug 2: Missing `--tui` CLI Argument (FIXED)
**Location**: `src/cli.rs`
**Issue**: The `tui` field existed but couldn't be set from CLI.
**Fix**: Added `#[arg(long, help = "Run with terminal UI interface")]` to the `tui` field.

### Bug 3: TUI Logic Simplified/Incomplete (FIXED)
**Location**: `src/tui.rs`
**Issue**: Comment admitted it was a demo version missing full main.rs logic.
**Fix**: Updated `run_process` with full feature parity: config loading, arXiv fetching, metadata extraction, failed download handling, undo log support.

### Bug 4: Panic-prone Unwraps in TUI (FIXED)
**Location**: `src/tui.rs`
**Issue**: Hash errors were silently ignored.
**Fix**: Hash errors now reported via AppEvent::Error and logged to todo_list.

### Bug 5: Regex Compiled in Hot Path (FIXED)
**Location**: `src/normalizer.rs`
**Issue**: Multiple `Regex::new().unwrap()` inside `parse_filename` function.
**Fix**: Used `once_cell::sync::Lazy` for pre-compiled static regexes.

### Bug 6: Dead Code Warnings (FIXED)
**Location**: Multiple modules
**Issue**: Various unused functions and constants causing warnings.
**Fix**: Added `#![allow(dead_code)]` to modules with intentionally kept utility code.

---

## ✅ IMPLEMENTED Features

### Feature 1: Undo Script for CLI Mode (IMPLEMENTED)
**Status**: ✅ Complete
**Flag**: `--undo-script <PATH>`
**Description**: Generates a shell script to reverse all renames.

### Feature 2: arXiv Metadata Fetching (IMPLEMENTED)
**Status**: ✅ Complete
**Flag**: `--fetch-arxiv`
**Description**: Fetches metadata from arXiv API for files matching arXiv ID patterns (e.g., `2312.12345.pdf`).

### Feature 3: EPUB/PDF Metadata Extraction (IMPLEMENTED)
**Status**: ✅ Complete
**Flag**: `--extract-metadata`
**Description**: Extracts title, author, year from PDF Info dictionary and EPUB OPF files.

### Feature 4: Progress Bar for CLI (IMPLEMENTED)
**Status**: ✅ Module created (`src/progress.rs`)
**Description**: `ProgressReporter` using `indicatif` crate. Ready for integration.

### Feature 5: CSV Export (IMPLEMENTED)
**Status**: ✅ Complete
**Flag**: `--csv`
**Description**: Outputs operations in CSV format for spreadsheet analysis.

### Feature 6: Backup Creation (IMPLEMENTED)
**Status**: ✅ Complete
**Flag**: `--backup-dir <PATH>`
**Description**: Creates timestamped backups of files before renaming/deleting.

### Feature 7: TUI Mode Flag (IMPLEMENTED)
**Status**: ✅ Complete
**Flag**: `--tui`
**Description**: Explicit opt-in for terminal UI interface.

---

## 🔄 Remaining/Future Features

### Feature: Config File Support (PARTIAL)
**Status**: Partially implemented in `src/config.rs`
**Location**: `~/.config/ebook-renamer/config.toml`
**TODO**: Document config options, add more settings.

### Feature: Parallel Processing (NOT STARTED)
**Priority**: Low
**Description**: Use `rayon` for parallel file operations.

### Feature: Interactive Mode (PARTIAL)
**Status**: Flag exists (`--interactive`, `--batch-interactive`)
**TODO**: Wire up interactive confirmation prompts.

### Feature: Duplicate Resolution Strategies (NOT STARTED)
**Priority**: Low
**Description**: Add `--duplicate-strategy` with options: newest, largest, manual.

---

## CLI Reference (Updated)

```
ebook-renamer [OPTIONS] [PATH]

Arguments:
  PATH                    Directory to scan (default: current directory)

Options:
  -d, --dry-run           Show changes without applying them
  --json                  Output in JSON format
  --csv                   Output in CSV format
  --tui                   Run with terminal UI interface
  --max-depth N           Maximum directory depth (default: unlimited)
  --no-recursive          Only scan top-level directory
  --extensions EXT        Comma-separated extensions (default: pdf,epub,txt)
  --no-delete             Don't delete duplicates, only list them
  --todo-file PATH        Custom todo.md location
  --delete-small          Delete files < 1KB instead of adding to todo
  --clean-failed          Delete failed downloads after logging to todo
  --preserve-unicode      Preserve non-Latin scripts
  --fetch-arxiv           Fetch arXiv metadata via API
  --extract-metadata      Extract metadata from EPUB/PDF files
  --backup-dir PATH       Create backups before changes
  --undo-log PATH         Write JSON undo log
  --undo-script PATH      Generate shell undo script
  --skip-cloud-hash       Skip MD5 for cloud storage
  --cleanup-downloads     Remove empty .download folders
  -v, --verbose           Enable verbose logging
```
