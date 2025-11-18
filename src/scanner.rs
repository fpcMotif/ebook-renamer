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

        let is_failed_download = original_name.ends_with(".download") || original_name.ends_with(".crdownload");
        // Only check size for PDF and EPUB files (txt files can be small)
        let is_ebook = extension == ".pdf" || extension == ".epub";
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
}

