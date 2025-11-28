# Cloud Storage Renaming Strategy

## Problem Statement

Current ebook-renamer requires:
1. **File access** for MD5 hash calculation (duplicate detection)
2. **Reading file content** to validate PDFs

For cloud storage (Dropbox, Google Drive, OneDrive):
- Downloading files just to compute hashes is **inefficient**
- Virtual file systems (mounted drives) may trigger downloads
- Large libraries (100GB+) make full download impractical

---

## Solution: Multi-Mode Architecture

### Mode 1: Metadata-Only (No Hash)
**Use Case**: Quick renaming without duplicate detection

**Strategy**:
- Rename files based on filename parsing only
- Use file size + filename similarity for "fuzzy" duplicate detection
- No file content reading required

**Limitations**:
- Cannot detect true duplicates (same content, different names)
- May miss duplicates or have false positives

**API Requirements**: None (works with any filesystem)

---

### Mode 2: Cloud-Native Hash (Recommended)
**Use Case**: Full duplicate detection using cloud provider APIs

**Strategy**:
- Use cloud provider's existing checksums
- Dropbox: `content_hash` (SHA-256 of 4MB blocks)
- Google Drive: `md5Checksum` (MD5)
- OneDrive: `file.hashes.sha1Hash` (SHA-1)

**Benefits**:
- ✅ No file download required
- ✅ True duplicate detection
- ✅ Fast (API calls only)
- ✅ Works with large files

**Limitations**:
- Requires API integration
- Different hash algorithms per provider
- Need authentication/tokens

**Implementation Complexity**: Medium

---

### Mode 3: Hybrid Smart Download
**Use Case**: Local processing with selective cloud downloads

**Strategy**:
1. Download small files (< 1MB) for hash
2. Use metadata for large files (> 100MB)
3. Sample-based hashing for medium files (hash first 10MB + last 10MB)

**Benefits**:
- Balance between accuracy and efficiency
- No API integration needed
- Works with virtual filesystems (rclone, Mountain Duck, etc.)

**Limitations**:
- Partial downloads may still trigger full download on some systems
- Sample hashing less accurate than full hash

---

## Recommended Implementation

### Phase 1: Add `--cloud-mode` Flag

```bash
# Metadata-only mode (fast, no hashing)
ebook-renamer --cloud-mode metadata /path/to/dropbox/books

# Cloud API mode (requires auth)
ebook-renamer --cloud-mode api --cloud-provider dropbox /path/to/dropbox/books

# Hybrid mode (smart downloading)
ebook-renamer --cloud-mode hybrid --size-threshold 10MB /path/to/books
```

### Phase 2: Implement Metadata-Only Mode

**Duplicate Detection Strategy**:
```rust
struct FileMetadata {
    filename: String,
    size: u64,
    modified_time: SystemTime,
}

// Fuzzy duplicate detection without hash
fn find_metadata_duplicates(files: Vec<FileMetadata>) -> Vec<DuplicateGroup> {
    // Group by normalized filename similarity + exact size match
    // Example: "BookA.pdf" (1.2MB) and "BookA - libgen.pdf" (1.2MB)
    //          → Likely duplicates
}
```

**Similarity Algorithm**:
1. Normalize both filenames (remove source markers, year, etc.)
2. Compute Levenshtein distance or Jaro-Winkler similarity
3. If similarity > 85% AND size matches → flag as likely duplicate
4. Human review required (use `--dry-run` by default in metadata mode)

---

### Phase 3: Cloud Provider API Integration

#### Dropbox API
```rust
// Use Dropbox API SDK
use dropbox_sdk::{files, default_client};

async fn get_dropbox_hash(path: &str, access_token: &str) -> Result<String> {
    let client = default_client(access_token)?;
    let metadata = files::get_metadata(&client, path).await?;
    Ok(metadata.content_hash) // SHA-256 based
}
```

**Authentication**:
- OAuth2 flow or API token
- Store token in `~/.config/ebook-renamer/tokens.toml`
- Support environment variable: `DROPBOX_ACCESS_TOKEN`

