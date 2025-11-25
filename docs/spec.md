# Ebook Renamer Specification

This document defines the canonical behavior for all ebook-renamer implementations across different languages.

## 1. CLI Parameters

### Required Arguments
- `[PATH]` - Target directory to scan and rename (defaults to current directory)

### Options
| Flag | Default | Behavior |
|------|---------|----------|
| `--dry-run`, `-d` | `false` | Show changes without applying them. Always writes `todo.md` even in dry-run mode. |
| `--max-depth <DEPTH>` | `usize::MAX` | Maximum directory depth to traverse. |
| `--no-recursive` | `false` | Sets effective max-depth to 1 (top-level only). |
| `--extensions <EXT1,EXT2>` | `"pdf,epub,txt"` | Comma-separated extensions to process (currently unused in scanning). |
| `--no-delete` | `false` | Don't delete duplicate files, only list them. |
| `--todo-file <PATH>` | `<target-dir>/todo.md` | Path to write todo.md file. |
| `--log-file <PATH>` | `None` | Optional path to write detailed operation log (currently unused). |
| `--preserve-unicode` | `false` | Preserve original non-Latin script (currently unused). |
| `--fetch-arxiv` | `false` | Fetch arXiv metadata via API (placeholder only). |
| `--verbose`, `-v` | `false` | Enable verbose logging (currently unused). |
| `--delete-small` | `false` | Silently delete small/corrupted files (< 1KB) **without** adding them to todo.md. (默认情况下依旧会记录 todo，然后清理文件) |
| `--json` | `false` | Output operations in JSON format instead of human-readable text. |

### Output Behavior
- Human-readable mode: Prints operations to stdout with status messages
- JSON mode: Outputs only valid JSON to stdout, suppresses all other messages
- `todo.md` is always written to `<target-dir>/todo.md` unless overridden

## 2. File Scanning Rules

### Extension Detection
- `.tar.gz` files: extension = `.tar.gz`
- `.download` files: extension = `.download`, marked as failed download
- `.crdownload` files: extension = `.crdownload`, marked as failed download
- Other files: extension = `.<ext>` from path, empty string if no extension

### File Classification
- **Failed download**: filename ends with `.download` or `.crdownload`
- **Too small**: extension is `.pdf` or `.epub` AND size < 1024 bytes AND not failed download
- **Normal file**: all other files

### Directory Traversal
- Uses `WalkDir` with configurable max depth
- Skips hidden files/directories (names starting with `.`)
- Skips specific directory names at any level: `Xcode`, `node_modules`, `.git`, `__pycache__`
- **Note**: Current implementation only skips the directory entry itself, not its subtree

### FileInfo Structure
```rust
struct FileInfo {
    original_path: PathBuf,
    original_name: String,
    extension: String,
    size: u64,
    modified_time: SystemTime,
    is_failed_download: bool,
    is_too_small: bool,
    new_name: Option<String>,
    new_path: PathBuf,
}
```

## 3. Filename Normalization Rules

### Processing Order
1. Remove `.download` suffix (if present)
2. Remove extension suffix
3. Strip leading/trailing whitespace
4. Remove series prefixes
5. Clean source indicators
6. Extract year
7. Remove year patterns from title
8. Split authors and title
9. Clean title components
10. Generate new filename

### Series Prefix Removal
These exact prefixes are stripped with following `-` or space:
- `London Mathematical Society Lecture Note Series`
- `Graduate Texts in Mathematics`
- `Progress in Mathematics`
- `[Springer-Lehrbuch]`
- `[Graduate studies in mathematics`
- `[Progress in Mathematics №`
- `[AMS Mathematical Surveys and Monographs`

