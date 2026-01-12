use anyhow::{anyhow, Result};
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub original_path: PathBuf,
    pub original_name: String,
    pub extension: String,
    pub size: u64,
    pub modified_time: std::time::SystemTime,
    pub is_failed_download: bool,
    pub is_too_small: bool,
    pub new_name: Option<String>,
    pub new_path: PathBuf,
}

pub struct Scanner {
    root_path: PathBuf,
    max_depth: usize,
}

impl Scanner {
    pub fn new(path: &Path, max_depth: usize) -> Result<Self> {
        let root_path = path.canonicalize()?;
        if !root_path.is_dir() {
            return Err(anyhow!("Path is not a directory: {:?}", path));
        }
        Ok(Scanner {
            root_path,
            max_depth,
        })
    }

    pub fn scan(&mut self) -> Result<Vec<FileInfo>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.root_path)
            .max_depth(self.max_depth)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip directories, hidden files, and system directories
            if path.is_dir() || self.should_skip(path) {
                continue;
            }

            // Check for interesting extensions
            if let Ok(file_info) = self.create_file_info(path) {
                files.push(file_info);
            }
        }

        debug!("Scanner found {} files", files.len());
        Ok(files)
    }

    fn create_file_info(&self, path: &Path) -> Result<FileInfo> {
        let metadata = fs::metadata(path)?;
        let size = metadata.len();
        let modified_time = metadata.modified()?;

        let original_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid filename: {:?}", path))?
            .to_string();

        // Detect extension (including .tar.gz and failed downloads)
        let extension = if original_name.ends_with(".tar.gz") {
            ".tar.gz".to_string()
        } else if original_name.ends_with(".download") {
            ".download".to_string()
        } else if original_name.ends_with(".crdownload") {
            ".crdownload".to_string()
        } else {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e))
                .unwrap_or_default()
        };

        let is_failed_download =
            original_name.ends_with(".download") || original_name.ends_with(".crdownload");
        let is_ebook = extension.eq_ignore_ascii_case(".pdf") || extension.eq_ignore_ascii_case(".epub");
        let is_too_small = !is_failed_download && is_ebook && size < 1024; // Less than 1KB

        Ok(FileInfo {
            original_path: path.to_path_buf(),
            original_name,
            extension,
            size,
            modified_time,
            is_failed_download,
            is_too_small,
            new_name: None,
            new_path: path.to_path_buf(),
        })
    }

    fn should_skip(&self, path: &Path) -> bool {
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            // Skip hidden files/folders
            if filename.starts_with('.') {
                return true;
            }

            // Skip download folders only (not files) - they're handled by download_recovery module
            if path.is_dir() && (filename.ends_with(".download") || filename.ends_with(".crdownload")) {
                return true;
            }

            // Skip known system directories
            let skip_dirs = ["Xcode", "node_modules", ".git", "__pycache__"];
            if skip_dirs.iter().any(|d| filename == *d) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_scanner_creates_correct_file_info() {
        let tmp_dir = TempDir::new().unwrap();
        let test_file = tmp_dir.path().join("test_book.pdf");
        // Create content larger than 1KB (1024 bytes)
        let large_content = "This is a test file that is definitely larger than 1KB. ".repeat(50);
        fs::write(&test_file, large_content).unwrap();

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        assert_eq!(file_info.original_name, "test_book.pdf");
        assert_eq!(file_info.extension, ".pdf");
        assert!(!file_info.is_failed_download);
        assert!(!file_info.is_too_small);
        // Check that modified_time is set (should be recent)
        assert!(file_info.modified_time <= std::time::SystemTime::now());
    }

    #[test]
    fn test_scanner_detects_tar_gz() {
        let tmp_dir = TempDir::new().unwrap();
        let test_file = tmp_dir.path().join("arXiv-2012.08669v1.tar.gz");
        fs::write(&test_file, "test content").unwrap();

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        assert_eq!(file_info.extension, ".tar.gz");
    }

    #[test]
    fn test_scanner_detects_download_files() {
        let tmp_dir = TempDir::new().unwrap();
        let test_file = tmp_dir.path().join("test_book.pdf.download");
        fs::write(&test_file, "").unwrap();

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        assert!(file_info.is_failed_download);
    }

    #[test]
    fn test_scanner_detects_small_files() {
        let tmp_dir = TempDir::new().unwrap();
        let test_file = tmp_dir.path().join("tiny.pdf");
        fs::write(&test_file, "x").unwrap(); // 1 byte

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        assert!(file_info.is_too_small);
    }

    // ========== EDGE CASE TESTS ==========

    #[test]
    fn test_scanner_detects_crdownload_files() {
        // Chrome partial downloads use .crdownload extension
        let tmp_dir = TempDir::new().unwrap();
        let test_file = tmp_dir.path().join("test_book.pdf.crdownload");
        fs::write(&test_file, "partial content").unwrap();

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        assert!(file_info.is_failed_download);
        assert_eq!(file_info.extension, ".crdownload");
    }

    #[test]
    fn test_scanner_skips_hidden_files() {
        let tmp_dir = TempDir::new().unwrap();
        let hidden_file = tmp_dir.path().join(".hidden_book.pdf");
        fs::write(&hidden_file, "content").unwrap();

        let mut scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let files = scanner.scan().unwrap();

        assert!(files.is_empty());
    }

    #[test]
    fn test_scanner_handles_nested_directories() {
        // Test that scanner can handle nested directory structures
        let tmp_dir = TempDir::new().unwrap();
        let nested_dir = tmp_dir.path().join("subdir1").join("subdir2");
        fs::create_dir_all(&nested_dir).unwrap();
        let large_content = "x".repeat(2000);
        let file_in_nested = nested_dir.join("book.pdf");
        fs::write(&file_in_nested, &large_content).unwrap();

        let mut scanner = Scanner::new(tmp_dir.path(), 5).unwrap();
        let files = scanner.scan().unwrap();

        // File in nested directory should be found
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].original_name, "book.pdf");
    }

    #[test]
    fn test_scanner_respects_depth() {
        // Verify that max_depth limits how deep we scan
        let tmp_dir = TempDir::new().unwrap();
        let large_content = "x".repeat(2000);

        // File at depth 1
        fs::write(tmp_dir.path().join("root.pdf"), &large_content).unwrap();

        // File at depth 2
        let level1 = tmp_dir.path().join("level1");
        fs::create_dir(&level1).unwrap();
        fs::write(level1.join("file1.pdf"), &large_content).unwrap();

        // Depth 1 should only find root.pdf
        let mut scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 1);

        // Depth 2 should find both
        let mut scanner = Scanner::new(tmp_dir.path(), 2).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_scanner_empty_directory() {
        let tmp_dir = TempDir::new().unwrap();

        let mut scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let files = scanner.scan().unwrap();

        assert!(files.is_empty());
    }

    #[test]
    fn test_scanner_multiple_file_types() {
        let tmp_dir = TempDir::new().unwrap();
        let large_content = "x".repeat(2000);

        fs::write(tmp_dir.path().join("book.pdf"), &large_content).unwrap();
        fs::write(tmp_dir.path().join("book.epub"), &large_content).unwrap();
        fs::write(tmp_dir.path().join("notes.txt"), &large_content).unwrap();
        fs::write(tmp_dir.path().join("archive.tar.gz"), &large_content).unwrap();
        fs::write(tmp_dir.path().join("document.mobi"), &large_content).unwrap();

        let mut scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let files = scanner.scan().unwrap();

        assert_eq!(files.len(), 5);

        let extensions: Vec<&str> = files.iter().map(|f| f.extension.as_str()).collect();
        assert!(extensions.contains(&".pdf"));
        assert!(extensions.contains(&".epub"));
        assert!(extensions.contains(&".txt"));
        assert!(extensions.contains(&".tar.gz"));
        assert!(extensions.contains(&".mobi"));
    }

    #[test]
    fn test_scanner_max_depth_limit() {
        let tmp_dir = TempDir::new().unwrap();
        let large_content = "x".repeat(2000);

        // Create file at depth 1
        fs::write(tmp_dir.path().join("book1.pdf"), &large_content).unwrap();

        // Create file at depth 2
        let subdir = tmp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("book2.pdf"), &large_content).unwrap();

        // Create file at depth 3
        let subsubdir = subdir.join("subsubdir");
        fs::create_dir(&subsubdir).unwrap();
        fs::write(subsubdir.join("book3.pdf"), &large_content).unwrap();

        // With max_depth=1, should only find book1.pdf
        let mut scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].original_name.contains("book1"));

        // With max_depth=2, should find book1.pdf and book2.pdf
        let mut scanner = Scanner::new(tmp_dir.path(), 2).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 2);

        // With max_depth=3, should find all three
        let mut scanner = Scanner::new(tmp_dir.path(), 3).unwrap();
        let files = scanner.scan().unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_scanner_epub_small_file_detection() {
        let tmp_dir = TempDir::new().unwrap();
        let test_file = tmp_dir.path().join("tiny.epub");
        fs::write(&test_file, "x").unwrap(); // 1 byte

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        assert!(file_info.is_too_small);
    }

    #[test]
    fn test_scanner_txt_not_checked_for_small_size() {
        // TXT files should NOT be flagged as too small
        let tmp_dir = TempDir::new().unwrap();
        let test_file = tmp_dir.path().join("notes.txt");
        fs::write(&test_file, "x").unwrap(); // 1 byte

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        // TXT files are allowed to be small
        assert!(!file_info.is_too_small);
    }

    #[test]
    fn test_scanner_mobi_not_checked_for_small_size() {
        // MOBI files should NOT be flagged as too small
        let tmp_dir = TempDir::new().unwrap();
        let test_file = tmp_dir.path().join("book.mobi");
        fs::write(&test_file, "x").unwrap(); // 1 byte

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        // MOBI is not in the small file check list
        assert!(!file_info.is_too_small);
    }

    #[test]
    fn test_scanner_size_threshold_exactly_1kb() {
        let tmp_dir = TempDir::new().unwrap();

        // Exactly 1024 bytes should NOT be too small
        let test_file = tmp_dir.path().join("exact_1kb.pdf");
        fs::write(&test_file, "x".repeat(1024)).unwrap();

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        assert!(!file_info.is_too_small);
    }

    #[test]
    fn test_scanner_size_threshold_1023_bytes() {
        let tmp_dir = TempDir::new().unwrap();

        // 1023 bytes should be too small
        let test_file = tmp_dir.path().join("under_1kb.pdf");
        fs::write(&test_file, "x".repeat(1023)).unwrap();

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        assert!(file_info.is_too_small);
    }

    #[test]
    fn test_scanner_file_without_extension() {
        let tmp_dir = TempDir::new().unwrap();
        let test_file = tmp_dir.path().join("README");
        fs::write(&test_file, "content").unwrap();

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        assert_eq!(file_info.extension, "");
        assert_eq!(file_info.original_name, "README");
    }

    #[test]
    fn test_scanner_unicode_filename() {
        let tmp_dir = TempDir::new().unwrap();
        let test_file = tmp_dir.path().join("数学入門.pdf");
        let large_content = "x".repeat(2000);
        fs::write(&test_file, large_content).unwrap();

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        assert_eq!(file_info.original_name, "数学入門.pdf");
        assert_eq!(file_info.extension, ".pdf");
    }

    #[test]
    fn test_scanner_invalid_directory() {
        let result = Scanner::new(Path::new("/nonexistent/path/12345"), 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_scanner_file_as_root_path() {
        let tmp_dir = TempDir::new().unwrap();
        let test_file = tmp_dir.path().join("book.pdf");
        fs::write(&test_file, "content").unwrap();

        // Trying to use a file as the scanner root should fail
        let result = Scanner::new(&test_file, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_scanner_handles_download_file_extension() {
        let tmp_dir = TempDir::new().unwrap();

        // Create a .download file (Safari partial download)
        let download_file = tmp_dir.path().join("book.pdf.download");
        fs::write(&download_file, "partial content").unwrap();

        let mut scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let files = scanner.scan().unwrap();

        // .download FILE should be found and marked as failed download
        assert_eq!(files.len(), 1);
        assert!(files[0].is_failed_download);
    }

    #[test]
    fn test_scanner_handles_crdownload_file_extension() {
        let tmp_dir = TempDir::new().unwrap();

        // Create a .crdownload file (Chrome partial download)
        let crdownload_file = tmp_dir.path().join("book.pdf.crdownload");
        fs::write(&crdownload_file, "partial content").unwrap();

        let mut scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let files = scanner.scan().unwrap();

        // .crdownload FILE should be found and marked as failed download
        assert_eq!(files.len(), 1);
        assert!(files[0].is_failed_download);
    }

    #[test]
    fn test_scanner_preserves_relative_path_structure() {
        let tmp_dir = TempDir::new().unwrap();
        let large_content = "x".repeat(2000);

        let subdir = tmp_dir.path().join("books").join("2024");
        fs::create_dir_all(&subdir).unwrap();
        fs::write(subdir.join("book.pdf"), large_content).unwrap();

        let mut scanner = Scanner::new(tmp_dir.path(), 10).unwrap();
        let files = scanner.scan().unwrap();

        assert_eq!(files.len(), 1);
        // The original_path should contain the full path
        assert!(files[0].original_path.to_string_lossy().contains("books"));
        assert!(files[0].original_path.to_string_lossy().contains("2024"));
    }

    #[test]
    fn test_scanner_file_size_recorded() {
        let tmp_dir = TempDir::new().unwrap();
        let content = "Hello, World!"; // 13 bytes
        let test_file = tmp_dir.path().join("small.txt");
        fs::write(&test_file, content).unwrap();

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        assert_eq!(file_info.size, 13);
    }

    #[test]
    fn test_scanner_failed_download_not_flagged_as_small() {
        let tmp_dir = TempDir::new().unwrap();

        // A failed download file that's also small should only be marked as failed download
        let test_file = tmp_dir.path().join("book.pdf.download");
        fs::write(&test_file, "x").unwrap(); // 1 byte

        let scanner = Scanner::new(tmp_dir.path(), 1).unwrap();
        let file_info = scanner.create_file_info(&test_file).unwrap();

        assert!(file_info.is_failed_download);
        // Even though it's small, it shouldn't be flagged as too_small because it's a failed download
        assert!(!file_info.is_too_small);
    }
}