#### Google Drive API
```rust
// Use Google Drive API v3
use google_drive3::{DriveHub, hyper, hyper_rustls, oauth2};

async fn get_gdrive_hash(file_id: &str) -> Result<String> {
    let hub = DriveHub::new(/* auth */);
    let (_, file) = hub.files()
        .get(file_id)
        .param("fields", "md5Checksum")
        .doit().await?;
    Ok(file.md5_checksum.unwrap())
}
```

**Challenges**:
- Google Drive stores files by ID, not path
- Need to map filesystem paths to Drive file IDs
- Requires Google Cloud project + OAuth consent

#### OneDrive API
```rust
// Use Microsoft Graph API
async fn get_onedrive_hash(item_id: &str) -> Result<String> {
    // GET /me/drive/items/{item-id}?select=file
    // Returns file.hashes.sha1Hash
}
```

---

### Phase 4: Virtual Filesystem Detection

**Auto-detect virtual mounts**:
```rust
fn is_virtual_filesystem(path: &Path) -> bool {
    // Check mount point type
    // - rclone: "fuse.rclone"
    // - Mountain Duck: "davfs"
    // - Dropbox desktop: "fuse.dropbox"
    // - Google Drive desktop: "fuse.google-drive"

    #[cfg(target_os = "linux")]
    {
        // Read /proc/mounts
        let output = Command::new("findmnt")
            .arg("-n")
            .arg("-o")
            .arg("FSTYPE")
            .arg(path)
            .output()?;
        let fstype = String::from_utf8(output.stdout)?;
        fstype.contains("fuse") || fstype.contains("dav")
    }

    #[cfg(target_os = "macos")]
    {
        // Use mount command or check filesystem type
        // Dropbox: /Users/name/Dropbox
        path.to_str().map(|s| s.contains("Dropbox")).unwrap_or(false)
    }
}

// Auto-suggest cloud mode
if is_virtual_filesystem(&target_path) && !args.cloud_mode {
    eprintln!("⚠️  Detected virtual filesystem. Consider using --cloud-mode");
}
```

---

## Configuration File Support

```toml
# ~/.config/ebook-renamer/config.toml

[cloud]
mode = "api"  # "metadata" | "api" | "hybrid" | "auto"
provider = "dropbox"  # "dropbox" | "gdrive" | "onedrive"

[cloud.dropbox]
access_token = "your-token-here"  # Or use env var

[cloud.gdrive]
client_id = "your-client-id"
client_secret = "your-secret"
refresh_token = "stored-after-oauth"

[duplicate_detection]
# Metadata-only mode settings
similarity_threshold = 0.85  # 85% filename similarity
require_size_match = true    # Must have exact size match
require_human_review = true  # Always use dry-run for metadata duplicates

# Hybrid mode settings
hash_size_threshold = 10485760  # 10MB - hash files smaller than this
skip_size_threshold = 104857600 # 100MB - skip hashing files larger than this
```

---

## CLI Examples

### Basic Cloud Workflow
```bash
# 1. Scan and rename (no duplicate detection)
ebook-renamer --cloud-mode metadata --dry-run ~/Dropbox/Books

# 2. Review proposed changes in output

# 3. Apply changes
ebook-renamer --cloud-mode metadata ~/Dropbox/Books

# 4. Separately find duplicates using API
ebook-renamer --cloud-mode api \
  --cloud-provider dropbox \
  --only-duplicates \
  ~/Dropbox/Books
```

### Advanced: Hybrid Mode
```bash
# Hash small files, skip large files, metadata for medium
ebook-renamer --cloud-mode hybrid \
  --hash-threshold 5MB \
  --skip-threshold 50MB \
  ~/GoogleDrive/Library
```

### Authentication Setup
```bash
# Interactive OAuth flow
ebook-renamer --cloud-auth dropbox
# Opens browser, saves token to config

# Or use environment variable
export DROPBOX_ACCESS_TOKEN="your-token"
ebook-renamer --cloud-mode api --cloud-provider dropbox ~/Dropbox/Books
```

---

## Implementation Checklist

### Core Features
- [ ] Add `--cloud-mode` flag with options: `metadata`, `api`, `hybrid`
- [ ] Implement metadata-only duplicate detection (filename similarity + size)
- [ ] Add fuzzy string matching library (e.g., `strsim` crate)
- [ ] Skip MD5 calculation when in metadata/api mode
- [ ] Virtual filesystem detection (Linux/macOS/Windows)

