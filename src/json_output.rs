use crate::scanner::FileInfo;
use crate::security::shell_escape;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct RenameOperation {
    pub from: String,
    pub to: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub keep: String,
    pub delete: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteOperation {
    pub path: String,
    pub issue: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TodoItem {
    pub category: String,
    pub file: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationsOutput {
    pub renames: Vec<RenameOperation>,
    pub duplicate_deletes: Vec<DuplicateGroup>,
    pub small_or_corrupted_deletes: Vec<DeleteOperation>,
    pub todo_items: Vec<TodoItem>,
}

/// An entry in the undo log for a single executed operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoEntry {
    /// Type of operation: "rename", "delete_duplicate", "delete_small"
    pub operation_type: String,
    /// Original path before the operation
    pub original_path: String,
    /// New path after rename (None for deletes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_path: Option<String>,
    /// File size in bytes (for recovery info on deletes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// MD5 hash if computed (for verification)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5_hash: Option<String>,
    /// Shell command to reverse this operation
    pub undo_command: String,
}

/// Complete undo log with metadata and all operations
#[derive(Debug, Serialize, Deserialize)]
pub struct UndoLog {
    /// Schema version for forward compatibility
    pub version: String,
    /// Timestamp when operations were executed
    pub timestamp: String,
    /// Base directory that was processed
    pub base_directory: String,
    /// Whether this was a dry run (should always be false for actual undo logs)
    pub dry_run: bool,
    /// Total count of operations
    pub total_operations: usize,
    /// All rename operations (reversible)
    pub renames: Vec<UndoEntry>,
    /// Deleted duplicates (NOT reversible - files are gone)
    pub deleted_duplicates: Vec<UndoEntry>,
    /// Deleted small/corrupted files (NOT reversible - files are gone)
    pub deleted_small_files: Vec<UndoEntry>,
}

impl UndoLog {
    /// Create a new undo log with current timestamp
    pub fn new(base_directory: &Path, dry_run: bool) -> Self {
        let now: DateTime<Utc> = Utc::now();
        Self {
            version: "1.0".to_string(),
            timestamp: now.to_rfc3339(),
            base_directory: base_directory.to_string_lossy().to_string(),
            dry_run,
            total_operations: 0,
            renames: Vec::new(),
            deleted_duplicates: Vec::new(),
            deleted_small_files: Vec::new(),
        }
    }

    /// Add a rename operation to the log
    pub fn add_rename(&mut self, from: &Path, to: &Path) {
        let from_str = from.to_string_lossy().to_string();
        let to_str = to.to_string_lossy().to_string();

        // Generate undo command (reverse rename)
        let undo_cmd = format!(
            "mv {} {}",
            shell_escape(&to_str),
            shell_escape(&from_str)
        );

        self.renames.push(UndoEntry {
            operation_type: "rename".to_string(),
            original_path: from_str,
            new_path: Some(to_str),
            size_bytes: None,
            md5_hash: None,
            undo_command: undo_cmd,
        });
        self.total_operations += 1;
    }

    /// Add a deleted duplicate to the log
    pub fn add_deleted_duplicate(&mut self, path: &Path, size: u64, hash: Option<String>) {
        let path_str = path.to_string_lossy().to_string();

        self.deleted_duplicates.push(UndoEntry {
            operation_type: "delete_duplicate".to_string(),
            original_path: path_str,
            new_path: None,
            size_bytes: Some(size),
            md5_hash: hash,
            undo_command: "# File deleted - cannot be undone automatically. Check backups or kept duplicate.".to_string(),
        });
        self.total_operations += 1;
    }

    /// Add a deleted small/corrupted file to the log
    pub fn add_deleted_small(&mut self, path: &Path, size: u64) {
        let path_str = path.to_string_lossy().to_string();

        self.deleted_small_files.push(UndoEntry {
            operation_type: "delete_small".to_string(),
            original_path: path_str,
            new_path: None,
            size_bytes: Some(size),
            md5_hash: None,
            undo_command: "# File deleted - cannot be undone automatically. Re-download if needed.".to_string(),
        });
        self.total_operations += 1;
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Write undo log to file
    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let json = self.to_json()?;
        let mut file = fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Generate a shell script that can undo all rename operations
    pub fn generate_undo_script(&self) -> String {
        let mut script = String::new();
        script.push_str("#!/bin/bash\n");
        script.push_str("# Undo script generated by ebook-renamer\n");
        script.push_str(&format!("# Generated: {}\n", self.timestamp));
        script.push_str(&format!("# Base directory: {}\n\n", self.base_directory));

        script.push_str("set -e  # Exit on any error\n\n");

        if self.renames.is_empty() {
            script.push_str("echo 'No rename operations to undo.'\n");
        } else {
            script.push_str(&format!("echo 'Undoing {} rename operations...'\n\n", self.renames.len()));

            // Reverse order for proper undo
            for entry in self.renames.iter().rev() {
                script.push_str(&format!("{}\n", entry.undo_command));
            }

            script.push_str("\necho 'All rename operations undone successfully.'\n");
        }

        if !self.deleted_duplicates.is_empty() || !self.deleted_small_files.is_empty() {
            script.push_str("\n# WARNING: The following files were deleted and cannot be automatically restored:\n");
            for entry in &self.deleted_duplicates {
                script.push_str(&format!("# - {} (duplicate, {} bytes)\n",
                    entry.original_path,
                    entry.size_bytes.unwrap_or(0)));
            }
            for entry in &self.deleted_small_files {
                script.push_str(&format!("# - {} (small/corrupted, {} bytes)\n",
                    entry.original_path,
                    entry.size_bytes.unwrap_or(0)));
            }
        }

        script
    }
}

impl OperationsOutput {
    pub fn new() -> Self {
        Self {
            renames: Vec::new(),
            duplicate_deletes: Vec::new(),
            small_or_corrupted_deletes: Vec::new(),
            todo_items: Vec::new(),
        }
    }

    pub fn from_results(
        clean_files: Vec<FileInfo>,
        duplicate_groups: Vec<Vec<PathBuf>>,
        files_to_delete: Vec<PathBuf>,
        todo_items: Vec<(String, String, String)>, // (category, file, message)
        target_dir: &PathBuf,
    ) -> Result<Self> {
        let mut output = Self::new();

        // Add renames
        let mut renames = Vec::new();
        for file_info in clean_files {
            if let Some(ref _new_name) = file_info.new_name {
                let from_path = file_info.original_path.strip_prefix(target_dir)
                    .unwrap_or(&file_info.original_path)
                    .to_string_lossy()
                    .to_string();
                let to_path = file_info.new_path.strip_prefix(target_dir)
                    .unwrap_or(&file_info.new_path)
                    .to_string_lossy()
                    .to_string();
                
                renames.push(RenameOperation {
                    from: from_path,
                    to: to_path,
                    reason: "normalized".to_string(),
                });
            }
        }
        // Sort renames by 'from' path for deterministic output
        renames.sort_by(|a, b| a.from.cmp(&b.from));
        output.renames = renames;

        // Add duplicate deletions
        let mut duplicate_deletes = Vec::new();
        for group in duplicate_groups {
            if group.len() > 1 {
                let keep_path = group[0].strip_prefix(target_dir)
                    .unwrap_or(&group[0])
                    .to_string_lossy()
                    .to_string();
                let mut delete_paths: Vec<String> = group.iter().skip(1)
                    .map(|p| p.strip_prefix(target_dir).unwrap_or(p).to_string_lossy().to_string())
                    .collect();
                // Sort delete paths for deterministic output
                delete_paths.sort();
                
                duplicate_deletes.push(DuplicateGroup {
                    keep: keep_path,
                    delete: delete_paths,
                });
            }
        }
        // Sort duplicate groups by 'keep' path for deterministic output
        duplicate_deletes.sort_by(|a, b| a.keep.cmp(&b.keep));
        output.duplicate_deletes = duplicate_deletes;

        // Add small/corrupted deletions
        let mut small_deletes = Vec::new();
        for path in files_to_delete {
            let path_str = path.strip_prefix(target_dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            small_deletes.push(DeleteOperation {
                path: path_str,
                issue: "deleted".to_string(),
            });
        }
        // Sort by path for deterministic output
        small_deletes.sort_by(|a, b| a.path.cmp(&b.path));
        output.small_or_corrupted_deletes = small_deletes;

        // Add todo items
        let mut todos = Vec::new();
        for (category, file, message) in todo_items {
            todos.push(TodoItem {
                category,
                file,
                message,
            });
        }
        // Sort todo items by category, then file for deterministic output
        todos.sort_by(|a, b| {
            a.category.cmp(&b.category)
                .then_with(|| a.file.cmp(&b.file))
        });
        output.todo_items = todos;

        Ok(output)
    }

    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::SystemTime;

    #[test]
    fn test_operations_output_json_serialization() {
        let output = OperationsOutput {
            renames: vec![RenameOperation {
                from: "old.pdf".to_string(),
                to: "new.pdf".to_string(),
                reason: "test".to_string(),
            }],
            duplicate_deletes: vec![DuplicateGroup {
                keep: "keep.pdf".to_string(),
                delete: vec!["delete.pdf".to_string()],
            }],
            small_or_corrupted_deletes: vec![DeleteOperation {
                path: "small.pdf".to_string(),
                issue: "small".to_string(),
            }],
            todo_items: vec![TodoItem {
                category: "Category".to_string(),
                file: "file.pdf".to_string(),
                message: "message".to_string(),
            }],
        };

        let json = output.to_json().unwrap();
        assert!(json.contains("\"from\": \"old.pdf\""));
        assert!(json.contains("\"to\": \"new.pdf\""));
        assert!(json.contains("\"keep\": \"keep.pdf\""));
        // Check for delete.pdf presence without relying on exact whitespace formatting
        assert!(json.contains("\"delete\": ["));
        assert!(json.contains("\"delete.pdf\""));
        assert!(json.contains("\"path\": \"small.pdf\""));
        assert!(json.contains("\"category\": \"Category\""));
    }

    #[test]
    fn test_from_results() {
        let target_dir = PathBuf::from("/tmp");

        // Setup Files
        let file_info = FileInfo {
            original_path: target_dir.join("original.pdf"),
            original_name: "original.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: Some("renamed.pdf".to_string()),
            new_path: target_dir.join("renamed.pdf"),
        };

        let duplicate_group = vec![
            target_dir.join("keep.pdf"),
            target_dir.join("delete.pdf"),
        ];

        let files_to_delete = vec![target_dir.join("small.pdf")];

        let todo_items = vec![
            ("Category".to_string(), "todo.pdf".to_string(), "Check me".to_string())
        ];

        let output = OperationsOutput::from_results(
            vec![file_info],
            vec![duplicate_group],
            files_to_delete,
            todo_items,
            &target_dir,
        ).unwrap();

        assert_eq!(output.renames.len(), 1);
        assert_eq!(output.renames[0].from, "original.pdf");
        assert_eq!(output.renames[0].to, "renamed.pdf");

        assert_eq!(output.duplicate_deletes.len(), 1);
        assert_eq!(output.duplicate_deletes[0].keep, "keep.pdf");
        assert_eq!(output.duplicate_deletes[0].delete[0], "delete.pdf");

        assert_eq!(output.small_or_corrupted_deletes.len(), 1);
        assert_eq!(output.small_or_corrupted_deletes[0].path, "small.pdf");

        assert_eq!(output.todo_items.len(), 1);
        assert_eq!(output.todo_items[0].file, "todo.pdf");
    }

    #[test]
    fn test_relative_paths() {
        let target_dir = PathBuf::from("/base/dir");

        // File path is deeper than target dir
        let file_path = target_dir.join("subdir").join("file.pdf");

        let file_info = FileInfo {
            original_path: file_path.clone(),
            original_name: "file.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: Some("new.pdf".to_string()),
            new_path: target_dir.join("subdir").join("new.pdf"),
        };

        let output = OperationsOutput::from_results(
            vec![file_info],
            vec![],
            vec![],
            vec![],
            &target_dir,
        ).unwrap();

        // Paths should be relative to target_dir
        #[cfg(not(windows))]
        assert_eq!(output.renames[0].from, "subdir/file.pdf");
    }

    // ========== EDGE CASE TESTS ==========

    #[test]
    fn test_empty_operations() {
        let output = OperationsOutput::new();

        assert!(output.renames.is_empty());
        assert!(output.duplicate_deletes.is_empty());
        assert!(output.small_or_corrupted_deletes.is_empty());
        assert!(output.todo_items.is_empty());

        let json = output.to_json().unwrap();
        assert!(json.contains("\"renames\": []"));
        assert!(json.contains("\"duplicate_deletes\": []"));
    }

    #[test]
    fn test_renames_sorted_by_from() {
        let target_dir = PathBuf::from("/tmp");

        let files = vec![
            FileInfo {
                original_path: target_dir.join("zebra.pdf"),
                original_name: "zebra.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 100,
                modified_time: SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("Zebra.pdf".to_string()),
                new_path: target_dir.join("Zebra.pdf"),
            },
            FileInfo {
                original_path: target_dir.join("apple.pdf"),
                original_name: "apple.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 100,
                modified_time: SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("Apple.pdf".to_string()),
                new_path: target_dir.join("Apple.pdf"),
            },
            FileInfo {
                original_path: target_dir.join("mango.pdf"),
                original_name: "mango.pdf".to_string(),
                extension: ".pdf".to_string(),
                size: 100,
                modified_time: SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some("Mango.pdf".to_string()),
                new_path: target_dir.join("Mango.pdf"),
            },
        ];

        let output = OperationsOutput::from_results(
            files,
            vec![],
            vec![],
            vec![],
            &target_dir,
        ).unwrap();

        // Should be sorted alphabetically by 'from'
        assert_eq!(output.renames[0].from, "apple.pdf");
        assert_eq!(output.renames[1].from, "mango.pdf");
        assert_eq!(output.renames[2].from, "zebra.pdf");
    }

    #[test]
    fn test_duplicate_groups_sorted_by_keep() {
        let target_dir = PathBuf::from("/tmp");

        let groups = vec![
            vec![
                target_dir.join("zebra_keep.pdf"),
                target_dir.join("zebra_delete.pdf"),
            ],
            vec![
                target_dir.join("apple_keep.pdf"),
                target_dir.join("apple_delete.pdf"),
            ],
        ];

        let output = OperationsOutput::from_results(
            vec![],
            groups,
            vec![],
            vec![],
            &target_dir,
        ).unwrap();

        // Groups should be sorted by 'keep' field
        assert_eq!(output.duplicate_deletes[0].keep, "apple_keep.pdf");
        assert_eq!(output.duplicate_deletes[1].keep, "zebra_keep.pdf");
    }

    #[test]
    fn test_duplicate_group_deletes_sorted() {
        let target_dir = PathBuf::from("/tmp");

        let groups = vec![
            vec![
                target_dir.join("keep.pdf"),
                target_dir.join("z_delete.pdf"),
                target_dir.join("a_delete.pdf"),
                target_dir.join("m_delete.pdf"),
            ],
        ];

        let output = OperationsOutput::from_results(
            vec![],
            groups,
            vec![],
            vec![],
            &target_dir,
        ).unwrap();

        // Deletes within a group should be sorted
        assert_eq!(output.duplicate_deletes[0].delete[0], "a_delete.pdf");
        assert_eq!(output.duplicate_deletes[0].delete[1], "m_delete.pdf");
        assert_eq!(output.duplicate_deletes[0].delete[2], "z_delete.pdf");
    }

    #[test]
    fn test_small_deletes_sorted_by_path() {
        let target_dir = PathBuf::from("/tmp");

        let deletes = vec![
            target_dir.join("zebra.pdf"),
            target_dir.join("apple.pdf"),
            target_dir.join("mango.pdf"),
        ];

        let output = OperationsOutput::from_results(
            vec![],
            vec![],
            deletes,
            vec![],
            &target_dir,
        ).unwrap();

        assert_eq!(output.small_or_corrupted_deletes[0].path, "apple.pdf");
        assert_eq!(output.small_or_corrupted_deletes[1].path, "mango.pdf");
        assert_eq!(output.small_or_corrupted_deletes[2].path, "zebra.pdf");
    }

    #[test]
    fn test_todo_items_sorted_by_category_then_file() {
        let target_dir = PathBuf::from("/tmp");

        let todos = vec![
            ("corrupted".to_string(), "zebra.pdf".to_string(), "msg".to_string()),
            ("corrupted".to_string(), "apple.pdf".to_string(), "msg".to_string()),
            ("small".to_string(), "mango.pdf".to_string(), "msg".to_string()),
            ("small".to_string(), "banana.pdf".to_string(), "msg".to_string()),
        ];

        let output = OperationsOutput::from_results(
            vec![],
            vec![],
            vec![],
            todos,
            &target_dir,
        ).unwrap();

        // Should be sorted by category first, then file
        assert_eq!(output.todo_items[0].category, "corrupted");
        assert_eq!(output.todo_items[0].file, "apple.pdf");
        assert_eq!(output.todo_items[1].category, "corrupted");
        assert_eq!(output.todo_items[1].file, "zebra.pdf");
        assert_eq!(output.todo_items[2].category, "small");
        assert_eq!(output.todo_items[2].file, "banana.pdf");
        assert_eq!(output.todo_items[3].category, "small");
        assert_eq!(output.todo_items[3].file, "mango.pdf");
    }

    #[test]
    fn test_json_contains_all_fields() {
        let output = OperationsOutput {
            renames: vec![RenameOperation {
                from: "from.pdf".to_string(),
                to: "to.pdf".to_string(),
                reason: "normalized".to_string(),
            }],
            duplicate_deletes: vec![],
            small_or_corrupted_deletes: vec![],
            todo_items: vec![],
        };

        let json = output.to_json().unwrap();

        // Verify JSON structure
        assert!(json.contains("\"from\":"));
        assert!(json.contains("\"to\":"));
        assert!(json.contains("\"reason\":"));
        assert!(json.contains("\"renames\":"));
        assert!(json.contains("\"duplicate_deletes\":"));
        assert!(json.contains("\"small_or_corrupted_deletes\":"));
        assert!(json.contains("\"todo_items\":"));
    }

    #[test]
    fn test_special_characters_in_paths() {
        let target_dir = PathBuf::from("/tmp");

        let file_info = FileInfo {
            original_path: target_dir.join("Book (Author) [2020].pdf"),
            original_name: "Book (Author) [2020].pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: Some("Author - Book (2020).pdf".to_string()),
            new_path: target_dir.join("Author - Book (2020).pdf"),
        };

        let output = OperationsOutput::from_results(
            vec![file_info],
            vec![],
            vec![],
            vec![],
            &target_dir,
        ).unwrap();

        // Special characters should be preserved
        assert_eq!(output.renames[0].from, "Book (Author) [2020].pdf");
        assert_eq!(output.renames[0].to, "Author - Book (2020).pdf");
    }

    #[test]
    fn test_unicode_in_paths() {
        let target_dir = PathBuf::from("/tmp");

        let file_info = FileInfo {
            original_path: target_dir.join("数学入門.pdf"),
            original_name: "数学入門.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: Some("Introduction to Math.pdf".to_string()),
            new_path: target_dir.join("Introduction to Math.pdf"),
        };

        let output = OperationsOutput::from_results(
            vec![file_info],
            vec![],
            vec![],
            vec![],
            &target_dir,
        ).unwrap();

        assert_eq!(output.renames[0].from, "数学入門.pdf");
    }

    #[test]
    fn test_files_without_new_name_not_included() {
        let target_dir = PathBuf::from("/tmp");

        let file_info = FileInfo {
            original_path: target_dir.join("unchanged.pdf"),
            original_name: "unchanged.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: None, // No rename needed
            new_path: target_dir.join("unchanged.pdf"),
        };

        let output = OperationsOutput::from_results(
            vec![file_info],
            vec![],
            vec![],
            vec![],
            &target_dir,
        ).unwrap();

        // Files without new_name should not be in renames
        assert!(output.renames.is_empty());
    }

    #[test]
    fn test_duplicate_group_with_single_file_not_included() {
        let target_dir = PathBuf::from("/tmp");

        // A group with only one file shouldn't create a duplicate entry
        let groups = vec![
            vec![target_dir.join("single.pdf")],
        ];

        let output = OperationsOutput::from_results(
            vec![],
            groups,
            vec![],
            vec![],
            &target_dir,
        ).unwrap();

        // Single file groups should not be included
        assert!(output.duplicate_deletes.is_empty());
    }

    #[test]
    fn test_deep_nested_paths() {
        let target_dir = PathBuf::from("/tmp");
        let deep_path = target_dir
            .join("level1")
            .join("level2")
            .join("level3")
            .join("level4")
            .join("book.pdf");

        let file_info = FileInfo {
            original_path: deep_path.clone(),
            original_name: "book.pdf".to_string(),
            extension: ".pdf".to_string(),
            size: 100,
            modified_time: SystemTime::now(),
            is_failed_download: false,
            is_too_small: false,
            new_name: Some("New Book.pdf".to_string()),
            new_path: target_dir.join("level1").join("level2").join("level3").join("level4").join("New Book.pdf"),
        };

        let output = OperationsOutput::from_results(
            vec![file_info],
            vec![],
            vec![],
            vec![],
            &target_dir,
        ).unwrap();

        // Deep paths should be preserved as relative
        #[cfg(not(windows))]
        assert_eq!(output.renames[0].from, "level1/level2/level3/level4/book.pdf");
    }

    #[test]
    fn test_json_pretty_print() {
        let output = OperationsOutput::new();
        let json = output.to_json().unwrap();

        // Pretty printed JSON should have newlines
        assert!(json.contains('\n'));
        // And indentation
        assert!(json.contains("  "));
    }

    #[test]
    fn test_large_number_of_operations() {
        let target_dir = PathBuf::from("/tmp");

        let mut files = Vec::new();
        for i in 0..100 {
            files.push(FileInfo {
                original_path: target_dir.join(format!("file{:03}.pdf", i)),
                original_name: format!("file{:03}.pdf", i),
                extension: ".pdf".to_string(),
                size: 100,
                modified_time: SystemTime::now(),
                is_failed_download: false,
                is_too_small: false,
                new_name: Some(format!("File {:03}.pdf", i)),
                new_path: target_dir.join(format!("File {:03}.pdf", i)),
            });
        }

        let output = OperationsOutput::from_results(
            files,
            vec![],
            vec![],
            vec![],
            &target_dir,
        ).unwrap();

        assert_eq!(output.renames.len(), 100);
        // Verify still sorted
        assert_eq!(output.renames[0].from, "file000.pdf");
        assert_eq!(output.renames[99].from, "file099.pdf");
    }

    // ========== UNDO LOG TESTS ==========

    #[test]
    fn test_undo_log_new() {
        let base_dir = PathBuf::from("/test/path");
        let log = UndoLog::new(&base_dir, false);

        assert_eq!(log.version, "1.0");
        assert_eq!(log.base_directory, "/test/path");
        assert!(!log.dry_run);
        assert_eq!(log.total_operations, 0);
        assert!(log.renames.is_empty());
        assert!(log.deleted_duplicates.is_empty());
        assert!(log.deleted_small_files.is_empty());
    }

    #[test]
    fn test_undo_log_add_rename() {
        let base_dir = PathBuf::from("/test");
        let mut log = UndoLog::new(&base_dir, false);

        log.add_rename(
            Path::new("/test/old name.pdf"),
            Path::new("/test/new name.pdf"),
        );

        assert_eq!(log.total_operations, 1);
        assert_eq!(log.renames.len(), 1);
        assert_eq!(log.renames[0].operation_type, "rename");
        assert_eq!(log.renames[0].original_path, "/test/old name.pdf");
        assert_eq!(log.renames[0].new_path, Some("/test/new name.pdf".to_string()));
        // Undo command should properly escape paths
        assert!(log.renames[0].undo_command.contains("mv"));
        assert!(log.renames[0].undo_command.contains("'"));
    }

    #[test]
    fn test_undo_log_add_deleted_duplicate() {
        let base_dir = PathBuf::from("/test");
        let mut log = UndoLog::new(&base_dir, false);

        log.add_deleted_duplicate(
            Path::new("/test/duplicate.pdf"),
            12345,
            Some("abc123hash".to_string()),
        );

        assert_eq!(log.total_operations, 1);
        assert_eq!(log.deleted_duplicates.len(), 1);
        assert_eq!(log.deleted_duplicates[0].operation_type, "delete_duplicate");
        assert_eq!(log.deleted_duplicates[0].original_path, "/test/duplicate.pdf");
        assert_eq!(log.deleted_duplicates[0].size_bytes, Some(12345));
        assert_eq!(log.deleted_duplicates[0].md5_hash, Some("abc123hash".to_string()));
        // Undo command should indicate file cannot be restored
        assert!(log.deleted_duplicates[0].undo_command.contains("deleted"));
    }

    #[test]
    fn test_undo_log_add_deleted_small() {
        let base_dir = PathBuf::from("/test");
        let mut log = UndoLog::new(&base_dir, false);

        log.add_deleted_small(Path::new("/test/small.pdf"), 100);

        assert_eq!(log.total_operations, 1);
        assert_eq!(log.deleted_small_files.len(), 1);
        assert_eq!(log.deleted_small_files[0].operation_type, "delete_small");
        assert_eq!(log.deleted_small_files[0].size_bytes, Some(100));
    }

    #[test]
    fn test_undo_log_to_json() {
        let base_dir = PathBuf::from("/test");
        let mut log = UndoLog::new(&base_dir, false);

        log.add_rename(
            Path::new("/test/old.pdf"),
            Path::new("/test/new.pdf"),
        );

        let json = log.to_json().unwrap();

        assert!(json.contains("\"version\": \"1.0\""));
        assert!(json.contains("\"base_directory\": \"/test\""));
        assert!(json.contains("\"total_operations\": 1"));
        assert!(json.contains("\"renames\":"));
        assert!(json.contains("\"operation_type\": \"rename\""));
    }

    #[test]
    fn test_undo_log_generate_undo_script() {
        let base_dir = PathBuf::from("/test");
        let mut log = UndoLog::new(&base_dir, false);

        log.add_rename(
            Path::new("/test/old.pdf"),
            Path::new("/test/new.pdf"),
        );
        log.add_deleted_duplicate(Path::new("/test/dup.pdf"), 1000, None);

        let script = log.generate_undo_script();

        assert!(script.starts_with("#!/bin/bash"));
        assert!(script.contains("set -e"));
        assert!(script.contains("mv"));
        // Deleted files should be noted as non-recoverable
        assert!(script.contains("WARNING"));
        assert!(script.contains("dup.pdf"));
    }

    #[test]
    fn test_undo_log_empty_script() {
        let base_dir = PathBuf::from("/test");
        let log = UndoLog::new(&base_dir, false);

        let script = log.generate_undo_script();

        assert!(script.contains("No rename operations to undo"));
    }

    #[test]
    fn test_undo_log_shell_escape_special_chars() {
        let base_dir = PathBuf::from("/test");
        let mut log = UndoLog::new(&base_dir, false);

        // Test with path containing single quotes (the tricky case)
        log.add_rename(
            Path::new("/test/Book's Title.pdf"),
            Path::new("/test/Author - Book's Title.pdf"),
        );

        let script = log.generate_undo_script();

        // Should properly escape single quotes
        assert!(script.contains("'\\''"));
    }

    #[test]
    fn test_undo_log_write_and_read() {
        use tempfile::TempDir;

        let tmp_dir = TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("undo.json");
        let base_dir = PathBuf::from("/test");
        let mut log = UndoLog::new(&base_dir, false);

        log.add_rename(
            Path::new("/test/a.pdf"),
            Path::new("/test/b.pdf"),
        );

        log.write_to_file(&log_path).unwrap();

        // Verify file exists and contains valid JSON
        let content = std::fs::read_to_string(&log_path).unwrap();
        let parsed: UndoLog = serde_json::from_str(&content).unwrap();

        assert_eq!(parsed.version, "1.0");
        assert_eq!(parsed.total_operations, 1);
        assert_eq!(parsed.renames.len(), 1);
    }

    #[test]
    fn test_undo_log_multiple_operations() {
        let base_dir = PathBuf::from("/test");
        let mut log = UndoLog::new(&base_dir, false);

        log.add_rename(Path::new("/a.pdf"), Path::new("/b.pdf"));
        log.add_rename(Path::new("/c.pdf"), Path::new("/d.pdf"));
        log.add_deleted_duplicate(Path::new("/e.pdf"), 100, None);
        log.add_deleted_small(Path::new("/f.pdf"), 50);

        assert_eq!(log.total_operations, 4);
        assert_eq!(log.renames.len(), 2);
        assert_eq!(log.deleted_duplicates.len(), 1);
        assert_eq!(log.deleted_small_files.len(), 1);
    }
}
