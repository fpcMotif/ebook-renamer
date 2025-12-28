// Integration tests for ebook_renamer main application flow

use std::process::Command;
use std::fs;
use tempfile::TempDir;
use crate::cli::Args;
use crate::json_output::OperationsOutput;
use serde_json;

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_pipeline_dry_run_with_json() -> Result<()> {
        let test_dir = TempDir::new()?;
        
        // Create test files with various scenarios
        create_test_files(&test_dir)?;
        
        // Run with dry-run and JSON
        let output = Command::new(&format!("{}/target/release/ebook_renamer", 
            env!("CARGO_MANIFEST_DIR")))
            .arg("--dry-run")
            .arg("--json")
            .arg("--no-recursive")
            .arg(test_dir.path())
            .current_dir(&env!("CARGO_MANIFEST_DIR"))
            .output()?;
        
        assert!(output.status.success(), "Command should succeed");
        assert!(output.stderr.is_empty(), "Should have no stderr output");
        
        // Parse JSON output
        let json_str = String::from_utf8(output.stdout)?;
        let operations: OperationsOutput = serde_json::from_str(&json_str)?;
        
        // Validate structure
        assert!(!operations.renames.is_empty() || !operations.duplicate_deletes.is_empty() || !operations.small_or_corrupted_deletes.is_empty());
        assert!(operations.todo_items.len() >= 0);
        
        // Verify no actual file changes occurred
        assert!(test_dir.path().join("test.pdf").exists());
        assert!(test_dir.path().join("book.epub").exists());
        
        Ok(())
    }

    #[test]
    fn test_cli_argument_combinations() -> Result<()> {
        let test_dir = TempDir::new()?;
        create_test_files(&test_dir)?;
        
        // Test --no-recursive overrides --max-depth
        let output = Command::new(&format!("{}/target/release/ebook_renamer", 
            env!("CARGO_MANIFEST_DIR")))
            .arg("--dry-run")
            .arg("--json")
            .arg("--no-recursive")
            .arg("--max-depth=10")
            .arg("--verbose")
            .arg(test_dir.path())
            .current_dir(&env!("CARGO_MANIFEST_DIR"))
            .output()?;
        
        assert!(output.status.success());
        
        // Should only scan top-level due to --no-recursive
        let json_str = String::from_utf8(output.stdout)?;
        let operations: OperationsOutput = serde_json::from_str(&json_str)?;
        
        // Count total files mentioned in output
        let mut total_files_mentioned = 0;
        total_files_mentioned += operations.renames.len();
        for group in &operations.duplicate_deletes {
            total_files_mentioned += group.len();
        }
        total_files_mentioned += operations.small_or_corrupted_deletes.len();
        total_files_mentioned += operations.todo_items.len();
        
        // Should only mention top-level files, not nested ones
        assert!(total_files_mentioned <= 2); // Only test.pdf and book.epub from top level
        
        Ok(())
    }

    #[test]
    fn test_error_handling_integration() -> Result<()> {
        let test_dir = TempDir::new()?;
        
        // Create problematic files
        fs::write(test_dir.path().join("small.pdf"), "x")?; // Too small
        fs::write(test_dir.path().join("not-pdf.pdf"), "not pdf content")?; // Invalid PDF
        fs::write(test_dir.path().join("incomplete.download"), "partial")?; // Incomplete download
        
        let output = Command::new(&format!("{}/target/release/ebook_renamer", 
            env!("CARGO_MANIFEST_DIR")))
            .arg("--dry-run")
            .arg("--json")
            .arg("--delete-small")
            .arg(test_dir.path())
            .current_dir(&env!("CARGO_MANIFEST_DIR"))
            .output()?;
        
        assert!(output.status.success());
        
        let json_str = String::from_utf8(output.stdout)?;
        let operations: OperationsOutput = serde_json::from_str(&json_str)?;
        
        // Should have todo items for problematic files
        let categories: std::collections::HashSet<_> = operations.todo_items
            .iter()
            .map(|item| &item.category)
            .collect();
        
        assert!(categories.contains("too_small"));
        assert!(categories.contains("corrupted"));
        assert!(categories.contains("failed_download"));
        
        Ok(())
    }

    #[test]
    fn test_unicode_and_special_characters() -> Result<()> {
        let test_dir = TempDir::new()?;
        
        // Create files with special characters and Unicode
        fs::write(test_dir.path().join("数学入門.pdf"), "content")?;
        fs::write(test_dir.path().join("file with spaces.pdf"), "content")?;
        fs::write(test_dir.path().join("file'with'quotes.pdf"), "content")?;
        
        let output = Command::new(&format!("{}/target/release/ebook_renamer", 
            env!("CARGO_MANIFEST_DIR")))
            .arg("--dry-run")
            .arg("--json")
            .arg("--preserve-unicode")
            .arg(test_dir.path())
            .current_dir(&env!("CARGO_MANIFEST_DIR"))
            .output()?;
        
        assert!(output.status.success());
        
        let json_str = String::from_utf8(output.stdout)?;
        let operations: OperationsOutput = serde_json::from_str(&json_str)?;
        
        // Should process Unicode files without crashing
        assert!(!operations.renames.is_empty());
        
        Ok(())
    }

    #[test]
    fn test_deep_recursion_limits() -> Result<()> {
        let test_dir = TempDir::new()?;
        
        // Create deeply nested directory structure
        let mut current = test_dir.path().to_path_buf();
        for i in 0..10 {
            current = current.join(format!("level_{}", i));
            fs::create_dir_all(&current)?;
            
            let file_path = current.join("test.pdf");
            fs::write(&file_path, "content")?;
        }
        
        // Test with depth limit
        let output = Command::new(&format!("{}/target/release/ebook_renamer", 
            env!("CARGO_MANIFEST_DIR")))
            .arg("--dry-run")
            .arg("--json")
            .arg("--max-depth=3")
            .arg(test_dir.path())
            .current_dir(&env!("CARGO_MANIFEST_DIR"))
            .output()?;
        
        assert!(output.status.success());
        
        let json_str = String::from_utf8(output.stdout)?;
        let operations: OperationsOutput = serde_json::from_str(&json_str)?;
        
        // Should only find files up to depth 3
        let mut total_files_found = 0;
        total_files_found += operations.renames.len();
        for group in &operations.duplicate_deletes {
            total_files_found += group.len();
        }
        total_files_found += operations.small_or_corrupted_deletes.len();
        
        // Should find at most 3 files (level_0, level_1, level_2)
        assert!(total_files_found <= 3);
        
        Ok(())
    }

    #[test]
    fn test_undo_script_generation_security() -> Result<()> {
        let test_dir = TempDir::new()?;
        
        // Create file with dangerous characters in name
        fs::write(test_dir.path().join("safe'file.pdf"), "content")?;
        
        let output = Command::new(&format!("{}/target/release/ebook_renamer", 
            env!("CARGO_MANIFEST_DIR")))
            .arg("--dry-run")
            .arg("--generate-undo-script")
            .arg(test_dir.path())
            .current_dir(&env!("CARGO_MANIFEST_DIR"))
            .output()?;
        
        assert!(output.status.success());
        
        // Check that undo script was created
        let undo_script = test_dir.path().join("undo_rename.sh");
        assert!(undo_script.exists());
        
        let script_content = fs::read_to_string(&undo_script)?;
        
        // Verify proper shell escaping
        assert!(script_content.contains("'safe'\\''file.pdf'"));
        assert!(!script_content.contains("'; rm -rf'"));
        assert!(!script_content.contains("&&"));
        assert!(!script_content.contains("|"));
        
        Ok(())
    }

    #[test]
    fn test_duplicate_detection_integration() -> Result<()> {
        let test_dir = TempDir::new()?;
        
        // Create duplicate files with identical content
        let content = "%PDF-1.4\n%....\n"; // Minimal valid PDF header
        fs::write(test_dir.path().join("book1.pdf"), &content)?;
        fs::write(test_dir.path().join("book2.pdf"), &content)?;
        fs::write(test_dir.path().join("different.pdf"), "different content")?;
        
        let output = Command::new(&format!("{}/target/release/ebook_renamer", 
            env!("CARGO_MANIFEST_DIR")))
            .arg("--dry-run")
            .arg("--json")
            .arg(test_dir.path())
            .current_dir(&env!("CARGO_MANIFEST_DIR"))
            .output()?;
        
        assert!(output.status.success());
        
        let json_str = String::from_utf8(output.stdout)?;
        let operations: OperationsOutput = serde_json::from_str(&json_str)?;
        
        // Should detect duplicates
        assert!(!operations.duplicate_deletes.is_empty());
        assert_eq!(operations.duplicate_deletes[0].len(), 2); // Two duplicate files
        
        Ok(())
    }

    #[test]
    fn test_json_output_structure() -> Result<()> {
        let test_dir = TempDir::new()?;
        create_test_files(&test_dir)?;
        
        let output = Command::new(&format!("{}/target/release/ebook_renamer", 
            env!("CARGO_MANIFEST_DIR")))
            .arg("--dry-run")
            .arg("--json")
            .arg(test_dir.path())
            .current_dir(&env!("CARGO_MANIFEST_DIR"))
            .output()?;
        
        assert!(output.status.success());
        
        let json_str = String::from_utf8(output.stdout)?;
        
        // Validate JSON structure
        serde_json::from_str::<OperationsOutput>(&json_str).map_err(|e| {
            anyhow::anyhow!("Invalid JSON structure: {}", e)
        })?;
        
        Ok(())
    }

    #[test]
    fn test_log_output_with_verbose() -> Result<()> {
        let test_dir = TempDir::new()?;
        create_test_files(&test_dir)?;
        
        let output = Command::new(&format!("{}/target/release/ebook_renamer", 
            env!("CARGO_MANIFEST_DIR")))
            .arg("--dry-run")
            .arg("--verbose")
            .arg(test_dir.path())
            .current_dir(&env!("CARGO_MANIFEST_DIR"))
            .output()?;
        
        assert!(output.status.success());
        
        // With verbose, should have more detailed output
        let stderr_str = String::from_utf8(output.stderr)?;
        assert!(!stderr_str.is_empty());
        assert!(stderr_str.contains("Found") || stderr_str.contains("files"));
        
        Ok(())
    }

    fn create_test_files(test_dir: &TempDir) -> Result<()> {
        // Create various test files for comprehensive testing
        
        // Normal files
        fs::write(test_dir.path().join("test.pdf"), "%PDF-1.4\n%....\n")?; // Valid PDF
        fs::write(test_dir.path().join("book.epub"), "epub content")?;
        
        // Small/corrupted files
        fs::write(test_dir.path().join("tiny.pdf"), "x")?; // Too small
        
        // Failed download
        fs::write(test_dir.path().join("incomplete.download"), "partial")?;
        
        // Nested file (for recursion testing)
        let nested_dir = test_dir.path().join("subdir");
        fs::create_dir(&nested_dir)?;
        fs::write(nested_dir.join("nested.pdf"), "nested content")?;
        
        Ok(())
    }
}