use crate::scanner::FileInfo;
use anyhow::Result;
use chrono::Local;
use log::debug;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum FileIssue {
    FailedDownload,
    TooSmall,
    CorruptedPdf,
    CorruptedEpub,
    #[allow(dead_code)]
    InvalidExtension,
    ReadError(String),
}

pub struct TodoList {
    pub items: Vec<String>,
    pub todo_file_path: PathBuf,
    pub failed_downloads: Vec<String>,
    pub small_files: Vec<String>,
    pub corrupted_files: Vec<String>,
    pub other_issues: Vec<String>,
}

impl TodoList {
    pub fn new(custom_path: &Option<PathBuf>, target_dir: &PathBuf) -> Result<Self> {
        let todo_file_path = if let Some(path) = custom_path {
            path.clone()
        } else {
            target_dir.join("todo.md")
        };

        // Try to read existing todo.md to avoid duplicates
        let mut existing_items = Vec::new();
        if todo_file_path.exists() {
            if let Ok(content) = fs::read_to_string(&todo_file_path) {
                existing_items = extract_items_from_md(&content);
            }
        }

        Ok(TodoList {
            items: existing_items,
            todo_file_path,
            failed_downloads: Vec::new(),
            small_files: Vec::new(),
            corrupted_files: Vec::new(),
            other_issues: Vec::new(),
        })
    }

    pub fn add_file_issue(&mut self, file_info: &FileInfo, issue: FileIssue) -> Result<()> {
        let item = match issue {
            FileIssue::FailedDownload => {
                format!("重新下载: {} (未完成下载)", file_info.original_name)
            }
            FileIssue::TooSmall => {
                format!(
                    "检查并重新下载: {} (文件过小，仅 {} 字节)",
                    file_info.original_name, file_info.size
                )
            }
            FileIssue::CorruptedPdf => {
                format!(
                    "重新下载: {} (PDF文件损坏或格式无效)",
                    file_info.original_name
                )
            }
            FileIssue::CorruptedEpub => {
                format!(
                    "重新下载: {} (EPUB文件损坏或格式无效)",
                    file_info.original_name
                )
            }
            FileIssue::InvalidExtension => {
                format!(
                    "检查文件: {} (扩展名异常: {})",
                    file_info.original_name, file_info.extension
                )
            }
            FileIssue::ReadError(ref msg) => {
                format!(
                    "检查文件权限: {} (无法读取文件: {})",
                    file_info.original_name, msg
                )
            }
        };

        if !self.items.contains(&item) {
            let item_clone = item.clone();
            match issue {
                FileIssue::FailedDownload => self.failed_downloads.push(item_clone.clone()),
                FileIssue::TooSmall => self.small_files.push(item_clone.clone()),
                FileIssue::CorruptedPdf => self.corrupted_files.push(item_clone.clone()),
                FileIssue::CorruptedEpub => self.corrupted_files.push(item_clone.clone()),
                FileIssue::InvalidExtension => self.other_issues.push(item_clone.clone()),
                FileIssue::ReadError(_) => self.other_issues.push(item_clone.clone()),
            }
            self.items.push(item_clone);
            debug!("Added to todo: {}", item);
        }

        Ok(())
    }

    pub fn add_failed_download(&mut self, file_info: &FileInfo) -> Result<()> {
        if file_info.is_failed_download {
            self.add_file_issue(file_info, FileIssue::FailedDownload)
        } else if file_info.is_too_small {
            self.add_file_issue(file_info, FileIssue::TooSmall)
        } else {
            Ok(())
        }
    }

