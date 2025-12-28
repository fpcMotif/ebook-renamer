# Security Analysis Report for ebook_renamer

## Critical Issues

### 1. Command Injection Risk in Undo Script Generation (CRITICAL)
**Location**: `src/tui.rs` lines 214-218

**Issue**: File paths are used directly in shell script without proper escaping:
```rust
let old = file_info.original_path.to_string_lossy().replace("\"", "\\"");
let new = file_info.new_path.to_string_lossy().replace("\"", "\\"");
writeln!(undo_file, "mv \"{}\" \"{}\"", new, old)?;
```

**Problem**: While double quotes are escaped, other shell metacharacters like `$`, `` ` ``, ``, `;`, `|`, `&` are not escaped. A maliciously crafted filename could execute arbitrary commands.

**Example Attack**: A file named `file.txt; rm -rf /; .pdf` would execute `rm -rf /`

**Fix**: Use proper shell escaping or alternative methods (e.g., use `std::process::Command` for operations).

### 2. Unvalidated User Input in File Paths (HIGH)
**Location**: Multiple locations throughout codebase

**Issue**: User-provided paths are not properly validated for:
- Path traversal attacks (`../`)
- Symbolic link attacks
- Windows UNC paths (if applicable)
- Extremely long paths

**Fix**: Implement path validation functions to canonicalize and check against target directory.

### 3. MD5 Hash for Security (MEDIUM)
**Location**: `src/duplicates.rs` line 264

**Issue**: Using MD5 for file deduplication. MD5 is cryptographically broken and not suitable for security purposes.

**Context**: While MD5 is acceptable for deduplication (not security), it's worth noting and potentially documenting.

**Fix**: Consider using SHA-256 for better collision resistance, or add comment about MD5's purpose.

## High Priority Issues

### 4. Race Condition in File Operations (HIGH)
**Location**: `src/tui.rs` lines 245-261

**Issue**: Multiple file operations without atomic checks:
```rust
std::fs::rename(&file_info.original_path, &file_info.new_path)?;
std::fs::remove_file(path)?;
```

**Problem**: Files can be created/deleted between operations, causing failures or incorrect behavior.

**Fix**: Implement proper error handling and atomic operations where possible.

### 5. Insufficient Error Handling (HIGH)
**Location**: Multiple modules

**Issue**: Many operations use `?` operator which propagates errors without detailed context.

**Examples**:
- `src/download_recovery.rs` line 95: `fs::rename(&pdf_file, &new_path)?;`
- `src/scanner.rs` line 63: `let metadata = fs::metadata(path)?;`

**Fix**: Add context to errors using `.context()` from anyhow.

## Medium Priority Issues

### 6. Hardcoded File Size Threshold (MEDIUM)
**Location**: `src/scanner.rs` line 90, `src/todo.rs` line 60

**Issue**: 1KB threshold is hardcoded:
```rust
let is_too_small = !is_failed_download && is_ebook && size < 1024;
```

**Problem**: Not configurable, may not suit all use cases.

**Fix**: Make threshold configurable via CLI argument.

### 7. Unicode Normalization Issues (MEDIUM)
**Location**: `src/normalizer.rs` throughout

**Issue**: Unicode handling is inconsistent - some places use `to_lowercase()`, some don't.

**Problem**: Different representations of the same string (e.g., "café" with and without combining accents) may not be recognized as duplicates.

**Fix**: Use `unicode-normalization` crate for consistent normalization.

### 8. Missing File Locking (MEDIUM)
**Location**: File writing operations throughout

**Issue**: No file locking when writing `todo.md`, `undo_rename.sh`, or renaming files.

**Problem**: Concurrent operations could corrupt files or lose data.

**Fix**: Implement file locking using `fs2` crate or similar.

## Low Priority Issues

### 9. Potential Integer Overflow (LOW)
**Location**: `src/duplicates.rs` line 267

**Issue**: File size is stored as `u64` but could overflow in rare cases.

**Context**: Very unlikely in practice, but worth noting.

### 10. Memory Efficiency (LOW)
**Location**: `src/duplicates.rs` lines 21-133

**Issue**: Entire file contents read into memory for MD5 calculation without streaming.

**Fix**: Implement streaming hash computation for large files.

## Positive Security Practices

1. ✅ Uses `anyhow` for error handling (provides context)
2. ✅ Path operations use `std::path::PathBuf` (avoids string manipulation)
3. ✅ `tempfile` crate used in tests (safe temporary file handling)
4. ✅ Proper use of `Result` types throughout
5. ✅ Hidden files and directories are skipped (`.git`, etc.)
