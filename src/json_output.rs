use crate::scanner::FileInfo;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}