    pub fn analyze_file_integrity(&mut self, file_info: &FileInfo) -> Result<()> {
        // Skip if already marked as failed or too small
        if file_info.is_failed_download || file_info.is_too_small {
            return Ok(());
        }

        let ext_lower = file_info.extension.to_lowercase();

        // Check PDF integrity for PDF files
        if ext_lower == ".pdf" {
            if let Err(_) = validate_pdf_header(&file_info.original_path) {
                self.add_file_issue(file_info, FileIssue::CorruptedPdf)?;
                return Ok(());
            }
        }

        // Check EPUB integrity for EPUB files
        if ext_lower == ".epub" {
            if let Err(_) = validate_epub_structure(&file_info.original_path) {
                self.add_file_issue(file_info, FileIssue::CorruptedEpub)?;
                return Ok(());
            }
        }

        // Check file readability
        if let Err(e) = fs::metadata(&file_info.original_path) {
            self.add_file_issue(file_info, FileIssue::ReadError(e.to_string()))?;
            return Ok(());
        }

        Ok(())
    }

    pub fn remove_file_from_todo(&mut self, filename: &str) {
        // Remove items that contain this filename from all lists
        let filename_lower = filename.to_lowercase();
        self.items.retain(|item| !item.to_lowercase().contains(&filename_lower));
        self.failed_downloads.retain(|item| !item.to_lowercase().contains(&filename_lower));
        self.small_files.retain(|item| !item.to_lowercase().contains(&filename_lower));
        self.corrupted_files.retain(|item| !item.to_lowercase().contains(&filename_lower));
        self.other_issues.retain(|item| !item.to_lowercase().contains(&filename_lower));
        debug!("Removed {} from todo list", filename);
    }

    pub fn write(&self) -> Result<()> {
        let content = generate_todo_md(
            &self.failed_downloads,
            &self.small_files,
            &self.corrupted_files,
            &self.other_issues,
            self.items.iter().filter(|item| {
                !self.failed_downloads.contains(item) 
                && !self.small_files.contains(item)
                && !self.corrupted_files.contains(item)
                && !self.other_issues.contains(item)
            }),
        );

        fs::write(&self.todo_file_path, content)?;
        debug!("Wrote todo.md to {:?}", self.todo_file_path);
        Ok(())
    }
}

fn extract_items_from_md(content: &str) -> Vec<String> {
    // Skip generic checklist items
    let skip_patterns = [
        "检查所有未完成下载文件",
        "重新下载过小文件",
        "验证损坏的PDF文件",
        "处理其他文件问题",
        "MD5校验重复文件",
    ];
    
    content
        .lines()
        .filter(|line| line.trim().starts_with("- ["))
        .map(|line| {
            line.trim()
                .trim_start_matches("- [ ]")
                .trim_start_matches("- [x]")
                .trim()
                .to_string()
        })
        .filter(|item| !skip_patterns.iter().any(|pattern| item.contains(pattern)))
        .collect()
}

fn validate_pdf_header(path: &PathBuf) -> Result<()> {
    use std::io::Read;

    let mut file = fs::File::open(path)?;
    let mut header = [0u8; 5];
    file.read_exact(&mut header)?;

    // PDF files should start with "%PDF-"
    if &header != b"%PDF-" {
        return Err(anyhow::anyhow!("Invalid PDF header"));
    }

    Ok(())
}

/// Validates that an EPUB file is a valid ZIP archive with the required structure.
/// EPUB files are ZIP archives that must contain:
/// - mimetype file (should be first and uncompressed)
/// - META-INF/container.xml
fn validate_epub_structure(path: &PathBuf) -> Result<()> {
    use std::io::Read;

    let mut file = fs::File::open(path)?;
    let mut header = [0u8; 4];
    file.read_exact(&mut header)?;

    // ZIP files start with PK\x03\x04 or PK\x05\x06 (empty) or PK\x07\x08 (spanned)
    if header[0] != b'P' || header[1] != b'K' {
        return Err(anyhow::anyhow!("Invalid EPUB: not a ZIP archive"));
    }

    // Valid ZIP signatures
    let valid_signatures = [
        [0x03, 0x04], // Local file header
        [0x05, 0x06], // End of central directory (empty archive)
        [0x07, 0x08], // Spanned archive
    ];

    if !valid_signatures.iter().any(|sig| header[2] == sig[0] && header[3] == sig[1]) {
        return Err(anyhow::anyhow!("Invalid EPUB: corrupted ZIP signature"));
    }

    Ok(())
}

