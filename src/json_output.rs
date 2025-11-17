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