### Source Indicator Removal
These exact suffixes are removed:
- ` - libgen.li`
- ` - libgen`
- ` - Z-Library`
- ` - z-Library`
- ` - Anna's Archive`
- ` (Z-Library)`
- ` (z-Library)`
- ` (libgen.li)`
- ` (libgen)`
- ` (Anna's Archive)`
- ` libgen.li.pdf`
- ` libgen.pdf`
- ` Z-Library.pdf`
- ` z-Library.pdf`
- ` Anna's Archive.pdf`

Additional patterns removed:
- `Uploaded by ...` (e.g., "Uploaded by user123")
- `Via ...`
- Website URLs (e.g., `www.example.com`, `site.net`, etc.)

### Year Extraction
- Pattern: `\b(19|20)\d{2}\b`
- Returns the **last** year found (rightmost match)
- Removes year patterns:
  - `(YYYY, Publisher)`
  - `(YYYY)`
  - `YYYY, Publisher`

### Author/Title Splitting
1. Check for trailing `(Author)` pattern - if content looks like author
2. Check for `" - "` separator (rightmost match)
3. Check for `":"` separator
4. If no clear separator, treat entire string as title

### Author Detection Rules
- Length ≥ 2 characters
- Does not contain: `auth.`, `translator`, `translated by`, `Z-Library`, `libgen`, `Anna's Archive`, `2-Library`
- Contains at least one uppercase letter
- Author name is cleaned by removing trailing `(auth.)` patterns

### Title Cleaning
- Remove source indicators (same list as above)
- Remove `.download` suffixes
- Remove `(auth.)` patterns
- Clean orphaned brackets/parentheses:
  - Unclosed opening brackets/parentheses are removed
  - Orphaned closing brackets/parentheses are removed
- Replace underscores with spaces
- Collapse multiple spaces to single space
- Trim leading/trailing `- : , ;`

### Final Filename Format
- With author and year: `Author - Title (Year).ext`
- With author, no year: `Author - Title.ext`
- No author: `Title (Year).ext` or `Title.ext`

## 4. Duplicate Detection Strategy

### Allowed Extensions
Only these extensions are considered for duplicate detection:
- `.pdf`
- `.epub`
- `.txt`
- `.mobi` is **NOT** included (per user decision)

### Retention Priority
When multiple files have identical MD5 hash:
1. **Files with `new_name` set** (already normalized) have priority
2. **Shallowest path** (fewest directory components)
3. **Newest modification time**

### MD5 Calculation
- Stream-based reading with 8KB buffer
- Applied only to non-failed, non-small files with allowed extensions

## 5. Todo List Generation

### Categories and Messages
| Category | Chinese Message Template | Example |
|----------|-------------------------|---------|
| `failed_download` | `重新下载: {filename} (未完成下载)` | `重新下载: book.pdf.download (未完成下载)` |
| `too_small` | `检查并重新下载: {filename} (文件过小，仅 {size} 字节)` | `检查并重新下载: tiny.pdf (文件过小，仅 500 字节)` |
| `corrupted_pdf` | `重新下载: {filename} (PDF文件损坏或格式无效)` | `重新下载: broken.pdf (PDF文件损坏或格式无效)` |
| `invalid_extension` | `检查文件: {filename} (扩展名异常: {ext})` | `检查文件: weird.xyz (扩展名异常: .xyz)` |
| `read_error` | `检查文件权限: {filename} (无法读取文件)` | `检查文件: locked.pdf (无法读取文件)` |

### Markdown Structure
```markdown
# 需要检查的任务

更新时间: YYYY-MM-DD HH:MM:SS

## 🔄 未完成下载文件（.download）
- [ ] Item 1
- [ ] Item 2

## 📁 异常小文件（< 1KB）
- [ ] Item 1

## 🚨 损坏的PDF文件
- [ ] Item 1

## ⚠️ 其他文件问题
- [ ] Item 1

---
*此文件由 ebook renamer 自动生成*
```

### Duplicate Prevention
- Reads existing `todo.md` and extracts current items
- Skips generic checklist items (检查所有未完成下载文件, etc.)
- Avoids adding duplicate entries
- Removes items from todo when files are deleted