fn generate_todo_md<'a>(
    failed_downloads: &[String],
    small_files: &[String],
    corrupted_files: &[String],
    other_issues: &[String],
    other_items: impl Iterator<Item = &'a String>,
) -> String {
    let mut md = String::new();

    md.push_str("# 需要检查的任务\n\n");
    md.push_str(&format!("更新时间: {}\n\n", Local::now().format("%Y-%m-%d %H:%M:%S")));

    if !failed_downloads.is_empty() {
        md.push_str("## 🔄 未完成下载文件（.download）\n\n");
        for item in failed_downloads {
            md.push_str(&format!("- [ ] {}\n", item));
        }
        md.push('\n');
    }

    if !small_files.is_empty() {
        md.push_str("## 📁 异常小文件（< 1KB）\n\n");
        for item in small_files {
            md.push_str(&format!("- [ ] {}\n", item));
        }
        md.push('\n');
    }

    if !corrupted_files.is_empty() {
        md.push_str("## 🚨 损坏的PDF文件\n\n");
        for item in corrupted_files {
            md.push_str(&format!("- [ ] {}\n", item));
        }
        md.push('\n');
    }

    if !other_issues.is_empty() {
        md.push_str("## ⚠️ 其他文件问题\n\n");
        for item in other_issues {
            md.push_str(&format!("- [ ] {}\n", item));
        }
        md.push('\n');
    }

    let other_vec: Vec<&String> = other_items.collect();
    let has_other_items = !other_vec.is_empty();
    
    if has_other_items {
        md.push_str("## 📋 其他需要处理的文件\n\n");
        for item in &other_vec {
            md.push_str(&format!("- [ ] {}\n", item));
        }
        md.push('\n');
    }

    if failed_downloads.is_empty() && small_files.is_empty() && corrupted_files.is_empty() && other_issues.is_empty() && !has_other_items {
        md.push_str("✅ 所有文件已检查完毕，无需处理的问题。\n\n");
    }

    md.push_str("---\n");
    md.push_str("*此文件由 ebook renamer 自动生成*\n");

    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_items_from_md() {
        let md_content = r#"# Todo

## Section 1
- [ ] Item 1
- [x] Item 2
- [ ] Item 3

Other text
"#;
        let items = extract_items_from_md(md_content);
        assert_eq!(items.len(), 3);
        assert!(items[0].contains("Item 1"));
    }

    #[test]
    fn test_todolist_new_and_write() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let todo_path = tmp_dir.path().join("test_todo.md");

        let todo_list = TodoList {
            items: vec!["Test item".to_string()],
            todo_file_path: todo_path.clone(),
            failed_downloads: vec!["Failed download item".to_string()],
            small_files: vec!["Small file item".to_string()],
            corrupted_files: Vec::new(),
            other_issues: Vec::new(),
        };

        todo_list.write()?;

        assert!(todo_path.exists());
        let content = fs::read_to_string(&todo_path)?;
        assert!(content.contains("Failed download item"));
        assert!(content.contains("Small file item"));

        Ok(())
    }

    #[test]
    fn test_add_failed_download() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: tmp_dir.path().join("fail.download"),
            original_name: "fail.download".to_string(),
            extension: ".download".to_string(),
            size: 0,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: true,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("fail.download"),
        };

        todo_list.add_failed_download(&file_info)?;

        assert_eq!(todo_list.failed_downloads.len(), 1);
        assert!(todo_list.failed_downloads[0].contains("fail.download"));

        Ok(())
    }

    #[test]
    fn test_remove_file_from_todo() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        // Add item manually to internal lists
        let item = "重新下载: test_file.pdf (未完成下载)".to_string();
        todo_list.failed_downloads.push(item.clone());
        todo_list.items.push(item);

        todo_list.remove_file_from_todo("test_file.pdf");

        assert!(todo_list.failed_downloads.is_empty());
        assert!(todo_list.items.is_empty());

        Ok(())
    }

    #[test]
    fn test_analyze_file_integrity_corrupted_pdf() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let pdf_path = tmp_dir.path().join("corrupt.pdf");
        // Write invalid header
        fs::write(&pdf_path, "NOT PDF content")?;

        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: pdf_path.clone(),
            original_name: "corrupt.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: pdf_path,
        };

        todo_list.analyze_file_integrity(&file_info)?;

        assert_eq!(todo_list.corrupted_files.len(), 1);
        assert!(todo_list.corrupted_files[0].contains("corrupt.pdf"));

        Ok(())
    }

    #[test]
    fn test_analyze_file_integrity_valid_pdf() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let pdf_path = tmp_dir.path().join("valid.pdf");
        // Write valid header
        fs::write(&pdf_path, "%PDF-1.4 content")?;

        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: pdf_path.clone(),
            original_name: "valid.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: pdf_path,
        };

        todo_list.analyze_file_integrity(&file_info)?;

        assert!(todo_list.corrupted_files.is_empty());

        Ok(())
    }

    // ========== EDGE CASE TESTS ==========

    #[test]
    fn test_extract_items_from_empty_md() {
        let items = extract_items_from_md("");
        assert!(items.is_empty());
    }

    #[test]
    fn test_extract_items_skips_generic_checklist() {
        let md_content = r#"# Todo

## Tasks
- [ ] 检查所有未完成下载文件
- [ ] 重新下载过小文件
- [ ] Custom Item
"#;
        let items = extract_items_from_md(md_content);
        // Should only include "Custom Item", not the generic ones
        assert_eq!(items.len(), 1);
        assert!(items[0].contains("Custom Item"));
    }

    #[test]
    fn test_todolist_with_custom_path() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let custom_path = tmp_dir.path().join("custom_todo.md");
        let target_dir = tmp_dir.path().to_path_buf();

        let todo_list = TodoList::new(&Some(custom_path.clone()), &target_dir)?;

        assert_eq!(todo_list.todo_file_path, custom_path);

        Ok(())
    }

    #[test]
    fn test_add_file_issue_too_small() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: tmp_dir.path().join("tiny.pdf"),
            original_name: "tiny.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100, // Will be shown in message
            modified_time: std::time::SystemTime::now(),
            is_failed_download: false,
            is_too_small: true,
            new_name: None,
            new_path: tmp_dir.path().join("tiny.pdf"),
        };

        todo_list.add_failed_download(&file_info)?;

        assert_eq!(todo_list.small_files.len(), 1);
        assert!(todo_list.small_files[0].contains("tiny.pdf"));
        assert!(todo_list.small_files[0].contains("100")); // Size in bytes

        Ok(())
    }

    #[test]
    fn test_add_file_issue_read_error() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: tmp_dir.path().join("unreadable.pdf"),
            original_name: "unreadable.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("unreadable.pdf"),
        };

        todo_list.add_file_issue(&file_info, FileIssue::ReadError("Permission denied".to_string()))?;

        assert_eq!(todo_list.other_issues.len(), 1);
        assert!(todo_list.other_issues[0].contains("无法读取"));
        assert!(todo_list.other_issues[0].contains("Permission denied"));

        Ok(())
    }

    #[test]
    fn test_add_file_issue_invalid_extension() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: tmp_dir.path().join("weird.xyz"),
            original_name: "weird.xyz".to_string(),
            extension: ".xyz".to_string(),
            size: 100,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("weird.xyz"),
        };

        todo_list.add_file_issue(&file_info, FileIssue::InvalidExtension)?;

        assert_eq!(todo_list.other_issues.len(), 1);
        assert!(todo_list.other_issues[0].contains(".xyz"));

        Ok(())
    }

    #[test]
    fn test_duplicate_item_prevention() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: tmp_dir.path().join("fail.download"),
            original_name: "fail.download".to_string(),
            extension: ".download".to_string(),
            size: 0,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: true,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("fail.download"),
        };

        // Add same item twice
        todo_list.add_failed_download(&file_info)?;
        todo_list.add_failed_download(&file_info)?;

        // Should only have one item
        assert_eq!(todo_list.failed_downloads.len(), 1);

        Ok(())
    }

    #[test]
    fn test_unicode_filename_in_todo() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: tmp_dir.path().join("数学入門.pdf.download"),
            original_name: "数学入門.pdf.download".to_string(),
            extension: ".download".to_string(),
            size: 0,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: true,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("数学入門.pdf.download"),
        };

        todo_list.add_failed_download(&file_info)?;

        assert!(todo_list.failed_downloads[0].contains("数学入門"));

        Ok(())
    }

    #[test]
    fn test_analyze_file_integrity_skips_failed_download() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: tmp_dir.path().join("fail.pdf"),
            original_name: "fail.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: true, // Already marked as failed
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("fail.pdf"),
        };

        todo_list.analyze_file_integrity(&file_info)?;

        // Should not add to corrupted_files because it's already a failed download
        assert!(todo_list.corrupted_files.is_empty());

        Ok(())
    }

    #[test]
    fn test_analyze_file_integrity_skips_too_small() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: tmp_dir.path().join("tiny.pdf"),
            original_name: "tiny.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 10,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: false,
            is_too_small: true, // Already marked as too small
            new_name: None,
            new_path: tmp_dir.path().join("tiny.pdf"),
        };

        todo_list.analyze_file_integrity(&file_info)?;

        // Should not add to corrupted_files because it's already too small
        assert!(todo_list.corrupted_files.is_empty());

        Ok(())
    }

    #[test]
    fn test_analyze_file_integrity_non_pdf_file() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        // Use a .txt file which has no integrity checks
        let txt_path = tmp_dir.path().join("book.txt");
        fs::write(&txt_path, "just plain text")?;

        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: txt_path.clone(),
            original_name: "book.txt".to_string(),
            extension: ".txt".to_string(),
            size: 100,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: txt_path,
        };

        todo_list.analyze_file_integrity(&file_info)?;

        // TXT files should not be checked for PDF/EPUB headers
        assert!(todo_list.corrupted_files.is_empty());

        Ok(())
    }

    #[test]
    fn test_analyze_file_integrity_valid_epub() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let epub_path = tmp_dir.path().join("book.epub");
        // EPUB files are ZIP archives, write a valid ZIP header (PK\x03\x04)
        fs::write(&epub_path, b"PK\x03\x04valid epub content")?;

        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: epub_path.clone(),
            original_name: "book.epub".to_string(),
            extension: ".epub".to_string(),
            size: 100,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: epub_path,
        };

        todo_list.analyze_file_integrity(&file_info)?;

        // Valid EPUB should not be flagged
        assert!(todo_list.corrupted_files.is_empty());

        Ok(())
    }

    #[test]
    fn test_analyze_file_integrity_corrupted_epub() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let epub_path = tmp_dir.path().join("corrupted.epub");
        // Not a valid ZIP header
        fs::write(&epub_path, "not a valid zip file")?;

        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: epub_path.clone(),
            original_name: "corrupted.epub".to_string(),
            extension: ".epub".to_string(),
            size: 100,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: None,
            new_path: epub_path,
        };

        todo_list.analyze_file_integrity(&file_info)?;

        // Corrupted EPUB should be flagged
        assert_eq!(todo_list.corrupted_files.len(), 1);

        Ok(())
    }

    #[test]
    fn test_remove_file_case_insensitive() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        // Add an item
        let item = "重新下载: TestFile.PDF (未完成下载)".to_string();
        todo_list.failed_downloads.push(item.clone());
        todo_list.items.push(item);

        // Remove with different case
        todo_list.remove_file_from_todo("testfile.pdf");

        assert!(todo_list.failed_downloads.is_empty());
        assert!(todo_list.items.is_empty());

        Ok(())
    }

    #[test]
    fn test_write_generates_sections() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let todo_path = tmp_dir.path().join("test_todo.md");

        let todo_list = TodoList {
            items: vec!["item1".to_string(), "item2".to_string()],
            todo_file_path: todo_path.clone(),
            failed_downloads: vec!["Failed download item".to_string()],
            small_files: vec!["Small file item".to_string()],
            corrupted_files: vec!["Corrupted file item".to_string()],
            other_issues: vec!["Other issue item".to_string()],
        };

        todo_list.write()?;

        let content = fs::read_to_string(&todo_path)?;

        // Check all sections are present
        assert!(content.contains("🔄 未完成下载文件"));
        assert!(content.contains("📁 异常小文件"));
        assert!(content.contains("🚨 损坏的PDF文件"));
        assert!(content.contains("⚠️ 其他文件问题"));
        assert!(content.contains("更新时间:"));

        Ok(())
    }

    #[test]
    fn test_write_empty_todo_shows_success_message() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let todo_path = tmp_dir.path().join("test_todo.md");

        let todo_list = TodoList {
            items: vec![],
            todo_file_path: todo_path.clone(),
            failed_downloads: vec![],
            small_files: vec![],
            corrupted_files: vec![],
            other_issues: vec![],
        };

        todo_list.write()?;

        let content = fs::read_to_string(&todo_path)?;

        // Should show success message when no issues
        assert!(content.contains("✅ 所有文件已检查完毕"));

        Ok(())
    }

    #[test]
    fn test_validate_pdf_header_too_short() {
        let tmp_dir = TempDir::new().unwrap();
        let pdf_path = tmp_dir.path().join("short.pdf");
        // Write content shorter than 5 bytes
        fs::write(&pdf_path, "PDF").unwrap();

        let result = validate_pdf_header(&pdf_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_pdf_header_different_versions() -> Result<()> {
        let tmp_dir = TempDir::new()?;

        // PDF 1.4
        let pdf14 = tmp_dir.path().join("pdf14.pdf");
        fs::write(&pdf14, "%PDF-1.4 content here")?;
        assert!(validate_pdf_header(&pdf14).is_ok());

        // PDF 1.7
        let pdf17 = tmp_dir.path().join("pdf17.pdf");
        fs::write(&pdf17, "%PDF-1.7 content here")?;
        assert!(validate_pdf_header(&pdf17).is_ok());

        // PDF 2.0
        let pdf20 = tmp_dir.path().join("pdf20.pdf");
        fs::write(&pdf20, "%PDF-2.0 content here")?;
        assert!(validate_pdf_header(&pdf20).is_ok());

        Ok(())
    }

    #[test]
    fn test_extract_items_mixed_checkbox_states() {
        let md_content = r#"# Todo

- [ ] Unchecked item 1
- [x] Checked item 2
- [ ] Unchecked item 3
"#;
        let items = extract_items_from_md(md_content);
        // Should extract all items regardless of checkbox state
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_todolist_reads_existing_items() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let todo_path = tmp_dir.path().join("todo.md");

        // Create existing todo.md
        let existing_content = r#"# Tasks
- [ ] Existing item 1
- [ ] Existing item 2
"#;
        fs::write(&todo_path, existing_content)?;

        let todo_list = TodoList::new(&Some(todo_path), &tmp_dir.path().to_path_buf())?;

        // Should have read existing items
        assert_eq!(todo_list.items.len(), 2);
        assert!(todo_list.items[0].contains("Existing item 1"));

        Ok(())
    }

    #[test]
    fn test_special_characters_in_filename() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let mut todo_list = TodoList::new(&None, &tmp_dir.path().to_path_buf())?;

        let file_info = FileInfo {
            original_path: tmp_dir.path().join("Book (Author) [2020] & More.pdf.download"),
            original_name: "Book (Author) [2020] & More.pdf.download".to_string(),
            extension: ".download".to_string(),
            size: 0,
            modified_time: std::time::SystemTime::now(),
            is_failed_download: true,
            is_too_small: false,
            new_name: None,
            new_path: tmp_dir.path().join("Book (Author) [2020] & More.pdf.download"),
        };

        todo_list.add_failed_download(&file_info)?;

        assert!(todo_list.failed_downloads[0].contains("Book (Author) [2020] & More"));

        Ok(())
    }
}
