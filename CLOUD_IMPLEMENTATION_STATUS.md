# Cloud Storage Implementation Status

## Overview
This document tracks the implementation status of cloud storage support across all language implementations of ebook-renamer.

## Feature Summary
- **Auto-detection** of cloud storage paths (Dropbox, Google Drive, OneDrive, macOS CloudStorage)
- **Metadata-only** duplicate detection using fuzzy string matching (Jaro-Winkler similarity ≥ 0.85)
- **No file content reading** to avoid triggering cloud file downloads
- **Exact size matching** required before fuzzy filename comparison

## Implementation Status

### ✅ Rust (COMPLETE)
**Status**: Fully implemented and tested

**Changes Made**:
1. Added `strsim` crate (v0.11) for Jaro-Winkler similarity
2. Created `src/cloud.rs` module:
   - `is_cloud_storage_path()` - Detects Dropbox/Google Drive/OneDrive paths
   - `CloudProvider` enum and warning messages
   - Auto-detection for macOS `Library/CloudStorage` paths
3. Updated `src/duplicates.rs`:
   - Enhanced `detect_duplicates()` to support fuzzy matching when `skip_hash=true`
   - Groups files by exact size first
   - Within size groups, compares filenames using Jaro-Winkler (threshold: 0.85)
   - Maintains same retention priority: normalized > shallowest > newest
4. Updated `src/main.rs`:
   - Auto-enables `skip_cloud_hash` when cloud path detected
   - Displays warning message for cloud mode
5. Updated `src/tui.rs`:
   - Shows cloud mode warning in TUI
6. Updated `docs/spec.md`:
   - Documented cloud storage mode behavior
   - Added detection patterns
   - Documented fuzzy matching algorithm

**Files Modified**:
- `Cargo.toml` - Added strsim dependency
- `src/cloud.rs` - New module
- `src/duplicates.rs` - Added fuzzy matching logic
- `src/main.rs` - Auto-detection logic
- `src/tui.rs` - TUI warnings
- `docs/spec.md` - Documentation

**Tests**: All existing tests pass, including new cloud detection tests

---

### ⚠️ Go (PARTIAL)
**Status**: Cloud detection module created, needs fuzzy matching implementation

**Completed**:
1. Created `source_go/internal/cloud/cloud.go`:
   - `IsCloudStoragePath()` - Detects cloud storage paths
   - `Provider` type and `CloudModeWarning()`
2. CLI already has `--skip-cloud-hash` flag

**Remaining Work**:
1. Add fuzzy string matching library:
   ```bash
   cd source_go
   go get github.com/xrash/smetrics
   ```
2. Update `source_go/internal/duplicates/duplicates.go`:
   - Import `smetrics` package
   - Implement fuzzy matching in `DetectDuplicates()` when `skipHash=true`
   - Group by size first, then apply Jaro-Winkler within size groups
3. Update `source_go/internal/cli/cli.go`:
   - Import cloud package
   - Auto-enable `skipCloudHashFlag` when cloud path detected
   - Display warning message
4. Update `source_go/internal/tui/model.go`:
   - Add cloud mode warning to TUI

**Estimated Effort**: 2-3 hours

---

### ❌ Python (NOT STARTED)
**Status**: No changes made yet

**Required Work**:
1. Add `jellyfish` or `python-Levenshtein` package for Jaro-Winkler:
   ```bash
   pip install jellyfish
   ```
2. Create `source_py/ebook_renamer/cloud.py`:
   - `is_cloud_storage_path()` function
   - `CloudProvider` enum
   - `cloud_mode_warning()` function
3. Update `source_py/ebook_renamer/duplicates.py`:
   - Import `jellyfish.jaro_winkler`
   - Implement fuzzy matching when `skip_hash=True`
4. Update `source_py/ebook_renamer/cli.py`:
   - Add `--skip-cloud-hash` flag
   - Auto-enable for cloud paths
5. Update `source_py/ebook_renamer/__main__.py`:
   - Display cloud mode warning

**Estimated Effort**: 3-4 hours

---

### ❌ Ruby (NOT STARTED)
**Status**: No changes made yet

**Required Work**:
1. Add `fuzzy-string-match` gem for Jaro-Winkler
2. Create `source_rb/lib/cloud.rb`
3. Update `source_rb/lib/duplicates.rb`
4. Update `source_rb/lib/cli.rb`

**Estimated Effort**: 3-4 hours

---

### ❌ Haskell (NOT STARTED)
**Status**: No changes made yet

**Required Work**:
1. Add `edit-distance` or `text-metrics` package
2. Create `source_hs/src/Cloud.hs`
3. Update `source_hs/src/Duplicates.hs`
4. Update `source_hs/src/Main.hs`

**Estimated Effort**: 4-5 hours

---

### ❌ OCaml (NOT STARTED)
**Status**: No changes made yet

**Required Work**:
1. Add string similarity library
2. Create `source_ml/lib/cloud.ml`
3. Update `source_ml/lib/duplicates.ml`

**Estimated Effort**: 4-5 hours

---

### ❌ Zig (NOT STARTED)
**Status**: No changes made yet

**Required Work**:
1. Implement Jaro-Winkler from scratch or find library
2. Create `source_zig/src/cloud.zig`
3. Update `source_zig/src/duplicates.zig`

**Estimated Effort**: 5-6 hours

---

## Testing Requirements

### Cross-Language Parity Test
**CRITICAL**: After implementing in all languages, run cross-language test:
```bash
./tests/tools/test_cross_language.sh /path/to/test/files --skip-cloud-hash
```

