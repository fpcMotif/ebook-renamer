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
                let from_path = file_info
                    .original_path
                    .strip_prefix(target_dir)
                    .unwrap_or(&file_info.original_path)
                    .to_string_lossy()
                    .to_string();
                let to_path = file_info
                    .new_path
                    .strip_prefix(target_dir)
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
                let keep_path = group[0]
                    .strip_prefix(target_dir)
                    .unwrap_or(&group[0])
                    .to_string_lossy()
                    .to_string();
                let mut delete_paths: Vec<String> = group
                    .iter()
                    .skip(1)
                    .map(|p| {
                        p.strip_prefix(target_dir)
                            .unwrap_or(p)
                            .to_string_lossy()
                            .to_string()
                    })
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
            let path_str = path
                .strip_prefix(target_dir)
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
            a.category
                .cmp(&b.category)
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
            cloud_metadata: crate::scanner::CloudMetadata::default(),
        };

        let duplicate_group = vec![target_dir.join("keep.pdf"), target_dir.join("delete.pdf")];

        let files_to_delete = vec![target_dir.join("small.pdf")];

        let todo_items = vec![(
            "Category".to_string(),
            "todo.pdf".to_string(),
            "Check me".to_string(),
        )];

        let output = OperationsOutput::from_results(
            vec![file_info],
            vec![duplicate_group],
            files_to_delete,
            todo_items,
            &target_dir,
        )
        .unwrap();

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
            cloud_metadata: crate::scanner::CloudMetadata::default(),
        };

        let output =
            OperationsOutput::from_results(vec![file_info], vec![], vec![], vec![], &target_dir)
                .unwrap();

        // Paths should be relative to target_dir
        #[cfg(not(windows))]
        assert_eq!(output.renames[0].from, "subdir/file.pdf");
    }
}