### API Integration (Optional, Phase 2)
- [ ] Dropbox SDK integration
- [ ] Google Drive API integration
- [ ] OneDrive Microsoft Graph API
- [ ] OAuth2 flow implementation
- [ ] Token storage and refresh

### Configuration
- [ ] Config file parsing (`~/.config/ebook-renamer/config.toml`)
- [ ] Token management
- [ ] Provider-specific settings

### Testing
- [ ] Unit tests for fuzzy matching algorithm
- [ ] Integration tests with mock cloud APIs
- [ ] Test with real Dropbox/GDrive/OneDrive accounts
- [ ] Cross-language parity for metadata mode

### Documentation
- [ ] Update README with cloud mode instructions
- [ ] OAuth setup guide
- [ ] Troubleshooting guide for virtual filesystems
- [ ] Performance comparison: local vs metadata vs API

---

## Performance Estimates

**Scenario**: 1000 ebook files, 50GB total

| Mode | Time | Network | Accuracy |
|------|------|---------|----------|
| Local (full hash) | ~15 min | 50GB download | 100% |
| Metadata-only | ~10 sec | 0 | ~85% |
| Cloud API | ~30 sec | <1MB | 100% |
| Hybrid (5MB threshold) | ~2 min | ~5GB | ~95% |

**Recommendation**: Use **Cloud API mode** for best balance of speed and accuracy.

---

## Alternative: External Hash Database

**Concept**: Pre-compute hashes, store in sidecar database

```bash
# One-time: build hash database (run on local machine)
ebook-renamer --build-hash-db ~/LocalCopy/Books > books.hashdb

# Upload database to cloud
cp books.hashdb ~/Dropbox/Books/.ebook-renamer.db

# Use database for duplicate detection (no file reading)
ebook-renamer --use-hash-db ~/Dropbox/Books/.ebook-renamer.db ~/Dropbox/Books
```

**Benefits**:
- No cloud API needed
- Fast duplicate detection
- Works offline

**Limitations**:
- Database becomes stale when files added/modified
- Requires initial local processing
- Extra storage for database file

---

## Security Considerations

1. **Token Storage**: Encrypt tokens in config file
2. **API Permissions**: Request minimal scopes (read-only if possible)
3. **Token Expiry**: Handle refresh token rotation
4. **Rate Limiting**: Respect API rate limits (batch requests)
5. **Privacy**: Warn users that API mode sends filenames to cloud provider

---

## Recommended First Step

**Start Simple**:
1. Implement `--cloud-mode metadata` (no API, no auth)
2. Use filename similarity + size matching
3. Always require `--dry-run` review in metadata mode
4. Add warning: "Metadata mode is less accurate - review carefully"

**Example Output**:
```
⚠️  Cloud Metadata Mode (No Hash Verification)
   Duplicate detection based on filename similarity + size match
   Review carefully before applying!

Likely duplicates found (85% confidence):
  KEEP: Category Theory - Steve Awodey.pdf (1.2MB)
  DEL:  Category Theory - Steve Awodey - libgen.pdf (1.2MB)

  KEEP: Abstract Algebra - Dummit Foote.pdf (5.4MB)
  DEL:  Dummit, Foote - Abstract Algebra (Z-Library).pdf (5.4MB)

Uncertain matches (review manually):
  - Topology - Munkres.pdf (3.1MB)
  - Munkres - Topology 2nd Ed.pdf (3.3MB)  # Different size!
```

This gives users value immediately without complex API integration.

---

## Next Steps

**Please clarify**:
1. Which cloud provider do you primarily use?
   - Dropbox
   - Google Drive
   - OneDrive
   - rclone (generic)
   - Other

2. Preferred implementation order?
   - A: Metadata mode first (simple, fast)
   - B: API mode first (accurate, complex)
   - C: Both simultaneously

3. Are you willing to do OAuth setup?
   - Yes: Full API integration
   - No: Stick with metadata mode

4. Acceptable false positive rate for metadata mode?
   - 5% (strict matching)
   - 15% (balanced)
   - 25% (loose matching)