This ensures all implementations produce **byte-for-byte identical JSON output** when using fuzzy matching mode.

### Test Data Requirements
Create test fixtures in `tests/fixtures/cloud_mode/`:
1. **Same size, similar names** (should match):
   - `Category Theory - Awodey.pdf` (1.2MB)
   - `Category Theory - Awodey - libgen.pdf` (1.2MB)
   - Expected: Jaro-Winkler similarity ~0.92, should group as duplicates

2. **Same size, different names** (should NOT match):
   - `Topology - Munkres.pdf` (3.1MB)
   - `Analysis - Rudin.pdf` (3.1MB)
   - Expected: Jaro-Winkler similarity ~0.15, should remain separate

3. **Different size, similar names** (should NOT match):
   - `Abstract Algebra - Dummit Foote.pdf` (5.4MB)
   - `Abstract Algebra - Dummit Foote 2nd Ed.pdf` (6.1MB)
   - Expected: Different sizes, should remain separate even if names match

### Manual Testing on Cloud Storage
Test on actual cloud storage:
```bash
# Dropbox
cargo run -- --dry-run ~/Dropbox/Books

# Google Drive (macOS)
cargo run -- --dry-run ~/Library/CloudStorage/GoogleDrive-*/Books

# Should see warning message
# Should NOT trigger file downloads
# Should detect fuzzy duplicates
```

---

## Algorithm Details

### Jaro-Winkler Similarity
- **Range**: 0.0 (completely different) to 1.0 (identical)
- **Threshold**: 0.85 (85% similarity required)
- **Advantages**:
  - Good for short strings (filenames)
  - Gives higher weight to common prefixes
  - Fast computation (O(n))

### Why Not Levenshtein?
- Levenshtein distance is absolute edit distance
- Jaro-Winkler is normalized (0-1 range), easier to set threshold
- Jaro-Winkler performs better for filenames with common prefixes

### Duplicate Detection Flow (Cloud Mode)
```
1. Filter files by allowed extensions (.pdf, .epub, .txt)
2. Group by exact file size
3. For each size group with 2+ files:
   a. Extract normalized filename (or original if not normalized)
   b. Compare all pairs using Jaro-Winkler
   c. If similarity ≥ 0.85, group as potential duplicates
4. Apply retention priority:
   - Keep normalized files over non-normalized
   - Keep shallowest path
   - Keep newest modification time
5. Return duplicate groups
```

---

## Performance Considerations

### Memory Usage
- Fuzzy matching requires O(n²) comparisons within each size group
- For 1000 files with same size: ~500,000 comparisons
- Jaro-Winkler is fast, but still more expensive than hash lookup

### Optimization Strategies
1. **Size grouping first** - Reduces comparison space dramatically
2. **Early termination** - Once file is grouped, skip further comparisons
3. **Parallel processing** (future): Process size groups in parallel

### Expected Performance
- **1000 files, 50 size groups**: ~1-2 seconds
- **10000 files, 200 size groups**: ~10-20 seconds
- **100000 files**: May need additional optimizations

---

## Known Limitations

### False Positives
Example: These may be incorrectly grouped as duplicates
- `Linear Algebra - Strang.pdf` (5MB)
- `Linear Algebra - Axler.pdf` (5MB)
- Similarity: ~0.87 > 0.85 threshold
- **Mitigation**: User must review with `--dry-run`

### False Negatives
Example: These may NOT be detected as duplicates
- `Topology.pdf` (3MB)
- `Topology - Munkres - 2nd Edition.pdf` (3MB)
- Similarity: ~0.75 < 0.85 threshold
- **Mitigation**: Lower threshold (but increases false positives)

### Threshold Tuning
Current: **0.85**
- **0.80**: More matches, more false positives
- **0.90**: Fewer matches, more false negatives
- **0.85**: Balanced (chosen empirically)

---

## Future Enhancements

### Phase 2: Cloud Provider API Integration
See `CLOUD_STORAGE_STRATEGY.md` for full details:
- Use Dropbox `content_hash` API
- Use Google Drive `md5Checksum` API
- 100% accurate without downloading files
- Requires OAuth setup

### Phase 3: Hybrid Mode
- Hash small files (< 1MB)
- Use fuzzy matching for large files (> 100MB)
- Best balance of speed and accuracy

---

## Development Checklist

Before marking cloud mode as "complete":
- [x] Rust implementation with fuzzy matching
- [x] Cloud path auto-detection (Rust)
- [x] TUI/CLI warnings (Rust)
- [x] Updated spec.md documentation
- [ ] Go implementation with fuzzy matching
- [ ] Python implementation with fuzzy matching
- [ ] Ruby implementation with fuzzy matching
- [ ] Haskell implementation with fuzzy matching
- [ ] OCaml implementation with fuzzy matching
- [ ] Zig implementation with fuzzy matching
- [ ] Cross-language parity test passing
- [ ] Manual testing on real Dropbox/Google Drive
- [ ] Update README.md with cloud mode examples
- [ ] Update CLAUDE.md with cloud mode guidance

---

## References

- **Jaro-Winkler**: https://en.wikipedia.org/wiki/Jaro%E2%80%93Winkler_distance
- **strsim crate** (Rust): https://docs.rs/strsim/
- **smetrics** (Go): https://github.com/xrash/smetrics
- **jellyfish** (Python): https://github.com/jamesturk/jellyfish
- **Cloud Storage Strategy**: See `CLOUD_STORAGE_STRATEGY.md`