### 自动清理逻辑
- `.download` / `.crdownload` 文件、异常小的 `.pdf/.epub`（<1KB）以及判定为 `CorruptedPdf` 的 PDF 均会在记录 todo 后立即从磁盘中删除，避免污染下载目录。
- `--delete-small` 选项仅控制这些文件是否写入 todo：开启后会静默删除且不生成提醒；默认情况下仍然会生成 todo 项以便后续重新下载。
- 删除动作会在 CLI 输出中按问题类型汇总，也会写入 JSON `small_or_corrupted_deletes` 数组。

## 6. JSON Output Schema

### Format
```json
{
  "renames": [
    {
      "from": "relative/path/from/root.ext",
      "to": "relative/path/from/root.ext", 
      "reason": "normalized"
    }
  ],
  "duplicate_deletes": [
    {
      "keep": "path/to/keep.ext",
      "delete": ["path/to/delete1.ext", "path/to/delete2.ext"]
    }
  ],
  "small_or_corrupted_deletes": [
    {
      "path": "path/to/small.ext",
      "issue": "failed_download"
    }
  ],
  "todo_items": [
    {
      "category": "failed_download",
      "file": "filename.ext",
      "message": "中文消息"
    }
  ]
}
```

### Path Conventions
- All paths are relative to the target directory
- Uses POSIX-style separators (`/`)
- Empty strings for paths that cannot be made relative

### Output Behavior
- When `--json` flag is used, only valid JSON is printed to stdout
- All other messages (success messages, progress info) are suppressed
- `todo.md` is still written to disk as usual

### Array Sorting Requirements
For cross-language consistency, all JSON arrays are sorted deterministically:
- `renames`: sorted by `from` field (lexicographically)
- `duplicate_deletes`: sorted by `keep` field, with `delete` arrays sorted internally
- `small_or_corrupted_deletes`: sorted by `path` field
- `todo_items`: sorted by `category` field, then by `file` field

### Cleanup Reason Codes
- `failed_download`: `.download` / `.crdownload` 文件被清理
- `too_small`: PDF/EPUB 体积 < 1KB
- `corrupted_pdf`: PDF 头部损坏（缺少 `%PDF-`）

## 7. Edge Cases and Current Limitations

### Known Issues
- Hidden directory traversal only skips the directory entry, not the entire subtree
- `--extensions`, `--log-file`, `--preserve-unicode`, `--verbose` flags are currently unused
- `--fetch-arxiv` is placeholder only

### File Encoding
- Filenames must be valid UTF-8
- Non-UTF-8 filenames are skipped with error

### PDF Validation
- Only checks first 5 bytes for `%PDF-` header
- Does not validate full PDF structure

### Unicode Handling
- Current implementation processes Unicode characters without special handling
- No transliteration performed (preserve-unicode flag unused)

## 8. Test Data Requirements

### Supported Extensions for Testing
- Input files: `.pdf`, `.epub`, `.txt`, `.mobi`, `.download`, `.crdownload`
- Processing: `.pdf`, `.epub`, `.txt` (duplicates and normalization)
- Ignored for duplicates: `.mobi` (but included in scanning)

### Test File Categories
- Clean files: Properly formatted `Author - Title (Year).ext`
- Noisy files: Various source indicators, years, series prefixes
- Failed downloads: `.download`, `.crdownload` extensions
- Small files: < 1KB PDF/EPUB files
- Corrupted files: Non-PDF files with `.pdf` extension
- Duplicates: Same content, different filenames/paths

### Integration with /Users/f/Downloads
- Real test fixtures should be copied from `/Users/f/Downloads/`
- Script: `tests/tools/import_from_downloads.py`
- Target: `tests/fixtures/noisy/`
- Extensions copied: `.pdf`, `.epub`, `.txt`, `.mobi`, `.download`, `.crdownload`
