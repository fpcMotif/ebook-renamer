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
    #[allow(dead_code)]
    InvalidExtension,
    ReadError,
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
            FileIssue::InvalidExtension => {
                format!(
                    "检查文件: {} (扩展名异常: {})",
                    file_info.original_name, file_info.extension
                )
            }
            FileIssue::ReadError => {
                format!("检查文件权限: {} (无法读取文件)", file_info.original_name)
            }
        };

        if !self.items.contains(&item) {
            let item_clone = item.clone();
            match issue {
                FileIssue::FailedDownload => self.failed_downloads.push(item_clone.clone()),
                FileIssue::TooSmall => self.small_files.push(item_clone.clone()),
                FileIssue::CorruptedPdf => self.corrupted_files.push(item_clone.clone()),
                FileIssue::InvalidExtension | FileIssue::ReadError => {
                    self.other_issues.push(item_clone.clone())
                }
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

    pub fn analyze_file_integrity(&mut self, file_info: &FileInfo) -> Result<Option<FileIssue>> {
        // Skip if already marked as failed or too small
        if file_info.is_failed_download || file_info.is_too_small {
            return Ok(None);
        }

        // Check PDF integrity for PDF files
        if file_info.extension.to_lowercase() == ".pdf" {
            if let Err(_) = validate_pdf_header(&file_info.original_path) {
                self.add_file_issue(file_info, FileIssue::CorruptedPdf)?;
                return Ok(Some(FileIssue::CorruptedPdf));
            }
        }

        // Check file readability
        if let Err(_) = fs::metadata(&file_info.original_path) {
            self.add_file_issue(file_info, FileIssue::ReadError)?;
            return Ok(Some(FileIssue::ReadError));
        }

        Ok(None)
    }

    pub fn remove_file_from_todo(&mut self, filename: &str) {
        // Remove items that contain this filename from all lists
        let filename_lower = filename.to_lowercase();
        self.items
            .retain(|item| !item.to_lowercase().contains(&filename_lower));
        self.failed_downloads
            .retain(|item| !item.to_lowercase().contains(&filename_lower));
        self.small_files
            .retain(|item| !item.to_lowercase().contains(&filename_lower));
        self.corrupted_files
            .retain(|item| !item.to_lowercase().contains(&filename_lower));
        self.other_issues
            .retain(|item| !item.to_lowercase().contains(&filename_lower));
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

fn generate_todo_md<'a>(
    failed_downloads: &[String],
    small_files: &[String],
    corrupted_files: &[String],
    other_issues: &[String],
    other_items: impl Iterator<Item = &'a String>,
) -> String {
    let mut md = String::new();

    md.push_str("# 需要检查的任务\n\n");
    md.push_str(&format!(
        "更新时间: {}\n\n",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    ));

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

    if failed_downloads.is_empty()
        && small_files.is_empty()
        && corrupted_files.is_empty()
        && other_issues.is_empty()
        && !has_other_items
    {
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

        let issue = todo_list.analyze_file_integrity(&file_info)?;

        assert_eq!(todo_list.corrupted_files.len(), 1);
        assert!(todo_list.corrupted_files[0].contains("corrupt.pdf"));
        assert!(matches!(issue, Some(FileIssue::CorruptedPdf)));

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

        let issue = todo_list.analyze_file_integrity(&file_info)?;

        assert!(todo_list.corrupted_files.is_empty());
        assert!(issue.is_none());

        Ok(())
    }
}
